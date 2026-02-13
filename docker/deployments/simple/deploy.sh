#!/bin/bash

# ============================================================================
# Synapse Rust 一键部署脚本
# ============================================================================
# 适用于 Ubuntu 20.04+ 服务器
# ============================================================================

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 打印函数
print_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
print_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
print_warning() { echo -e "${YELLOW}[WARNING]${NC} $1"; }
print_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# 检查 root 权限
check_root() {
    if [[ $EUID -eq 0 ]]; then
        print_warning "检测到 root 用户，建议使用普通用户运行"
    fi
}

# 检查 Docker
check_docker() {
    print_info "检查 Docker 安装..."
    if ! command -v docker &> /dev/null; then
        print_warning "Docker 未安装，正在安装..."
        curl -fsSL https://get.docker.com | sh
        sudo usermod -aG docker $USER
        print_success "Docker 安装完成，请重新登录后继续"
        exit 0
    fi
    print_success "Docker 已安装: $(docker --version)"
}

# 检查 Docker Compose
check_docker_compose() {
    print_info "检查 Docker Compose 安装..."
    if ! docker compose version &> /dev/null; then
        print_error "Docker Compose 未安装"
        print_info "请安装 Docker Compose v2: https://docs.docker.com/compose/install/"
        exit 1
    fi
    print_success "Docker Compose 已安装: $(docker compose version)"
}

# 创建必要目录
create_directories() {
    print_info "创建必要目录..."
    
    # 数据目录
    mkdir -p data
    mkdir -p data/media
    mkdir -p data/uploads
    mkdir -p data/thumbnails
    mkdir -p data/cache
    mkdir -p data/keys
    
    # 日志目录
    mkdir -p logs
    mkdir -p logs/nginx
    
    # SSL 目录
    mkdir -p ssl
    
    print_success "目录创建完成"
    
    echo ""
    echo "目录结构:"
    echo "  data/           - 数据目录"
    echo "  data/media/     - 媒体文件存储"
    echo "  data/uploads/   - 上传临时文件"
    echo "  data/thumbnails/ - 缩略图存储"
    echo "  data/cache/     - 缓存目录"
    echo "  data/keys/      - 签名密钥存储"
    echo "  logs/           - 日志目录"
    echo "  logs/nginx/     - Nginx 日志"
    echo "  ssl/            - SSL 证书"
    echo ""
}

# 生成随机密钥
generate_secrets() {
    print_info "生成安全密钥..."
    
    SECRET_KEY=$(openssl rand -hex 32)
    REGISTRATION_SECRET=$(openssl rand -hex 16)
    ADMIN_SECRET=$(openssl rand -hex 16)
    DB_PASSWORD=$(openssl rand -hex 16)
    
    print_success "密钥生成完成"
    echo ""
    echo "生成的密钥 (请保存):"
    echo "  SECRET_KEY=$SECRET_KEY"
    echo "  REGISTRATION_SECRET=$REGISTRATION_SECRET"
    echo "  ADMIN_SECRET=$ADMIN_SECRET"
    echo "  DB_PASSWORD=$DB_PASSWORD"
    echo ""
}

# 创建 .env 文件
create_env_file() {
    print_info "创建环境变量文件..."
    
    if [[ -f .env ]]; then
        print_warning ".env 文件已存在，跳过创建"
        return
    fi
    
    read -p "请输入服务器域名 (例如: matrix.example.com): " SERVER_NAME
    read -p "请输入管理员邮箱 (例如: admin@example.com): " ADMIN_EMAIL
    
    cat > .env << EOF
# Synapse Rust 环境变量配置
# 生成时间: $(date)

# 服务器配置
SERVER_NAME=${SERVER_NAME}
ADMIN_EMAIL=${ADMIN_EMAIL}

# 数据库配置
DB_PASSWORD=${DB_PASSWORD:-synapse_secure_password}

# 安全密钥 (生产环境必须修改!)
SECRET_KEY=${SECRET_KEY:-change_this_to_random_hex_string}
REGISTRATION_SECRET=${REGISTRATION_SECRET:-change_this_registration_secret}
ADMIN_SECRET=${ADMIN_SECRET:-change_this_admin_secret}

# 联邦签名密钥 (使用 generate_test_keypair 生成)
SIGNING_KEY=${SIGNING_KEY:-5XVPWT8O/DyaXT17qJpUnEO5aRl1Cmevojx0+8uPYV8=}
KEY_ID=${KEY_ID:-ed25519:testkb1OUw}

# 日志级别
RUST_LOG=warn
RUST_BACKTRACE=0

# 注册设置
ENABLE_REGISTRATION=false
EOF
    
    print_success ".env 文件创建完成"
}

# 拉取镜像
pull_images() {
    print_info "拉取 Docker 镜像..."
    docker compose pull
    print_success "镜像拉取完成"
}

# 启动服务
start_services() {
    print_info "启动服务..."
    docker compose up -d
    print_success "服务启动完成"
}

# 检查服务状态
check_status() {
    print_info "检查服务状态..."
    sleep 5
    docker compose ps
    
    echo ""
    print_info "检查服务健康..."
    if curl -sf http://localhost:8008/_matrix/client/versions > /dev/null 2>&1; then
        print_success "Synapse 服务正常运行"
    else
        print_warning "Synapse 服务可能未完全启动，请稍后检查"
    fi
}

# 显示后续步骤
show_next_steps() {
    echo ""
    echo "=========================================="
    echo "          部署完成！"
    echo "=========================================="
    echo ""
    echo "后续步骤:"
    echo ""
    echo "1. 配置 SSL 证书:"
    echo "   sudo certbot certonly --standalone -d \$SERVER_NAME"
    echo "   sudo cp /etc/letsencrypt/live/\$SERVER_NAME/fullchain.pem ssl/"
    echo "   sudo cp /etc/letsencrypt/live/\$SERVER_NAME/privkey.pem ssl/"
    echo ""
    echo "2. 创建管理员账户:"
    echo "   curl -X POST http://localhost:8008/_synapse/admin/v1/register \\"
    echo "     -H 'Content-Type: application/json' \\"
    echo "     -d '{\"username\":\"admin\",\"password\":\"PASSWORD\",\"admin\":true}'"
    echo ""
    echo "3. 查看日志:"
    echo "   docker compose logs -f synapse-rust"
    echo ""
    echo "4. 访问服务:"
    echo "   客户端 API: http://\$SERVER_NAME:8008"
    echo "   联邦 API: https://\$SERVER_NAME:8448"
    echo ""
    echo "5. 目录说明:"
    echo "   data/media/     - 媒体文件存储"
    echo "   data/keys/      - 签名密钥存储"
    echo "   logs/           - 日志文件"
    echo ""
}

# 主函数
main() {
    echo ""
    echo "=========================================="
    echo "    Synapse Rust 一键部署脚本"
    echo "=========================================="
    echo ""
    
    check_root
    check_docker
    check_docker_compose
    create_directories
    generate_secrets
    create_env_file
    pull_images
    start_services
    check_status
    show_next_steps
}

# 运行主函数
main "$@"
