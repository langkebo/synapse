#!/bin/bash
# Synapse2 Docker 容器入口脚本
# 针对低配置服务器优化的启动脚本

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 日志函数
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_debug() {
    if [ "$DEBUG" = "1" ]; then
        echo -e "${BLUE}[DEBUG]${NC} $1"
    fi
}

# 检查必需的环境变量
check_environment() {
    log_info "检查环境变量..."
    
    if [ -z "$SYNAPSE_CONFIG_PATH" ]; then
        export SYNAPSE_CONFIG_PATH="/data/homeserver.yaml"
        log_warn "SYNAPSE_CONFIG_PATH 未设置，使用默认值: $SYNAPSE_CONFIG_PATH"
    fi
    
    if [ -z "$SYNAPSE_SERVER_NAME" ]; then
        export SYNAPSE_SERVER_NAME="localhost"
        log_warn "SYNAPSE_SERVER_NAME 未设置，使用默认值: $SYNAPSE_SERVER_NAME"
    fi
    
    log_info "服务器名称: $SYNAPSE_SERVER_NAME"
    log_info "配置文件路径: $SYNAPSE_CONFIG_PATH"
}

# 创建必需的目录
create_directories() {
    log_info "创建必需的目录..."
    
    directories=(
        "/data/logs"
        "/data/media_store"
        "/data/uploads"
        "/data/keys"
        "/data/signing_keys"
    )
    
    for dir in "${directories[@]}"; do
        if [ ! -d "$dir" ]; then
            mkdir -p "$dir"
            log_info "创建目录: $dir"
        fi
    done
    
    # 设置正确的权限
    chown -R synapse:synapse /data
    chmod -R 755 /data
}

# 生成配置文件
generate_config() {
    if [ ! -f "$SYNAPSE_CONFIG_PATH" ]; then
        log_info "生成 Synapse 配置文件..."
        
        python -m synapse.app.homeserver \
            --server-name="$SYNAPSE_SERVER_NAME" \
            --config-path="$SYNAPSE_CONFIG_PATH" \
            --generate-config \
            --report-stats=no
            
        log_info "配置文件已生成: $SYNAPSE_CONFIG_PATH"
    else
        log_info "配置文件已存在: $SYNAPSE_CONFIG_PATH"
    fi
}

# 生成签名密钥
generate_keys() {
    log_info "检查签名密钥..."
    
    if [ ! -f "/data/signing_keys/signing.key" ]; then
        log_info "生成签名密钥..."
        mkdir -p /data/signing_keys
        python -m synapse.app.homeserver \
            --config-path="$SYNAPSE_CONFIG_PATH" \
            --generate-keys
        log_info "签名密钥已生成"
    else
        log_info "签名密钥已存在"
    fi
}

# 数据库迁移
run_migrations() {
    log_info "运行数据库迁移..."
    
    python -m synapse.app.homeserver \
        --config-path="$SYNAPSE_CONFIG_PATH" \
        --run-migrations
        
    log_info "数据库迁移完成"
}

# 性能优化设置
optimize_performance() {
    log_info "应用性能优化设置..."
    
    # 设置 Python 优化
    export PYTHONOPTIMIZE=1
    export PYTHONHASHSEED=random
    
    # 设置缓存因子
    if [ -z "$SYNAPSE_CACHE_FACTOR" ]; then
        export SYNAPSE_CACHE_FACTOR=0.5
    fi
    
    # 设置工作线程数 (单核优化)
    if [ -z "$SYNAPSE_WORKER_COUNT" ]; then
        export SYNAPSE_WORKER_COUNT=1
    fi
    
    log_info "缓存因子: $SYNAPSE_CACHE_FACTOR"
    log_info "工作线程数: $SYNAPSE_WORKER_COUNT"
}

# 健康检查
health_check() {
    log_info "执行健康检查..."
    
    # 检查配置文件
    if [ ! -f "$SYNAPSE_CONFIG_PATH" ]; then
        log_error "配置文件不存在: $SYNAPSE_CONFIG_PATH"
        exit 1
    fi
    
    # 检查 Python 模块
    if ! python -c "import synapse" 2>/dev/null; then
        log_error "无法导入 Synapse 模块"
        exit 1
    fi
    
    # 检查磁盘空间
    available_space=$(df /data | awk 'NR==2 {print $4}')
    if [ "$available_space" -lt 1048576 ]; then  # 1GB in KB
        log_warn "磁盘空间不足 1GB，可能影响性能"
    fi
    
    log_info "健康检查通过"
}

# 信号处理
handle_signal() {
    log_info "收到停止信号，正在优雅关闭..."
    if [ ! -z "$SYNAPSE_PID" ]; then
        kill -TERM "$SYNAPSE_PID"
        wait "$SYNAPSE_PID"
    fi
    exit 0
}

# 设置信号处理器
trap handle_signal SIGTERM SIGINT

# 主函数
main() {
    log_info "启动 Synapse2 容器..."
    log_info "版本: $(python -c 'import synapse; print(synapse.__version__)' 2>/dev/null || echo 'unknown')"
    
    # 执行初始化步骤
    check_environment
    create_directories
    optimize_performance
    
    # 如果是生成配置模式
    if [ "$1" = "generate" ]; then
        generate_config
        generate_keys
        log_info "配置生成完成"
        exit 0
    fi
    
    # 如果是迁移模式
    if [ "$1" = "migrate" ]; then
        run_migrations
        log_info "数据库迁移完成"
        exit 0
    fi
    
    # 正常启动模式
    generate_config
    generate_keys
    run_migrations
    health_check
    
    log_info "启动 Synapse 服务器..."
    log_info "配置文件: $SYNAPSE_CONFIG_PATH"
    log_info "监听端口: 8008"
    
    # 启动 Synapse
    exec "$@" &
    SYNAPSE_PID=$!
    
    log_info "Synapse 已启动，PID: $SYNAPSE_PID"
    
    # 等待进程结束
    wait "$SYNAPSE_PID"
}

# 如果脚本被直接执行
if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
    main "$@"
fi