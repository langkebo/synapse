#!/bin/bash
# -*- coding: utf-8 -*-

# Synapse Docker 启动脚本
# Synapse Docker Startup Script
#
# 用于在 Docker 容器内启动 Synapse 服务
# Used to start Synapse service inside Docker container
#
# 功能 (Features):
# - 环境检查和配置生成 (Environment check and configuration generation)
# - 数据库初始化和迁移 (Database initialization and migration)
# - 缓存预热 (Cache warmup)
# - 服务启动和监控 (Service startup and monitoring)
# - 优雅关闭处理 (Graceful shutdown handling)

set -e  # 遇到错误立即退出 (Exit immediately on error)

# 颜色定义 (Color definitions)
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 配置变量 (Configuration variables)
SYNAPSE_SERVER_NAME="${SYNAPSE_SERVER_NAME:-matrix.example.com}"
SYNAPSE_REPORT_STATS="${SYNAPSE_REPORT_STATS:-no}"
SYNAPSE_CONFIG_DIR="${SYNAPSE_CONFIG_DIR:-/data}"
SYNAPSE_CONFIG_PATH="${SYNAPSE_CONFIG_PATH:-/data/homeserver.yaml}"
SYNAPSE_DATA_DIR="${SYNAPSE_DATA_DIR:-/data}"
SYNAPSE_LOG_CONFIG="${SYNAPSE_LOG_CONFIG:-/data/log.config}"

# 数据库配置 (Database configuration)
POSTGRES_HOST="${POSTGRES_HOST:-postgres}"
POSTGRES_PORT="${POSTGRES_PORT:-5432}"
POSTGRES_DB="${POSTGRES_DB:-synapse}"
POSTGRES_USER="${POSTGRES_USER:-synapse_user}"
POSTGRES_PASSWORD="${POSTGRES_PASSWORD:-}"

# Redis 配置 (Redis configuration)
REDIS_HOST="${REDIS_HOST:-redis}"
REDIS_PORT="${REDIS_PORT:-6379}"
REDIS_PASSWORD="${REDIS_PASSWORD:-}"

# 性能配置 (Performance configuration)
SYNAPSE_CACHE_FACTOR="${SYNAPSE_CACHE_FACTOR:-0.5}"
SYNAPSE_EVENT_CACHE_SIZE="${SYNAPSE_EVENT_CACHE_SIZE:-5K}"
SYNAPSE_GLOBAL_CACHE_FACTOR="${SYNAPSE_GLOBAL_CACHE_FACTOR:-0.5}"

# 日志函数 (Logging functions)
log_info() {
    echo -e "${GREEN}[INFO]${NC} $(date '+%Y-%m-%d %H:%M:%S') - $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $(date '+%Y-%m-%d %H:%M:%S') - $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $(date '+%Y-%m-%d %H:%M:%S') - $1"
}

log_debug() {
    if [[ "${DEBUG:-false}" == "true" ]]; then
        echo -e "${BLUE}[DEBUG]${NC} $(date '+%Y-%m-%d %H:%M:%S') - $1"
    fi
}

# 信号处理函数 (Signal handling functions)
cleanup() {
    log_info "接收到终止信号，正在优雅关闭... (Received termination signal, gracefully shutting down...)"
    
    # 停止 Synapse 进程 (Stop Synapse process)
    if [[ -n "$SYNAPSE_PID" ]]; then
        log_info "停止 Synapse 进程 (Stopping Synapse process) (PID: $SYNAPSE_PID)"
        kill -TERM "$SYNAPSE_PID" 2>/dev/null || true
        
        # 等待进程退出 (Wait for process to exit)
        local count=0
        while kill -0 "$SYNAPSE_PID" 2>/dev/null && [[ $count -lt 30 ]]; do
            sleep 1
            ((count++))
        done
        
        if kill -0 "$SYNAPSE_PID" 2>/dev/null; then
            log_warn "进程未响应 TERM 信号，发送 KILL 信号 (Process not responding to TERM signal, sending KILL signal)"
            kill -KILL "$SYNAPSE_PID" 2>/dev/null || true
        fi
    fi
    
    log_info "容器关闭完成 (Container shutdown completed)"
    exit 0
}

# 设置信号处理 (Set up signal handling)
trap cleanup SIGTERM SIGINT SIGQUIT

# 等待服务就绪 (Wait for service to be ready)
wait_for_service() {
    local host="$1"
    local port="$2"
    local service_name="$3"
    local timeout="${4:-60}"
    
    log_info "等待 $service_name 服务就绪... (Waiting for $service_name service to be ready...)"
    
    local count=0
    while ! nc -z "$host" "$port" 2>/dev/null; do
        if [[ $count -ge $timeout ]]; then
            log_error "等待 $service_name 服务超时 (Timeout waiting for $service_name service)"
            return 1
        fi
        sleep 1
        ((count++))
    done
    
    log_info "$service_name 服务已就绪 ($service_name service is ready)"
    return 0
}

# 检查必要的环境变量 (Check required environment variables)
check_environment() {
    log_info "检查环境变量 (Checking environment variables)"
    
    if [[ -z "$SYNAPSE_SERVER_NAME" ]]; then
        log_error "SYNAPSE_SERVER_NAME 环境变量未设置 (SYNAPSE_SERVER_NAME environment variable not set)"
        return 1
    fi
    
    if [[ -z "$POSTGRES_PASSWORD" ]]; then
        log_error "POSTGRES_PASSWORD 未设置，出于安全考虑拒绝启动 (POSTGRES_PASSWORD not set; refusing to start for security)"
        return 1
    fi
    
    log_debug "服务器名称: $SYNAPSE_SERVER_NAME (Server name: $SYNAPSE_SERVER_NAME)"
    log_debug "数据库主机: $POSTGRES_HOST:$POSTGRES_PORT (Database host: $POSTGRES_HOST:$POSTGRES_PORT)"
    log_debug "Redis 主机: $REDIS_HOST:$REDIS_PORT (Redis host: $REDIS_HOST:$REDIS_PORT)"
    
    return 0
}

# 生成配置文件 (Generate configuration file)
generate_config() {
    log_info "生成 Synapse 配置文件 (Generating Synapse configuration file)"
    
    # 如果配置文件已存在，备份它 (If config file exists, backup it)
    if [[ -f "$SYNAPSE_CONFIG_PATH" ]]; then
        log_debug "备份现有配置文件 (Backing up existing configuration file)"
        cp "$SYNAPSE_CONFIG_PATH" "${SYNAPSE_CONFIG_PATH}.backup.$(date +%s)"
    fi
    
    # 生成基础配置 (Generate base configuration)
    if [[ ! -f "$SYNAPSE_CONFIG_PATH" ]]; then
        log_info "生成新的配置文件 (Generating new configuration file)"
        python -m synapse.app.homeserver \
            --server-name="$SYNAPSE_SERVER_NAME" \
            --config-path="$SYNAPSE_CONFIG_PATH" \
            --generate-config \
            --report-stats="$SYNAPSE_REPORT_STATS" \
            --data-directory="$SYNAPSE_DATA_DIR"
    fi
    
    # 更新数据库配置 (Update database configuration)
    log_debug "更新数据库配置 (Updating database configuration)"
    python3 << EOF
import yaml
import sys

try:
    with open('$SYNAPSE_CONFIG_PATH', 'r') as f:
        config = yaml.safe_load(f)
    
    # 数据库配置 (Database configuration)
    config['database'] = {
        'name': 'psycopg2',
        'args': {
            'user': '$POSTGRES_USER',
            'password': '$POSTGRES_PASSWORD',
            'database': '$POSTGRES_DB',
            'host': '$POSTGRES_HOST',
            'port': $POSTGRES_PORT,
            'cp_min': 5,
            'cp_max': 10,
            'cp_reconnect': True,
            'cp_openfun': 'synapse.storage.engines.postgres.on_new_connection'
        }
    }
    
    # Redis 配置 (Redis configuration)
    redis_config = {
        'enabled': True,
        'host': '$REDIS_HOST',
        'port': $REDIS_PORT,
        'dbid': 0
    }
    if '$REDIS_PASSWORD':
        redis_config['password'] = '$REDIS_PASSWORD'
    
    config['redis'] = redis_config
    
    # 缓存配置 (Cache configuration)
    config['caches'] = {
        'global_factor': float('$SYNAPSE_GLOBAL_CACHE_FACTOR'),
        'per_cache_factors': {
            'get_users_who_share_room_with_user': 2.0,
            'get_users_in_room': 2.0,
            'get_room_summary': 2.0
        },
        'event_cache_size': '$SYNAPSE_EVENT_CACHE_SIZE',
        'cache_autotuning': {
            'max_cache_memory_usage': '512M',
            'target_cache_memory_usage': '256M',
            'min_cache_ttl': '5m'
        }
    }
    
    # 性能优化配置 (Performance optimization configuration)
    config['performance'] = {
        'database': {
            'connection_pool': {
                'min_size': 5,
                'max_size': 10,
                'max_overflow': 5,
                'pool_timeout': 30,
                'pool_recycle': 3600
            },
            'query_optimization': {
                'enable_query_cache': True,
                'query_cache_size': 1000,
                'enable_prepared_statements': True,
                'statement_timeout': 30
            }
        },
        'memory': {
            'gc_thresholds': [700, 10, 10],
            'gc_min_interval': 1.0,
            'enable_memory_profiling': False
        },
        'network': {
            'tcp_keepalive': True,
            'tcp_keepalive_idle': 600,
            'tcp_keepalive_interval': 60,
            'tcp_keepalive_count': 3,
            'connection_timeout': 30,
            'read_timeout': 60
        }
    }
    
    # 好友功能配置 (Friends feature configuration)
    config['friends'] = {
        'enabled': True,
        'max_friends_per_user': 1000,
        'friend_request_timeout': 2592000,  # 30 days
        'enable_friend_recommendations': True,
        'recommendation_limit': 50,
        'enable_online_status': True,
        'online_status_timeout': 300,  # 5 minutes
        'cache_ttl': 3600  # 1 hour
    }
    
    # 日志配置 (Logging configuration)
    config['log_config'] = '$SYNAPSE_LOG_CONFIG'
    
    # 媒体存储配置 (Media storage configuration)
    config['media_store_path'] = '$SYNAPSE_DATA_DIR/media_store'
    config['max_upload_size'] = '50M'
    config['max_image_pixels'] = '32M'
    
    # 速率限制配置 (Rate limiting configuration)
    config['rc_message'] = {
        'per_second': 0.2,
        'burst_count': 10
    }
    config['rc_registration'] = {
        'per_second': 0.17,
        'burst_count': 3
    }
    config['rc_login'] = {
        'address': {
            'per_second': 0.17,
            'burst_count': 3
        },
        'account': {
            'per_second': 0.17,
            'burst_count': 3
        },
        'failed_attempts': {
            'per_second': 0.17,
            'burst_count': 3
        }
    }
    
    # 联邦配置 (Federation configuration)
    config['federation_domain_whitelist'] = None
    config['federation_ip_range_blacklist'] = [
        '127.0.0.0/8',
        '10.0.0.0/8',
        '172.16.0.0/12',
        '192.168.0.0/16',
        '100.64.0.0/10',
        '169.254.0.0/16',
        '::1/128',
        'fe80::/64',
        'fc00::/7'
    ]
    
    # 写入配置文件 (Write configuration file)
    with open('$SYNAPSE_CONFIG_PATH', 'w') as f:
        yaml.dump(config, f, default_flow_style=False, allow_unicode=True)
    
    print("配置文件更新完成 (Configuration file updated)")
    
except Exception as e:
    print(f"配置文件更新失败: {e} (Configuration file update failed: {e})")
    sys.exit(1)
EOF
    
    if [[ $? -eq 0 ]]; then
        log_info "配置文件生成完成 (Configuration file generation completed)"
    else
        log_error "配置文件生成失败 (Configuration file generation failed)"
        return 1
    fi
    
    return 0
}

# 生成日志配置 (Generate log configuration)
generate_log_config() {
    log_info "生成日志配置文件 (Generating log configuration file)"
    
    cat > "$SYNAPSE_LOG_CONFIG" << 'EOF'
version: 1

formatters:
  precise:
    format: '%(asctime)s - %(name)s - %(lineno)d - %(levelname)s - %(request)s - %(message)s'
  brief:
    format: '%(asctime)s - %(levelname)s - %(message)s'

handlers:
  file:
    class: logging.handlers.TimedRotatingFileHandler
    formatter: precise
    filename: /var/log/synapse/homeserver.log
    when: midnight
    interval: 1
    backupCount: 7
    encoding: utf8
  
  console:
    class: logging.StreamHandler
    formatter: brief
    stream: ext://sys.stdout

loggers:
  synapse.storage.SQL:
    level: WARN
  synapse.access.http.8008:
    level: INFO
  synapse.federation.transport.server:
    level: WARN
  synapse.handlers.friends:
    level: INFO
  synapse.util.caches:
    level: WARN

root:
  level: INFO
  handlers: [file, console]

disable_existing_loggers: false
EOF
    
    log_debug "日志配置文件生成完成 (Log configuration file generation completed)"
}

# 初始化数据库 (Initialize database)
init_database() {
    log_info "初始化数据库 (Initializing database)"
    
    # 等待数据库就绪 (Wait for database to be ready)
    if ! wait_for_service "$POSTGRES_HOST" "$POSTGRES_PORT" "PostgreSQL" 120; then
        return 1
    fi
    
    # 运行数据库迁移 (Run database migrations)
    log_debug "运行数据库迁移 (Running database migrations)"
    python -m synapse.app.homeserver \
        --config-path="$SYNAPSE_CONFIG_PATH" \
        --run-background-updates
    
    if [[ $? -eq 0 ]]; then
        log_info "数据库初始化完成 (Database initialization completed)"
    else
        log_error "数据库初始化失败 (Database initialization failed)"
        return 1
    fi
    
    return 0
}

# 执行缓存预热 (Execute cache warmup)
execute_warmup() {
    log_info "开始缓存预热 (Starting cache warmup)"
    
    # 等待 Redis 就绪 (Wait for Redis to be ready)
    if ! wait_for_service "$REDIS_HOST" "$REDIS_PORT" "Redis" 60; then
        log_warn "Redis 不可用，跳过缓存预热 (Redis not available, skipping cache warmup)"
        return 0
    fi
    
    # 执行缓存预热脚本 (Execute cache warmup script)
    if [[ -f "/scripts/cache_warmup.py" ]]; then
        log_debug "执行缓存预热脚本 (Executing cache warmup script)"
        python /scripts/cache_warmup.py \
            --config "$SYNAPSE_CONFIG_PATH" \
            --quiet \
            --batch-size 100 \
            --max-users 1000
        
        if [[ $? -eq 0 ]]; then
            log_info "缓存预热完成 (Cache warmup completed)"
        else
            log_warn "缓存预热失败，但继续启动服务 (Cache warmup failed, but continuing to start service)"
        fi
    else
        log_warn "缓存预热脚本不存在 (Cache warmup script not found)"
    fi
    
    return 0
}

# 启动 Synapse 服务 (Start Synapse service)
start_synapse() {
    log_info "启动 Synapse 服务 (Starting Synapse service)"
    
    # 创建必要的目录 (Create necessary directories)
    mkdir -p "$SYNAPSE_DATA_DIR" /var/log/synapse /var/run/synapse
    
    # 启动 Synapse (Start Synapse)
    log_debug "执行 Synapse 主进程 (Executing Synapse main process)"
    exec python -m synapse.app.homeserver \
        --config-path="$SYNAPSE_CONFIG_PATH" &
    
    SYNAPSE_PID=$!
    log_info "Synapse 服务已启动 (Synapse service started) (PID: $SYNAPSE_PID)"
    
    # 等待进程结束 (Wait for process to end)
    wait $SYNAPSE_PID
}

# 主函数 (Main function)
main() {
    log_info "Synapse Docker 容器启动 (Synapse Docker container startup)"
    
    # 检查环境 (Check environment)
    if ! check_environment; then
        log_error "环境检查失败 (Environment check failed)"
        exit 1
    fi
    
    # 生成配置文件 (Generate configuration files)
    if ! generate_config; then
        log_error "配置文件生成失败 (Configuration file generation failed)"
        exit 1
    fi
    
    generate_log_config
    
    # 初始化数据库 (Initialize database)
    if ! init_database; then
        log_error "数据库初始化失败 (Database initialization failed)"
        exit 1
    fi
    
    # 执行缓存预热 (Execute cache warmup)
    execute_warmup
    
    # 启动服务 (Start service)
    start_synapse
}

# 脚本入口 (Script entry point)
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi