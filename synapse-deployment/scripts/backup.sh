#!/bin/bash

# Synapse Matrix服务器备份脚本
# 自动备份数据库、配置文件和媒体文件

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 配置
BACKUP_DIR="backups"
MAX_BACKUPS=7  # 保留最近7个备份
MAX_MEDIA_SIZE=2048  # 媒体文件备份最大大小(MB)
COMPRESS_BACKUPS=true
ENCRYPT_BACKUPS=false
ENCRYPTION_PASSWORD=""  # 如果启用加密，设置密码

# 日志函数
log_info() {
    echo -e "${BLUE}[INFO]${NC} $(date '+%Y-%m-%d %H:%M:%S') $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $(date '+%Y-%m-%d %H:%M:%S') $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $(date '+%Y-%m-%d %H:%M:%S') $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $(date '+%Y-%m-%d %H:%M:%S') $1"
}

# 显示帮助信息
show_help() {
    echo "Synapse Matrix服务器备份脚本"
    echo
    echo "用法: $0 [选项]"
    echo
    echo "选项:"
    echo "  -h, --help          显示此帮助信息"
    echo "  -f, --full          完整备份（包括媒体文件）"
    echo "  -d, --database      仅备份数据库"
    echo "  -c, --config        仅备份配置文件"
    echo "  -m, --media         仅备份媒体文件"
    echo "  -n, --no-compress   不压缩备份文件"
    echo "  -e, --encrypt       加密备份文件"
    echo "  -r, --restore FILE  从备份文件恢复"
    echo "  -l, --list          列出所有备份"
    echo "  -x, --cleanup       清理旧备份"
    echo "  -t, --test          测试备份完整性"
    echo "  -s, --schedule      设置定时备份"
    echo
    echo "示例:"
    echo "  $0                  # 标准备份（数据库+配置）"
    echo "  $0 -f               # 完整备份（包括媒体）"
    echo "  $0 -d               # 仅备份数据库"
    echo "  $0 -e               # 加密备份"
    echo "  $0 -r backup.tar.gz # 从备份恢复"
    echo "  $0 -l               # 列出备份"
    echo
}

# 检查依赖
check_dependencies() {
    local missing_deps=()
    
    if ! command -v docker-compose &> /dev/null && ! docker compose version &> /dev/null; then
        missing_deps+=("docker-compose")
    fi
    
    if [ "$COMPRESS_BACKUPS" = true ] && ! command -v tar &> /dev/null; then
        missing_deps+=("tar")
    fi
    
    if [ "$ENCRYPT_BACKUPS" = true ] && ! command -v gpg &> /dev/null; then
        missing_deps+=("gpg")
    fi
    
    if [ ${#missing_deps[@]} -gt 0 ]; then
        log_error "缺少依赖: ${missing_deps[*]}"
        exit 1
    fi
}

# 创建备份目录
create_backup_dir() {
    local timestamp=$(date +%Y%m%d_%H%M%S)
    local backup_path="$BACKUP_DIR/$timestamp"
    
    mkdir -p "$backup_path"
    echo "$backup_path"
}

# 备份数据库
backup_database() {
    local backup_path="$1"
    
    log_info "开始备份PostgreSQL数据库..."
    
    # 检查PostgreSQL服务状态
    if ! docker-compose ps postgres | grep -q "Up"; then
        log_error "PostgreSQL服务未运行"
        return 1
    fi
    
    # 备份数据库
    local db_backup_file="$backup_path/synapse_db.sql"
    
    if docker-compose exec -T postgres pg_dump -U synapse synapse > "$db_backup_file"; then
        local db_size=$(du -h "$db_backup_file" | cut -f1)
        log_success "数据库备份完成: $db_backup_file ($db_size)"
        
        # 验证备份文件
        if [ ! -s "$db_backup_file" ]; then
            log_error "数据库备份文件为空"
            return 1
        fi
        
        # 创建备份元数据
        cat > "$backup_path/database_info.txt" << EOF
备份时间: $(date)
数据库版本: $(docker-compose exec -T postgres psql -U synapse -d synapse -c "SELECT version();" | head -3 | tail -1)
数据库大小: $(docker-compose exec -T postgres psql -U synapse -d synapse -c "SELECT pg_size_pretty(pg_database_size('synapse'));" | head -3 | tail -1)
表数量: $(docker-compose exec -T postgres psql -U synapse -d synapse -c "SELECT count(*) FROM information_schema.tables WHERE table_schema = 'public';" | head -3 | tail -1)
EOF
        
    else
        log_error "数据库备份失败"
        return 1
    fi
}

# 备份配置文件
backup_config() {
    local backup_path="$1"
    
    log_info "开始备份配置文件..."
    
    # 创建配置备份目录
    mkdir -p "$backup_path/config"
    
    # 备份Synapse配置
    if [ -d "synapse" ]; then
        cp -r synapse/ "$backup_path/config/"
        log_success "Synapse配置备份完成"
    fi
    
    # 备份Nginx配置
    if [ -d "nginx" ]; then
        cp -r nginx/ "$backup_path/config/"
        log_success "Nginx配置备份完成"
    fi
    
    # 备份Redis配置
    if [ -d "redis" ]; then
        cp -r redis/ "$backup_path/config/"
        log_success "Redis配置备份完成"
    fi
    
    # 备份监控配置
    if [ -d "prometheus" ]; then
        cp -r prometheus/ "$backup_path/config/"
        log_success "Prometheus配置备份完成"
    fi
    
    if [ -d "grafana" ]; then
        cp -r grafana/ "$backup_path/config/"
        log_success "Grafana配置备份完成"
    fi
    
    # 备份环境变量和Docker配置
    [ -f ".env" ] && cp .env "$backup_path/config/"
    [ -f "docker-compose.yml" ] && cp docker-compose.yml "$backup_path/config/"
    [ -f "Dockerfile" ] && cp Dockerfile "$backup_path/config/"
    
    # 备份脚本
    if [ -d "scripts" ]; then
        cp -r scripts/ "$backup_path/config/"
        log_success "脚本备份完成"
    fi
    
    log_success "配置文件备份完成"
}

# 备份媒体文件
backup_media() {
    local backup_path="$1"
    local force_backup="$2"
    
    log_info "开始备份媒体文件..."
    
    local media_dir="data/synapse/media_store"
    
    if [ ! -d "$media_dir" ]; then
        log_warning "媒体目录不存在，跳过媒体备份"
        return 0
    fi
    
    # 检查媒体文件大小
    local media_size=$(du -sm "$media_dir" 2>/dev/null | cut -f1 || echo "0")
    
    if [ "$media_size" -gt "$MAX_MEDIA_SIZE" ] && [ "$force_backup" != "true" ]; then
        log_warning "媒体文件过大(${media_size}MB > ${MAX_MEDIA_SIZE}MB)，跳过备份"
        log_info "使用 -f 选项强制备份媒体文件"
        return 0
    fi
    
    # 备份媒体文件
    log_info "备份媒体文件(${media_size}MB)..."
    
    if cp -r "$media_dir" "$backup_path/media_store"; then
        log_success "媒体文件备份完成"
        
        # 创建媒体备份信息
        cat > "$backup_path/media_info.txt" << EOF
备份时间: $(date)
媒体目录大小: ${media_size}MB
文件数量: $(find "$media_dir" -type f | wc -l)
目录数量: $(find "$media_dir" -type d | wc -l)
EOF
    else
        log_error "媒体文件备份失败"
        return 1
    fi
}

# 压缩备份
compress_backup() {
    local backup_path="$1"
    
    if [ "$COMPRESS_BACKUPS" != "true" ]; then
        return 0
    fi
    
    log_info "压缩备份文件..."
    
    local backup_name=$(basename "$backup_path")
    local compressed_file="$BACKUP_DIR/${backup_name}.tar.gz"
    
    if tar -czf "$compressed_file" -C "$BACKUP_DIR" "$backup_name"; then
        local original_size=$(du -sh "$backup_path" | cut -f1)
        local compressed_size=$(du -sh "$compressed_file" | cut -f1)
        
        log_success "备份压缩完成: $compressed_file"
        log_info "压缩前: $original_size, 压缩后: $compressed_size"
        
        # 删除原始备份目录
        rm -rf "$backup_path"
        
        echo "$compressed_file"
    else
        log_error "备份压缩失败"
        return 1
    fi
}

# 加密备份
encrypt_backup() {
    local backup_file="$1"
    
    if [ "$ENCRYPT_BACKUPS" != "true" ]; then
        return 0
    fi
    
    if [ -z "$ENCRYPTION_PASSWORD" ]; then
        read -s -p "请输入加密密码: " ENCRYPTION_PASSWORD
        echo
    fi
    
    log_info "加密备份文件..."
    
    local encrypted_file="${backup_file}.gpg"
    
    if echo "$ENCRYPTION_PASSWORD" | gpg --batch --yes --passphrase-fd 0 --symmetric --cipher-algo AES256 --output "$encrypted_file" "$backup_file"; then
        log_success "备份加密完成: $encrypted_file"
        
        # 删除未加密文件
        rm -f "$backup_file"
        
        echo "$encrypted_file"
    else
        log_error "备份加密失败"
        return 1
    fi
}

# 清理旧备份
cleanup_old_backups() {
    log_info "清理旧备份文件..."
    
    if [ ! -d "$BACKUP_DIR" ]; then
        return 0
    fi
    
    # 查找备份文件（包括压缩和加密的）
    local backup_files=()
    while IFS= read -r -d '' file; do
        backup_files+=("$file")
    done < <(find "$BACKUP_DIR" -maxdepth 1 \( -name "*.tar.gz" -o -name "*.tar.gz.gpg" -o -type d -name "[0-9]*_[0-9]*" \) -print0 | sort -z)
    
    local backup_count=${#backup_files[@]}
    
    if [ $backup_count -gt $MAX_BACKUPS ]; then
        local files_to_delete=$((backup_count - MAX_BACKUPS))
        log_info "发现 $backup_count 个备份，删除最旧的 $files_to_delete 个"
        
        for ((i=0; i<files_to_delete; i++)); do
            local file_to_delete="${backup_files[i]}"
            log_info "删除旧备份: $(basename "$file_to_delete")"
            rm -rf "$file_to_delete"
        done
        
        log_success "清理完成，保留 $MAX_BACKUPS 个最新备份"
    else
        log_info "当前有 $backup_count 个备份，无需清理"
    fi
}

# 列出所有备份
list_backups() {
    log_info "备份文件列表:"
    
    if [ ! -d "$BACKUP_DIR" ]; then
        log_info "没有找到备份目录"
        return 0
    fi
    
    echo
    printf "%-20s %-10s %-15s %s\n" "备份名称" "大小" "类型" "创建时间"
    printf "%-20s %-10s %-15s %s\n" "--------------------" "----------" "---------------" "-------------------"
    
    # 列出目录备份
    for backup_dir in "$BACKUP_DIR"/[0-9]*_[0-9]*; do
        if [ -d "$backup_dir" ]; then
            local name=$(basename "$backup_dir")
            local size=$(du -sh "$backup_dir" 2>/dev/null | cut -f1 || echo "未知")
            local date=$(echo "$name" | sed 's/_/ /' | sed 's/\([0-9]\{4\}\)\([0-9]\{2\}\)\([0-9]\{2\}\)/\1-\2-\3/')
            printf "%-20s %-10s %-15s %s\n" "$name" "$size" "目录" "$date"
        fi
    done
    
    # 列出压缩备份
    for backup_file in "$BACKUP_DIR"/*.tar.gz; do
        if [ -f "$backup_file" ]; then
            local name=$(basename "$backup_file" .tar.gz)
            local size=$(du -sh "$backup_file" 2>/dev/null | cut -f1 || echo "未知")
            local date=$(echo "$name" | sed 's/_/ /' | sed 's/\([0-9]\{4\}\)\([0-9]\{2\}\)\([0-9]\{2\}\)/\1-\2-\3/')
            printf "%-20s %-10s %-15s %s\n" "$name" "$size" "压缩" "$date"
        fi
    done
    
    # 列出加密备份
    for backup_file in "$BACKUP_DIR"/*.tar.gz.gpg; do
        if [ -f "$backup_file" ]; then
            local name=$(basename "$backup_file" .tar.gz.gpg)
            local size=$(du -sh "$backup_file" 2>/dev/null | cut -f1 || echo "未知")
            local date=$(echo "$name" | sed 's/_/ /' | sed 's/\([0-9]\{4\}\)\([0-9]\{2\}\)\([0-9]\{2\}\)/\1-\2-\3/')
            printf "%-20s %-10s %-15s %s\n" "$name" "$size" "加密" "$date"
        fi
    done
    
    echo
}

# 测试备份完整性
test_backup() {
    local backup_file="$1"
    
    if [ -z "$backup_file" ]; then
        log_error "请指定要测试的备份文件"
        return 1
    fi
    
    log_info "测试备份完整性: $backup_file"
    
    if [[ "$backup_file" == *.gpg ]]; then
        log_info "测试加密备份..."
        if [ -z "$ENCRYPTION_PASSWORD" ]; then
            read -s -p "请输入解密密码: " ENCRYPTION_PASSWORD
            echo
        fi
        
        if echo "$ENCRYPTION_PASSWORD" | gpg --batch --yes --passphrase-fd 0 --decrypt "$backup_file" | tar -tzf - > /dev/null; then
            log_success "加密备份完整性测试通过"
        else
            log_error "加密备份完整性测试失败"
            return 1
        fi
    elif [[ "$backup_file" == *.tar.gz ]]; then
        log_info "测试压缩备份..."
        if tar -tzf "$backup_file" > /dev/null; then
            log_success "压缩备份完整性测试通过"
        else
            log_error "压缩备份完整性测试失败"
            return 1
        fi
    elif [ -d "$backup_file" ]; then
        log_info "测试目录备份..."
        
        # 检查必要文件
        local required_files=("synapse_db.sql")
        for file in "${required_files[@]}"; do
            if [ ! -f "$backup_file/$file" ]; then
                log_error "缺少必要文件: $file"
                return 1
            fi
        done
        
        # 检查数据库备份
        if [ -f "$backup_file/synapse_db.sql" ]; then
            if grep -q "PostgreSQL database dump" "$backup_file/synapse_db.sql"; then
                log_success "数据库备份格式正确"
            else
                log_error "数据库备份格式错误"
                return 1
            fi
        fi
        
        log_success "目录备份完整性测试通过"
    else
        log_error "不支持的备份文件格式"
        return 1
    fi
}

# 设置定时备份
setup_schedule() {
    log_info "设置定时备份..."
    
    local script_path=$(realpath "$0")
    local cron_job="0 2 * * * $script_path -f > /var/log/synapse-backup.log 2>&1"
    
    echo "建议的crontab条目（每天凌晨2点执行完整备份）:"
    echo "$cron_job"
    echo
    
    read -p "是否添加到当前用户的crontab？(y/N): " add_cron
    
    if [[ $add_cron =~ ^[Yy]$ ]]; then
        (crontab -l 2>/dev/null; echo "$cron_job") | crontab -
        log_success "定时备份已添加到crontab"
    else
        log_info "请手动添加crontab条目"
    fi
}

# 从备份恢复
restore_from_backup() {
    local backup_file="$1"
    
    if [ -z "$backup_file" ]; then
        log_error "请指定备份文件"
        return 1
    fi
    
    if [ ! -f "$backup_file" ] && [ ! -d "$backup_file" ]; then
        log_error "备份文件不存在: $backup_file"
        return 1
    fi
    
    log_warning "恢复操作将覆盖现有数据！"
    read -p "确定要继续吗？输入 'YES' 确认: " confirm
    
    if [ "$confirm" != "YES" ]; then
        log_info "恢复操作已取消"
        return 0
    fi
    
    log_info "开始从备份恢复: $backup_file"
    
    # 停止服务
    log_info "停止Synapse服务..."
    docker-compose stop synapse postgres redis
    
    # 解压备份（如果需要）
    local restore_dir="/tmp/synapse_restore_$$"
    mkdir -p "$restore_dir"
    
    if [[ "$backup_file" == *.gpg ]]; then
        log_info "解密备份文件..."
        if [ -z "$ENCRYPTION_PASSWORD" ]; then
            read -s -p "请输入解密密码: " ENCRYPTION_PASSWORD
            echo
        fi
        
        echo "$ENCRYPTION_PASSWORD" | gpg --batch --yes --passphrase-fd 0 --decrypt "$backup_file" | tar -xzf - -C "$restore_dir"
    elif [[ "$backup_file" == *.tar.gz ]]; then
        log_info "解压备份文件..."
        tar -xzf "$backup_file" -C "$restore_dir"
    elif [ -d "$backup_file" ]; then
        log_info "复制备份目录..."
        cp -r "$backup_file"/* "$restore_dir/"
    fi
    
    # 恢复数据库
    if [ -f "$restore_dir/synapse_db.sql" ]; then
        log_info "恢复数据库..."
        
        # 启动PostgreSQL
        docker-compose up -d postgres
        sleep 10
        
        # 删除现有数据库并重新创建
        docker-compose exec -T postgres psql -U synapse -c "DROP DATABASE IF EXISTS synapse;"
        docker-compose exec -T postgres psql -U synapse -c "CREATE DATABASE synapse;"
        
        # 恢复数据
        docker-compose exec -T postgres psql -U synapse synapse < "$restore_dir/synapse_db.sql"
        
        log_success "数据库恢复完成"
    fi
    
    # 恢复配置文件
    if [ -d "$restore_dir/config" ]; then
        log_info "恢复配置文件..."
        
        # 备份当前配置
        local current_config_backup="config_backup_$(date +%Y%m%d_%H%M%S)"
        mkdir -p "$current_config_backup"
        
        [ -d "synapse" ] && cp -r synapse/ "$current_config_backup/"
        [ -d "nginx" ] && cp -r nginx/ "$current_config_backup/"
        [ -f ".env" ] && cp .env "$current_config_backup/"
        
        # 恢复配置
        cp -r "$restore_dir/config"/* ./
        
        log_success "配置文件恢复完成（原配置备份到: $current_config_backup）"
    fi
    
    # 恢复媒体文件
    if [ -d "$restore_dir/media_store" ]; then
        log_info "恢复媒体文件..."
        
        mkdir -p data/synapse/
        
        # 备份当前媒体文件
        if [ -d "data/synapse/media_store" ]; then
            mv "data/synapse/media_store" "data/synapse/media_store_backup_$(date +%Y%m%d_%H%M%S)"
        fi
        
        cp -r "$restore_dir/media_store" "data/synapse/"
        
        log_success "媒体文件恢复完成"
    fi
    
    # 清理临时目录
    rm -rf "$restore_dir"
    
    # 重启服务
    log_info "重启服务..."
    docker-compose up -d
    
    log_success "恢复完成！"
    log_info "请检查服务状态: docker-compose ps"
}

# 主函数
main() {
    local backup_type="standard"  # standard, full, database, config, media
    local restore_file=""
    local list_backups=false
    local cleanup_only=false
    local test_file=""
    local setup_cron=false
    
    # 解析命令行参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_help
                exit 0
                ;;
            -f|--full)
                backup_type="full"
                shift
                ;;
            -d|--database)
                backup_type="database"
                shift
                ;;
            -c|--config)
                backup_type="config"
                shift
                ;;
            -m|--media)
                backup_type="media"
                shift
                ;;
            -n|--no-compress)
                COMPRESS_BACKUPS=false
                shift
                ;;
            -e|--encrypt)
                ENCRYPT_BACKUPS=true
                shift
                ;;
            -r|--restore)
                restore_file="$2"
                shift 2
                ;;
            -l|--list)
                list_backups=true
                shift
                ;;
            -x|--cleanup)
                cleanup_only=true
                shift
                ;;
            -t|--test)
                test_file="$2"
                shift 2
                ;;
            -s|--schedule)
                setup_cron=true
                shift
                ;;
            *)
                log_error "未知选项: $1"
                show_help
                exit 1
                ;;
        esac
    done
    
    check_dependencies
    
    # 创建备份目录
    mkdir -p "$BACKUP_DIR"
    
    # 执行相应操作
    if [ -n "$restore_file" ]; then
        restore_from_backup "$restore_file"
    elif [ "$list_backups" = true ]; then
        list_backups
    elif [ "$cleanup_only" = true ]; then
        cleanup_old_backups
    elif [ -n "$test_file" ]; then
        test_backup "$test_file"
    elif [ "$setup_cron" = true ]; then
        setup_schedule
    else
        # 执行备份
        log_info "开始 $backup_type 备份..."
        
        local backup_path=$(create_backup_dir)
        local backup_success=true
        
        # 创建备份信息文件
        cat > "$backup_path/backup_info.txt" << EOF
备份类型: $backup_type
备份时间: $(date)
服务器信息: $(uname -a)
Docker版本: $(docker --version)
Docker Compose版本: $(docker-compose --version 2>/dev/null || docker compose version)
Synapse版本: $(docker-compose exec -T synapse python -c "import synapse; print(synapse.__version__)" 2>/dev/null || echo "未知")
EOF
        
        # 根据备份类型执行相应操作
        case $backup_type in
            "full")
                backup_database "$backup_path" || backup_success=false
                backup_config "$backup_path" || backup_success=false
                backup_media "$backup_path" "true" || backup_success=false
                ;;
            "database")
                backup_database "$backup_path" || backup_success=false
                ;;
            "config")
                backup_config "$backup_path" || backup_success=false
                ;;
            "media")
                backup_media "$backup_path" "true" || backup_success=false
                ;;
            "standard")
                backup_database "$backup_path" || backup_success=false
                backup_config "$backup_path" || backup_success=false
                backup_media "$backup_path" "false" || backup_success=false
                ;;
        esac
        
        if [ "$backup_success" = true ]; then
            # 压缩备份
            local final_backup="$backup_path"
            if [ "$COMPRESS_BACKUPS" = true ]; then
                final_backup=$(compress_backup "$backup_path")
            fi
            
            # 加密备份
            if [ "$ENCRYPT_BACKUPS" = true ]; then
                final_backup=$(encrypt_backup "$final_backup")
            fi
            
            log_success "备份完成: $final_backup"
            
            # 清理旧备份
            cleanup_old_backups
            
            # 显示备份信息
            if [ -f "$final_backup" ]; then
                local backup_size=$(du -h "$final_backup" | cut -f1)
                log_info "备份文件大小: $backup_size"
            fi
        else
            log_error "备份过程中出现错误"
            # 清理失败的备份
            rm -rf "$backup_path"
            exit 1
        fi
    fi
}

# 错误处理
trap 'log_error "备份脚本执行失败"; exit 1' ERR

# 执行主函数
main "$@"