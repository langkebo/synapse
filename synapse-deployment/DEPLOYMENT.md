# Synapse Matrix 服务器部署指南

本文档提供了在 Ubuntu 服务器上部署 Synapse Matrix 服务器的完整指南，包含好友功能和监控系统。

## 目录

- [系统要求](#系统要求)
- [域名配置](#域名配置)
- [服务器准备](#服务器准备)
- [部署步骤](#部署步骤)
- [配置说明](#配置说明)
- [服务管理](#服务管理)
- [监控和维护](#监控和维护)
- [故障排除](#故障排除)
- [GitHub 文件清单](#github-文件清单)

## 系统要求

### 硬件要求
- **CPU**: 2核心或以上
- **内存**: 2GB RAM 或以上（推荐 4GB+）
- **存储**: 20GB 可用空间或以上
- **网络**: 稳定的互联网连接

### 软件要求
- **操作系统**: Ubuntu 20.04 LTS 或更高版本
- **Docker**: 20.10 或更高版本
- **Docker Compose**: 1.29 或更高版本
- **Git**: 用于克隆代码仓库

## 域名配置

本部署使用以下域名配置：
- **主域名**: `cjystx.top`
- **Matrix 服务器**: `matrix.cjystx.top`
- **监控服务**: `monitoring.cjystx.top`

### DNS 记录配置

在您的 DNS 提供商处添加以下记录：

```
# A 记录
cjystx.top                A    YOUR_SERVER_IP
matrix.cjystx.top         A    YOUR_SERVER_IP
monitoring.cjystx.top     A    YOUR_SERVER_IP

# CNAME 记录（可选）
www.cjystx.top           CNAME cjystx.top
```

## 服务器准备

### 1. 更新系统

```bash
sudo apt update && sudo apt upgrade -y
```

### 2. 安装必要软件

```bash
# 安装基础工具
sudo apt install -y curl wget git unzip

# 安装 Docker
curl -fsSL https://get.docker.com -o get-docker.sh
sudo sh get-docker.sh
sudo usermod -aG docker $USER

# 安装 Docker Compose
sudo curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
sudo chmod +x /usr/local/bin/docker-compose

# 重新登录以应用 Docker 组权限
newgrp docker
```

### 3. 配置防火墙

```bash
# 启用 UFW
sudo ufw enable

# 允许必要端口
sudo ufw allow 22/tcp      # SSH
sudo ufw allow 80/tcp      # HTTP
sudo ufw allow 443/tcp     # HTTPS
sudo ufw allow 8448/tcp    # Matrix Federation

# 检查状态
sudo ufw status
```

## 部署步骤

### 1. 克隆代码仓库

```bash
# 克隆项目（包含部署目录）
git clone https://github.com/langkebo/synapse.git
cd synapse

# 进入部署目录
cd synapse-deployment
```

### 2. 配置环境变量（可选）

如果需要自定义配置，可以手动创建 `.env` 文件：

```bash
cp .env.example .env
nano .env
```

**注意**: 如果不创建 `.env` 文件，部署脚本会自动生成包含随机密码的配置。

### 3. 执行一键部署

```bash
# 给脚本执行权限
chmod +x scripts/start.sh

# 执行部署
./scripts/start.sh
```

部署脚本会自动：
- 检查系统依赖
- 生成 `.env` 文件（如果不存在）
- 创建必要目录
- 生成 SSL 证书（测试用）
- 构建 Docker 镜像
- 启动所有服务
- 检查服务状态

### 4. 验证部署

部署完成后，检查以下服务：

```bash
# 检查容器状态
docker-compose ps

# 检查 Synapse 健康状态
curl http://localhost:8008/health

# 检查服务日志
docker-compose logs synapse
```

## 配置说明

### 主要配置文件

- **`.env`**: 环境变量配置
- **`docker-compose.yml`**: Docker 服务编排
- **`nginx/nginx.conf`**: Nginx 反向代理配置
- **`synapse/homeserver.yaml`**: Synapse 主配置文件

### 重要配置项

#### 域名配置
```env
SYNAPSE_SERVER_NAME=cjystx.top
SYNAPSE_PUBLIC_BASEURL=https://matrix.cjystx.top
```

#### 数据库配置
```env
POSTGRES_DB=synapse
POSTGRES_USER=synapse
POSTGRES_PASSWORD=<自动生成的32位密码>
```

#### SSL 配置
```env
LETSENCRYPT_DOMAIN=cjystx.top,matrix.cjystx.top,monitoring.cjystx.top
LETSENCRYPT_EMAIL=admin@cjystx.top
```

## 服务管理

### 常用命令

```bash
# 查看服务状态
docker-compose ps

# 查看服务日志
docker-compose logs -f [service_name]

# 重启服务
docker-compose restart [service_name]

# 停止所有服务
docker-compose down

# 启动所有服务
docker-compose up -d

# 更新服务
docker-compose pull
docker-compose up -d
```

### 服务端口

- **Synapse API**: 8008 (内部)
- **Synapse Federation**: 8448
- **Nginx**: 80, 443
- **PostgreSQL**: 5432 (内部)
- **Redis**: 6379 (内部)
- **Grafana**: 3000
- **Prometheus**: 9091

## 监控和维护

### 访问监控界面

- **Grafana**: `http://monitoring.cjystx.top:3000`
  - 用户名: `admin`
  - 密码: 查看 `.env` 文件中的 `GRAFANA_ADMIN_PASSWORD`

- **Prometheus**: `http://monitoring.cjystx.top:9091`

### 备份策略

```bash
# 备份数据库
docker-compose exec postgres pg_dump -U synapse synapse > backup_$(date +%Y%m%d).sql

# 备份配置文件
tar -czf config_backup_$(date +%Y%m%d).tar.gz .env docker-compose.yml nginx/ synapse/

# 备份媒体文件
tar -czf media_backup_$(date +%Y%m%d).tar.gz data/synapse/media/
```

### 日志管理

```bash
# 查看实时日志
docker-compose logs -f synapse

# 清理日志
docker system prune -f

# 限制日志大小（在 docker-compose.yml 中配置）
logging:
  driver: "json-file"
  options:
    max-size: "10m"
    max-file: "3"
```

## 故障排除

### 常见问题

#### 1. 服务启动失败

```bash
# 检查容器状态
docker-compose ps

# 查看错误日志
docker-compose logs [service_name]

# 重新构建镜像
docker-compose build --no-cache
```

#### 2. 数据库连接失败

```bash
# 检查数据库状态
docker-compose exec postgres pg_isready -U synapse

# 重启数据库
docker-compose restart postgres
```

#### 3. SSL 证书问题

```bash
# 重新生成证书
rm -rf ssl/*
./scripts/start.sh

# 或使用 Let's Encrypt（生产环境）
docker run --rm -v $(pwd)/ssl:/etc/letsencrypt certbot/certbot certonly --standalone -d cjystx.top -d matrix.cjystx.top
```

#### 4. 端口冲突

```bash
# 检查端口占用
sudo netstat -tulpn | grep :80
sudo netstat -tulpn | grep :443

# 停止冲突服务
sudo systemctl stop apache2  # 如果安装了 Apache
sudo systemctl stop nginx    # 如果安装了系统 Nginx
```

### 性能优化

#### 1. 数据库优化

在 `docker-compose.yml` 中调整 PostgreSQL 配置：

```yaml
postgres:
  command: >
    postgres
    -c shared_preload_libraries=pg_stat_statements
    -c max_connections=200
    -c shared_buffers=256MB
    -c effective_cache_size=1GB
```

#### 2. Synapse 优化

在 `synapse/homeserver.yaml` 中调整：

```yaml
# 增加工作进程
worker_app: synapse.app.generic_worker
worker_listeners:
  - type: http
    port: 8008
    resources:
      - names: [client, federation]

# 缓存配置
caches:
  global_factor: 2.0
  per_cache_factors:
    get_users_who_share_room_with_user: 5.0
```

## GitHub 文件清单

以下是需要上传到 GitHub 仓库 `https://github.com/langkebo/synapse` 的文件清单：

### 必需文件

```
synapse-deployment/
├── README.md                     # 项目说明
├── DEPLOYMENT.md                 # 部署文档（本文件）
├── docker-compose.yml            # Docker 编排文件
├── Dockerfile                    # Synapse 镜像构建文件
├── .env.example                  # 环境变量模板
├── .gitignore                    # Git 忽略文件
│
├── scripts/
│   └── start.sh                  # 一键部署脚本
│
├── nginx/
│   ├── nginx.conf                # Nginx 主配置
│   └── conf.d/
│       └── default.conf          # 默认站点配置
│
├── synapse/
│   ├── homeserver.yaml           # Synapse 主配置
│   ├── log.config                # 日志配置
│   └── conf.d/
│       ├── server.yaml           # 服务器配置
│       ├── database.yaml         # 数据库配置
│       ├── registration.yaml     # 注册配置
│       ├── media.yaml            # 媒体配置
│       └── friends.yaml          # 好友功能配置
│
├── monitoring/
│   ├── prometheus.yml            # Prometheus 配置
│   └── grafana/
│       ├── provisioning/
│       │   ├── dashboards/
│       │   │   └── dashboard.yml
│       │   └── datasources/
│       │       └── datasource.yml
│       └── dashboards/
│           └── synapse.json      # Synapse 监控面板
│
└── docs/
    ├── API.md                    # API 文档
    ├── CONFIGURATION.md          # 配置说明
    └── TROUBLESHOOTING.md        # 故障排除
```

### 不应上传的文件

```
# 运行时生成的文件和目录
.env                              # 包含敏感信息
data/                             # 数据目录
logs/                             # 日志目录
ssl/                              # SSL 证书
backups/                          # 备份文件

# Docker 相关
.docker/                          # Docker 缓存

# 临时文件
*.log
*.tmp
*.pid
```

### .gitignore 文件内容

```gitignore
# 环境变量文件
.env
.env.local
.env.production

# 数据目录
data/
logs/
backups/

# SSL 证书
ssl/

# Docker 相关
.docker/

# 临时文件
*.log
*.tmp
*.pid
*.swp
*.swo
*~

# 系统文件
.DS_Store
Thumbs.db

# IDE 文件
.vscode/
.idea/
*.sublime-*

# 备份文件
*.bak
*.backup
*.sql
```

## 安全建议

1. **定期更新**: 保持系统和 Docker 镜像更新
2. **强密码**: 使用复杂密码和密钥
3. **防火墙**: 只开放必要端口
4. **备份**: 定期备份数据和配置
5. **监控**: 监控系统资源和服务状态
6. **SSL**: 生产环境使用有效的 SSL 证书

## 支持和反馈

如果在部署过程中遇到问题，请：

1. 查看本文档的故障排除部分
2. 检查服务日志：`docker-compose logs [service_name]`
3. 在 GitHub 仓库提交 Issue：`https://github.com/langkebo/synapse/issues`

---

**版本**: 1.0.0  
**更新日期**: 2024年12月  
**维护者**: langkebo