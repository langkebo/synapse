#!/bin/bash

# Synapse Matrix服务器维护脚本
# 用于日常维护任务：清理日志、更新配置、数据库维护等

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
LOG_RETENTION_DAYS=30  # 日志保留天数
BACKUP_RETENTION_DAYS=7  # 备份保留天数
MEDIA_RETENTION_DAYS=365  # 媒体文件保留天数
DB_VACUUM_THRESHOLD=1000  # 数据库清理阈值(MB)
LOG_FILE="logs/maintenance.log"
DRY_RUN=false
FORCE=false

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
}

log_error() {
    local message="$1"
    echo -e "${RED}[ERROR]${NC} $(date '+%Y-%m-%d %H:%M:%S') $message"
    echo "$(date '+%Y-%m-%d %H:%M:%S') [ERROR] $message" >> "$LOG_FILE"
}

# 显示帮助信息
show_help() {
    echo "Synapse Matrix服务器维护脚本"
    echo
    echo "用法: $0 [选项] [任务]"
    echo
    echo "任务:"
    echo "  cleanup-logs        清理旧日志文件"
    echo "  cleanup-backups     清理旧备份文件"
    echo "  cleanup-media       清理旧媒体文件"
    echo "  vacuum-db           数据库清理和优化"
    echo "  update-config       更新配置文件"
    echo "  rotate-logs         轮转日志文件"
    echo "  check-disk          检查磁盘空间"
    echo "  optimize-redis      优化Redis缓存"
    echo "  update-friends      更新好友功能配置"
    echo "  security-scan       安全检查"
    echo "  performance-tune    性能调优"
    echo "  all                 执行所有维护任务"
    echo
    echo "选项:"
    echo "  -h, --help          显示此帮助信息"
    echo "  -d, --dry-run       模拟运行，不执行实际操作"
    echo "  -f, --force         强制执行，跳过确认"
    echo "  -v, --verbose       详细输出"
    echo "  --log-days DAYS     日志保留天数（默认30）"
    echo "  --backup-days DAYS  备份保留天数（默认7）"
    echo "  --media-days DAYS   媒体保留天数（默认365）"
    echo
    echo "示例:"
    echo "  $0 cleanup-logs                    # 清理旧日志"
    echo "  $0 --dry-run all                   # 模拟执行所有任务"
    echo "  $0 --log-days 7 cleanup-logs       # 清理7天前的日志"
    echo "  $0 vacuum-db                       # 数据库清理"
    echo
}

# 确认操作
confirm_action() {
    local message="$1"
    
    if [ "$FORCE" = true ]; then
        return 0
    fi
    
    echo -e "${YELLOW}$message${NC}"
    read -p "是否继续？(y/N): " confirm
    [[ $confirm =~ ^[Yy]$ ]]
}

# 获取文件大小（人类可读格式）
get_size() {
    local path="$1"
    if [ -e "$path" ]; then
        du -sh "$path" 2>/dev/null | cut -f1
    else
        echo "0B"
    fi
}

# 清理日志文件
cleanup_logs() {
    log_info "开始清理日志文件（保留 $LOG_RETENTION_DAYS 天）"
    
    local log_dirs=("logs" "data/logs" "nginx/logs")
    local total_cleaned=0
    local total_size_before=0
    local total_size_after=0
    
    for log_dir in "${log_dirs[@]}"; do
        if [ ! -d "$log_dir" ]; then
            continue
        fi
        
        log_info "清理目录: $log_dir"
        
        # 计算清理前大小
        local size_before=$(du -s "$log_dir" 2>/dev/null | cut -f1 || echo "0")
        total_size_before=$((total_size_before + size_before))
        
        # 查找并清理旧日志文件
        local old_files=$(find "$log_dir" -name "*.log*" -type f -mtime +$LOG_RETENTION_DAYS 2>/dev/null || true)
        
        if [ -n "$old_files" ]; then
            local file_count=$(echo "$old_files" | wc -l | tr -d ' ')
            log_info "发现 $file_count 个旧日志文件"
            
            if [ "$DRY_RUN" = true ]; then
                echo "[DRY RUN] 将删除以下文件:"
                echo "$old_files"
            else
                if confirm_action "即将删除 $file_count 个日志文件"; then
                    echo "$old_files" | xargs rm -f
                    total_cleaned=$((total_cleaned + file_count))
                    log_success "已删除 $file_count 个旧日志文件"
                fi
            fi
        else
            log_info "未发现需要清理的旧日志文件"
        fi
        
        # 计算清理后大小
        local size_after=$(du -s "$log_dir" 2>/dev/null | cut -f1 || echo "0")
        total_size_after=$((total_size_after + size_after))
    done
    
    # 清理压缩日志文件
    log_info "清理压缩日志文件"
    local compressed_logs=$(find . -name "*.log.gz" -o -name "*.log.bz2" -o -name "*.log.xz" -type f -mtime +$LOG_RETENTION_DAYS 2>/dev/null || true)
    
    if [ -n "$compressed_logs" ]; then
        local compressed_count=$(echo "$compressed_logs" | wc -l | tr -d ' ')
        log_info "发现 $compressed_count 个压缩日志文件"
        
        if [ "$DRY_RUN" = true ]; then
            echo "[DRY RUN] 将删除以下压缩日志文件:"
            echo "$compressed_logs"
        else
            if confirm_action "即将删除 $compressed_count 个压缩日志文件"; then
                echo "$compressed_logs" | xargs rm -f
                total_cleaned=$((total_cleaned + compressed_count))
                log_success "已删除 $compressed_count 个压缩日志文件"
            fi
        fi
    fi
    
    local space_freed=$((total_size_before - total_size_after))
    log_success "日志清理完成，共清理 $total_cleaned 个文件，释放 ${space_freed}KB 空间"
}

# 清理备份文件
cleanup_backups() {
    log_info "开始清理备份文件（保留 $BACKUP_RETENTION_DAYS 天）"
    
    local backup_dirs=("backups" "data/backups")
    local total_cleaned=0
    
    for backup_dir in "${backup_dirs[@]}"; do
        if [ ! -d "$backup_dir" ]; then
            continue
        fi
        
        log_info "清理备份目录: $backup_dir"
        
        # 查找旧备份文件
        local old_backups=$(find "$backup_dir" -name "*.tar.gz" -o -name "*.sql.gz" -o -name "*.backup" -type f -mtime +$BACKUP_RETENTION_DAYS 2>/dev/null || true)
        
        if [ -n "$old_backups" ]; then
            local backup_count=$(echo "$old_backups" | wc -l | tr -d ' ')
            log_info "发现 $backup_count 个旧备份文件"
            
            if [ "$DRY_RUN" = true ]; then
                echo "[DRY RUN] 将删除以下备份文件:"
                echo "$old_backups"
            else
                if confirm_action "即将删除 $backup_count 个备份文件"; then
                    echo "$old_backups" | xargs rm -f
                    total_cleaned=$((total_cleaned + backup_count))
                    log_success "已删除 $backup_count 个旧备份文件"
                fi
            fi
        else
            log_info "未发现需要清理的旧备份文件"
        fi
    done
    
    log_success "备份清理完成，共清理 $total_cleaned 个文件"
}

# 清理媒体文件
cleanup_media() {
    log_info "开始清理媒体文件（保留 $MEDIA_RETENTION_DAYS 天）"
    
    local media_dir="data/media_store"
    
    if [ ! -d "$media_dir" ]; then
        log_warning "媒体目录不存在: $media_dir"
        return
    fi
    
    # 计算媒体目录大小
    local media_size_before=$(get_size "$media_dir")
    log_info "媒体目录当前大小: $media_size_before"
    
    # 查找旧媒体文件
    local old_media=$(find "$media_dir" -type f -mtime +$MEDIA_RETENTION_DAYS 2>/dev/null || true)
    
    if [ -n "$old_media" ]; then
        local media_count=$(echo "$old_media" | wc -l | tr -d ' ')
        log_info "发现 $media_count 个旧媒体文件"
        
        if [ "$DRY_RUN" = true ]; then
            echo "[DRY RUN] 将删除 $media_count 个媒体文件"
        else
            if confirm_action "即将删除 $media_count 个媒体文件，这可能需要一些时间"; then
                echo "$old_media" | xargs rm -f
                
                # 清理空目录
                find "$media_dir" -type d -empty -delete 2>/dev/null || true
                
                local media_size_after=$(get_size "$media_dir")
                log_success "媒体文件清理完成，目录大小从 $media_size_before 减少到 $media_size_after"
            fi
        fi
    else
        log_info "未发现需要清理的旧媒体文件"
    fi
}

# 数据库清理和优化
vacuum_database() {
    log_info "开始数据库清理和优化"
    
    # 检查数据库大小
    local db_size_mb=$(docker-compose exec -T postgres psql -U synapse -d synapse -c "SELECT pg_size_pretty(pg_database_size('synapse'));" 2>/dev/null | sed -n '3p' | tr -d ' ' || echo "未知")
    log_info "当前数据库大小: $db_size_mb"
    
    # 检查表统计信息
    log_info "获取表统计信息..."
    docker-compose exec -T postgres psql -U synapse -d synapse -c "
        SELECT 
            schemaname,
            tablename,
            pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) as size,
            n_tup_ins as inserts,
            n_tup_upd as updates,
            n_tup_del as deletes
        FROM pg_stat_user_tables 
        ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC 
        LIMIT 10;
    " 2>/dev/null || log_warning "无法获取表统计信息"
    
    if [ "$DRY_RUN" = true ]; then
        echo "[DRY RUN] 将执行数据库清理操作"
        return
    fi
    
    if confirm_action "即将执行数据库清理操作，这可能需要较长时间"; then
        # 执行VACUUM ANALYZE
        log_info "执行 VACUUM ANALYZE..."
        docker-compose exec -T postgres psql -U synapse -d synapse -c "VACUUM ANALYZE;" 2>/dev/null || log_error "VACUUM ANALYZE 执行失败"
        
        # 重建索引（仅对大表）
        log_info "检查是否需要重建索引..."
        docker-compose exec -T postgres psql -U synapse -d synapse -c "
            SELECT tablename 
            FROM pg_tables 
            WHERE schemaname = 'public' 
            AND pg_total_relation_size(schemaname||'.'||tablename) > 100*1024*1024;
        " 2>/dev/null | while read -r table; do
            if [ -n "$table" ] && [ "$table" != "tablename" ] && [ "$table" != "(0 rows)" ]; then
                log_info "重建表 $table 的索引..."
                docker-compose exec -T postgres psql -U synapse -d synapse -c "REINDEX TABLE $table;" 2>/dev/null || log_warning "重建表 $table 索引失败"
            fi
        done
        
        # 更新统计信息
        log_info "更新统计信息..."
        docker-compose exec -T postgres psql -U synapse -d synapse -c "ANALYZE;" 2>/dev/null || log_error "ANALYZE 执行失败"
        
        # 清理好友功能相关的过期数据
        log_info "清理好友功能过期数据..."
        docker-compose exec -T postgres psql -U synapse -d synapse -c "
            DELETE FROM friend_requests 
            WHERE created_at < NOW() - INTERVAL '30 days' 
            AND status IN ('rejected', 'cancelled');
        " 2>/dev/null || log_warning "清理好友请求数据失败"
        
        local db_size_after=$(docker-compose exec -T postgres psql -U synapse -d synapse -c "SELECT pg_size_pretty(pg_database_size('synapse'));" 2>/dev/null | sed -n '3p' | tr -d ' ' || echo "未知")
        log_success "数据库优化完成，大小从 $db_size_mb 变为 $db_size_after"
    fi
}

# 轮转日志文件
rotate_logs() {
    log_info "开始轮转日志文件"
    
    local log_files=("logs/synapse.log" "logs/maintenance.log" "logs/monitor.log")
    
    for log_file in "${log_files[@]}"; do
        if [ -f "$log_file" ]; then
            local file_size=$(stat -f%z "$log_file" 2>/dev/null || stat -c%s "$log_file" 2>/dev/null || echo "0")
            
            # 如果文件大于10MB，进行轮转
            if [ "$file_size" -gt 10485760 ]; then
                log_info "轮转日志文件: $log_file ($(get_size "$log_file"))"
                
                if [ "$DRY_RUN" = true ]; then
                    echo "[DRY RUN] 将轮转日志文件: $log_file"
                else
                    # 创建轮转文件
                    local timestamp=$(date +%Y%m%d_%H%M%S)
                    local rotated_file="${log_file}.${timestamp}"
                    
                    cp "$log_file" "$rotated_file"
                    > "$log_file"  # 清空原文件
                    
                    # 压缩轮转文件
                    gzip "$rotated_file"
                    
                    log_success "日志文件已轮转: ${rotated_file}.gz"
                fi
            fi
        fi
    done
    
    # 发送USR1信号给Nginx重新打开日志文件
    if docker-compose ps nginx | grep -q "Up"; then
        log_info "重新打开Nginx日志文件"
        docker-compose exec nginx nginx -s reopen 2>/dev/null || log_warning "重新打开Nginx日志失败"
    fi
}

# 检查磁盘空间
check_disk_space() {
    log_info "检查磁盘空间使用情况"
    
    # 检查根目录磁盘使用率
    local disk_usage=$(df . | tail -1 | awk '{print $5}' | sed 's/%//')
    local disk_available=$(df -h . | tail -1 | awk '{print $4}')
    
    echo "磁盘使用率: ${disk_usage}%"
    echo "可用空间: ${disk_available}"
    
    if [ "$disk_usage" -gt 90 ]; then
        log_warning "磁盘使用率过高: ${disk_usage}%"
        
        # 显示最大的目录
        echo "最大的目录:"
        du -sh */ 2>/dev/null | sort -hr | head -5
        
    elif [ "$disk_usage" -gt 80 ]; then
        log_warning "磁盘使用率较高: ${disk_usage}%"
    else
        log_success "磁盘空间充足"
    fi
    
    # 检查inode使用情况
    local inode_usage=$(df -i . | tail -1 | awk '{print $5}' | sed 's/%//')
    echo "Inode使用率: ${inode_usage}%"
    
    if [ "$inode_usage" -gt 90 ]; then
        log_warning "Inode使用率过高: ${inode_usage}%"
    fi
}

# 优化Redis缓存
optimize_redis() {
    log_info "开始优化Redis缓存"
    
    if ! docker-compose ps redis | grep -q "Up"; then
        log_error "Redis服务未运行"
        return 1
    fi
    
    # 获取Redis信息
    local redis_memory=$(docker-compose exec -T redis redis-cli info memory 2>/dev/null | grep "used_memory_human" | cut -d: -f2 | tr -d '\r')
    local redis_keys=$(docker-compose exec -T redis redis-cli dbsize 2>/dev/null | tr -d '\r')
    
    log_info "Redis内存使用: $redis_memory"
    log_info "Redis键数量: $redis_keys"
    
    if [ "$DRY_RUN" = true ]; then
        echo "[DRY RUN] 将执行Redis优化操作"
        return
    fi
    
    # 清理过期键
    log_info "清理过期键..."
    docker-compose exec -T redis redis-cli --scan --pattern "*" | while read -r key; do
        local ttl=$(docker-compose exec -T redis redis-cli ttl "$key" 2>/dev/null | tr -d '\r')
        if [ "$ttl" = "-1" ]; then
            # 为没有TTL的键设置合理的过期时间
            if [[ $key == *"friends:"* ]]; then
                docker-compose exec -T redis redis-cli expire "$key" 3600 > /dev/null  # 1小时
            elif [[ $key == *"cache:"* ]]; then
                docker-compose exec -T redis redis-cli expire "$key" 1800 > /dev/null  # 30分钟
            fi
        fi
    done
    
    # 执行内存碎片整理
    log_info "执行内存碎片整理..."
    docker-compose exec -T redis redis-cli memory purge 2>/dev/null || log_warning "内存碎片整理失败"
    
    # 保存数据到磁盘
    log_info "保存数据到磁盘..."
    docker-compose exec -T redis redis-cli bgsave 2>/dev/null || log_warning "后台保存失败"
    
    log_success "Redis优化完成"
}

# 更新好友功能配置
update_friends_config() {
    log_info "检查好友功能配置更新"
    
    local config_file="synapse/friends_config.yaml"
    
    if [ ! -f "$config_file" ]; then
        log_warning "好友功能配置文件不存在: $config_file"
        return
    fi
    
    # 备份当前配置
    local backup_file="${config_file}.backup.$(date +%Y%m%d_%H%M%S)"
    
    if [ "$DRY_RUN" = true ]; then
        echo "[DRY RUN] 将备份配置文件到: $backup_file"
        return
    fi
    
    cp "$config_file" "$backup_file"
    log_info "配置文件已备份到: $backup_file"
    
    # 检查配置文件语法
    if python3 -c "import yaml; yaml.safe_load(open('$config_file'))" 2>/dev/null; then
        log_success "好友功能配置文件语法正确"
    else
        log_error "好友功能配置文件语法错误"
        return 1
    fi
    
    # 重新加载配置（如果支持）
    if docker-compose ps synapse | grep -q "Up"; then
        log_info "重新加载Synapse配置..."
        docker-compose exec synapse kill -HUP 1 2>/dev/null || log_warning "重新加载配置失败"
    fi
}

# 安全检查
security_scan() {
    log_info "开始安全检查"
    
    # 检查文件权限
    log_info "检查文件权限..."
    
    local sensitive_files=(".env" "synapse/homeserver.yaml" "synapse/friends_config.yaml")
    
    for file in "${sensitive_files[@]}"; do
        if [ -f "$file" ]; then
            local perms=$(stat -c "%a" "$file" 2>/dev/null || stat -f "%A" "$file" 2>/dev/null)
            if [ "$perms" != "600" ] && [ "$perms" != "644" ]; then
                log_warning "文件权限不安全: $file ($perms)"
                
                if [ "$DRY_RUN" = false ] && confirm_action "修复文件权限: $file"; then
                    chmod 600 "$file"
                    log_success "已修复文件权限: $file"
                fi
            fi
        fi
    done
    
    # 检查密码强度
    log_info "检查配置中的密码强度..."
    
    if [ -f ".env" ]; then
        # 检查数据库密码
        local db_password=$(grep "POSTGRES_PASSWORD" .env | cut -d= -f2 | tr -d '"')
        if [ ${#db_password} -lt 12 ]; then
            log_warning "数据库密码强度不足（少于12位）"
        fi
        
        # 检查Redis密码
        local redis_password=$(grep "REDIS_PASSWORD" .env | cut -d= -f2 | tr -d '"')
        if [ -n "$redis_password" ] && [ ${#redis_password} -lt 12 ]; then
            log_warning "Redis密码强度不足（少于12位）"
        fi
    fi
    
    # 检查SSL证书
    log_info "检查SSL证书..."
    
    local cert_file="ssl/cert.pem"
    if [ -f "$cert_file" ]; then
        local cert_expiry=$(openssl x509 -in "$cert_file" -noout -enddate 2>/dev/null | cut -d= -f2)
        if [ -n "$cert_expiry" ]; then
            local expiry_timestamp=$(date -d "$cert_expiry" +%s 2>/dev/null || date -j -f "%b %d %H:%M:%S %Y %Z" "$cert_expiry" +%s 2>/dev/null)
            local current_timestamp=$(date +%s)
            local days_until_expiry=$(( (expiry_timestamp - current_timestamp) / 86400 ))
            
            if [ "$days_until_expiry" -lt 30 ]; then
                log_warning "SSL证书将在 $days_until_expiry 天后过期"
            else
                log_success "SSL证书有效期充足（$days_until_expiry 天）"
            fi
        fi
    else
        log_warning "未找到SSL证书文件"
    fi
    
    # 检查开放端口
    log_info "检查开放端口..."
    
    local open_ports=$(netstat -tuln 2>/dev/null | grep LISTEN | awk '{print $4}' | cut -d: -f2 | sort -n | uniq)
    echo "开放端口: $(echo $open_ports | tr '\n' ' ')"
    
    # 检查不必要的开放端口
    local unnecessary_ports=("22" "3389" "5432" "6379")
    for port in "${unnecessary_ports[@]}"; do
        if echo "$open_ports" | grep -q "^$port$"; then
            log_warning "发现可能不必要的开放端口: $port"
        fi
    done
    
    log_success "安全检查完成"
}

# 性能调优
performance_tune() {
    log_info "开始性能调优"
    
    # 检查系统资源
    local cpu_count=$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo "1")
    local memory_gb=$(free -g 2>/dev/null | grep Mem | awk '{print $2}' || echo "2")
    
    log_info "系统配置: ${cpu_count}核CPU, ${memory_gb}GB内存"
    
    # 调优建议
    echo "性能调优建议:"
    
    # PostgreSQL调优
    echo "PostgreSQL:"
    echo "  - shared_buffers: $((memory_gb * 256))MB"
    echo "  - effective_cache_size: $((memory_gb * 768))MB"
    echo "  - work_mem: $((memory_gb * 4))MB"
    echo "  - max_connections: $((cpu_count * 25))"
    
    # Redis调优
    echo "Redis:"
    echo "  - maxmemory: $((memory_gb * 256))mb"
    echo "  - tcp-keepalive: 60"
    echo "  - timeout: 300"
    
    # Synapse调优
    echo "Synapse:"
    echo "  - worker_processes: $cpu_count"
    echo "  - database_pool_size: $((cpu_count * 5))"
    echo "  - cache_size: $((memory_gb * 128))M"
    
    if [ "$DRY_RUN" = false ] && confirm_action "应用性能调优配置"; then
        # 这里可以添加自动应用配置的逻辑
        log_info "性能调优配置需要手动应用到相应的配置文件中"
    fi
}

# 执行所有维护任务
run_all_maintenance() {
    log_info "开始执行所有维护任务"
    
    local tasks=("cleanup-logs" "cleanup-backups" "rotate-logs" "check-disk" "vacuum-db" "optimize-redis" "security-scan")
    
    for task in "${tasks[@]}"; do
        log_info "执行任务: $task"
        
        case $task in
            "cleanup-logs")
                cleanup_logs
                ;;
            "cleanup-backups")
                cleanup_backups
                ;;
            "cleanup-media")
                cleanup_media
                ;;
            "vacuum-db")
                vacuum_database
                ;;
            "rotate-logs")
                rotate_logs
                ;;
            "check-disk")
                check_disk_space
                ;;
            "optimize-redis")
                optimize_redis
                ;;
            "update-friends")
                update_friends_config
                ;;
            "security-scan")
                security_scan
                ;;
            "performance-tune")
                performance_tune
                ;;
        esac
        
        echo
    done
    
    log_success "所有维护任务执行完成"
}

# 主函数
main() {
    local task=""
    local verbose=false
    
    # 解析命令行参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_help
                exit 0
                ;;
            -d|--dry-run)
                DRY_RUN=true
                shift
                ;;
            -f|--force)
                FORCE=true
                shift
                ;;
            -v|--verbose)
                verbose=true
                shift
                ;;
            --log-days)
                LOG_RETENTION_DAYS="$2"
                shift 2
                ;;
            --backup-days)
                BACKUP_RETENTION_DAYS="$2"
                shift 2
                ;;
            --media-days)
                MEDIA_RETENTION_DAYS="$2"
                shift 2
                ;;
            cleanup-logs|cleanup-backups|cleanup-media|vacuum-db|update-config|rotate-logs|check-disk|optimize-redis|update-friends|security-scan|performance-tune|all)
                task="$1"
                shift
                ;;
            *)
                log_error "未知选项或任务: $1"
                show_help
                exit 1
                ;;
        esac
    done
    
    # 如果没有指定任务，显示帮助
    if [ -z "$task" ]; then
        show_help
        exit 1
    fi
    
    # 创建日志目录
    mkdir -p "$(dirname "$LOG_FILE")"
    
    # 详细模式设置
    if [ "$verbose" = true ]; then
        set -x
    fi
    
    log_info "开始维护任务: $task"
    
    if [ "$DRY_RUN" = true ]; then
        log_info "运行模式: 模拟运行（不执行实际操作）"
    fi
    
    # 执行相应任务
    case $task in
        "cleanup-logs")
            cleanup_logs
            ;;
        "cleanup-backups")
            cleanup_backups
            ;;
        "cleanup-media")
            cleanup_media
            ;;
        "vacuum-db")
            vacuum_database
            ;;
        "update-config")
            update_friends_config
            ;;
        "rotate-logs")
            rotate_logs
            ;;
        "check-disk")
            check_disk_space
            ;;
        "optimize-redis")
            optimize_redis
            ;;
        "update-friends")
            update_friends_config
            ;;
        "security-scan")
            security_scan
            ;;
        "performance-tune")
            performance_tune
            ;;
        "all")
            run_all_maintenance
            ;;
        *)
            log_error "未知任务: $task"
            exit 1
            ;;
    esac
    
    log_success "维护任务完成: $task"
}

# 错误处理
trap 'log_error "维护脚本执行失败"; exit 1' ERR

# 执行主函数
main "$@"