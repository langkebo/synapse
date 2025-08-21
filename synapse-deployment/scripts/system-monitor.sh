#!/bin/bash

# 系统监控脚本
# 实时监控Synapse Matrix服务器和好友功能的系统性能
# 适用于1核2GB服务器环境
# 作者: Synapse开发团队
# 版本: 1.0.0
# 日期: 2024年

set -euo pipefail

# ============================================================================
# 全局变量和配置
# ============================================================================

# 脚本信息
SCRIPT_NAME="Synapse系统监控脚本"
SCRIPT_VERSION="1.0.0"
SCRIPT_DATE="2024年"

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
WHITE='\033[1;37m'
NC='\033[0m' # No Color

# 配置文件
CONFIG_FILE="/opt/synapse/config/monitor.conf"
LOG_DIR="/var/log/synapse"
DATA_DIR="/opt/synapse/data/monitoring"
ALERT_LOG="$LOG_DIR/alerts.log"
METRICS_LOG="$LOG_DIR/metrics.log"
PERF_LOG="$LOG_DIR/performance.log"

# 监控间隔（秒）
MONITOR_INTERVAL=5
ALERT_INTERVAL=60
REPORT_INTERVAL=300

# 告警阈值
CPU_THRESHOLD=80
MEMORY_THRESHOLD=85
DISK_THRESHOLD=85
LOAD_THRESHOLD=2.0
CONN_THRESHOLD=500
RESPONSE_TIME_THRESHOLD=5000

# 服务列表
SERVICES=("synapse" "postgresql" "redis" "nginx")
DOCKER_CONTAINERS=("synapse" "postgres" "redis" "nginx" "grafana" "prometheus")

# API端点
SYNAPSE_API="http://localhost:8008"
HEALTH_ENDPOINTS=(
    "$SYNAPSE_API/_matrix/client/versions"
    "$SYNAPSE_API/_synapse/admin/v1/server_version"
    "http://localhost:3000/api/health"  # Grafana
    "http://localhost:9090/-/healthy"   # Prometheus
)

# ============================================================================
# 工具函数
# ============================================================================

# 日志函数
log() {
    local level=$1
    shift
    local message="$*"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    
    case $level in
        "INFO")
            echo -e "${GREEN}[INFO]${NC} ${timestamp} - $message"
            ;;
        "WARN")
            echo -e "${YELLOW}[WARN]${NC} ${timestamp} - $message"
            echo "[$timestamp] WARN: $message" >> "$ALERT_LOG"
            ;;
        "ERROR")
            echo -e "${RED}[ERROR]${NC} ${timestamp} - $message"
            echo "[$timestamp] ERROR: $message" >> "$ALERT_LOG"
            ;;
        "ALERT")
            echo -e "${RED}[ALERT]${NC} ${timestamp} - $message"
            echo "[$timestamp] ALERT: $message" >> "$ALERT_LOG"
            ;;
        "SUCCESS")
            echo -e "${GREEN}[SUCCESS]${NC} ${timestamp} - $message"
            ;;
        "DEBUG")
            if [[ "${DEBUG:-false}" == "true" ]]; then
                echo -e "${BLUE}[DEBUG]${NC} ${timestamp} - $message"
            fi
            ;;
    esac
}

# 创建目录
create_dir() {
    local dir="$1"
    if [[ ! -d "$dir" ]]; then
        mkdir -p "$dir"
        log "INFO" "创建目录: $dir"
    fi
}

# 检查命令是否存在
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# 获取时间戳
get_timestamp() {
    date '+%Y-%m-%d %H:%M:%S'
}

# 获取Unix时间戳
get_unix_timestamp() {
    date '+%s'
}

# 格式化字节数
format_bytes() {
    local bytes=$1
    if [[ $bytes -ge 1073741824 ]]; then
        echo "$(echo "scale=2; $bytes/1073741824" | bc)GB"
    elif [[ $bytes -ge 1048576 ]]; then
        echo "$(echo "scale=2; $bytes/1048576" | bc)MB"
    elif [[ $bytes -ge 1024 ]]; then
        echo "$(echo "scale=2; $bytes/1024" | bc)KB"
    else
        echo "${bytes}B"
    fi
}

# 发送告警
send_alert() {
    local level="$1"
    local message="$2"
    local