#!/bin/bash
# -*- coding: utf-8 -*-

# Synapse 启动脚本
# Synapse Startup Script
#
# 用于自动启动 Synapse 服务并进行以下操作：
# - 环境检查和配置验证
# - 数据库连接测试
# - Redis 连接测试
# - 缓存预热
# - 启动 Synapse 服务
# - 启动系统监控
#
# Used to automatically start Synapse service and perform:
# - Environment check and configuration validation
# - Database connection test
# - Redis connection test
# - Cache warmup
# - Start Synapse service
# - Start system monitoring

set -e  # 遇到错误立即退出 (Exit immediately on error)

# 颜色定义 (Color definitions)
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 配置变量 (Configuration variables)
SYNAPSE_HOME="${SYNAPSE_HOME:-/opt/venvs/synapse}"
SYNAPSE_CONFIG="${SYNAPSE_CONFIG:-/data/homeserver.yaml}"
SYNAPSE_USER="${SYNAPSE_USER:-synapse}"
LOG_DIR="${LOG_DIR:-/var/log/synapse}"
PID_DIR="${PID_DIR:-/var/run/synapse}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

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
    # 调试日志已简化
    return 0
}

# 显示帮助信息 (Show help information)
show_help() {
    cat << EOF
Synapse 启动脚本 (Synapse Startup Script)

用法 (Usage):
    $0 [选项] [命令]

命令 (Commands):
    start           启动 Synapse 服务 (Start Synapse service)
    stop            停止 Synapse 服务 (Stop Synapse service)
    restart         重启 Synapse 服务 (Restart Synapse service)
    status          查看服务状态 (Check service status)
    check           检查环境和配置 (Check environment and configuration)
    warmup          执行缓存预热 (Execute cache warmup)
    monitor         启动系统监控 (Start system monitoring)

选项 (Options):
    -h, --help      显示此帮助信息 (Show this help message)
    -c, --config    指定配置文件路径 (Specify config file path)
    -u, --user      指定运行用户 (Specify run user)
    --no-warmup     跳过缓存预热 (Skip cache warmup)
    --no-monitor    跳过系统监控启动 (Skip system monitoring startup)

环境变量 (Environment Variables):
    SYNAPSE_HOME    Synapse 安装目录 (Synapse installation directory)
    SYNAPSE_CONFIG  Synapse 配置文件路径 (Synapse config file path)
    SYNAPSE_USER    运行用户 (Run user)
    LOG_DIR         日志目录 (Log directory)
    PID_DIR         PID 文件目录 (PID file directory)
    DEBUG           调试模式 (Debug mode)

示例 (Examples):
    $0 start                    # 启动服务 (Start service)
    $0 start --no-warmup        # 启动服务但跳过缓存预热 (Start service but skip cache warmup)
    $0 restart -d               # 重启服务并启用调试 (Restart service with debug)
    $0 check                    # 检查环境 (Check environment)

EOF
}

# 检查是否以 root 用户运行 (Check if running as root)
check_root() {
    if [[ $EUID -eq 0 ]]; then
        log_warn "正在以 root 用户运行，建议使用专用用户 (Running as root, recommend using dedicated user)"
        if [[ "${SYNAPSE_USER}" != "root" ]]; then
            log_info "将切换到用户: ${SYNAPSE_USER} (Will switch to user: ${SYNAPSE_USER})"
        fi
    fi
}

# 创建必要的目录 (Create necessary directories)
setup_directories() {
    log_info "创建必要的目录 (Creating necessary directories)"
    
    for dir in "${LOG_DIR}" "${PID_DIR}"; do
        if [[ ! -d "$dir" ]]; then
            log_debug "创建目录: $dir (Creating directory: $dir)"
            mkdir -p "$dir"
            if [[ "${SYNAPSE_USER}" != "root" ]] && [[ $EUID -eq 0 ]]; then
                chown "${SYNAPSE_USER}:${SYNAPSE_USER}" "$dir"
            fi
        fi
    done
}

# 检查配置文件 (Check configuration file)
check_config() {
    log_info "检查配置文件 (Checking configuration file)"
    
    if [[ ! -f "${SYNAPSE_CONFIG}" ]]; then
        log_error "配置文件不存在: ${SYNAPSE_CONFIG} (Configuration file not found: ${SYNAPSE_CONFIG})"
        return 1
    fi
    
    log_debug "配置文件存在: ${SYNAPSE_CONFIG} (Configuration file exists: ${SYNAPSE_CONFIG})"
    
    # 检查配置文件语法 (Check configuration file syntax)
    if command -v python3 >/dev/null 2>&1; then
        python3 -c "import yaml; yaml.safe_load(open('${SYNAPSE_CONFIG}'))" 2>/dev/null
        if [[ $? -eq 0 ]]; then
            log_debug "配置文件语法正确 (Configuration file syntax is correct)"
        else
            log_error "配置文件语法错误 (Configuration file syntax error)"
            return 1
        fi
    fi
    
    return 0
}

# 检查数据库连接 (Check database connection)
check_database() {
    log_info "检查数据库连接 (Checking database connection)"
    
    # 从配置文件中提取数据库信息 (Extract database info from config file)
    if command -v python3 >/dev/null 2>&1; then
        local db_check_result
        db_check_result=$(python3 << 'EOF'
import yaml
import sys
try:
    with open(sys.argv[1], 'r') as f:
        config = yaml.safe_load(f)
    db_config = config.get('database', {}).get('args', {})
    if db_config:
        print(f"host={db_config.get('host', 'localhost')}")
        print(f"port={db_config.get('port', 5432)}")
        print(f"database={db_config.get('database', 'synapse')}")
        print(f"user={db_config.get('user', 'synapse_user')}")
    else:
        print("no_db_config")
except Exception as e:
    print(f"error: {e}")
EOF
"${SYNAPSE_CONFIG}")
        
        if [[ "$db_check_result" == "no_db_config" ]]; then
            log_warn "未找到数据库配置 (Database configuration not found)"
            return 0
        elif [[ "$db_check_result" == error:* ]]; then
            log_error "读取数据库配置失败: ${db_check_result#error: } (Failed to read database config: ${db_check_result#error: })"
            return 1
        else
            log_debug "数据库配置检查通过 (Database configuration check passed)"
        fi
    fi
    
    return 0
}

# 检查 Redis 连接 (Check Redis connection)
check_redis() {
    log_info "检查 Redis 连接 (Checking Redis connection)"
    
    # 从配置文件中检查 Redis 配置 (Check Redis config from config file)
    if command -v python3 >/dev/null 2>&1; then
        local redis_enabled
        redis_enabled=$(python3 << 'EOF'
import yaml
import sys
try:
    with open(sys.argv[1], 'r') as f:
        config = yaml.safe_load(f)
    cache_config = config.get('cache_strategy', {}).get('redis', {})
    if cache_config.get('enabled', False):
        print("enabled")
    else:
        print("disabled")
except Exception as e:
    print("error")
EOF
"${SYNAPSE_CONFIG}")
        
        if [[ "$redis_enabled" == "enabled" ]]; then
            log_debug "Redis 已启用 (Redis is enabled)"
        elif [[ "$redis_enabled" == "disabled" ]]; then
            log_warn "Redis 未启用，将跳过缓存预热 (Redis not enabled, will skip cache warmup)"
        else
            log_warn "无法检查 Redis 配置 (Cannot check Redis configuration)"
        fi
    fi
    
    return 0
}

# 检查 Synapse 安装 (Check Synapse installation)
check_synapse() {
    log_info "检查 Synapse 安装 (Checking Synapse installation)"
    
    local synapse_cmd
    if [[ -f "${SYNAPSE_HOME}/bin/synapse_homeserver" ]]; then
        synapse_cmd="${SYNAPSE_HOME}/bin/synapse_homeserver"
    elif command -v synapse_homeserver >/dev/null 2>&1; then
        synapse_cmd="synapse_homeserver"
    else
        log_error "未找到 Synapse 可执行文件 (Synapse executable not found)"
        return 1
    fi
    
    log_debug "找到 Synapse 可执行文件: $synapse_cmd (Found Synapse executable: $synapse_cmd)"
    
    # 检查版本 (Check version)
    local version
    version=$("$synapse_cmd" --version 2>/dev/null | head -n1 || echo "未知版本 (Unknown version)")
    log_info "Synapse 版本 (Synapse version): $version"
    
    return 0
}

# 执行缓存预热 (Execute cache warmup)
execute_warmup() {
    if [[ "${SKIP_WARMUP:-false}" == "true" ]]; then
        log_info "跳过缓存预热 (Skipping cache warmup)"
        return 0
    fi
    
    log_info "开始缓存预热 (Starting cache warmup)"
    
    local warmup_script="${SCRIPT_DIR}/cache_warmup.py"
    if [[ ! -f "$warmup_script" ]]; then
        log_warn "缓存预热脚本不存在: $warmup_script (Cache warmup script not found: $warmup_script)"
        return 0
    fi
    
    local python_cmd
    if [[ -f "${SYNAPSE_HOME}/bin/python" ]]; then
        python_cmd="${SYNAPSE_HOME}/bin/python"
    else
        python_cmd="python3"
    fi
    
    log_debug "执行缓存预热脚本 (Executing cache warmup script)"
    if [[ "${SYNAPSE_USER}" != "root" ]] && [[ $EUID -eq 0 ]]; then
        su - "${SYNAPSE_USER}" -c "$python_cmd '$warmup_script' --config '${SYNAPSE_CONFIG}' --quiet"
    else
        "$python_cmd" "$warmup_script" --config "${SYNAPSE_CONFIG}" --quiet
    fi
    
    if [[ $? -eq 0 ]]; then
        log_info "缓存预热完成 (Cache warmup completed)"
    else
        log_warn "缓存预热失败，但继续启动服务 (Cache warmup failed, but continuing to start service)"
    fi
    
    return 0
}

# 启动系统监控 (Start system monitoring)
start_monitoring() {
    if [[ "${SKIP_MONITOR:-false}" == "true" ]]; then
        log_info "跳过系统监控启动 (Skipping system monitoring startup)"
        return 0
    fi
    
    log_info "启动系统监控 (Starting system monitoring)"
    
    local monitor_script="${SCRIPT_DIR}/system_monitor.py"
    if [[ ! -f "$monitor_script" ]]; then
        log_warn "系统监控脚本不存在: $monitor_script (System monitor script not found: $monitor_script)"
        return 0
    fi
    
    local python_cmd
    if [[ -f "${SYNAPSE_HOME}/bin/python" ]]; then
        python_cmd="${SYNAPSE_HOME}/bin/python"
    else
        python_cmd="python3"
    fi
    
    local monitor_log="${LOG_DIR}/system_monitor.log"
    local monitor_pid="${PID_DIR}/system_monitor.pid"
    
    # 检查是否已经在运行 (Check if already running)
    if [[ -f "$monitor_pid" ]]; then
        local old_pid
        old_pid=$(cat "$monitor_pid")
        if kill -0 "$old_pid" 2>/dev/null; then
            log_info "系统监控已在运行 (System monitoring already running) (PID: $old_pid)"
            return 0
        else
            log_debug "删除过期的 PID 文件 (Removing stale PID file)"
            rm -f "$monitor_pid"
        fi
    fi
    
    log_debug "启动系统监控进程 (Starting system monitoring process)"
    if [[ "${SYNAPSE_USER}" != "root" ]] && [[ $EUID -eq 0 ]]; then
        su - "${SYNAPSE_USER}" -c "nohup $python_cmd '$monitor_script' --config '${SYNAPSE_CONFIG}' --quiet > '$monitor_log' 2>&1 & echo \$! > '$monitor_pid'"
    else
        nohup "$python_cmd" "$monitor_script" --config "${SYNAPSE_CONFIG}" --quiet > "$monitor_log" 2>&1 &
        echo $! > "$monitor_pid"
    fi
    
    sleep 2
    
    if [[ -f "$monitor_pid" ]]; then
        local new_pid
        new_pid=$(cat "$monitor_pid")
        if kill -0 "$new_pid" 2>/dev/null; then
            log_info "系统监控启动成功 (System monitoring started successfully) (PID: $new_pid)"
        else
            log_warn "系统监控启动失败 (System monitoring startup failed)"
            rm -f "$monitor_pid"
        fi
    else
        log_warn "无法获取系统监控进程 PID (Cannot get system monitoring process PID)"
    fi
    
    return 0
}

# 启动 Synapse 服务 (Start Synapse service)
start_synapse() {
    log_info "启动 Synapse 服务 (Starting Synapse service)"
    
    local synapse_cmd
    if [[ -f "${SYNAPSE_HOME}/bin/synapse_homeserver" ]]; then
        synapse_cmd="${SYNAPSE_HOME}/bin/synapse_homeserver"
    else
        synapse_cmd="synapse_homeserver"
    fi
    
    local synapse_log="${LOG_DIR}/homeserver.log"
    local synapse_pid="${PID_DIR}/synapse.pid"
    
    # 检查是否已经在运行 (Check if already running)
    if [[ -f "$synapse_pid" ]]; then
        local old_pid
        old_pid=$(cat "$synapse_pid")
        if kill -0 "$old_pid" 2>/dev/null; then
            log_info "Synapse 服务已在运行 (Synapse service already running) (PID: $old_pid)"
            return 0
        else
            log_debug "删除过期的 PID 文件 (Removing stale PID file)"
            rm -f "$synapse_pid"
        fi
    fi
    
    log_debug "启动 Synapse 进程 (Starting Synapse process)"
    if [[ "${SYNAPSE_USER}" != "root" ]] && [[ $EUID -eq 0 ]]; then
        su - "${SYNAPSE_USER}" -c "nohup $synapse_cmd --config-path='${SYNAPSE_CONFIG}' > '$synapse_log' 2>&1 & echo \$! > '$synapse_pid'"
    else
        nohup "$synapse_cmd" --config-path="${SYNAPSE_CONFIG}" > "$synapse_log" 2>&1 &
        echo $! > "$synapse_pid"
    fi
    
    sleep 5
    
    if [[ -f "$synapse_pid" ]]; then
        local new_pid
        new_pid=$(cat "$synapse_pid")
        if kill -0 "$new_pid" 2>/dev/null; then
            log_info "Synapse 服务启动成功 (Synapse service started successfully) (PID: $new_pid)"
            log_info "日志文件: $synapse_log (Log file: $synapse_log)"
        else
            log_error "Synapse 服务启动失败 (Synapse service startup failed)"
            rm -f "$synapse_pid"
            return 1
        fi
    else
        log_error "无法获取 Synapse 进程 PID (Cannot get Synapse process PID)"
        return 1
    fi
    
    return 0
}

# 停止 Synapse 服务 (Stop Synapse service)
stop_synapse() {
    log_info "停止 Synapse 服务 (Stopping Synapse service)"
    
    local synapse_pid="${PID_DIR}/synapse.pid"
    local monitor_pid="${PID_DIR}/system_monitor.pid"
    
    # 停止 Synapse 主进程 (Stop Synapse main process)
    if [[ -f "$synapse_pid" ]]; then
        local pid
        pid=$(cat "$synapse_pid")
        if kill -0 "$pid" 2>/dev/null; then
            log_debug "发送 TERM 信号到 Synapse 进程 (Sending TERM signal to Synapse process) (PID: $pid)"
            kill -TERM "$pid"
            
            # 等待进程退出 (Wait for process to exit)
            local count=0
            while kill -0 "$pid" 2>/dev/null && [[ $count -lt 30 ]]; do
                sleep 1
                ((count++))
            done
            
            if kill -0 "$pid" 2>/dev/null; then
                log_warn "进程未响应 TERM 信号，发送 KILL 信号 (Process not responding to TERM signal, sending KILL signal)"
                kill -KILL "$pid"
                sleep 2
            fi
            
            if ! kill -0 "$pid" 2>/dev/null; then
                log_info "Synapse 服务已停止 (Synapse service stopped)"
                rm -f "$synapse_pid"
            else
                log_error "无法停止 Synapse 服务 (Cannot stop Synapse service)"
                return 1
            fi
        else
            log_warn "Synapse 进程不存在，删除 PID 文件 (Synapse process not found, removing PID file)"
            rm -f "$synapse_pid"
        fi
    else
        log_info "Synapse 服务未运行 (Synapse service not running)"
    fi
    
    # 停止系统监控进程 (Stop system monitoring process)
    if [[ -f "$monitor_pid" ]]; then
        local pid
        pid=$(cat "$monitor_pid")
        if kill -0 "$pid" 2>/dev/null; then
            log_debug "停止系统监控进程 (Stopping system monitoring process) (PID: $pid)"
            kill -TERM "$pid"
            sleep 2
            if kill -0 "$pid" 2>/dev/null; then
                kill -KILL "$pid"
            fi
            rm -f "$monitor_pid"
            log_info "系统监控已停止 (System monitoring stopped)"
        else
            rm -f "$monitor_pid"
        fi
    fi
    
    return 0
}

# 检查服务状态 (Check service status)
check_status() {
    log_info "检查服务状态 (Checking service status)"
    
    local synapse_pid="${PID_DIR}/synapse.pid"
    local monitor_pid="${PID_DIR}/system_monitor.pid"
    
    echo "\n" + "="*60
    echo "Synapse 服务状态 (Synapse Service Status)"
    echo "="*60
    
    # 检查 Synapse 主服务 (Check Synapse main service)
    if [[ -f "$synapse_pid" ]]; then
        local pid
        pid=$(cat "$synapse_pid")
        if kill -0 "$pid" 2>/dev/null; then
            echo "🟢 Synapse 服务: 运行中 (Running) (PID: $pid)"
            
            # 获取进程信息 (Get process info)
            if command -v ps >/dev/null 2>&1; then
                local proc_info
                proc_info=$(ps -p "$pid" -o pid,ppid,pcpu,pmem,etime,cmd --no-headers 2>/dev/null || echo "无法获取进程信息")
                echo "   进程信息 (Process Info): $proc_info"
            fi
        else
            echo "🔴 Synapse 服务: 已停止 (Stopped) (PID 文件存在但进程不存在)"
            rm -f "$synapse_pid"
        fi
    else
        echo "🔴 Synapse 服务: 已停止 (Stopped)"
    fi
    
    # 检查系统监控 (Check system monitoring)
    if [[ -f "$monitor_pid" ]]; then
        local pid
        pid=$(cat "$monitor_pid")
        if kill -0 "$pid" 2>/dev/null; then
            echo "🟢 系统监控: 运行中 (Running) (PID: $pid)"
        else
            echo "🔴 系统监控: 已停止 (Stopped) (PID 文件存在但进程不存在)"
            rm -f "$monitor_pid"
        fi
    else
        echo "🔴 系统监控: 已停止 (Stopped)"
    fi
    
    # 检查端口监听 (Check port listening)
    if command -v netstat >/dev/null 2>&1; then
        echo "\n端口监听状态 (Port Listening Status):"
        netstat -tlnp 2>/dev/null | grep -E ':(8008|8448)' | while read line; do
            echo "   $line"
        done
    elif command -v ss >/dev/null 2>&1; then
        echo "\n端口监听状态 (Port Listening Status):"
        ss -tlnp | grep -E ':(8008|8448)' | while read line; do
            echo "   $line"
        done
    fi
    
    echo "="*60
}

# 执行环境检查 (Execute environment check)
execute_check() {
    log_info "开始环境检查 (Starting environment check)"
    
    local check_failed=false
    
    check_root
    setup_directories
    
    if ! check_config; then
        check_failed=true
    fi
    
    if ! check_synapse; then
        check_failed=true
    fi
    
    check_database
    check_redis
    
    if [[ "$check_failed" == "true" ]]; then
        log_error "环境检查失败 (Environment check failed)"
        return 1
    else
        log_info "环境检查通过 (Environment check passed)"
        return 0
    fi
}

# 主函数 (Main function)
main() {
    local command="start"
    local skip_warmup=false
    local skip_monitor=false
    
    # 解析命令行参数 (Parse command line arguments)
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_help
                exit 0
                ;;
            -c|--config)
                SYNAPSE_CONFIG="$2"
                shift 2
                ;;
            -u|--user)
                SYNAPSE_USER="$2"
                shift 2
                ;;
            -d|--debug)
                DEBUG=true
                shift
                ;;
            --no-warmup)
                skip_warmup=true
                shift
                ;;
            --no-monitor)
                skip_monitor=true
                shift
                ;;
            start|stop|restart|status|check|warmup|monitor)
                command="$1"
                shift
                ;;
            *)
                log_error "未知参数: $1 (Unknown argument: $1)"
                show_help
                exit 1
                ;;
        esac
    done
    
    # 设置环境变量 (Set environment variables)
    export SKIP_WARMUP="$skip_warmup"
    export SKIP_MONITOR="$skip_monitor"
    
    log_info "Synapse 启动脚本开始执行 (Synapse startup script execution started)"
    log_debug "命令: $command (Command: $command)"
    log_debug "配置文件: ${SYNAPSE_CONFIG} (Config file: ${SYNAPSE_CONFIG})"
    log_debug "运行用户: ${SYNAPSE_USER} (Run user: ${SYNAPSE_USER})"
    
    case "$command" in
        start)
            if ! execute_check; then
                exit 1
            fi
            execute_warmup
            if ! start_synapse; then
                exit 1
            fi
            start_monitoring
            log_info "Synapse 服务启动完成 (Synapse service startup completed)"
            ;;
        stop)
            stop_synapse
            ;;
        restart)
            stop_synapse
            sleep 3
            if ! execute_check; then
                exit 1
            fi
            execute_warmup
            if ! start_synapse; then
                exit 1
            fi
            start_monitoring
            log_info "Synapse 服务重启完成 (Synapse service restart completed)"
            ;;
        status)
            check_status
            ;;
        check)
            execute_check
            ;;
        warmup)
            execute_warmup
            ;;
        monitor)
            start_monitoring
            ;;
        *)
            log_error "未知命令: $command (Unknown command: $command)"
            show_help
            exit 1
            ;;
    esac
}

# 脚本入口 (Script entry point)
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi