# Synapse Matrix 服务器部署指南

## 📋 目录

1. [部署前准备](#1-部署前准备)
2. [快速部署](#2-快速部署)
3. [手动部署](#3-手动部署)
4. [配置详解](#4-配置详解)
5. [SSL/TLS 配置](#5-ssltls-配置)
6. [监控配置](#6-监控配置)
7. [故障排除](#7-故障排除)
8. [维护指南](#8-维护指南)

---

## 1. 部署前准备

### 1.1 系统要求

#### 最低硬件配置

| 组件 | 最低要求 | 推荐配置 | 说明 |
|------|----------|----------|------|
| **CPU** | 1 核心 | 2 核心+ | x86_64 架构 |
| **内存** | 2GB RAM | 4GB+ RAM | 包含系统和应用 |
| **存储** | 20GB | 50GB+ | SSD 推荐 |
| **网络** | 100Mbps | 1Gbps+ | 稳定互联网连接 |

#### 支持的操作系统

- ✅ **Ubuntu 20.04 LTS+** (强烈推荐)
- ✅ Ubuntu 22.04 LTS
- ✅ Debian 11+
- ✅ CentOS 8+ / RHEL 8+
- ⚠️ 其他 Linux 发行版 (需要手动调整)

### 1.2 域名和 DNS 配置

#### 必需的 DNS 记录

```dns
# A 记录 - 指向服务器 IP
matrix.example.com.    IN  A     YOUR_SERVER_IP

# SRV 记录 - Matrix 联邦发现 (可选但推荐)
_matrix._tcp.example.com. IN SRV 10 5 8448 matrix.example.com.

# 委托记录 (如果使用子域名)
.well-known/matrix/server  -> {"m.server": "matrix.example.com:8448"}
.well-known/matrix/client  -> {"m.homeserver": {"base_url": "https://matrix.example.com"}}
```

#### DNS 验证

```bash
# 验证 A 记录
nslookup matrix.example.com

# 验证 SRV 记录
nslookup -type=SRV _matrix._tcp.example.com

# 测试连通性
ping matrix.example.com
```

### 1.3 防火墙配置

```bash
# Ubuntu/Debian 使用 UFW
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow ssh
sudo ufw allow 80/tcp    # HTTP (重定向到 HTTPS)
sudo ufw allow 443/tcp   # HTTPS
sudo ufw allow 8448/tcp  # Matrix 联邦
sudo ufw --force enable

# CentOS/RHEL 使用 firewalld
sudo firewall-cmd --permanent --add-service=ssh
sudo firewall-cmd --permanent --add-service=http
sudo firewall-cmd --permanent --add-service=https
sudo firewall-cmd --permanent --add-port=8448/tcp
sudo firewall-cmd --reload
```

---

## 2. 快速部署

### 2.1 一键部署脚本

```bash
# 1. 下载项目
git clone https://github.com/langkebo/synapse.git
cd synapse

# 2. 运行部署脚本
chmod +x scripts/enhanced_deploy.sh
./scripts/enhanced_deploy.sh
```

### 2.2 部署过程说明

部署脚本将自动执行以下步骤：

1. ✅ **系统检查** - 验证硬件和软件要求
2. ✅ **Docker 安装** - 自动安装 Docker 和 Docker Compose
3. ✅ **SSL 证书** - 申请 Let's Encrypt 证书或生成自签名证书
4. ✅ **Nginx 配置** - 自动配置反向代理和负载均衡
5. ✅ **服务启动** - 启动所有容器服务
6. ✅ **健康检查** - 验证服务状态

### 2.3 部署参数

在部署过程中，您需要提供以下信息：

```bash
请输入服务器域名 (例: matrix.example.com): matrix.yourdomain.com
请输入管理员邮箱: admin@yourdomain.com
```

### 2.4 部署完成验证

```bash
# 检查服务状态
docker-compose ps

# 验证 Synapse 健康状态
curl -f http://localhost:8008/health

# 检查 HTTPS 访问
curl -I https://matrix.yourdomain.com/health
```

---

## 3. 手动部署

### 3.1 环境准备

#### 3.1.1 更新系统

```bash
# Ubuntu/Debian
sudo apt update && sudo apt upgrade -y
sudo apt install -y curl wget git openssl ca-certificates gnupg lsb-release

# CentOS/RHEL
sudo yum update -y
sudo yum install -y curl wget git openssl ca-certificates gnupg
```

#### 3.1.2 安装 Docker

```bash
# 下载 Docker 安装脚本
curl -fsSL https://get.docker.com -o get-docker.sh

# 运行安装脚本
sudo sh get-docker.sh

# 添加用户到 docker 组
sudo usermod -aG docker $USER

# 启动 Docker 服务
sudo systemctl start docker
sudo systemctl enable docker

# 验证安装
docker --version
docker-compose --version
```

#### 3.1.3 重新登录

```bash
# 重新登录以应用组权限
newgrp docker

# 或者重新登录 SSH
exit
# 重新 SSH 连接
```

### 3.2 项目配置

#### 3.2.1 下载项目

```bash
git clone https://github.com/langkebo/synapse.git
cd synapse
```

#### 3.2.2 环境配置

```bash
# 复制环境配置模板
cp .env.example .env

# 编辑配置文件
nano .env
```

#### 3.2.3 必需的配置项

```bash
# 服务器配置
SYNAPSE_SERVER_NAME=matrix.yourdomain.com
SYNAPSE_REPORT_STATS=no

# 数据库配置
POSTGRES_DB=synapse
POSTGRES_USER=synapse
POSTGRES_PASSWORD=your_secure_password_here

# Redis 配置
REDIS_PASSWORD=your_redis_password_here

# 监控配置
GRAFANA_PASSWORD=your_grafana_password_here

# SMTP 配置 (可选)
SMTP_HOST=smtp.gmail.com
SMTP_PORT=587
SMTP_USER=your_email@gmail.com
SMTP_PASS=your_app_password
SMTP_FROM=noreply@yourdomain.com
```

### 3.3 SSL 证书配置

#### 3.3.1 Let's Encrypt 证书 (推荐)

```bash
# 安装 Certbot
sudo apt install -y certbot python3-certbot-nginx

# 申请证书
sudo certbot certonly --standalone \
    --non-interactive \
    --agree-tos \
    --email admin@yourdomain.com \
    -d matrix.yourdomain.com

# 设置自动续期
echo "0 12 * * * /usr/bin/certbot renew --quiet" | sudo crontab -
```

#### 3.3.2 自签名证书 (测试环境)

```bash
# 创建证书目录
sudo mkdir -p /etc/ssl/synapse

# 生成自签名证书
sudo openssl req -x509 -newkey rsa:4096 \
    -keyout /etc/ssl/synapse/key.pem \
    -out /etc/ssl/synapse/cert.pem \
    -days 365 -nodes \
    -subj "/C=CN/ST=Beijing/L=Beijing/O=Matrix/OU=Synapse/CN=matrix.yourdomain.com"

# 设置权限
sudo chmod 600 /etc/ssl/synapse/key.pem
sudo chmod 644 /etc/ssl/synapse/cert.pem
```

### 3.4 Nginx 配置

#### 3.4.1 安装 Nginx

```bash
# Ubuntu/Debian
sudo apt install -y nginx

# CentOS/RHEL
sudo yum install -y nginx

# 启动服务
sudo systemctl start nginx
sudo systemctl enable nginx
```

#### 3.4.2 配置文件

```bash
# 创建 Synapse 配置
sudo nano /etc/nginx/sites-available/synapse
```

```nginx
# Synapse Matrix 服务器 Nginx 配置

# 限流配置
limit_req_zone $binary_remote_addr zone=api:10m rate=10r/s;
limit_req_zone $binary_remote_addr zone=login:10m rate=1r/s;
limit_conn_zone $binary_remote_addr zone=conn_limit_per_ip:10m;

# 上游服务器
upstream synapse_backend {
    server 127.0.0.1:8008 max_fails=3 fail_timeout=30s;
    keepalive 32;
}

# HTTP 重定向到 HTTPS
server {
    listen 80;
    listen [::]:80;
    server_name matrix.yourdomain.com;
    return 301 https://$server_name$request_uri;
}

# HTTPS 主配置
server {
    listen 443 ssl http2;
    listen [::]:443 ssl http2;
    server_name matrix.yourdomain.com;
    
    # SSL 配置
    ssl_certificate /etc/letsencrypt/live/matrix.yourdomain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/matrix.yourdomain.com/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-RSA-AES128-GCM-SHA256:ECDHE-RSA-AES256-GCM-SHA384;
    ssl_prefer_server_ciphers off;
    ssl_session_cache shared:SSL:10m;
    ssl_session_timeout 10m;
    
    # 安全头
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;
    add_header X-Content-Type-Options nosniff;
    add_header X-Frame-Options DENY;
    add_header X-XSS-Protection "1; mode=block";
    
    # Matrix 客户端 API
    location /_matrix {
        limit_req zone=api burst=20 nodelay;
        
        proxy_pass http://synapse_backend;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # WebSocket 支持
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        
        # 超时配置
        proxy_connect_timeout 5s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;
    }
    
    # 健康检查
    location /health {
        proxy_pass http://synapse_backend;
        access_log off;
    }
    
    # Well-known 配置
    location /.well-known/matrix/server {
        return 200 '{"m.server": "matrix.yourdomain.com:8448"}';
        add_header Content-Type application/json;
    }
    
    location /.well-known/matrix/client {
        return 200 '{"m.homeserver": {"base_url": "https://matrix.yourdomain.com"}}';
        add_header Content-Type application/json;
        add_header Access-Control-Allow-Origin *;
    }
}

# Matrix 联邦 API (端口 8448)
server {
    listen 8448 ssl http2;
    listen [::]:8448 ssl http2;
    server_name matrix.yourdomain.com;
    
    # SSL 配置
    ssl_certificate /etc/letsencrypt/live/matrix.yourdomain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/matrix.yourdomain.com/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    
    location / {
        proxy_pass http://synapse_backend;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

#### 3.4.3 启用配置

```bash
# 启用站点配置
sudo ln -sf /etc/nginx/sites-available/synapse /etc/nginx/sites-enabled/

# 删除默认配置
sudo rm -f /etc/nginx/sites-enabled/default

# 测试配置
sudo nginx -t

# 重新加载 Nginx
sudo systemctl reload nginx
```

### 3.5 启动服务

```bash
# 启动所有服务
docker-compose -f synapse-deployment/docker-compose.yml up -d

# 查看服务状态
docker-compose ps

# 查看启动日志
docker-compose logs -f synapse
```

### 3.6 创建管理员用户

```bash
# 等待服务完全启动 (约 2-3 分钟)
sleep 180

# 创建管理员用户
docker-compose exec synapse register_new_matrix_user \
    -c /data/homeserver.yaml \
    -a http://localhost:8008

# 按提示输入用户名和密码
# 用户名格式: admin (不需要包含域名)
# 密码: 设置强密码
# 是否为管理员: yes
```

---

## 4. 配置详解

### 4.1 Synapse 主配置

#### 4.1.1 homeserver.yaml 核心配置

```yaml
# 服务器基本信息
server_name: "matrix.yourdomain.com"
pid_file: /data/homeserver.pid
web_client_location: https://app.element.io/
public_baseurl: https://matrix.yourdomain.com/

# 监听配置
listeners:
  - port: 8008
    tls: false
    type: http
    x_forwarded: true
    bind_addresses: ['0.0.0.0']
    resources:
      - names: [client, federation]
        compress: false

# 数据库配置
database:
  name: psycopg2
  args:
    user: synapse
    password: your_secure_password
    database: synapse
    host: postgres
    port: 5432
    cp_min: 5
    cp_max: 10
    cp_timeout: 30
    cp_reconnect: true

# Redis 配置
redis:
  enabled: true
  host: redis
  port: 6379
  password: your_redis_password

# 日志配置
log_config: "/data/log_config.yaml"

# 媒体存储
media_store_path: "/data/media_store"
max_upload_size: 50M
max_image_pixels: 32M

# 注册配置
enable_registration: false
registration_requires_token: true
allow_guest_access: false

# 安全配置
use_presence: true
require_auth_for_profile_requests: true
limit_profile_requests_to_users_who_share_rooms: true
allow_public_rooms_without_auth: false

# 速率限制
rc_message:
  per_second: 0.2
  burst_count: 10

rc_registration:
  per_second: 0.17
  burst_count: 3

rc_login:
  address:
    per_second: 0.17
    burst_count: 3
  account:
    per_second: 0.17
    burst_count: 3
  failed_attempts:
    per_second: 0.17
    burst_count: 3

# 联邦配置
federation_domain_whitelist: []
federation_ip_range_blacklist:
  - '127.0.0.0/8'
  - '10.0.0.0/8'
  - '172.16.0.0/12'
  - '192.168.0.0/16'
  - '100.64.0.0/10'
  - '169.254.0.0/16'
  - '::1/128'
  - 'fe80::/64'
  - 'fc00::/7'

# 监控配置
enable_metrics: true
report_stats: false

# 签名密钥
signing_key_path: "/data/signing.key"
trusted_key_servers:
  - server_name: "matrix.org"

# 应用服务
app_service_config_files: []

# 推送配置
push:
  include_content: true
  group_unread_count_by_room: false
```

#### 4.1.2 日志配置 (log_config.yaml)

```yaml
version: 1

formatters:
  precise:
    format: '%(asctime)s - %(name)s - %(lineno)d - %(levelname)s - %(request)s - %(message)s'

handlers:
  file:
    class: logging.handlers.TimedRotatingFileHandler
    formatter: precise
    filename: /data/homeserver.log
    when: midnight
    interval: 1
    backupCount: 7
    encoding: utf8

  console:
    class: logging.StreamHandler
    formatter: precise
    stream: ext://sys.stdout

loggers:
    synapse.storage.SQL:
        level: INFO
    synapse.access.http.8008:
        level: INFO
    synapse.federation.transport.server:
        level: INFO

root:
    level: INFO
    handlers: [file, console]

disable_existing_loggers: false
```

### 4.2 性能优化配置

#### 4.2.1 PostgreSQL 优化

```sql
-- postgresql.conf 优化配置
max_connections = 200
shared_buffers = 256MB
effective_cache_size = 1GB
maintenance_work_mem = 64MB
checkpoint_completion_target = 0.9
wal_buffers = 16MB
default_statistics_target = 100
random_page_cost = 1.1
effective_io_concurrency = 200
min_wal_size = 1GB
max_wal_size = 4GB

-- 启用查询统计
shared_preload_libraries = 'pg_stat_statements'
track_activity_query_size = 2048
pg_stat_statements.track = all
```

#### 4.2.2 Redis 优化

```redis
# redis.conf 优化配置
maxmemory 256mb
maxmemory-policy allkeys-lru
save 900 1
save 300 10
save 60 10000
tcp-keepalive 300
timeout 0

# 性能优化
hash-max-ziplist-entries 512
hash-max-ziplist-value 64
list-max-ziplist-size -2
set-max-intset-entries 512
zset-max-ziplist-entries 128
zset-max-ziplist-value 64
```

---

## 5. SSL/TLS 配置

### 5.1 Let's Encrypt 自动化

#### 5.1.1 Certbot 安装和配置

```bash
# 安装 Certbot
sudo apt install -y certbot python3-certbot-nginx

# 申请证书
sudo certbot certonly \
    --standalone \
    --non-interactive \
    --agree-tos \
    --email admin@yourdomain.com \
    -d matrix.yourdomain.com
```

#### 5.1.2 自动续期脚本

```bash
#!/bin/bash
# /usr/local/bin/renew-certs.sh

set -euo pipefail

LOG_FILE="/var/log/certbot-renew.log"

log() {
    echo "[$(date +'%Y-%m-%d %H:%M:%S')] $1" | tee -a "$LOG_FILE"
}

log "开始证书续期检查..."

# 续期证书
if certbot renew --quiet --no-self-upgrade; then
    log "证书续期检查完成"
    
    # 重新加载 Nginx
    if systemctl reload nginx; then
        log "Nginx 重新加载成功"
    else
        log "错误: Nginx 重新加载失败"
        exit 1
    fi
else
    log "错误: 证书续期失败"
    exit 1
fi

log "证书续期流程完成"
```

#### 5.1.3 设置定时任务

```bash
# 设置可执行权限
sudo chmod +x /usr/local/bin/renew-certs.sh

# 添加到 crontab
echo "0 2 * * * /usr/local/bin/renew-certs.sh" | sudo crontab -

# 验证 crontab
sudo crontab -l
```

### 5.2 SSL 安全配置

#### 5.2.1 强化 SSL 配置

```nginx
# 在 Nginx 配置中添加
ssl_protocols TLSv1.2 TLSv1.3;
ssl_ciphers ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-GCM-SHA384;
ssl_prefer_server_ciphers off;
ssl_session_cache shared:SSL:10m;
ssl_session_timeout 10m;
ssl_session_tickets off;

# OCSP Stapling
ssl_stapling on;
ssl_stapling_verify on;
ssl_trusted_certificate /etc/letsencrypt/live/matrix.yourdomain.com/chain.pem;
resolver 8.8.8.8 8.8.4.4 valid=300s;
resolver_timeout 5s;

# 安全头
add_header Strict-Transport-Security "max-age=31536000; includeSubDomains; preload" always;
add_header X-Content-Type-Options nosniff;
add_header X-Frame-Options DENY;
add_header X-XSS-Protection "1; mode=block";
add_header Referrer-Policy "strict-origin-when-cross-origin";
```

#### 5.2.2 SSL 测试

```bash
# 测试 SSL 配置
ssl-cert-check -c /etc/letsencrypt/live/matrix.yourdomain.com/cert.pem

# 在线 SSL 测试
# 访问: https://www.ssllabs.com/ssltest/
# 输入: matrix.yourdomain.com

# 本地测试
openssl s_client -connect matrix.yourdomain.com:443 -servername matrix.yourdomain.com
```

---

## 6. 监控配置

### 6.1 Prometheus 配置

#### 6.1.1 prometheus.yml

```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

rule_files:
  - "synapse_rules.yml"
  - "system_rules.yml"

alerting:
  alertmanagers:
    - static_configs:
        - targets:
          - alertmanager:9093

scrape_configs:
  - job_name: 'synapse'
    static_configs:
      - targets: ['synapse:8008']
    metrics_path: '/_synapse/metrics'
    scrape_interval: 30s

  - job_name: 'postgres'
    static_configs:
      - targets: ['postgres-exporter:9187']
    scrape_interval: 30s

  - job_name: 'redis'
    static_configs:
      - targets: ['redis-exporter:9121']
    scrape_interval: 30s

  - job_name: 'nginx'
    static_configs:
      - targets: ['nginx-exporter:9113']
    scrape_interval: 30s

  - job_name: 'node'
    static_configs:
      - targets: ['node-exporter:9100']
    scrape_interval: 30s
```

#### 6.1.2 告警规则 (synapse_rules.yml)

```yaml
groups:
  - name: synapse
    rules:
      - alert: SynapseDown
        expr: up{job="synapse"} == 0
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "Synapse 服务不可用"
          description: "Synapse 服务已停止响应超过 5 分钟"

      - alert: SynapseHighMemoryUsage
        expr: process_resident_memory_bytes{job="synapse"} / 1024 / 1024 > 1500
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Synapse 内存使用过高"
          description: "Synapse 内存使用超过 1.5GB"

      - alert: SynapseHighCPUUsage
        expr: rate(process_cpu_seconds_total{job="synapse"}[5m]) * 100 > 80
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Synapse CPU 使用过高"
          description: "Synapse CPU 使用率超过 80%"

      - alert: SynapseHighDatabaseConnections
        expr: synapse_database_connections > 80
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "数据库连接数过高"
          description: "数据库连接数超过 80"
```

### 6.2 Grafana 仪表板

#### 6.2.1 数据源配置

```json
{
  "name": "Prometheus",
  "type": "prometheus",
  "url": "http://prometheus:9090",
  "access": "proxy",
  "isDefault": true
}
```

#### 6.2.2 关键监控指标

```json
{
  "dashboard": {
    "title": "Synapse Matrix 服务器监控",
    "panels": [
      {
        "title": "服务状态",
        "type": "stat",
        "targets": [
          {
            "expr": "up{job=\"synapse\"}",
            "legendFormat": "Synapse"
          }
        ]
      },
      {
        "title": "内存使用",
        "type": "graph",
        "targets": [
          {
            "expr": "process_resident_memory_bytes{job=\"synapse\"} / 1024 / 1024",
            "legendFormat": "内存使用 (MB)"
          }
        ]
      },
      {
        "title": "CPU 使用率",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(process_cpu_seconds_total{job=\"synapse\"}[5m]) * 100",
            "legendFormat": "CPU 使用率 (%)"
          }
        ]
      },
      {
        "title": "HTTP 请求",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(synapse_http_requests_total[5m])",
            "legendFormat": "{{method}} {{servlet}}"
          }
        ]
      }
    ]
  }
}
```

---

## 7. 故障排除

### 7.1 常见问题诊断

#### 7.1.1 服务启动失败

```bash
# 检查服务状态
docker-compose ps

# 查看详细日志
docker-compose logs synapse

# 检查配置文件
docker-compose exec synapse python -m synapse.config.homeserver \
    --config-path /data/homeserver.yaml \
    --generate-config \
    --server-name matrix.yourdomain.com

# 常见问题:
# 1. 端口被占用
sudo netstat -tlnp | grep :8008

# 2. 权限问题
sudo chown -R 991:991 data/

# 3. 内存不足
free -h
docker stats
```

#### 7.1.2 数据库连接问题

```bash
# 检查数据库状态
docker-compose exec postgres pg_isready -U synapse

# 测试数据库连接
docker-compose exec postgres psql -U synapse -d synapse -c "SELECT version();"

# 检查数据库日志
docker-compose logs postgres

# 重置数据库密码
docker-compose exec postgres psql -U postgres -c \
    "ALTER USER synapse PASSWORD 'new_password';"
```

#### 7.1.3 SSL 证书问题

```bash
# 检查证书有效性
openssl x509 -in /etc/letsencrypt/live/matrix.yourdomain.com/fullchain.pem \
    -text -noout | grep -A 2 "Validity"

# 测试证书链
openssl verify -CAfile /etc/ssl/certs/ca-certificates.crt \
    /etc/letsencrypt/live/matrix.yourdomain.com/fullchain.pem

# 手动续期
sudo certbot renew --force-renewal -d matrix.yourdomain.com

# 检查 Nginx 配置
sudo nginx -t
```

#### 7.1.4 联邦连接问题

```bash
# 测试联邦连接
curl -X GET "https://federationtester.matrix.org/api/report?server_name=matrix.yourdomain.com"

# 检查 SRV 记录
nslookup -type=SRV _matrix._tcp.yourdomain.com

# 测试端口连通性
telnet matrix.yourdomain.com 8448

# 检查防火墙
sudo ufw status
sudo iptables -L
```

### 7.2 性能问题诊断

#### 7.2.1 系统资源监控

```bash
# 实时系统监控
htop

# 磁盘使用情况
df -h
du -sh /var/lib/docker/

# 网络连接
ss -tulpn | grep :8008

# 容器资源使用
docker stats --no-stream
```

#### 7.2.2 数据库性能

```sql
-- 检查慢查询
SELECT query, mean_time, calls, total_time 
FROM pg_stat_statements 
ORDER BY mean_time DESC 
LIMIT 10;

-- 检查数据库大小
SELECT 
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) as size
FROM pg_tables 
WHERE schemaname = 'public' 
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;

-- 检查连接数
SELECT count(*) FROM pg_stat_activity;
```

#### 7.2.3 应用性能

```bash
# Synapse 性能指标
curl -s http://localhost:8008/_synapse/metrics | grep -E "(synapse_http_requests|synapse_database|process_)"

# 检查缓存命中率
curl -s http://localhost:8008/_synapse/metrics | grep cache

# 检查队列长度
curl -s http://localhost:8008/_synapse/metrics | grep queue
```

### 7.3 日志分析

#### 7.3.1 日志位置

```bash
# Synapse 日志
tail -f data/homeserver.log

# 容器日志
docker-compose logs -f synapse
docker-compose logs -f postgres
docker-compose logs -f redis

# Nginx 日志
sudo tail -f /var/log/nginx/access.log
sudo tail -f /var/log/nginx/error.log

# 系统日志
sudo journalctl -u docker -f
sudo journalctl -u nginx -f
```

#### 7.3.2 关键日志模式

```bash
# 启动成功标志
grep "Synapse now listening" data/homeserver.log

# 错误日志
grep -i error data/homeserver.log | tail -20

# 数据库错误
grep -i "database" data/homeserver.log | grep -i error

# 内存警告
grep -i "memory" data/homeserver.log | grep -i warning

# 联邦错误
grep -i "federation" data/homeserver.log | grep -i error
```

---

## 8. 维护指南

### 8.1 日常维护任务

#### 8.1.1 每日检查脚本

```bash
#!/bin/bash
# /usr/local/bin/daily-check.sh

set -euo pipefail

LOG_FILE="/var/log/synapse-maintenance.log"
DATE=$(date +'%Y-%m-%d %H:%M:%S')

log() {
    echo "[$DATE] $1" | tee -a "$LOG_FILE"
}

log "开始每日检查..."

# 检查服务状态
if docker-compose ps | grep -q "Up"; then
    log "✅ 所有服务正常运行"
else
    log "❌ 发现服务异常"
    docker-compose ps | tee -a "$LOG_FILE"
fi

# 检查磁盘空间
DISK_USAGE=$(df / | awk 'NR==2 {print $5}' | sed 's/%//')
if [ "$DISK_USAGE" -gt 85 ]; then
    log "⚠️  磁盘使用率过高: ${DISK_USAGE}%"
else
    log "✅ 磁盘使用率正常: ${DISK_USAGE}%"
fi

# 检查内存使用
MEM_USAGE=$(free | awk 'NR==2{printf "%.0f", $3*100/$2}')
if [ "$MEM_USAGE" -gt 90 ]; then
    log "⚠️  内存使用率过高: ${MEM_USAGE}%"
else
    log "✅ 内存使用率正常: ${MEM_USAGE}%"
fi

# 检查 SSL 证书有效期
CERT_DAYS=$(openssl x509 -in /etc/letsencrypt/live/matrix.yourdomain.com/cert.pem -noout -dates | grep notAfter | cut -d= -f2 | xargs -I {} date -d "{}" +%s)
CURRENT_DAYS=$(date +%s)
DAYS_LEFT=$(( (CERT_DAYS - CURRENT_DAYS) / 86400 ))

if [ "$DAYS_LEFT" -lt 30 ]; then
    log "⚠️  SSL 证书将在 ${DAYS_LEFT} 天后过期"
else
    log "✅ SSL 证书有效期正常: ${DAYS_LEFT} 天"
fi

# 检查数据库连接
if docker-compose exec -T postgres pg_isready -U synapse > /dev/null 2>&1; then
    log "✅ 数据库连接正常"
else
    log "❌ 数据库连接异常"
fi

log "每日检查完成"
```

#### 8.1.2 设置定时任务

```bash
# 设置可执行权限
sudo chmod +x /usr/local/bin/daily-check.sh

# 添加到 crontab (每天早上 8 点执行)
echo "0 8 * * * /usr/local/bin/daily-check.sh" | sudo crontab -
```

### 8.2 备份策略

#### 8.2.1 数据库备份

```bash
#!/bin/bash
# /usr/local/bin/backup-database.sh

set -euo pipefail

BACKUP_DIR="/var/backups/synapse"
DATE=$(date +'%Y%m%d_%H%M%S')
BACKUP_FILE="$BACKUP_DIR/synapse_backup_$DATE.sql"
RETENTION_DAYS=7

# 创建备份目录
mkdir -p "$BACKUP_DIR"

# 备份数据库
echo "开始备份数据库..."
docker-compose exec -T postgres pg_dump -U synapse synapse > "$BACKUP_FILE"

# 压缩备份文件
gzip "$BACKUP_FILE"

echo "数据库备份完成: ${BACKUP_FILE}.gz"

# 清理旧备份
find "$BACKUP_DIR" -name "synapse_backup_*.sql.gz" -mtime +$RETENTION_DAYS -delete

echo "备份清理完成"
```

#### 8.2.2 完整备份

```bash
#!/bin/bash
# /usr/local/bin/full-backup.sh

set -euo pipefail

BACKUP_DIR="/var/backups/synapse"
DATE=$(date +'%Y%m%d_%H%M%S')
FULL_BACKUP="$BACKUP_DIR/full_backup_$DATE"

# 创建备份目录
mkdir -p "$FULL_BACKUP"

echo "开始完整备份..."

# 备份数据库
docker-compose exec -T postgres pg_dump -U synapse synapse > "$FULL_BACKUP/database.sql"

# 备份配置文件
cp -r synapse-deployment/synapse/ "$FULL_BACKUP/config/"
cp .env "$FULL_BACKUP/"

# 备份媒体文件 (如果不大)
if [ $(du -sm data/media_store | cut -f1) -lt 1000 ]; then
    cp -r data/media_store "$FULL_BACKUP/"
else
    echo "媒体文件过大，跳过备份"
fi

# 备份签名密钥
cp data/signing.key "$FULL_BACKUP/"

# 创建压缩包
tar -czf "${FULL_BACKUP}.tar.gz" -C "$BACKUP_DIR" "$(basename "$FULL_BACKUP")"
rm -rf "$FULL_BACKUP"

echo "完整备份完成: ${FULL_BACKUP}.tar.gz"
```

### 8.3 更新和升级

#### 8.3.1 Synapse 升级

```bash
#!/bin/bash
# /usr/local/bin/upgrade-synapse.sh

set -euo pipefail

echo "开始 Synapse 升级..."

# 备份当前版本
./full-backup.sh

# 拉取最新镜像
docker-compose pull synapse

# 停止服务
docker-compose stop synapse

# 运行数据库迁移
docker-compose run --rm synapse migrate_config

# 启动服务
docker-compose up -d synapse

# 等待服务启动
sleep 30

# 健康检查
if curl -f http://localhost:8008/health > /dev/null 2>&1; then
    echo "✅ Synapse 升级成功"
else
    echo "❌ Synapse 升级失败，请检查日志"
    docker-compose logs synapse
    exit 1
fi
```

#### 8.3.2 系统更新

```bash
#!/bin/bash
# /usr/local/bin/system-update.sh

set -euo pipefail

echo "开始系统更新..."

# 更新包列表
sudo apt update

# 升级系统包
sudo apt upgrade -y

# 清理不需要的包
sudo apt autoremove -y
sudo apt autoclean

# 更新 Docker
sudo apt install -y docker-ce docker-ce-cli containerd.io

# 重启 Docker 服务
sudo systemctl restart docker

# 等待 Docker 启动
sleep 10

# 重启 Synapse 服务
docker-compose restart

echo "系统更新完成"
```

### 8.4 监控和告警

#### 8.4.1 健康检查脚本

```bash
#!/bin/bash
# /usr/local/bin/health-check.sh

set -euo pipefail

SERVER_NAME="matrix.yourdomain.com"
ALERT_EMAIL="admin@yourdomain.com"

# 检查 HTTP 响应
if ! curl -f "https://$SERVER_NAME/health" > /dev/null 2>&1; then
    echo "❌ HTTP 健康检查失败" | mail -s "Synapse 服务异常" "$ALERT_EMAIL"
    exit 1
fi

# 检查联邦端口
if ! nc -z "$SERVER_NAME" 8448; then
    echo "❌ 联邦端口 8448 不可达" | mail -s "Synapse 联邦异常" "$ALERT_EMAIL"
    exit 1
fi

# 检查数据库
if ! docker-compose exec -T postgres pg_isready -U synapse > /dev/null 2>&1; then
    echo "❌ 数据库连接失败" | mail -s "Synapse 数据库异常" "$ALERT_EMAIL"
    exit 1
fi

echo "✅ 所有健康检查通过"
```

#### 8.4.2 设置监控定时任务

```bash
# 每 5 分钟执行健康检查
echo "*/5 * * * * /usr/local/bin/health-check.sh" | crontab -

# 每天凌晨 2 点备份数据库
echo "0 2 * * * /usr/local/bin/backup-database.sh" | crontab -

# 每周日凌晨 3 点完整备份
echo "0 3 * * 0 /usr/local/bin/full-backup.sh" | crontab -

# 每月第一天检查系统更新
echo "0 4 1 * * /usr/local/bin/system-update.sh" | crontab -
```

---

## 📞 技术支持

如果在部署过程中遇到问题，请按以下步骤获取帮助：

1. **查看日志**: 首先检查相关服务的日志文件
2. **搜索文档**: 在本文档中搜索相关错误信息
3. **社区支持**: 访问 [Matrix 社区](https://matrix.to/#/#synapse:matrix.org)
4. **提交问题**: 在 [GitHub Issues](https://github.com/langkebo/synapse/issues) 提交问题

---

**部署完成后，您的 Synapse Matrix 服务器将提供企业级的即时通信服务，支持端到端加密、文件共享、音视频通话等功能。**