#!/bin/bash

# Synapse Matrix服务器监控脚本
# 检查服务状态、系统健康状况和性能指标

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# 配置
CHECK_INTERVAL=30  # 检查间隔（秒）
ALERT_THRESHOLD_CPU=80  # CPU使用率告警阈值
ALERT_THRESHOLD_MEMORY=85  # 内存使用率告警阈值
ALERT_THRESHOLD_DISK=90  # 磁盘使用率告警阈值
LOG_FILE="logs/monitor.log"
ALERT_LOG="logs/alerts.log"
WEBHOOK_URL=""  # 告警Webhook URL
EMAIL_ALERTS=false
EMAIL_TO=""

# 日志函数
log_info() {
    local message="$1"
    echo -e "${BLUE}[INFO]${NC} $(date '+%Y-%m-%d %H:%M:%S') $message"
    echo "$(date '+%Y-%m-%d %H:%M:%S') [INFO] $message" >> "$LOG_FILE"
}

log_success() {
    local message="$1"
    echo -e "${GREEN}[SUCCESS]${NC} $(date '+%Y-%m-%d %H:%M:%S') $message"
    echo "$(date '+%Y-%m-%d %H:%M:%S') [SUCCESS] $message" >> "$LOG_FILE"
}

log_warning() {
    local message="$1"
    echo -e "${YELLOW}[WARNING]${NC} $(date '+%Y-%m-%d %H:%M:%S') $message"
    echo "$(date '+%Y-%m-%d %H:%M:%S') [WARNING] $message" >> "$LOG_FILE"
    echo "$(date '+%Y-%m-%d %H:%M:%S') [WARNING] $message" >> "$ALERT_LOG"
}

log_error() {
    local message="$1"
    echo -e "${RED}[ERROR]${NC} $(date '+%Y-%m-%d %H:%M:%S') $message"
    echo "$(date '+%Y-%m-%d %H:%M:%S') [ERROR] $message" >> "$LOG_FILE"
    echo "$(date '+%Y-%m-%d %H:%M:%S') [ERROR] $message" >> "$ALERT_LOG"
}

log_critical() {
    local message="$1"
    echo -e "${RED}[CRITICAL]${NC} $(date '+%Y-%m-%d %H:%M:%S') $message"
    echo "$(date '+%Y-%m-%d %H:%M:%S') [CRITICAL] $message" >> "$LOG_FILE"
    echo "$(date '+%Y-%m-%d %H:%M:%S') [CRITICAL] $message" >> "$ALERT_LOG"
    send_alert "CRITICAL" "$message"
}

# 显示帮助信息
show_help() {
    echo "Synapse Matrix服务器监控脚本"
    echo
    echo "用法: $0 [选项]"
    echo
    echo "选项:"
    echo "  -h, --help          显示此帮助信息"
    echo "  -c, --continuous    连续监控模式"
    echo "  -i, --interval SEC  设置检查间隔（秒，默认30）"
    echo "  -s, --status        显示服务状态"
    echo "  -p, --performance   显示性能指标"
    echo "  -l, --logs          显示最近日志"
    echo "  -a, --alerts        显示告警历史"
    echo "  -t, --test          测试告警功能"
    echo "  -r, --report        生成监控报告"
    echo "  -w, --webhook URL   设置告警Webhook URL"
    echo "  -e, --email EMAIL   设置告警邮箱"
    echo "  -q, --quiet         静默模式"
    echo
    echo "示例:"
    echo "  $0                  # 单次检查"
    echo "  $0 -c               # 连续监控"
    echo "  $0 -s               # 显示服务状态"
    echo "  $0 -p               # 显示性能指标"
    echo "  $0 -r               # 生成监控报告"
    echo
}

# 发送告警
send_alert() {
    local level="$1"
    local message="$2"
    
    # Webhook告警
    if [ -n "$WEBHOOK_URL" ]; then
        local payload=$(cat << EOF
{
    "level": "$level",
    "message": "$message",
    "timestamp": "$(date -Iseconds)",
    "hostname": "$(hostname)",
    "service": "Synapse Matrix Server"
}
EOF
)
        
        curl -s -X POST -H "Content-Type: application/json" -d "$payload" "$WEBHOOK_URL" > /dev/null || true
    fi
    
    # 邮件告警
    if [ "$EMAIL_ALERTS" = true ] && [ -n "$EMAIL_TO" ] && command -v mail &> /dev/null; then
        echo "$message" | mail -s "[$level] Synapse监控告警" "$EMAIL_TO" || true
    fi
}

# 检查Docker服务
check_docker_services() {
    local all_healthy=true
    local services=("postgres" "redis" "synapse" "nginx" "prometheus" "grafana")
    
    echo -e "${CYAN}=== Docker服务状态 ===${NC}"
    printf "%-15s %-10s %-15s %-20s\n" "服务" "状态" "健康检查" "运行时间"
    printf "%-15s %-10s %-15s %-20s\n" "---------------" "----------" "---------------" "--------------------"
    
    for service in "${services[@]}"; do
        local status="未知"
        local health="未知"
        local uptime="未知"
        
        if docker-compose ps "$service" 2>/dev/null | grep -q "Up"; then
            status="运行中"
            
            # 获取健康检查状态
            local container_id=$(docker-compose ps -q "$service" 2>/dev/null)
            if [ -n "$container_id" ]; then
                health=$(docker inspect --format='{{.State.Health.Status}}' "$container_id" 2>/dev/null || echo "无检查")
                uptime=$(docker inspect --format='{{.State.StartedAt}}' "$container_id" 2>/dev/null | xargs -I {} date -d {} '+%Y-%m-%d %H:%M' 2>/dev/null || echo "未知")
            fi
            
            # 检查健康状态
            if [ "$health" = "unhealthy" ]; then
                status="${RED}异常${NC}"
                all_healthy=false
                log_error "服务 $service 健康检查失败"
            elif [ "$health" = "healthy" ] || [ "$health" = "无检查" ]; then
                status="${GREEN}正常${NC}"
            fi
        else
            status="${RED}停止${NC}"
            all_healthy=false
            log_error "服务 $service 未运行"
        fi
        
        printf "%-15s %-20s %-15s %-20s\n" "$service" "$status" "$health" "$uptime"
    done
    
    echo
    
    if [ "$all_healthy" = true ]; then
        log_success "所有Docker服务运行正常"
    else
        log_warning "部分Docker服务存在问题"
    fi
    
    return $([[ "$all_healthy" == "true" ]])
}

# 检查系统资源
check_system_resources() {
    echo -e "${CYAN}=== 系统资源使用情况 ===${NC}"
    
    # CPU使用率
    local cpu_usage=$(top -l 1 -s 0 | grep "CPU usage" | awk '{print $3}' | sed 's/%//' 2>/dev/null || echo "0")
    if [ -z "$cpu_usage" ]; then
        cpu_usage=$(ps -A -o %cpu | awk '{s+=$1} END {print s}' 2>/dev/null || echo "0")
    fi
    
    printf "%-15s: %s%%" "CPU使用率" "$cpu_usage"
    if (( $(echo "$cpu_usage > $ALERT_THRESHOLD_CPU" | bc -l 2>/dev/null || echo "0") )); then
        echo -e " ${RED}[告警]${NC}"
        log_warning "CPU使用率过高: ${cpu_usage}%"
    else
        echo -e " ${GREEN}[正常]${NC}"
    fi
    
    # 内存使用率
    local memory_info=$(free 2>/dev/null || vm_stat | grep -E "Pages (free|active|inactive|speculative|wired)" | awk '{print $3}' | sed 's/\.//' | paste -sd+ | bc 2>/dev/null)
    local memory_usage=0
    
    if command -v free &> /dev/null; then
        memory_usage=$(free | grep Mem | awk '{printf "%.1f", $3/$2 * 100.0}')
    else
        # macOS计算内存使用率
        local page_size=$(vm_stat | grep "page size" | awk '{print $8}' | sed 's/\.//')
        local pages_free=$(vm_stat | grep "Pages free" | awk '{print $3}' | sed 's/\.//')
        local pages_active=$(vm_stat | grep "Pages active" | awk '{print $3}' | sed 's/\.//')
        local pages_inactive=$(vm_stat | grep "Pages inactive" | awk '{print $3}' | sed 's/\.//')
        local pages_speculative=$(vm_stat | grep "Pages speculative" | awk '{print $3}' | sed 's/\.//')
        local pages_wired=$(vm_stat | grep "Pages wired down" | awk '{print $4}' | sed 's/\.//')
        
        if [ -n "$page_size" ] && [ -n "$pages_free" ]; then
            local total_pages=$((pages_free + pages_active + pages_inactive + pages_speculative + pages_wired))
            local used_pages=$((pages_active + pages_inactive + pages_wired))
            memory_usage=$(echo "scale=1; $used_pages * 100 / $total_pages" | bc 2>/dev/null || echo "0")
        fi
    fi
    
    printf "%-15s: %s%%" "内存使用率" "$memory_usage"
    if (( $(echo "$memory_usage > $ALERT_THRESHOLD_MEMORY" | bc -l 2>/dev/null || echo "0") )); then
        echo -e " ${RED}[告警]${NC}"
        log_warning "内存使用率过高: ${memory_usage}%"
    else
        echo -e " ${GREEN}[正常]${NC}"
    fi
    
    # 磁盘使用率
    local disk_usage=$(df . | tail -1 | awk '{print $5}' | sed 's/%//')
    printf "%-15s: %s%%" "磁盘使用率" "$disk_usage"
    if [ "$disk_usage" -gt "$ALERT_THRESHOLD_DISK" ]; then
        echo -e " ${RED}[告警]${NC}"
        log_warning "磁盘使用率过高: ${disk_usage}%"
    else
        echo -e " ${GREEN}[正常]${NC}"
    fi
    
    # 系统负载
    local load_avg=$(uptime | awk -F'load average:' '{print $2}' | awk '{print $1}' | sed 's/,//')
    printf "%-15s: %s\n" "系统负载" "$load_avg"
    
    # 网络连接数
    local connections=$(netstat -an 2>/dev/null | grep ESTABLISHED | wc -l | tr -d ' ')
    printf "%-15s: %s\n" "网络连接数" "$connections"
    
    echo
}

# 检查端口连通性
check_port_connectivity() {
    echo -e "${CYAN}=== 端口连通性检查 ===${NC}"
    
    local ports=("80:HTTP" "443:HTTPS" "8008:Synapse客户端" "8448:Synapse联邦" "3000:Grafana" "9090:Prometheus")
    
    for port_info in "${ports[@]}"; do
        local port=$(echo "$port_info" | cut -d: -f1)
        local service=$(echo "$port_info" | cut -d: -f2)
        
        printf "%-20s (端口 %s): " "$service" "$port"
        
        if nc -z localhost "$port" 2>/dev/null || timeout 3 bash -c "</dev/tcp/localhost/$port" 2>/dev/null; then
            echo -e "${GREEN}可访问${NC}"
        else
            echo -e "${RED}不可访问${NC}"
            log_warning "端口 $port ($service) 不可访问"
        fi
    done
    
    echo
}

# 检查数据库连接
check_database_connection() {
    echo -e "${CYAN}=== 数据库连接检查 ===${NC}"
    
    # PostgreSQL连接检查
    printf "%-20s: " "PostgreSQL"
    if docker-compose exec -T postgres pg_isready -U synapse 2>/dev/null | grep -q "accepting connections"; then
        echo -e "${GREEN}连接正常${NC}"
        
        # 检查数据库大小
        local db_size=$(docker-compose exec -T postgres psql -U synapse -d synapse -c "SELECT pg_size_pretty(pg_database_size('synapse'));" 2>/dev/null | sed -n '3p' | tr -d ' ')
        printf "%-20s: %s\n" "数据库大小" "$db_size"
        
        # 检查连接数
        local connections=$(docker-compose exec -T postgres psql -U synapse -d synapse -c "SELECT count(*) FROM pg_stat_activity;" 2>/dev/null | sed -n '3p' | tr -d ' ')
        printf "%-20s: %s\n" "当前连接数" "$connections"
        
    else
        echo -e "${RED}连接失败${NC}"
        log_error "PostgreSQL数据库连接失败"
    fi
    
    # Redis连接检查
    printf "%-20s: " "Redis"
    if docker-compose exec -T redis redis-cli ping 2>/dev/null | grep -q "PONG"; then
        echo -e "${GREEN}连接正常${NC}"
        
        # 检查Redis信息
        local redis_memory=$(docker-compose exec -T redis redis-cli info memory 2>/dev/null | grep "used_memory_human" | cut -d: -f2 | tr -d '\r')
        printf "%-20s: %s\n" "Redis内存使用" "$redis_memory"
        
        local redis_keys=$(docker-compose exec -T redis redis-cli dbsize 2>/dev/null | tr -d '\r')
        printf "%-20s: %s\n" "Redis键数量" "$redis_keys"
        
    else
        echo -e "${RED}连接失败${NC}"
        log_error "Redis连接失败"
    fi
    
    echo
}

# 检查Synapse API
check_synapse_api() {
    echo -e "${CYAN}=== Synapse API检查 ===${NC}"
    
    # 检查健康状态
    printf "%-20s: " "健康检查"
    if curl -s -f "http://localhost:8008/health" > /dev/null 2>&1; then
        echo -e "${GREEN}正常${NC}"
    else
        echo -e "${RED}异常${NC}"
        log_error "Synapse健康检查失败"
    fi
    
    # 检查版本信息
    printf "%-20s: " "版本信息"
    local version=$(curl -s "http://localhost:8008/_synapse/admin/v1/server_version" 2>/dev/null | grep -o '"server_version":"[^"]*"' | cut -d'"' -f4 2>/dev/null || echo "未知")
    echo "$version"
    
    # 检查注册用户数
    printf "%-20s: " "注册用户数"
    local user_count=$(docker-compose exec -T postgres psql -U synapse -d synapse -c "SELECT count(*) FROM users;" 2>/dev/null | sed -n '3p' | tr -d ' ' || echo "未知")
    echo "$user_count"
    
    # 检查房间数
    printf "%-20s: " "房间数量"
    local room_count=$(docker-compose exec -T postgres psql -U synapse -d synapse -c "SELECT count(*) FROM rooms;" 2>/dev/null | sed -n '3p' | tr -d ' ' || echo "未知")
    echo "$room_count"
    
    # 检查好友功能
    printf "%-20s: " "好友功能"
    if curl -s -f "http://localhost:8008/_matrix/client/v1/friends" > /dev/null 2>&1; then
        echo -e "${GREEN}可用${NC}"
        
        # 检查好友关系数
        local friends_count=$(docker-compose exec -T postgres psql -U synapse -d synapse -c "SELECT count(*) FROM friends;" 2>/dev/null | sed -n '3p' | tr -d ' ' || echo "未知")
        printf "%-20s: %s\n" "好友关系数" "$friends_count"
        
    else
        echo -e "${RED}不可用${NC}"
        log_warning "好友功能API不可用"
    fi
    
    echo
}

# 检查日志错误
check_logs_for_errors() {
    echo -e "${CYAN}=== 日志错误检查 ===${NC}"
    
    local log_files=("logs/synapse.log" "logs/nginx/error.log")
    local error_count=0
    
    for log_file in "${log_files[@]}"; do
        if [ -f "$log_file" ]; then
            local recent_errors=$(tail -n 100 "$log_file" 2>/dev/null | grep -i "error\|critical\|fatal" | wc -l | tr -d ' ')
            printf "%-20s: %s 个错误\n" "$(basename "$log_file")" "$recent_errors"
            error_count=$((error_count + recent_errors))
        fi
    done
    
    # 检查Docker容器日志
    local services=("synapse" "postgres" "redis" "nginx")
    for service in "${services[@]}"; do
        local container_errors=$(docker-compose logs --tail=50 "$service" 2>/dev/null | grep -i "error\|critical\|fatal" | wc -l | tr -d ' ')
        printf "%-20s: %s 个错误\n" "$service 容器" "$container_errors"
        error_count=$((error_count + container_errors))
    done
    
    if [ "$error_count" -gt 10 ]; then
        log_warning "发现较多错误日志: $error_count 个"
    elif [ "$error_count" -gt 0 ]; then
        log_info "发现少量错误日志: $error_count 个"
    else
        log_success "未发现错误日志"
    fi
    
    echo
}

# 性能指标检查
check_performance_metrics() {
    echo -e "${CYAN}=== 性能指标 ===${NC}"
    
    # 检查Prometheus指标
    if curl -s "http://localhost:9090/api/v1/query?query=up" > /dev/null 2>&1; then
        # HTTP请求速率
        local http_rate=$(curl -s "http://localhost:9090/api/v1/query?query=rate(synapse_http_requests_total[5m])" 2>/dev/null | grep -o '"value":\[[^]]*\]' | grep -o '[0-9.]*' | tail -1 || echo "0")
        printf "%-20s: %.2f 请求/秒\n" "HTTP请求速率" "$http_rate"
        
        # 响应时间
        local response_time=$(curl -s "http://localhost:9090/api/v1/query?query=synapse_http_request_duration_seconds" 2>/dev/null | grep -o '"value":\[[^]]*\]' | grep -o '[0-9.]*' | tail -1 || echo "0")
        printf "%-20s: %.3f 秒\n" "平均响应时间" "$response_time"
        
        # 数据库查询时间
        local db_query_time=$(curl -s "http://localhost:9090/api/v1/query?query=synapse_database_query_time" 2>/dev/null | grep -o '"value":\[[^]]*\]' | grep -o '[0-9.]*' | tail -1 || echo "0")
        printf "%-20s: %.3f 秒\n" "数据库查询时间" "$db_query_time"
        
    else
        log_warning "无法获取Prometheus指标"
    fi
    
    # 容器资源使用
    echo
    echo "容器资源使用情况:"
    docker stats --no-stream --format "table {{.Container}}\t{{.CPUPerc}}\t{{.MemUsage}}\t{{.NetIO}}\t{{.BlockIO}}" 2>/dev/null | head -10
    
    echo
}

# 生成监控报告
generate_report() {
    local report_file="reports/monitor_report_$(date +%Y%m%d_%H%M%S).txt"
    mkdir -p "reports"
    
    log_info "生成监控报告: $report_file"
    
    {
        echo "Synapse Matrix服务器监控报告"
        echo "生成时间: $(date)"
        echo "服务器: $(hostname)"
        echo "系统: $(uname -a)"
        echo "======================================"
        echo
        
        check_docker_services
        echo
        
        check_system_resources
        echo
        
        check_port_connectivity
        echo
        
        check_database_connection
        echo
        
        check_synapse_api
        echo
        
        check_logs_for_errors
        echo
        
        check_performance_metrics
        echo
        
        echo "======================================"
        echo "报告生成完成: $(date)"
        
    } > "$report_file"
    
    log_success "监控报告已保存: $report_file"
    
    # 显示报告摘要
    echo -e "${PURPLE}=== 报告摘要 ===${NC}"
    echo "报告文件: $report_file"
    echo "文件大小: $(du -h "$report_file" | cut -f1)"
    echo
}

# 显示最近日志
show_recent_logs() {
    echo -e "${CYAN}=== 最近日志 ===${NC}"
    
    if [ -f "$LOG_FILE" ]; then
        echo "监控日志 (最近20条):"
        tail -n 20 "$LOG_FILE"
        echo
    fi
    
    echo "Synapse容器日志 (最近10条):"
    docker-compose logs --tail=10 synapse 2>/dev/null || echo "无法获取Synapse日志"
    echo
}

# 显示告警历史
show_alerts() {
    echo -e "${CYAN}=== 告警历史 ===${NC}"
    
    if [ -f "$ALERT_LOG" ]; then
        echo "最近告警 (最近20条):"
        tail -n 20 "$ALERT_LOG"
    else
        echo "暂无告警记录"
    fi
    
    echo
}

# 测试告警功能
test_alerts() {
    log_info "测试告警功能..."
    
    # 测试不同级别的告警
    log_warning "这是一个测试警告"
    log_error "这是一个测试错误"
    
    # 测试关键告警（会触发外部通知）
    read -p "是否测试关键告警（会发送外部通知）？(y/N): " test_critical
    if [[ $test_critical =~ ^[Yy]$ ]]; then
        log_critical "这是一个测试关键告警"
    fi
    
    log_success "告警功能测试完成"
}

# 连续监控模式
continuous_monitoring() {
    log_info "开始连续监控模式，检查间隔: ${CHECK_INTERVAL}秒"
    log_info "按 Ctrl+C 停止监控"
    
    # 设置信号处理
    trap 'log_info "停止连续监控"; exit 0' INT TERM
    
    while true; do
        clear
        echo -e "${PURPLE}=== Synapse Matrix服务器监控 ===${NC}"
        echo "当前时间: $(date)"
        echo "检查间隔: ${CHECK_INTERVAL}秒"
        echo
        
        # 执行检查
        check_docker_services
        check_system_resources
        check_database_connection
        check_synapse_api
        
        echo -e "${BLUE}下次检查: $(date -d "+${CHECK_INTERVAL} seconds" '+%H:%M:%S')${NC}"
        echo "按 Ctrl+C 停止监控"
        
        sleep "$CHECK_INTERVAL"
    done
}

# 主函数
main() {
    local continuous=false
    local show_status=false
    local show_performance=false
    local show_logs=false
    local show_alerts_flag=false
    local test_alerts_flag=false
    local generate_report_flag=false
    local quiet=false
    
    # 解析命令行参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_help
                exit 0
                ;;
            -c|--continuous)
                continuous=true
                shift
                ;;
            -i|--interval)
                CHECK_INTERVAL="$2"
                shift 2
                ;;
            -s|--status)
                show_status=true
                shift
                ;;
            -p|--performance)
                show_performance=true
                shift
                ;;
            -l|--logs)
                show_logs=true
                shift
                ;;
            -a|--alerts)
                show_alerts_flag=true
                shift
                ;;
            -t|--test)
                test_alerts_flag=true
                shift
                ;;
            -r|--report)
                generate_report_flag=true
                shift
                ;;
            -w|--webhook)
                WEBHOOK_URL="$2"
                shift 2
                ;;
            -e|--email)
                EMAIL_TO="$2"
                EMAIL_ALERTS=true
                shift 2
                ;;
            -q|--quiet)
                quiet=true
                shift
                ;;
            *)
                log_error "未知选项: $1"
                show_help
                exit 1
                ;;
        esac
    done
    
    # 创建日志目录
    mkdir -p "$(dirname "$LOG_FILE")"
    mkdir -p "$(dirname "$ALERT_LOG")"
    
    # 静默模式设置
    if [ "$quiet" = true ]; then
        exec > /dev/null 2>&1
    fi
    
    # 执行相应操作
    if [ "$continuous" = true ]; then
        continuous_monitoring
    elif [ "$show_status" = true ]; then
        check_docker_services
    elif [ "$show_performance" = true ]; then
        check_performance_metrics
    elif [ "$show_logs" = true ]; then
        show_recent_logs
    elif [ "$show_alerts_flag" = true ]; then
        show_alerts
    elif [ "$test_alerts_flag" = true ]; then
        test_alerts
    elif [ "$generate_report_flag" = true ]; then
        generate_report
    else
        # 默认执行完整检查
        log_info "开始系统监控检查..."
        
        check_docker_services
        check_system_resources
        check_port_connectivity
        check_database_connection
        check_synapse_api
        check_logs_for_errors
        
        log_success "监控检查完成"
    fi
}

# 错误处理
trap 'log_error "监控脚本执行失败"; exit 1' ERR

# 执行主函数
main "$@"