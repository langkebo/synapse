#!/bin/bash

# Synapse Matrix服务器停止脚本
# 安全停止所有服务并可选择清理数据

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 日志函数
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 显示帮助信息
show_help() {
    echo "Synapse Matrix服务器停止脚本"
    echo
    echo "用法: $0 [选项]"
    echo
    echo "选项:"
    echo "  -h, --help          显示此帮助信息"
    echo "  -f, --force         强制停止所有容器"
    echo "  -c, --clean         停止并清理所有数据（危险操作）"
    echo "  -b, --backup        停止前创建备份"
    echo "  -s, --service NAME  只停止指定服务"
    echo "  -q, --quiet         静默模式"
    echo
    echo "示例:"
    echo "  $0                  # 正常停止所有服务"
    echo "  $0 -f               # 强制停止所有服务"
    echo "  $0 -b               # 停止前创建备份"
    echo "  $0 -s synapse       # 只停止Synapse服务"
    echo "  $0 -c               # 停止并清理所有数据"
    echo
}

# 检查Docker Compose
check_docker_compose() {
    if ! command -v docker-compose &> /dev/null && ! docker compose version &> /dev/null; then
        log_error "Docker Compose未安装"
        exit 1
    fi
}

# 创建备份
create_backup() {
    log_info "创建备份..."
    
    # 创建备份目录
    backup_dir="backups/$(date +%Y%m%d_%H%M%S)"
    mkdir -p "$backup_dir"
    
    # 备份数据库
    if docker-compose ps postgres | grep -q "Up"; then
        log_info "备份PostgreSQL数据库..."
        docker-compose exec -T postgres pg_dump -U synapse synapse > "$backup_dir/synapse_db.sql"
        log_success "数据库备份完成: $backup_dir/synapse_db.sql"
    else
        log_warning "PostgreSQL服务未运行，跳过数据库备份"
    fi
    
    # 备份配置文件
    log_info "备份配置文件..."
    cp -r synapse/ "$backup_dir/synapse_config/" 2>/dev/null || true
    cp .env "$backup_dir/" 2>/dev/null || true
    cp docker-compose.yml "$backup_dir/" 2>/dev/null || true
    
    # 备份媒体文件（如果存在且不太大）
    if [ -d "data/synapse/media_store" ]; then
        media_size=$(du -sm data/synapse/media_store 2>/dev/null | cut -f1 || echo "0")
        if [ "$media_size" -lt 1000 ]; then  # 小于1GB
            log_info "备份媒体文件..."
            cp -r data/synapse/media_store "$backup_dir/" 2>/dev/null || true
        else
            log_warning "媒体文件过大(${media_size}MB)，跳过备份"
        fi
    fi
    
    log_success "备份完成: $backup_dir"
}

# 停止指定服务
stop_service() {
    local service_name="$1"
    
    if [ -z "$service_name" ]; then
        log_error "服务名称不能为空"
        return 1
    fi
    
    log_info "停止服务: $service_name"
    
    if docker-compose ps "$service_name" | grep -q "Up"; then
        docker-compose stop "$service_name"
        log_success "服务 $service_name 已停止"
    else
        log_warning "服务 $service_name 未运行"
    fi
}

# 优雅停止所有服务
stop_all_services() {
    log_info "开始停止所有服务..."
    
    # 按依赖顺序停止服务
    local services=(
        "nginx-exporter"
        "redis-exporter"
        "postgres-exporter"
        "node-exporter"
        "grafana"
        "prometheus"
        "nginx"
        "synapse"
        "redis"
        "postgres"
    )
    
    for service in "${services[@]}"; do
        if docker-compose ps "$service" 2>/dev/null | grep -q "Up"; then
            log_info "停止服务: $service"
            docker-compose stop "$service"
            
            # 等待服务完全停止
            local timeout=30
            local count=0
            while docker-compose ps "$service" 2>/dev/null | grep -q "Up" && [ $count -lt $timeout ]; do
                sleep 1
                ((count++))
            done
            
            if [ $count -ge $timeout ]; then
                log_warning "服务 $service 停止超时，将强制停止"
                docker-compose kill "$service"
            fi
            
            log_success "服务 $service 已停止"
        else
            log_info "服务 $service 未运行，跳过"
        fi
    done
    
    # 停止所有剩余容器
    docker-compose down
    
    log_success "所有服务已停止"
}

# 强制停止所有服务
force_stop_all() {
    log_warning "强制停止所有服务..."
    
    # 强制停止所有容器
    docker-compose kill
    docker-compose down
    
    # 清理悬挂的容器
    if [ "$(docker ps -aq -f status=exited)" ]; then
        log_info "清理已退出的容器..."
        docker rm $(docker ps -aq -f status=exited) 2>/dev/null || true
    fi
    
    log_success "强制停止完成"
}

# 清理所有数据
clean_all_data() {
    log_warning "这将删除所有数据，包括数据库、媒体文件和配置！"
    read -p "确定要继续吗？输入 'YES' 确认: " confirm
    
    if [ "$confirm" != "YES" ]; then
        log_info "操作已取消"
        return 0
    fi
    
    log_warning "开始清理所有数据..."
    
    # 停止所有服务
    force_stop_all
    
    # 删除Docker卷
    log_info "删除Docker卷..."
    docker-compose down -v
    
    # 删除数据目录
    if [ -d "data" ]; then
        log_info "删除数据目录..."
        rm -rf data/
    fi
    
    # 删除日志
    if [ -d "logs" ]; then
        log_info "删除日志目录..."
        rm -rf logs/
    fi
    
    # 清理Docker镜像（可选）
    read -p "是否删除构建的Docker镜像？(y/N): " clean_images
    if [[ $clean_images =~ ^[Yy]$ ]]; then
        log_info "删除Docker镜像..."
        docker-compose down --rmi all 2>/dev/null || true
    fi
    
    log_success "数据清理完成"
}

# 显示服务状态
show_status() {
    log_info "当前服务状态:"
    
    if docker-compose ps 2>/dev/null | grep -q "Up\|Exit"; then
        docker-compose ps
    else
        log_info "没有运行的服务"
    fi
    
    echo
    
    # 显示资源使用情况
    if command -v docker stats &> /dev/null; then
        log_info "容器资源使用情况:"
        docker stats --no-stream --format "table {{.Container}}\t{{.CPUPerc}}\t{{.MemUsage}}\t{{.NetIO}}" 2>/dev/null || true
    fi
}

# 主函数
main() {
    local force=false
    local clean=false
    local backup=false
    local service_name=""
    local quiet=false
    
    # 解析命令行参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_help
                exit 0
                ;;
            -f|--force)
                force=true
                shift
                ;;
            -c|--clean)
                clean=true
                shift
                ;;
            -b|--backup)
                backup=true
                shift
                ;;
            -s|--service)
                service_name="$2"
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
    
    # 静默模式设置
    if [ "$quiet" = true ]; then
        exec > /dev/null 2>&1
    fi
    
    check_docker_compose
    
    # 显示当前状态
    if [ "$quiet" != true ]; then
        show_status
        echo
    fi
    
    # 创建备份
    if [ "$backup" = true ]; then
        create_backup
    fi
    
    # 执行相应操作
    if [ "$clean" = true ]; then
        clean_all_data
    elif [ -n "$service_name" ]; then
        stop_service "$service_name"
    elif [ "$force" = true ]; then
        force_stop_all
    else
        stop_all_services
    fi
    
    if [ "$quiet" != true ]; then
        echo
        log_success "操作完成"
        
        # 显示最终状态
        show_status
    fi
}

# 错误处理
trap 'log_error "脚本执行失败"; exit 1' ERR

# 执行主函数
main "$@"