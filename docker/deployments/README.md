# Synapse Rust Docker 部署指南

## 概述

本项目提供两套独立的 Docker 部署配置方案：

| 方案 | 目录 | 说明 | 适用场景 |
|------|------|------|----------|
| **完整配置** | `deployments/full/` | 包含所有监控组件 | 生产环境、需要完整监控 |
| **简化配置** | `deployments/simple/` | 仅核心服务 | 开发环境、资源受限场景 |

## 目录结构

```
docker/
├── deployments/
│   ├── full/                    # 完整配置方案
│   │   ├── docker-compose.yml   # Docker Compose 配置
│   │   ├── .env.example         # 环境变量模板
│   │   ├── config/              # 应用配置
│   │   │   └── homeserver.yaml
│   │   ├── nginx/               # Nginx 配置
│   │   │   ├── nginx.conf
│   │   │   └── .well-known/
│   │   ├── monitoring/          # 监控配置
│   │   │   ├── prometheus.yml
│   │   │   ├── alerts.yml
│   │   │   ├── alertmanager.yml
│   │   │   └── grafana/
│   │   └── ssl/                 # SSL 证书
│   │
│   └── simple/                  # 简化配置方案
│       ├── docker-compose.yml
│       ├── .env.example
│       ├── config/
│       ├── nginx/
│       └── ssl/
│
├── Dockerfile                   # 生产环境 Dockerfile
├── Dockerfile.multistage        # 多阶段构建 Dockerfile
└── config/
    └── homeserver.yaml          # 原始配置文件
```

## 快速开始

### 1. 编译应用

```bash
# 在项目根目录执行
cargo build --release
```

### 2. 选择部署方案

#### 完整配置方案（含监控）

```bash
cd docker/deployments/full

# 复制环境变量配置
cp .env.example .env

# 编辑配置
vim .env

# 启动服务
docker compose up -d
```

#### 简化配置方案（无监控）

```bash
cd docker/deployments/simple

# 复制环境变量配置
cp .env.example .env

# 编辑配置
vim .env

# 启动服务
docker compose up -d
```

### 3. 验证部署

```bash
# 检查服务状态
docker compose ps

# 测试 API
curl http://localhost:8008/_matrix/federation/v1/version

# 查看日志
docker compose logs -f synapse-rust
```

## 配置说明

### 环境变量

| 变量名 | 说明 | 默认值 |
|--------|------|--------|
| `SERVER_NAME` | 服务器名称 | `cjystx.top` |
| `SECRET_KEY` | 安全密钥 | - |
| `DB_PASSWORD` | 数据库密码 | `synapse` |
| `REDIS_HOST` | Redis 主机 | `redis` |
| `RUST_LOG` | 日志级别 | `info`/`warn` |

### 监控组件（仅完整配置）

| 服务 | 端口 | 说明 |
|------|------|------|
| Prometheus | 9090 | 指标收集 |
| Grafana | 3000 | 可视化仪表板 |
| AlertManager | 9093 | 告警管理 |
| Node Exporter | 9100 | 系统指标 |
| cAdvisor | 8080 | 容器指标 |

## 切换部署方案

### 从简化配置切换到完整配置

```bash
# 停止简化配置
cd docker/deployments/simple
docker compose down

# 启动完整配置
cd ../full
docker compose up -d
```

### 从完整配置切换到简化配置

```bash
# 停止完整配置
cd docker/deployments/full
docker compose down

# 启动简化配置
cd ../simple
docker compose up -d
```

## SSL 证书配置

1. 将 SSL 证书放置到 `ssl/` 目录：
   ```
   ssl/
   ├── fullchain.pem    # 证书链
   └── privkey.pem      # 私钥
   ```

2. 或使用自签名证书（仅测试）：
   ```bash
   openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
     -keyout ssl/privkey.pem \
     -out ssl/fullchain.pem \
     -subj "/CN=matrix.cjystx.top"
   ```

## 健康检查

所有服务都配置了健康检查：

```bash
# 查看健康状态
docker compose ps

# 手动健康检查
curl -f http://localhost:8008/_matrix/federation/v1/version
```

## 日志管理

```bash
# 查看所有日志
docker compose logs

# 实时跟踪日志
docker compose logs -f synapse-rust

# 查看最近 100 行
docker compose logs --tail=100 synapse-rust
```

## 备份与恢复

### 数据备份

```bash
# 备份 PostgreSQL
docker compose exec db pg_dump -U synapse synapse > backup.sql

# 备份 Redis
docker compose exec redis redis-cli BGSAVE
docker cp synapse-redis:/data/dump.rdb ./redis_backup.rdb
```

### 数据恢复

```bash
# 恢复 PostgreSQL
cat backup.sql | docker compose exec -T db psql -U synapse synapse

# 恢复 Redis
docker cp ./redis_backup.rdb synapse-redis:/data/dump.rdb
docker compose restart redis
```

## 故障排除

### 常见问题

1. **服务无法启动**
   ```bash
   # 检查日志
   docker compose logs synapse-rust
   
   # 检查配置
   docker compose config
   ```

2. **数据库连接失败**
   ```bash
   # 检查数据库状态
   docker compose logs db
   
   # 测试连接
   docker compose exec db pg_isready -U synapse
   ```

3. **监控指标无法访问**
   ```bash
   # 检查 Prometheus 目标状态
   curl http://localhost:9090/api/v1/targets
   ```

## 性能调优

### 资源限制

在 `docker-compose.yml` 中调整资源限制：

```yaml
deploy:
  resources:
    limits:
      memory: 1G
    reservations:
      memory: 256M
```

### 数据库优化

```yaml
command: >
  postgres
  -c shared_buffers=128MB
  -c effective_cache_size=384MB
  -c max_connections=100
```

### Redis 优化

```yaml
command: >
  redis-server
  --maxmemory 256mb
  --maxmemory-policy allkeys-lru
```

## 安全建议

1. **更改默认密码**：修改 `.env` 中的所有密码
2. **启用 HTTPS**：配置有效的 SSL 证书
3. **限制端口暴露**：仅暴露必要的端口
4. **定期更新**：保持镜像和依赖更新
5. **启用防火墙**：限制外部访问

## 参考链接

- [Matrix 协议规范](https://matrix.org/docs/spec/)
- [Docker Compose 文档](https://docs.docker.com/compose/)
- [Prometheus 文档](https://prometheus.io/docs/)
- [Grafana 文档](https://grafana.com/docs/)
