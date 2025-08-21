# Synapse Matrix服务器部署指南

这是一个针对1核2GB服务器优化的Synapse Matrix服务器完整部署方案，包含好友功能、监控、日志和维护工具。

## 🚀 快速开始

### 系统要求

- **最低配置**: 1核CPU, 2GB内存, 20GB磁盘空间
- **推荐配置**: 2核CPU, 4GB内存, 50GB磁盘空间
- **操作系统**: Ubuntu 20.04+ / CentOS 8+ / Debian 11+
- **软件依赖**: Docker, Docker Compose

### 一键部署

```bash
# 1. 克隆或下载仓库（包含部署目录）
git clone https://github.com/langkebo/synapse.git
cd synapse/synapse-deployment

# 2. 配置环境变量
cp .env.example .env
vim .env  # 编辑配置文件

# 3. 启动服务
./scripts/start.sh
```

## 📁 项目结构

```
synapse/
├── synapse-deployment/           # 部署目录（本指南所在位置）
│   ├── docker-compose.yml        # Docker Compose配置
│   ├── .env.example              # 环境变量模板
│   ├── README.md                 # 本文档
│   ├── synapse/                  # Synapse配置
│   │   ├── homeserver.yaml       # 主配置文件
│   │   ├── friends_config.yaml   # 好友功能配置
│   │   └── log.yaml              # 日志配置
│   ├── nginx/                    # Nginx配置
│   ├── redis/                    # Redis配置
│   ├── prometheus/               # Prometheus监控
│   ├── grafana/                  # Grafana可视化
│   ├── scripts/                  # 管理脚本
│   ├── data/                     # 数据目录
│   ├── logs/                     # 日志目录
│   └── ssl/                      # SSL证书目录
├── synapse/                      # 上游 Synapse 源码（用于构建）
└── ...
```

## ⚙️ 配置说明

### 域名委托配置

本项目支持通过 `.well-known` 机制实现域名委托，允许用户使用简洁的域名格式（如 `@username:cjystx.top`），而实际的Matrix服务器运行在子域名（如 `matrix.cjystx.top`）上。

#### 域名配置说明

- **主域名**: `cjystx.top` - 用户ID格式，通过 `.well-known` 发现实际服务器
- **Matrix服务器**: `matrix.cjystx.top` - 实际的Matrix服务器地址
- **Element客户端**: `element.cjystx.top` - Web客户端访问地址
- **监控面板**: `monitoring.cjystx.top` - Grafana/Prometheus监控界面

#### .well-known 配置

项目已在 Nginx 配置中自动设置了以下端点：

```bash
# 服务器发现
https://cjystx.top/.well-known/matrix/server
# 返回: {"m.server": "matrix.cjystx.top:443"}

# 客户端发现
https://cjystx.top/.well-known/matrix/client
# 返回: {"m.homeserver": {"base_url": "https://matrix.cjystx.top"}, "m.identity_server": {"base_url": "https://vector.im"}}
```

#### DNS 配置要求

确保以下DNS记录正确配置：

```
cjystx.top           A    23.95.215.88
matrix.cjystx.top    A    23.95.215.88
element.cjystx.top   A    23.95.215.88
monitoring.cjystx.top A   23.95.215.88
```

#### 用户体验

配置完成后，用户可以：
- 使用 `@username:cjystx.top` 格式的用户ID
- 访问 `https://element.cjystx.top` 使用Web客户端
- 访问 `https://monitoring.cjystx.top` 查看监控面板
- Matrix客户端会自动发现实际服务器地址 `matrix.cjystx.top`

### 环境变量配置

复制 `.env.example` 到 `.env` 并根据需要修改：

```bash
# 基本配置
SERVER_NAME=your-domain.com
SYNAPSE_REPORT_STATS=no

# 数据库配置
POSTGRES_PASSWORD=your-secure-password
POSTGRES_DB=synapse
POSTGRES_USER=synapse

# Redis配置
REDIS_PASSWORD=your-redis-password

# 监控配置
GRAFANA_ADMIN_PASSWORD=your-grafana-password
PROMETHEUS_RETENTION_TIME=7d

# 好友功能配置
FRIENDS_ENABLED=true
FRIENDS_MAX_FRIENDS=500
FRIENDS_REQUEST_TIMEOUT=7d
```

### SSL证书配置

#### 使用Let's Encrypt（推荐）

```bash
# 安装certbot
sudo apt install certbot

# 获取证书
sudo certbot certonly --standalone -d your-domain.com

# 复制证书到项目目录
sudo cp /etc/letsencrypt/live/your-domain.com/fullchain.pem ssl/cert.pem
sudo cp /etc/letsencrypt/live/your-domain.com/privkey.pem ssl/key.pem
sudo chown $USER:$USER ssl/*.pem
```

#### 使用自签名证书（测试用）

```bash
# 脚本会自动生成自签名证书
./scripts/start.sh
```

## 🛠️ 管理脚本

### 启动服务

```bash
./scripts/start.sh
```

功能：
- 检查系统依赖
- 创建必要目录
- 生成SSL证书（如果不存在）
- 启动所有服务
- 显示访问信息

### 停止服务

```bash
# 正常停止
./scripts/stop.sh

# 强制停止
./scripts/stop.sh --force

# 停止并清理数据
./scripts/stop.sh --cleanup
```

### 备份数据

```bash
# 完整备份
./scripts/backup.sh

# 仅备份数据库
./scripts/backup.sh --database-only

# 仅备份配置
./scripts/backup.sh --config-only

# 加密备份
./scripts/backup.sh --encrypt
```

### 监控检查

```bash
# 单次检查
./scripts/monitor.sh

# 连续监控
./scripts/monitor.sh --continuous

# 显示性能指标
./scripts/monitor.sh --performance

# 生成监控报告
./scripts/monitor.sh --report
```

### 维护任务

```bash
# 清理日志
./scripts/maintenance.sh cleanup-logs

# 数据库优化
./scripts/maintenance.sh vacuum-db

# 执行所有维护任务
./scripts/maintenance.sh all

# 模拟运行（不执行实际操作）
./scripts/maintenance.sh --dry-run all
```

## 📊 监控和可视化

### 访问地址

- **Synapse服务器**: https://your-domain.com
- **Grafana监控**: http://your-domain.com:3000
- **Prometheus**: http://your-domain.com:9090
- **系统监控**: http://your-domain.com:8080

### 默认账户

- **Grafana**: admin / (在.env中设置的密码)

### 监控面板

1. **Synapse概览**: 服务状态、性能指标、用户统计
2. **好友功能**: 好友请求、关系统计、API性能
3. **系统监控**: CPU、内存、磁盘、网络
4. **数据库监控**: 连接数、查询性能、表统计

## 🔧 好友功能

### API端点

```bash
# 发送好友请求
POST /_matrix/client/v1/friends/request
{
    "user_id": "@friend:cjystx.top",
    "message": "你好，我想加你为好友"
}

# 接受好友请求
PUT /_matrix/client/v1/friends/request/@friend:cjystx.top/accept

# 拒绝好友请求
PUT /_matrix/client/v1/friends/request/@friend:cjystx.top/reject

# 获取好友列表
GET /_matrix/client/v1/friends

# 删除好友
DELETE /_matrix/client/v1/friends/@friend:cjystx.top

# 搜索用户
GET /_matrix/client/v1/friends/search?q=username
```

### 配置选项

在 `synapse/friends_config.yaml` 中配置：

```yaml
friends:
  enabled: true
  max_friends: 500
  request_timeout: "7d"
  search:
    enabled: true
    max_results: 50
  rate_limiting:
    requests_per_minute: 10
    search_per_minute: 20
```

## 🚨 故障排除

### 常见问题

#### 1. 服务启动失败

```bash
# 检查日志
docker-compose logs synapse

# 检查配置文件语法
python3 -c "import yaml; yaml.safe_load(open('synapse/homeserver.yaml'))"

# 重新构建镜像
docker-compose build --no-cache synapse
```

#### 2. 数据库连接失败

```bash
# 检查数据库状态
docker-compose exec postgres pg_isready -U synapse

# 检查数据库日志
docker-compose logs postgres

# 重置数据库密码
docker-compose exec postgres psql -U postgres -c "ALTER USER synapse PASSWORD 'new-password';"
```

#### 3. SSL证书问题

```bash
# 检查证书有效性
openssl x509 -in ssl/cert.pem -text -noout

# 重新生成自签名证书
rm ssl/*.pem
./scripts/start.sh
```

#### 4. 内存不足

```bash
# 检查内存使用
free -h
docker stats

# 调整服务资源限制
vim docker-compose.yml  # 修改 mem_limit
```

### 日志位置

- **Synapse**: `logs/synapse.log`
- **Nginx**: `logs/nginx/access.log`, `logs/nginx/error.log`
- **PostgreSQL**: `docker-compose logs postgres`
- **Redis**: `docker-compose logs redis`
- **监控**: `logs/monitor.log`
- **维护**: `logs/maintenance.log`

## 🔒 安全建议

### 基本安全

1. **更改默认密码**: 修改所有默认密码
2. **使用强密码**: 至少12位，包含大小写字母、数字和特殊字符
3. **定期更新**: 定期更新系统和Docker镜像
4. **防火墙配置**: 只开放必要端口

```bash
# UFW防火墙配置示例
sudo ufw allow 22/tcp      # SSH
sudo ufw allow 80/tcp      # HTTP
sudo ufw allow 443/tcp     # HTTPS
sudo ufw allow 8448/tcp    # Matrix联邦
sudo ufw enable
```

### SSL/TLS配置

- 使用Let's Encrypt证书
- 启用HSTS
- 禁用弱加密套件
- 定期检查SSL配置

### 数据库安全

- 使用强密码
- 限制网络访问
- 定期备份
- 启用审计日志

## 📈 性能优化

### 1核2GB服务器优化

#### PostgreSQL优化

```sql
-- 在数据库中执行
ALTER SYSTEM SET shared_buffers = '512MB';
ALTER SYSTEM SET effective_cache_size = '1536MB';
ALTER SYSTEM SET work_mem = '8MB';
ALTER SYSTEM SET max_connections = 25;
SELECT pg_reload_conf();
```

#### Redis优化

```bash
# 在redis.conf中设置
maxmemory 256mb
maxmemory-policy allkeys-lru
tcp-keepalive 60
```

#### Synapse优化

```yaml
# 在homeserver.yaml中设置
database:
  args:
    cp_min: 5
    cp_max: 10

caches:
  global_factor: 0.5
  per_cache_factors:
    get_users_who_share_room_with_user: 0.5
```

### 监控指标

关注以下关键指标：

- **CPU使用率**: < 80%
- **内存使用率**: < 85%
- **磁盘使用率**: < 90%
- **数据库连接数**: < 20
- **HTTP响应时间**: < 1秒
- **好友API错误率**: < 5%

## 🔄 备份和恢复

### 自动备份

```bash
# 设置定时备份（每天凌晨2点）
crontab -e
# 添加以下行：
0 2 * * * /path/to/synapse-deployment/scripts/backup.sh --quiet
```

### 恢复数据

```bash
# 从备份恢复
./scripts/backup.sh --restore /path/to/backup.tar.gz

# 仅恢复数据库
./scripts/backup.sh --restore-database /path/to/database.sql.gz
```

## 📞 支持和帮助

### 获取帮助

```bash
# 查看脚本帮助
./scripts/start.sh --help
./scripts/monitor.sh --help
./scripts/maintenance.sh --help
./scripts/backup.sh --help
```

### 常用命令

```bash
# 查看服务状态
docker-compose ps

# 查看实时日志
docker-compose logs -f synapse

# 进入容器
docker-compose exec synapse bash

# 重启单个服务
docker-compose restart synapse

# 更新镜像
docker-compose pull
docker-compose up -d
```

### 社区资源

- [Matrix官方文档](https://matrix.org/docs/)
- [Synapse管理员指南](https://matrix-org.github.io/synapse/latest/)
- [Docker Compose文档](https://docs.docker.com/compose/)

## 📝 更新日志

### v1.0.0
- 初始版本发布
- 支持1核2GB服务器部署
- 集成好友功能
- 完整监控和日志系统
- 自动化脚本工具

## 📄 许可证

本项目基于 Apache 2.0 许可证开源。

## 🤝 贡献

欢迎提交Issue和Pull Request来改进这个项目。

---

**注意**: 这是一个针对小型部署优化的配置。对于生产环境或大规模部署，请根据实际需求调整配置参数。