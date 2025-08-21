# Synapse2 Matrix 服务器部署版

这是一个针对低配置服务器（1核2GB）优化的 Synapse Matrix 服务器部署版本，集成了好友功能、性能优化、监控告警等特性。

## 🚀 快速开始

### 一键部署

```bash
# 下载并运行快速部署脚本
wget -O - https://raw.githubusercontent.com/your-repo/synapse2/main/scripts/quick_deploy.sh | bash
```

或者手动部署：

```bash
# 1. 克隆项目
git clone https://github.com/your-repo/synapse2.git
cd synapse2

# 2. 运行部署脚本
./scripts/quick_deploy.sh
```

### 手动部署

```bash
# 1. 复制环境变量模板
cp .env.example .env

# 2. 编辑配置文件
nano .env

# 3. 启动服务
docker-compose -f docker/docker-compose.low-spec.yml up -d

# 4. 创建管理员用户
docker-compose exec synapse register_new_matrix_user -c /data/homeserver.yaml -a http://localhost:8008
```

## 📋 系统要求

### 最低配置
- **CPU**: 1核心
- **内存**: 2GB RAM
- **存储**: 20GB 可用空间
- **操作系统**: Ubuntu 20.04 LTS+

### 软件依赖
- Docker 20.10+
- Docker Compose 2.0+
- Git

## ✨ 主要特性

### 🤝 好友功能
- 好友添加/删除
- 好友请求管理
- 在线状态显示
- 好友推荐
- 好友搜索

### ⚡ 性能优化
- 针对低配置服务器优化
- 智能缓存策略
- 内存使用优化
- 数据库连接池
- Redis 缓存集成

### 📊 监控告警
- 系统资源监控
- 服务健康检查
- 性能指标收集
- 邮件/Webhook 告警
- 实时日志监控

### 🌐 中文化支持
- 中文错误消息
- 本地化配置
- 中文文档

### 🐳 Docker 部署
- 一键部署
- 容器化服务
- 资源限制
- 自动重启

## 📁 项目结构

```
synapse2/
├── api/                          # 后端 API 代码
│   ├── friends/                  # 好友功能 API
│   └── ...
├── synapse/                      # Synapse 核心代码
│   ├── config/                   # 配置模块
│   │   ├── performance.py        # 性能配置
│   │   ├── cache_strategy.py     # 缓存策略
│   │   └── ...
│   ├── handlers/                 # 业务逻辑处理器
│   │   ├── friends.py            # 好友功能处理器
│   │   └── ...
│   ├── storage/                  # 数据存储层
│   │   ├── friends.py            # 好友数据存储
│   │   └── ...
│   ├── rest/                     # REST API
│   │   ├── client/
│   │   │   └── friends.py        # 好友 REST API
│   │   └── ...
│   └── util/
│       └── caches/
│           ├── cache_manager.py  # 缓存管理器
│           └── ...
├── docker/                       # Docker 配置
│   ├── docker-compose.low-spec.yml  # 低配置 Docker Compose
│   ├── Dockerfile.low-spec       # 优化的 Dockerfile
│   ├── start.sh                  # 容器启动脚本
│   └── monitor/                  # 监控相关
├── scripts/                      # 部署和管理脚本
│   ├── quick_deploy.sh           # 快速部署脚本
│   ├── performance_monitor.py    # 性能监控
│   ├── cache_warmup.py           # 缓存预热
│   ├── system_monitor.py         # 系统监控
│   └── synapse_startup.sh        # 服务启动脚本
├── contrib/docker/conf/          # 配置文件模板
│   └── homeserver-performance.yaml  # 性能优化配置
├── docs/                         # 文档
│   └── DEPLOYMENT_GUIDE.md       # 详细部署指南
├── .env.example                  # 环境变量模板
└── README_DEPLOYMENT.md          # 本文件
```

## 🔧 配置说明

### 环境变量配置

主要配置项（详见 `.env.example`）：

```bash
# 服务器配置
SERVER_NAME=matrix.example.com

# 数据库配置
POSTGRES_PASSWORD=your_secure_password

# Redis 配置
REDIS_PASSWORD=your_redis_password

# 性能配置
SYNAPSE_CACHE_FACTOR=0.5
SYNAPSE_EVENT_CACHE_SIZE=5K

# 监控配置
MONITOR_ENABLED=true
ALERT_THRESHOLD_CPU=80
```

### 性能优化配置

针对 1核2GB 服务器的优化配置：

```yaml
performance:
  memory:
    cache_factor: 0.5
    event_cache_size: "5K"
    max_memory_usage: 1536
  
  database:
    connection_pool:
      min_connections: 2
      max_connections: 5
  
  concurrency:
    worker_processes: 1
    max_concurrent_requests: 50
```

## 📊 监控和维护

### 查看服务状态

```bash
# 查看所有服务状态
docker-compose ps

# 查看服务日志
docker-compose logs -f synapse

# 健康检查
curl http://localhost:8008/health
```

### 性能监控

```bash
# 查看系统资源
docker-compose exec monitor python /scripts/system_monitor.py --stats

# 查看性能指标
docker-compose exec monitor python /scripts/performance_monitor.py --report

# 查看缓存状态
docker-compose exec synapse python -c "from synapse.util.caches.cache_manager import CacheManager; print(CacheManager.get_stats())"
```

### 备份和恢复

```bash
# 数据库备份
docker-compose exec postgres pg_dump -U synapse_user synapse > backup_$(date +%Y%m%d).sql

# 配置文件备份
tar -czf config_backup_$(date +%Y%m%d).tar.gz data/ .env docker-compose.yml

# 恢复数据库
docker-compose exec -T postgres psql -U synapse_user synapse < backup_20231201.sql
```

## 🔐 安全配置

### SSL/TLS 配置

```bash
# 使用 Let's Encrypt 获取证书
sudo certbot --nginx -d your-domain.com

# 自动续期
sudo crontab -e
# 添加: 0 12 * * * /usr/bin/certbot renew --quiet
```

### 防火墙配置

```bash
# 配置 UFW
sudo ufw enable
sudo ufw allow 22/tcp      # SSH
sudo ufw allow 80/tcp      # HTTP
sudo ufw allow 443/tcp     # HTTPS
sudo ufw allow 8008/tcp    # Synapse HTTP
sudo ufw allow 8448/tcp    # Synapse HTTPS
```

## 🛠️ 故障排除

### 常见问题

#### 1. 服务启动失败
```bash
# 查看详细日志
docker-compose logs synapse

# 检查配置文件
docker-compose exec synapse python -m synapse.config.homeserver --config-path /data/homeserver.yaml
```

#### 2. 内存不足
```bash
# 检查内存使用
free -h
docker stats

# 调整缓存配置
# 编辑 data/homeserver.yaml，降低 cache_factor
```

#### 3. 数据库连接失败
```bash
# 检查数据库状态
docker-compose exec postgres pg_isready -U synapse_user

# 重启数据库
docker-compose restart postgres
```

### 性能调优

#### CPU 优化
- 减少 worker 进程数
- 降低并发请求数
- 增加请求超时时间

#### 内存优化
- 降低缓存因子
- 减少事件缓存大小
- 启用更激进的垃圾回收

#### 磁盘优化
- 定期清理日志
- 压缩媒体文件
- 使用 SSD 存储

## 📚 文档和支持

- [详细部署指南](docs/DEPLOYMENT_GUIDE.md)
- [API 文档](docs/API.md)
- [配置参考](docs/CONFIG.md)
- [故障排除指南](docs/TROUBLESHOOTING.md)

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

## 📄 许可证

本项目基于 Apache License 2.0 许可证。

## 🙏 致谢

- [Matrix.org](https://matrix.org/) - Matrix 协议和 Synapse 服务器
- [Docker](https://docker.com/) - 容器化技术
- [PostgreSQL](https://postgresql.org/) - 数据库
- [Redis](https://redis.io/) - 缓存服务

---

**注意**: 本项目针对低配置服务器进行了优化，在生产环境中请根据实际负载调整配置参数。