# Synapse Rust 简单部署指南

适用于 **1CPU 2GB 内存** 的小型服务器部署。

## 📋 目录结构

```
simple/
├── config/
│   └── homeserver.yaml    # Synapse 配置文件 (含详细说明)
├── nginx/
│   ├── nginx.conf         # Nginx 反向代理配置
│   └── .well-known/
│       └── matrix/
│           └── server     # 联邦发现配置
├── ssl/                   # SSL 证书目录 (需自行配置)
│   └── README.md          # 证书配置说明
├── data/                  # 数据目录 (自动创建)
├── logs/                  # 日志目录 (自动创建)
│   └── nginx/             # Nginx 日志 (自动创建)
├── .env.example           # 环境变量示例
├── docker-compose.yml     # Docker Compose 配置
├── deploy.sh              # 一键部署脚本
└── README.md              # 本文件
```

---

## �️ 部署环境要求

### 服务器最低配置

| 配置项 | 最低要求 | 推荐配置 |
|--------|----------|----------|
| CPU | 1 核 | 2 核+ |
| 内存 | 2 GB | 4 GB+ |
| 磁盘 | 20 GB SSD | 50 GB+ SSD |
| 带宽 | 1 Mbps | 10 Mbps+ |

### 支持的操作系统

| 系统 | 版本 | 架构 |
|------|------|------|
| Ubuntu | 20.04 LTS+ | x86_64, ARM64 |
| Debian | 11+ | x86_64, ARM64 |
| CentOS | 8+ | x86_64, ARM64 |
| Rocky Linux | 8+ | x86_64, ARM64 |

### 软件依赖

| 软件 | 最低版本 | 检查命令 |
|------|----------|----------|
| Docker | 20.10+ | `docker --version` |
| Docker Compose | v2.0+ | `docker compose version` |
| curl | 任意 | `curl --version` |
| openssl | 任意 | `openssl version` |

### 网络要求

| 端口 | 协议 | 用途 | 是否必须 |
|------|------|------|----------|
| 80 | TCP | HTTP (证书验证) | 是 |
| 443 | TCP | HTTPS 客户端 API | 是 |
| 8448 | TCP | Matrix 联邦通信 | 是 (联邦功能) |

### 域名要求

- **必须**: 一个已解析到服务器的域名
- **推荐**: 配置 `matrix.` 子域名用于联邦
- **DNS 记录**:
  ```
  # A 记录
  your-domain.com      A      服务器IP
  matrix.your-domain.com  A   服务器IP
  
  # 可选: SRV 记录 (联邦发现)
  _matrix._tcp.your-domain.com  SRV  10 5 8448 matrix.your-domain.com.
  ```

---

## 🚀 快速部署

### 方式一: 使用一键部署脚本 (推荐)

```bash
# 1. 上传部署文件夹到服务器
scp -r simple/ user@server:/opt/synapse/

# 2. 进入部署目录
cd /opt/synapse/simple

# 3. 运行部署脚本
chmod +x deploy.sh
./deploy.sh
```

### 方式二: 手动部署

#### 1. 安装 Docker

```bash
# 安装 Docker
curl -fsSL https://get.docker.com | sh

# 添加当前用户到 docker 组
sudo usermod -aG docker $USER

# 重新登录或执行
newgrp docker

# 验证安装
docker --version
docker compose version
```

#### 2. 上传并配置

```bash
# 上传部署文件夹
scp -r simple/ user@server:/opt/synapse/

# 进入部署目录
cd /opt/synapse/simple

# 复制环境变量文件
cp .env.example .env

# 编辑环境变量 (重要!)
nano .env
```

#### 3. 修改必要配置

编辑 `.env` 文件，修改以下配置：

```bash
# 必须修改的配置
SERVER_NAME=your-domain.com           # 你的域名
DB_PASSWORD=your_secure_password      # 数据库密码
SECRET_KEY=generated_by_openssl       # JWT 密钥
REGISTRATION_SECRET=generated_by_openssl  # 注册密钥
ADMIN_SECRET=generated_by_openssl     # 管理员密钥
SIGNING_KEY=generated_by_keypair      # 联邦签名密钥
```

生成密钥：

```bash
# 生成随机密钥
openssl rand -hex 32  # 用于 SECRET_KEY
openssl rand -hex 16  # 用于 REGISTRATION_SECRET, ADMIN_SECRET

# 生成联邦签名密钥 (需要项目二进制)
# 或使用默认测试密钥 (仅开发环境)
```

#### 4. 配置 SSL 证书

```bash
# 创建 SSL 目录
mkdir -p ssl

# 使用 Let's Encrypt 获取证书
sudo apt install certbot
sudo certbot certonly --standalone -d your-domain.com -d matrix.your-domain.com

# 复制证书
sudo cp /etc/letsencrypt/live/your-domain.com/fullchain.pem ssl/
sudo cp /etc/letsencrypt/live/your-domain.com/privkey.pem ssl/
sudo chown -R $USER:$USER ssl/
```

#### 5. 启动服务

```bash
# 创建必要目录
mkdir -p data logs logs/nginx

# 拉取镜像
docker compose pull

# 启动服务
docker compose up -d

# 查看日志
docker compose logs -f synapse-rust
```

---

## ✅ 验证部署

### 1. 检查服务状态

```bash
# 查看容器状态
docker compose ps

# 预期输出: 所有服务状态为 "healthy" 或 "running"
```

### 2. 测试客户端 API

```bash
# 本地测试
curl http://localhost:8008/_matrix/client/versions

# 预期输出: {"versions":["v1.11","v1.12",...]}
```

### 3. 测试联邦 API

```bash
# 本地测试
curl http://localhost:8008/_matrix/federation/v1/version

# 远程测试 (需要 SSL)
curl https://matrix.your-domain.com/_matrix/federation/v1/version
```

### 4. 测试服务发现

```bash
# 测试 .well-known
curl https://your-domain.com/.well-known/matrix/server

# 预期输出: {"m.server":"matrix.your-domain.com:443"}
```

---

## 👤 创建管理员账户

### 方式一: 使用 API

```bash
# 获取 nonce
NONCE=$(curl -s http://localhost:8008/_synapse/admin/v1/register | jq -r '.nonce')

# 计算 MAC (需要 hmac)
# 注意: 需要安装 jq 和 openssl

# 或使用管理员工具
```

### 方式二: 直接注册 (开发环境)

如果开启了公开注册 (`ENABLE_REGISTRATION=true`)：

```bash
curl -X POST http://localhost:8008/_matrix/client/v3/register \
  -H "Content-Type: application/json" \
  -d '{
    "username": "admin",
    "password": "your_secure_password",
    "device_id": "ADMIN_DEVICE"
  }'
```

---

## ⚙️ 配置说明

### 环境变量 (.env)

| 变量名 | 说明 | 默认值 | 是否必须修改 |
|--------|------|--------|--------------|
| `SERVER_NAME` | 服务器域名 | cjystx.top | ✅ 是 |
| `ADMIN_EMAIL` | 管理员邮箱 | admin@cjystx.top | ✅ 是 |
| `DB_PASSWORD` | 数据库密码 | synapse | ✅ 是 |
| `SECRET_KEY` | JWT 签名密钥 | - | ✅ 是 |
| `REGISTRATION_SECRET` | 注册共享密钥 | - | ✅ 是 |
| `ADMIN_SECRET` | 管理员注册密钥 | - | ✅ 是 |
| `SIGNING_KEY` | 联邦签名密钥 | - | ✅ 是 |
| `KEY_ID` | 签名密钥 ID | ed25519:testkb1OUw | ❌ 否 |
| `RUST_LOG` | 日志级别 | warn | ❌ 否 |
| `ENABLE_REGISTRATION` | 是否允许公开注册 | false | ❌ 否 |

### Docker 镜像信息

```
镜像: vmuser232922/synapse-rust:2.0
大小: 61.9 MB
基础镜像: gcr.io/distroless/cc-debian12
架构: ARM64 / x86_64
```

---

## 📊 资源使用

针对 2GB 内存服务器的优化配置：

| 服务 | CPU 限制 | 内存限制 | 内存预留 | 磁盘使用 |
|------|----------|----------|----------|----------|
| synapse-rust | 0.5 核 | 256 MB | 64 MB | ~100 MB |
| postgresql | 0.5 核 | 512 MB | 128 MB | 数据增长 |
| redis | 0.2 核 | 64 MB | 16 MB | ~50 MB |
| nginx | 0.2 核 | 64 MB | 16 MB | ~10 MB |
| **总计** | **1.4 核** | **896 MB** | **224 MB** | - |

### 内存优化说明

- PostgreSQL: 使用 `shared_buffers=128MB` 优化
- Redis: 限制最大内存 48MB，使用 LRU 淘汰策略
- Nginx: 使用 Alpine 镜像，最小化内存占用
- Synapse Rust: 使用 distroless 基础镜像，仅 61.9MB

---

## 🔧 常用命令

### 服务管理

```bash
# 启动服务
docker compose up -d

# 停止服务
docker compose down

# 重启服务
docker compose restart

# 重启单个服务
docker compose restart synapse-rust

# 查看服务状态
docker compose ps

# 查看资源使用
docker stats
```

### 日志查看

```bash
# 查看所有日志
docker compose logs

# 实时查看 Synapse 日志
docker compose logs -f synapse-rust

# 查看最近 100 行日志
docker compose logs --tail=100 synapse-rust

# 查看 Nginx 日志
docker compose logs -f nginx
```

### 数据备份

```bash
# 备份数据库
docker compose exec db pg_dump -U synapse synapse > backup_$(date +%Y%m%d).sql

# 备份 Redis
docker compose exec redis redis-cli BGSAVE
docker cp synapse-redis:/data/dump.rdb redis_backup_$(date +%Y%m%d).rdb

# 恢复数据库
cat backup.sql | docker compose exec -T db psql -U synapse synapse
```

### 进入容器

```bash
# 进入 Synapse 容器 (distroless 无 shell)
# 不支持进入容器

# 进入数据库容器
docker compose exec db bash

# 进入 Redis 容器
docker compose exec redis sh
```

---

## 🌐 联邦配置

### DNS 配置示例

```
# A 记录
your-domain.com           A      服务器IP
matrix.your-domain.com    A      服务器IP

# SRV 记录 (可选，用于联邦发现)
_matrix._tcp.your-domain.com    SRV    10 5 8448 matrix.your-domain.com.
```

### .well-known 配置

`nginx/.well-known/matrix/server` 文件内容：

```json
{
  "m.server": "matrix.your-domain.com:443"
}
```

### 联邦测试

```bash
# 测试联邦连接
curl https://matrix.your-domain.com/_matrix/federation/v1/version

# 测试服务发现
curl https://your-domain.com/.well-known/matrix/server

# 使用 Matrix Federation Tester
# 访问: https://federationtester.matrix.org/
```

---

## 🛡️ 安全建议

### 1. 修改所有默认密码

```bash
# 生成强密码
openssl rand -hex 32

# 修改 .env 文件中的所有密钥
```

### 2. 配置防火墙

```bash
# Ubuntu/Debian
sudo ufw allow 22/tcp    # SSH
sudo ufw allow 80/tcp    # HTTP
sudo ufw allow 443/tcp   # HTTPS
sudo ufw allow 8448/tcp  # Federation
sudo ufw enable

# 查看状态
sudo ufw status
```

### 3. 启用 HTTPS

- 使用 Let's Encrypt 免费证书
- 配置自动续期

```bash
# 设置自动续期
sudo crontab -e
# 添加:
0 0,12 * * * certbot renew --quiet && docker compose restart nginx
```

### 4. 关闭公开注册

生产环境设置 `ENABLE_REGISTRATION=false`，通过管理员创建账户。

### 5. 定期备份

```bash
# 添加备份 cron 任务
crontab -e
# 每天凌晨 3 点备份
0 3 * * * /opt/synapse/simple/backup.sh
```

---

## 📱 客户端推荐

### Web 客户端

- **Element Web**: https://app.element.io (官方)
- **Cinny**: https://cinny.in (轻量级)

### 桌面客户端

- **Element Desktop**: https://element.io/get-started
- **Nheko**: https://nheko-reborn.github.io

### 移动客户端

- **Element (iOS/Android)**: 各应用商店搜索
- **FluffyChat**: https://fluffychat.im

---

## ❓ 常见问题

### Q: 服务启动失败？

```bash
# 检查日志
docker compose logs synapse-rust

# 常见原因:
# 1. 数据库未就绪 - 等待几秒后重试
# 2. 配置文件错误 - 检查 homeserver.yaml 语法
# 3. 端口被占用 - 检查 8008/8448 端口
# 4. 内存不足 - 检查 docker stats
```

### Q: 无法连接联邦？

```bash
# 检查 8448 端口
curl https://your-domain.com:8448/_matrix/federation/v1/version

# 检查 .well-known
curl https://your-domain.com/.well-known/matrix/server

# 检查防火墙
sudo ufw status

# 使用 Federation Tester
# https://federationtester.matrix.org/
```

### Q: SSL 证书错误？

```bash
# 检查证书文件
ls -la ssl/

# 续期证书
sudo certbot renew

# 重启 Nginx
docker compose restart nginx
```

### Q: 内存不足？

```bash
# 查看内存使用
docker stats
free -h

# 解决方案:
# 1. 调整 docker-compose.yml 中的内存限制
# 2. 添加 swap 分区
sudo fallocate -l 2G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile
```

### Q: 数据库连接失败？

```bash
# 检查数据库状态
docker compose exec db pg_isready -U synapse

# 检查数据库日志
docker compose logs db

# 重启数据库
docker compose restart db
```

---

## � 相关链接

- [Matrix 协议官网](https://matrix.org)
- [Matrix 规范文档](https://spec.matrix.org)
- [Synapse Rust 项目](https://github.com/langkebo/synapse)
- [Docker Hub 镜像](https://hub.docker.com/r/vmuser232922/synapse-rust)
- [Element 客户端](https://element.io)
- [Federation Tester](https://federationtester.matrix.org/)

---

## �📝 更新日志

| 版本 | 日期 | 更新内容 |
|------|------|----------|
| v2.0 | 2026-02-13 | 使用 Docker Hub 官方镜像，优化低配服务器部署 |
| v1.0 | 2026-02-06 | 初始版本 |

---

## 📄 许可证

本项目采用 Apache 2.0 许可证。
