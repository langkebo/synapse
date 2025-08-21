# Synapse2 Ubuntu 服务器部署指南

本指南提供了在 Ubuntu 服务器上部署 Synapse2 Matrix 服务器的完整说明，特别针对 1核2GB 的低配置服务器进行了优化。

## 目录

- [系统要求](#系统要求)
- [快速开始](#快速开始)
- [详细部署步骤](#详细部署步骤)
- [配置说明](#配置说明)
- [监控和维护](#监控和维护)
- [故障排除](#故障排除)
- [性能优化](#性能优化)

## 系统要求

### 最低配置
- **CPU**: 1核心
- **内存**: 2GB RAM
- **存储**: 20GB 可用空间
- **操作系统**: Ubuntu 20.04 LTS 或更高版本
- **网络**: 稳定的互联网连接

### 推荐配置
- **CPU**: 2核心
- **内存**: 4GB RAM
- **存储**: 50GB SSD
- **操作系统**: Ubuntu 22.04 LTS

### 软件依赖
- Docker 20.10+
- Docker Compose 2.0+
- Git
- Nginx (可选，用于反向代理)

## 快速开始

### 1. 克隆项目

```bash
git clone https://github.com/matrix-org/synapse.git synapse2
cd synapse2
```

### 2. 使用 Docker Compose 部署

```bash
# 复制低配置优化的 Docker Compose 文件
cp docker/docker-compose.low-spec.yml docker-compose.yml

# 创建环境变量文件
cp .env.example .env

# 编辑环境变量
nano .env
```

### 3. 配置环境变量

在 `.env` 文件中设置以下变量：

```bash
# 服务器配置
SERVER_NAME=your-domain.com
SYNAPSE_CONFIG_DIR=./data
SYNAPSE_DATA_DIR=./data

# 数据库配置
POSTGRES_DB=synapse
POSTGRES_USER=synapse_user
POSTGRES_PASSWORD=your_secure_password
POSTGRES_HOST=postgres
POSTGRES_PORT=5432

# Redis 配置
REDIS_HOST=redis
REDIS_PORT=6379
REDIS_PASSWORD=your_redis_password

# 性能配置
SYNAPSE_WORKERS=1
SYNAPSE_CACHE_FACTOR=0.5
SYNAPSE_EVENT_CACHE_SIZE=5K

# 监控配置
MONITOR_ENABLED=true
MONITOR_INTERVAL=60
ALERT_THRESHOLD_CPU=80
ALERT_THRESHOLD_MEMORY=85
```

### 4. 启动服务

```bash
# 构建并启动所有服务
docker-compose up -d

# 查看服务状态
docker-compose ps

# 查看日志
docker-compose logs -f synapse
```

### 5. 创建管理员用户

```bash
# 进入 Synapse 容器
docker-compose exec synapse bash

# 创建管理员用户
register_new_matrix_user -c /data/homeserver.yaml http://localhost:8008
```

## 详细部署步骤

### 步骤 1: 系统准备

#### 更新系统

```bash
sudo apt update && sudo apt upgrade -y
```

#### 安装 Docker

```bash
# 安装 Docker
curl -fsSL https://get.docker.com -o get-docker.sh
sudo sh get-docker.sh

# 添加用户到 docker 组
sudo usermod -aG docker $USER

# 重新登录或运行
newgrp docker
```

#### 安装 Docker Compose

```bash
sudo curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
sudo chmod +x /usr/local/bin/docker-compose
```

### 步骤 2: 项目配置

#### 创建项目目录

```bash
mkdir -p /opt/synapse2
cd /opt/synapse2
```

#### 下载项目文件

```bash
# 克隆项目
git clone https://github.com/matrix-org/synapse.git .

# 切换到稳定分支
git checkout main
```

#### 配置文件准备

```bash
# 创建数据目录
mkdir -p data logs

# 复制配置文件
cp contrib/docker/conf/homeserver-performance.yaml data/homeserver.yaml

# 设置权限
sudo chown -R 991:991 data logs
```

### 步骤 3: 服务配置

#### 数据库配置

编辑 `data/homeserver.yaml`，确保数据库配置正确：

```yaml
database:
  name: psycopg2
  args:
    user: synapse_user
    password: your_secure_password
    database: synapse
    host: postgres
    port: 5432
    cp_min: 2
    cp_max: 5
    cp_reconnect: true
```

#### Redis 配置

```yaml
redis:
  enabled: true
  host: redis
  port: 6379
  password: your_redis_password
```

#### 性能优化配置

```yaml
performance:
  database:
    connection_pool:
      min_connections: 2
      max_connections: 5
      connection_timeout: 30
  
  memory:
    cache_factor: 0.5
    event_cache_size: "5K"
    gc_thresholds: [700, 10, 10]
  
  concurrency:
    worker_processes: 1
    max_concurrent_requests: 50
```

### 步骤 4: 启动和验证

#### 启动服务

```bash
# 启动所有服务
docker-compose -f docker/docker-compose.low-spec.yml up -d

# 等待服务启动
sleep 30

# 检查服务状态
docker-compose ps
```

#### 健康检查

```bash
# 检查 Synapse 健康状态
curl http://localhost:8008/health

# 检查数据库连接
docker-compose exec postgres psql -U synapse_user -d synapse -c "SELECT version();"

# 检查 Redis 连接
docker-compose exec redis redis-cli ping
```

#### 创建用户

```bash
# 创建管理员用户
docker-compose exec synapse register_new_matrix_user -c /data/homeserver.yaml -a http://localhost:8008
```

## 配置说明

### 主要配置文件

| 文件 | 用途 | 位置 |
|------|------|------|
| `homeserver.yaml` | Synapse 主配置 | `data/homeserver.yaml` |
| `docker-compose.yml` | Docker 服务编排 | `docker/docker-compose.low-spec.yml` |
| `.env` | 环境变量 | `.env` |
| `nginx.conf` | Nginx 配置 | `docker/nginx/nginx.conf` |

### 性能配置参数

#### 内存优化

```yaml
performance:
  memory:
    cache_factor: 0.5          # 缓存因子，降低内存使用
    event_cache_size: "5K"     # 事件缓存大小
    gc_thresholds: [700, 10, 10]  # 垃圾回收阈值
    max_memory_usage: 1536     # 最大内存使用 (MB)
```

#### 数据库优化

```yaml
performance:
  database:
    connection_pool:
      min_connections: 2       # 最小连接数
      max_connections: 5       # 最大连接数
      connection_timeout: 30   # 连接超时
    query_optimization:
      statement_timeout: 30000 # 语句超时 (ms)
      batch_size: 100         # 批处理大小
```

#### 缓存策略

```yaml
cache_strategy:
  redis:
    enabled: true
    host: redis
    port: 6379
    max_memory: 256mb         # Redis 最大内存
    eviction_policy: allkeys-lru
  
  memory_cache:
    enabled: true
    max_size: 128mb           # 内存缓存最大大小
    ttl: 3600                 # 缓存 TTL (秒)
```

### 好友功能配置

```yaml
friends:
  enabled: true
  max_friends_per_user: 500
  friend_request_timeout: 604800  # 7天
  
  cache:
    friends_list_ttl: 3600
    friend_requests_ttl: 1800
    online_status_ttl: 300
    recommendations_ttl: 7200
  
  rate_limits:
    send_request: "10/hour"
    accept_request: "20/hour"
    search_users: "30/minute"
```

## 监控和维护

### 系统监控

#### 启动监控服务

```bash
# 监控服务已包含在 docker-compose 中
docker-compose logs -f monitor
```

#### 查看监控指标

```bash
# 访问 Prometheus 指标
curl http://localhost:9090/metrics

# 查看系统资源
docker-compose exec monitor python /scripts/system_monitor.py --stats
```

#### 健康检查脚本

```bash
# 运行健康检查
docker-compose exec monitor python /scripts/monitor/health_check.py

# 查看详细报告
docker-compose exec monitor python /scripts/monitor/health_check.py --output text
```

### 日志管理

#### 查看日志

```bash
# Synapse 日志
docker-compose logs -f synapse

# 数据库日志
docker-compose logs -f postgres

# Redis 日志
docker-compose logs -f redis

# 监控日志
docker-compose logs -f monitor
```

#### 日志轮转

```bash
# 配置 logrotate
sudo nano /etc/logrotate.d/synapse
```

```
/opt/synapse2/logs/*.log {
    daily
    missingok
    rotate 7
    compress
    delaycompress
    notifempty
    copytruncate
}
```

### 备份策略

#### 数据库备份

```bash
#!/bin/bash
# backup.sh

BACKUP_DIR="/opt/synapse2/backups"
DATE=$(date +%Y%m%d_%H%M%S)

# 创建备份目录
mkdir -p $BACKUP_DIR

# 备份数据库
docker-compose exec -T postgres pg_dump -U synapse_user synapse > $BACKUP_DIR/synapse_$DATE.sql

# 压缩备份
gzip $BACKUP_DIR/synapse_$DATE.sql

# 清理旧备份 (保留7天)
find $BACKUP_DIR -name "*.sql.gz" -mtime +7 -delete

echo "备份完成: synapse_$DATE.sql.gz"
```

#### 配置文件备份

```bash
# 备份配置文件
tar -czf /opt/synapse2/backups/config_$(date +%Y%m%d).tar.gz data/*.yaml docker/*.yml .env
```

### 更新和升级

#### 更新 Synapse

```bash
# 停止服务
docker-compose down

# 备份数据
./backup.sh

# 拉取最新镜像
docker-compose pull

# 启动服务
docker-compose up -d

# 检查服务状态
docker-compose ps
```

#### 数据库迁移

```bash
# 运行数据库迁移
docker-compose exec synapse python -m synapse.app.homeserver --config-path /data/homeserver.yaml --generate-config --report-stats=no
```

## 故障排除

### 常见问题

#### 1. 服务启动失败

**症状**: 容器无法启动或立即退出

**解决方案**:
```bash
# 查看详细日志
docker-compose logs synapse

# 检查配置文件
docker-compose exec synapse python -m synapse.config.homeserver --config-path /data/homeserver.yaml --generate-config --report-stats=no

# 检查权限
sudo chown -R 991:991 data logs
```

#### 2. 数据库连接失败

**症状**: 无法连接到 PostgreSQL 数据库

**解决方案**:
```bash
# 检查数据库状态
docker-compose exec postgres pg_isready -U synapse_user

# 检查网络连接
docker-compose exec synapse ping postgres

# 重置数据库密码
docker-compose exec postgres psql -U postgres -c "ALTER USER synapse_user PASSWORD 'new_password';"
```

#### 3. 内存不足

**症状**: 服务频繁重启或响应缓慢

**解决方案**:
```bash
# 检查内存使用
free -h
docker stats

# 调整缓存配置
# 编辑 data/homeserver.yaml
caches:
  global_factor: 0.3  # 降低缓存因子
  event_cache_size: 3K
```

#### 4. 磁盘空间不足

**症状**: 写入失败或服务异常

**解决方案**:
```bash
# 检查磁盘使用
df -h

# 清理 Docker 镜像
docker system prune -a

# 清理日志
docker-compose exec synapse find /data -name "*.log*" -mtime +7 -delete
```

### 性能调优

#### CPU 优化

```yaml
# homeserver.yaml
performance:
  concurrency:
    worker_processes: 1      # 单核服务器使用1个进程
    max_concurrent_requests: 30  # 降低并发请求数
    request_timeout: 60      # 增加请求超时
```

#### 内存优化

```yaml
performance:
  memory:
    cache_factor: 0.3        # 进一步降低缓存因子
    event_cache_size: "3K"   # 减少事件缓存
    gc_thresholds: [500, 5, 5]  # 更激进的垃圾回收
```

#### 网络优化

```yaml
performance:
  network:
    max_upload_size: "10M"   # 限制上传大小
    max_image_pixels: 32000000  # 限制图片像素
    federation_timeout: 30   # 联邦超时
```

### 监控告警

#### 设置告警阈值

```yaml
# monitor.yaml
alerts:
  cpu:
    warning: 70
    critical: 85
  memory:
    warning: 75
    critical: 90
  disk:
    warning: 80
    critical: 95
```

#### Webhook 通知

```yaml
channels:
  webhook:
    enabled: true
    url: "https://your-webhook-url.com/alerts"
    timeout: 10
```

## 安全配置

### SSL/TLS 配置

#### 使用 Let's Encrypt

```bash
# 安装 certbot
sudo apt install certbot python3-certbot-nginx

# 获取证书
sudo certbot --nginx -d your-domain.com

# 自动续期
sudo crontab -e
# 添加: 0 12 * * * /usr/bin/certbot renew --quiet
```

#### Nginx SSL 配置

```nginx
server {
    listen 443 ssl http2;
    server_name your-domain.com;
    
    ssl_certificate /etc/letsencrypt/live/your-domain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/your-domain.com/privkey.pem;
    
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-RSA-AES256-GCM-SHA512:DHE-RSA-AES256-GCM-SHA512;
    ssl_prefer_server_ciphers off;
    
    location / {
        proxy_pass http://synapse:8008;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

### 防火墙配置

```bash
# 启用 UFW
sudo ufw enable

# 允许必要端口
sudo ufw allow 22/tcp      # SSH
sudo ufw allow 80/tcp      # HTTP
sudo ufw allow 443/tcp     # HTTPS
sudo ufw allow 8008/tcp    # Synapse (如果直接暴露)

# 查看状态
sudo ufw status
```

### 用户权限

```yaml
# homeserver.yaml
registration:
  enable_registration: false  # 禁用公开注册
  registration_shared_secret: "your-secret-key"
  
api:
  registration_requires_token: true
  
rate_limiting:
  login:
    per_second: 0.17
    burst_count: 3
```

## 性能基准测试

### 系统性能测试

```bash
# CPU 测试
sysbench cpu --cpu-max-prime=20000 run

# 内存测试
sysbench memory --memory-total-size=1G run

# 磁盘测试
sysbench fileio --file-total-size=2G prepare
sysbench fileio --file-total-size=2G --file-test-mode=rndrw run
sysbench fileio --file-total-size=2G cleanup
```

### Synapse 性能测试

```bash
# 连接测试
time curl -s http://localhost:8008/health

# 负载测试 (使用 ab)
ab -n 100 -c 10 http://localhost:8008/_matrix/client/versions

# 内存使用监控
watch -n 1 'docker stats --no-stream'
```

## 附录

### 有用的命令

```bash
# 查看容器资源使用
docker stats

# 进入容器
docker-compose exec synapse bash

# 查看 Synapse 版本
docker-compose exec synapse python -m synapse.app.homeserver --version

# 重新生成配置
docker-compose exec synapse python -m synapse.app.homeserver --config-path /data/homeserver.yaml --generate-config --report-stats=no

# 数据库维护
docker-compose exec postgres psql -U synapse_user -d synapse -c "VACUUM ANALYZE;"
```

### 配置模板

完整的配置文件模板可以在以下位置找到：

- `contrib/docker/conf/homeserver-performance.yaml` - 性能优化配置
- `docker/docker-compose.low-spec.yml` - 低配置 Docker Compose
- `docker/nginx/nginx.conf` - Nginx 反向代理配置

### 社区资源

- [Matrix.org 官方文档](https://matrix.org/docs/)
- [Synapse 管理员指南](https://matrix-org.github.io/synapse/latest/)
- [Matrix 社区](https://matrix.to/#/#synapse:matrix.org)

---

**注意**: 本指南针对低配置服务器进行了优化。在生产环境中，建议根据实际负载调整配置参数。