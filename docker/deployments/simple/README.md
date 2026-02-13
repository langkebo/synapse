# Synapse Rust 简单部署指南

适用于 **1CPU 2GB 内存** 的小型服务器部署。

## 📋 目录结构

```
simple/
├── config/
│   └── homeserver.yaml    # Synapse 配置文件
├── nginx/
│   ├── nginx.conf         # Nginx 配置
│   └── .well-known/
│       └── matrix/
│           └── server     # 联邦发现配置
├── ssl/                   # SSL 证书目录 (需自行配置)
├── data/                  # 数据目录 (自动创建)
├── logs/                  # 日志目录 (自动创建)
├── .env.example           # 环境变量示例
├── docker-compose.yml     # Docker Compose 配置
├── deploy.sh              # 一键部署脚本
└── README.md              # 本文件
```

## 🚀 快速部署

### 1. 前置要求

- Ubuntu 20.04+ 或其他 Linux 发行版
- Docker 20.10+
- Docker Compose v2+
- 至少 2GB 内存
- 域名并已解析到服务器

### 2. 安装 Docker

```bash
# 安装 Docker
curl -fsSL https://get.docker.com | sh

# 添加当前用户到 docker 组
sudo usermod -aG docker $USER

# 重新登录或执行
newgrp docker
```

### 3. 部署步骤

```bash
# 1. 上传部署文件夹到服务器
scp -r simple/ user@server:/opt/synapse/

# 2. 进入部署目录
cd /opt/synapse/simple

# 3. 复制环境变量文件
cp .env.example .env

# 4. 编辑环境变量 (重要!)
nano .env

# 5. 创建必要目录
mkdir -p data logs ssl

# 6. 启动服务
docker compose up -d

# 7. 查看日志
docker compose logs -f synapse-rust
```

### 4. 创建管理员账户

```bash
# 使用注册密钥创建管理员
curl -X POST http://localhost:8008/_synapse/admin/v1/register \
  -H "Content-Type: application/json" \
  -d '{
    "username": "admin",
    "password": "your_secure_password",
    "admin": true,
    "mac": "shared_secret"
  }'
```

## ⚙️ 配置说明

### 环境变量 (.env)

| 变量名 | 说明 | 默认值 | 是否必须修改 |
|--------|------|--------|--------------|
| `SERVER_NAME` | 服务器域名 | cjystx.top | ✅ 是 |
| `DB_PASSWORD` | 数据库密码 | synapse | ✅ 是 |
| `SECRET_KEY` | JWT 签名密钥 | - | ✅ 是 |
| `REGISTRATION_SECRET` | 注册共享密钥 | - | ✅ 是 |
| `ADMIN_SECRET` | 管理员注册密钥 | - | ✅ 是 |
| `SIGNING_KEY` | 联邦签名密钥 | - | ✅ 是 |
| `RUST_LOG` | 日志级别 | warn | ❌ 否 |

### 生成密钥

```bash
# 生成随机密钥
openssl rand -hex 32

# 生成联邦签名密钥 (在项目根目录执行)
./target/release/generate_test_keypair
```

### SSL 证书配置

将 SSL 证书文件放入 `ssl/` 目录：

```
ssl/
├── fullchain.pem    # 证书链
└── privkey.pem      # 私钥
```

推荐使用 Let's Encrypt 免费证书：

```bash
# 安装 certbot
sudo apt install certbot

# 获取证书
sudo certbot certonly --standalone -d your-domain.com

# 复制证书
sudo cp /etc/letsencrypt/live/your-domain.com/fullchain.pem ssl/
sudo cp /etc/letsencrypt/live/your-domain.com/privkey.pem ssl/
sudo chown -R $USER:$USER ssl/
```

## 📊 资源使用

针对 2GB 内存服务器的优化配置：

| 服务 | CPU 限制 | 内存限制 | 内存预留 |
|------|----------|----------|----------|
| synapse-rust | 0.5 | 256MB | 64MB |
| postgresql | 0.5 | 512MB | 128MB |
| redis | 0.2 | 64MB | 16MB |
| nginx | 0.2 | 64MB | 16MB |
| **总计** | **1.4** | **896MB** | **224MB** |

## 🔧 常用命令

```bash
# 启动服务
docker compose up -d

# 停止服务
docker compose down

# 重启服务
docker compose restart

# 查看日志
docker compose logs -f synapse-rust

# 查看服务状态
docker compose ps

# 进入容器
docker compose exec synapse-rust sh

# 备份数据库
docker compose exec db pg_dump -U synapse synapse > backup.sql

# 恢复数据库
cat backup.sql | docker compose exec -T db psql -U synapse synapse
```

## 🌐 联邦配置

### DNS 配置

```
# A 记录
your-domain.com    A      服务器IP

# SRV 记录 (可选，用于联邦发现)
_matrix._tcp.your-domain.com    SRV    10 5 8448 your-domain.com.
```

### .well-known 配置

编辑 `nginx/.well-known/matrix/server`：

```json
{
  "m.server": "your-domain.com:8448"
}
```

## 🛡️ 安全建议

1. **修改所有默认密码和密钥**
2. **启用 HTTPS** (通过 Nginx)
3. **配置防火墙**：
   ```bash
   sudo ufw allow 80/tcp
   sudo ufw allow 443/tcp
   sudo ufw allow 8448/tcp
   sudo ufw enable
   ```
4. **定期备份数据库**
5. **关闭公开注册** (生产环境设置 `ENABLE_REGISTRATION=false`)

## 📱 客户端推荐

- **Element Web**: https://app.element.io
- **Element Desktop**: https://element.io/get-started
- **FluffyChat**: https://fluffychat.im
- **Nheko**: https://nheko-reborn.github.io

## 🔗 相关链接

- [Matrix 协议](https://matrix.org)
- [Synapse Rust 项目](https://github.com/langkebo/synapse)
- [Docker Hub 镜像](https://hub.docker.com/r/vmuser232922/synapse-rust)

## ❓ 常见问题

### Q: 服务启动失败？

```bash
# 检查日志
docker compose logs synapse-rust

# 常见原因:
# 1. 数据库未就绪 - 等待几秒后重试
# 2. 配置文件错误 - 检查 homeserver.yaml
# 3. 端口被占用 - 检查 8008/8448 端口
```

### Q: 无法连接联邦？

```bash
# 检查 8448 端口
curl https://your-domain.com:8448/_matrix/federation/v1/version

# 检查 .well-known
curl https://your-domain.com/.well-known/matrix/server
```

### Q: 内存不足？

```bash
# 查看内存使用
docker stats

# 调整 docker-compose.yml 中的内存限制
# 或添加 swap 分区
```

## 📝 更新日志

- **v2.0** - 使用 Docker Hub 官方镜像，优化低配服务器部署
- **v1.0** - 初始版本
