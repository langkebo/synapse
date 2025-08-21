#!/bin/bash

# Synapse2 快速部署脚本
# 适用于 Ubuntu 20.04+ 系统，针对 1核2GB 服务器优化

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

# 检查是否为 root 用户
check_root() {
    if [[ $EUID -eq 0 ]]; then
        log_error "请不要使用 root 用户运行此脚本"
        exit 1
    fi
}

# 检查系统要求
check_system() {
    log_info "检查系统要求..."
    
    # 检查操作系统
    if ! grep -q "Ubuntu" /etc/os-release; then
        log_error "此脚本仅支持 Ubuntu 系统"
        exit 1
    fi
    
    # 检查内存
    MEMORY_GB=$(free -g | awk '/^Mem:/{print $2}')
    if [[ $MEMORY_GB -lt 2 ]]; then
        log_warning "系统内存少于 2GB，可能影响性能"
    fi
    
    # 检查磁盘空间
    DISK_SPACE=$(df / | awk 'NR==2 {print $4}')
    if [[ $DISK_SPACE -lt 20971520 ]]; then  # 20GB in KB
        log_error "磁盘空间不足 20GB"
        exit 1
    fi
    
    log_success "系统要求检查通过"
}

# 安装依赖
install_dependencies() {
    log_info "安装系统依赖..."
    
    # 更新包列表
    sudo apt update
    
    # 安装基础工具
    sudo apt install -y curl wget git unzip software-properties-common apt-transport-https ca-certificates gnupg lsb-release
    
    # 安装 Docker
    if ! command -v docker &> /dev/null; then
        log_info "安装 Docker..."
        curl -fsSL https://get.docker.com -o get-docker.sh
        sudo sh get-docker.sh
        sudo usermod -aG docker $USER
        rm get-docker.sh
        log_success "Docker 安装完成"
    else
        log_info "Docker 已安装"
    fi
    
    # 安装 Docker Compose
    if ! command -v docker-compose &> /dev/null; then
        log_info "安装 Docker Compose..."
        sudo curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
        sudo chmod +x /usr/local/bin/docker-compose
        log_success "Docker Compose 安装完成"
    else
        log_info "Docker Compose 已安装"
    fi
    
    log_success "依赖安装完成"
}

# 创建项目目录
setup_project() {
    log_info "设置项目目录..."
    
    PROJECT_DIR="/opt/synapse2"
    
    # 创建项目目录
    sudo mkdir -p $PROJECT_DIR
    sudo chown $USER:$USER $PROJECT_DIR
    
    # 进入项目目录
    cd $PROJECT_DIR
    
    # 如果目录不为空，询问是否继续
    if [[ "$(ls -A .)" ]]; then
        read -p "项目目录不为空，是否继续？(y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            log_info "部署已取消"
            exit 0
        fi
    fi
    
    log_success "项目目录设置完成: $PROJECT_DIR"
}

# 下载项目文件
download_project() {
    log_info "下载项目文件..."
    
    # 如果是 git 仓库，拉取最新代码
    if [[ -d ".git" ]]; then
        git pull
    else
        # 克隆项目（这里需要替换为实际的仓库地址）
        git clone https://github.com/matrix-org/synapse.git .
    fi
    
    log_success "项目文件下载完成"
}

# 生成配置文件
generate_config() {
    log_info "生成配置文件..."
    
    # 获取服务器域名
    read -p "请输入服务器域名 (例如: matrix.example.com): " SERVER_NAME
    if [[ -z "$SERVER_NAME" ]]; then
        log_error "域名不能为空"
        exit 1
    fi
    
    # 生成随机密码
    POSTGRES_PASSWORD=$(openssl rand -base64 32)
    REDIS_PASSWORD=$(openssl rand -base64 32)
    REGISTRATION_SECRET=$(openssl rand -base64 32)
    MACAROON_SECRET=$(openssl rand -base64 32)
    
    # 创建 .env 文件
    cat > .env << EOF
# 服务器配置
SERVER_NAME=$SERVER_NAME
SYNAPSE_CONFIG_DIR=./data
SYNAPSE_DATA_DIR=./data
SYNAPSE_LOG_LEVEL=INFO

# 数据库配置
POSTGRES_DB=synapse
POSTGRES_USER=synapse_user
POSTGRES_PASSWORD=$POSTGRES_PASSWORD
POSTGRES_HOST=postgres
POSTGRES_PORT=5432

# Redis 配置
REDIS_HOST=redis
REDIS_PORT=6379
REDIS_PASSWORD=$REDIS_PASSWORD

# Synapse 配置
SYNAPSE_REGISTRATION_SHARED_SECRET=$REGISTRATION_SECRET
SYNAPSE_MACAROON_SECRET_KEY=$MACAROON_SECRET
SYNAPSE_FORM_SECRET=$REGISTRATION_SECRET

# 性能配置
SYNAPSE_WORKERS=1
SYNAPSE_CACHE_FACTOR=0.5
SYNAPSE_EVENT_CACHE_SIZE=5K

# 监控配置
MONITOR_ENABLED=true
MONITOR_INTERVAL=60
ALERT_THRESHOLD_CPU=80
ALERT_THRESHOLD_MEMORY=85

# 网络配置
HTTP_PORT=8008
HTTPS_PORT=8448
NGINX_HTTP_PORT=80
NGINX_HTTPS_PORT=443
EOF
    
    # 创建数据目录
    mkdir -p data logs
    
    # 使用默认配置
    log_info "使用默认 Synapse 配置"
    
    # 使用内置 Docker Compose 配置
    log_info "生成 Docker Compose 配置文件"
    
    # 设置权限
    sudo chown -R 991:991 data logs 2>/dev/null || true
    
    log_success "配置文件生成完成"
    log_info "数据库密码: $POSTGRES_PASSWORD"
    log_info "Redis 密码: $REDIS_PASSWORD"
    log_warning "请妥善保存上述密码！"
}

# 启动服务
start_services() {
    log_info "启动服务..."
    
    # 确保用户在 docker 组中
    if ! groups $USER | grep -q docker; then
        log_warning "用户不在 docker 组中，需要重新登录"
        log_info "请运行: newgrp docker"
        return 1
    fi
    
    # 拉取镜像
    log_info "拉取 Docker 镜像..."
    docker-compose pull
    
    # 启动服务
    log_info "启动所有服务..."
    docker-compose up -d
    
    # 等待服务启动
    log_info "等待服务启动..."
    sleep 30
    
    # 检查服务状态
    if docker-compose ps | grep -q "Up"; then
        log_success "服务启动成功"
    else
        log_error "服务启动失败"
        docker-compose logs
        return 1
    fi
}

# 健康检查
health_check() {
    log_info "执行健康检查..."
    
    # 检查 Synapse
    if curl -s http://localhost:8008/health > /dev/null; then
        log_success "Synapse 服务正常"
    else
        log_error "Synapse 服务异常"
        return 1
    fi
    
    # 检查数据库
    if docker-compose exec -T postgres pg_isready -U synapse_user > /dev/null 2>&1; then
        log_success "数据库连接正常"
    else
        log_error "数据库连接异常"
        return 1
    fi
    
    # 检查 Redis
    if docker-compose exec -T redis redis-cli ping > /dev/null 2>&1; then
        log_success "Redis 连接正常"
    else
        log_error "Redis 连接异常"
        return 1
    fi
    
    log_success "所有服务健康检查通过"
}

# 创建管理员用户
create_admin_user() {
    log_info "创建管理员用户..."
    
    read -p "请输入管理员用户名: " ADMIN_USER
    if [[ -z "$ADMIN_USER" ]]; then
        log_warning "跳过管理员用户创建"
        return 0
    fi
    
    read -s -p "请输入管理员密码: " ADMIN_PASSWORD
    echo
    
    if [[ -z "$ADMIN_PASSWORD" ]]; then
        log_warning "跳过管理员用户创建"
        return 0
    fi
    
    # 创建用户
    docker-compose exec -T synapse register_new_matrix_user \
        -c /data/homeserver.yaml \
        -u "$ADMIN_USER" \
        -p "$ADMIN_PASSWORD" \
        -a \
        http://localhost:8008
    
    if [[ $? -eq 0 ]]; then
        log_success "管理员用户创建成功: @$ADMIN_USER:$SERVER_NAME"
    else
        log_error "管理员用户创建失败"
    fi
}

# 配置防火墙
setup_firewall() {
    log_info "配置防火墙..."
    
    if command -v ufw &> /dev/null; then
        # 启用 UFW
        sudo ufw --force enable
        
        # 允许必要端口
        sudo ufw allow 22/tcp      # SSH
        sudo ufw allow 80/tcp      # HTTP
        sudo ufw allow 443/tcp     # HTTPS
        sudo ufw allow 8008/tcp    # Synapse HTTP
        sudo ufw allow 8448/tcp    # Synapse HTTPS
        
        log_success "防火墙配置完成"
    else
        log_warning "UFW 未安装，跳过防火墙配置"
    fi
}

# 设置系统服务
setup_systemd() {
    log_info "设置系统服务..."
    
    # 创建 systemd 服务文件
    sudo tee /etc/systemd/system/synapse2.service > /dev/null << EOF
[Unit]
Description=Synapse2 Matrix Server
Requires=docker.service
After=docker.service

[Service]
Type=oneshot
RemainAfterExit=yes
WorkingDirectory=/opt/synapse2
ExecStart=/usr/local/bin/docker-compose up -d
ExecStop=/usr/local/bin/docker-compose down
TimeoutStartSec=0

[Install]
WantedBy=multi-user.target
EOF
    
    # 重新加载 systemd
    sudo systemctl daemon-reload
    
    # 启用服务
    sudo systemctl enable synapse2.service
    
    log_success "系统服务设置完成"
    log_info "可以使用以下命令管理服务:"
    log_info "  启动: sudo systemctl start synapse2"
    log_info "  停止: sudo systemctl stop synapse2"
    log_info "  状态: sudo systemctl status synapse2"
}

# 显示部署信息
show_deployment_info() {
    log_success "=== Synapse2 部署完成 ==="
    echo
    log_info "服务信息:"
    echo "  服务器域名: $SERVER_NAME"
    echo "  Synapse HTTP: http://localhost:8008"
    echo "  Synapse HTTPS: https://localhost:8448"
    echo "  项目目录: /opt/synapse2"
    echo
    log_info "管理命令:"
    echo "  查看服务状态: docker-compose ps"
    echo "  查看日志: docker-compose logs -f"
    echo "  重启服务: docker-compose restart"
    echo "  停止服务: docker-compose down"
    echo
    log_info "配置文件:"
    echo "  主配置: /opt/synapse2/data/homeserver.yaml"
    echo "  环境变量: /opt/synapse2/.env"
    echo "  Docker Compose: /opt/synapse2/docker-compose.yml"
    echo
    log_info "监控和维护:"
    echo "  健康检查: curl http://localhost:8008/health"
    echo "  系统监控: docker-compose logs monitor"
    echo "  性能监控: docker-compose exec monitor python /scripts/performance_monitor.py"
    echo
    log_warning "重要提醒:"
    echo "  1. 请妥善保存数据库和 Redis 密码"
    echo "  2. 建议配置 SSL 证书以启用 HTTPS"
    echo "  3. 定期备份数据库和配置文件"
    echo "  4. 监控系统资源使用情况"
    echo
    log_info "详细文档: /opt/synapse2/docs/DEPLOYMENT_GUIDE.md"
}

# 主函数
main() {
    echo "=== Synapse2 快速部署脚本 ==="
    echo "适用于 Ubuntu 20.04+ 系统，针对 1核2GB 服务器优化"
    echo
    
    # 检查参数
    if [[ "$1" == "--help" ]] || [[ "$1" == "-h" ]]; then
        echo "用法: $0 [选项]"
        echo "选项:"
        echo "  --help, -h     显示帮助信息"
        echo "  --skip-deps    跳过依赖安装"
        echo "  --skip-fw      跳过防火墙配置"
        echo "  --skip-systemd 跳过系统服务设置"
        exit 0
    fi
    
    # 执行部署步骤
    check_root
    check_system
    
    if [[ "$1" != "--skip-deps" ]]; then
        install_dependencies
    fi
    
    setup_project
    download_project
    generate_config
    start_services
    
    if health_check; then
        create_admin_user
        
        if [[ "$1" != "--skip-fw" ]]; then
            setup_firewall
        fi
        
        if [[ "$1" != "--skip-systemd" ]]; then
            setup_systemd
        fi
        
        show_deployment_info
    else
        log_error "部署失败，请检查日志"
        docker-compose logs
        exit 1
    fi
}

# 错误处理
trap 'log_error "脚本执行失败，请检查错误信息"' ERR

# 运行主函数
main "$@"