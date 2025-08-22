#!/bin/bash

# Synapse Matrix 服务器启动脚本
# 适用于 Ubuntu 服务器 (1核2GB)
# 版本: 3.1
# 更新时间: 2025-08-21

set -euo pipefail

# 解析脚本所在目录与部署目录（允许从任意目录调用脚本）
SCRIPT_DIR=$(cd -- "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)
DEPLOY_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 配置变量（支持通过环境变量覆盖，或使用脚本参数覆盖）
DOMAIN_NAME="${DOMAIN_NAME:-cjystx.top}"
MATRIX_DOMAIN="${MATRIX_DOMAIN:-matrix.${DOMAIN_NAME}}"
ELEMENT_DOMAIN="${ELEMENT_DOMAIN:-element.${DOMAIN_NAME}}"
MONITORING_DOMAIN="${MONITORING_DOMAIN:-monitoring.${DOMAIN_NAME}}"

# Docker Compose 命令（自动探测）
COMPOSE_CMD=""

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

# 新增：参数解析函数
parse_args() {
    while getopts ":d:m:e:o:" opt; do
        case "${opt}" in
            d) DOMAIN_NAME="$OPTARG" ;;
            m) MATRIX_DOMAIN="$OPTARG" ;;
            e) ELEMENT_DOMAIN="$OPTARG" ;;
            o) MONITORING_DOMAIN="$OPTARG" ;;
            *) ;;
        esac
    done

    # 根据主域名自动派生未显式指定的子域名
    : "${DOMAIN_NAME}"
    : "${MATRIX_DOMAIN:=matrix.${DOMAIN_NAME}}"
    : "${ELEMENT_DOMAIN:=element.${DOMAIN_NAME}}"
    : "${MONITORING_DOMAIN:=monitoring.${DOMAIN_NAME}}"

    export DOMAIN_NAME MATRIX_DOMAIN ELEMENT_DOMAIN MONITORING_DOMAIN
}

# 新增：项目结构校验，避免因错误的工作目录或缺失文件导致构建失败
verify_project_structure() {
    log_info "校验项目结构和关键文件..."
    local missing=0
    for f in \
        "${DEPLOY_DIR}/docker-compose.yml" \
        "${DEPLOY_DIR}/Dockerfile" \
        "${DEPLOY_DIR}/requirements.txt" \
        "${DEPLOY_DIR}/scripts/docker-entrypoint.sh" \
        "${DEPLOY_DIR}/synapse/homeserver.yaml"; do
        if [[ ! -f "$f" ]]; then
            log_error "缺少文件: $f"
            missing=1
        fi
    done
    if [[ $missing -eq 1 ]]; then
        log_error "项目结构不完整，请确认从仓库根目录克隆的代码未缺失，并在 ${DEPLOY_DIR} 下运行。"
        exit 1
    fi
    log_success "项目结构校验通过"
}

# 生成随机密码函数
generate_password() {
    local length=${1:-32}
    openssl rand -base64 48 | tr -d "=+/" | cut -c1-${length}
}

# 生成 .env 文件函数
generate_env_file() {
    log_info "生成环境配置文件..."
    
    if [[ -f ".env" ]]; then
        log_warning "发现现有 .env 文件，备份为 .env.backup"
        cp .env .env.backup
    fi
    
    # 生成随机密码/密钥
    POSTGRES_PASSWORD=$(generate_password 32)
    REDIS_PASSWORD=$(generate_password 32)
    GRAFANA_PASSWORD=$(generate_password 32)
    SMTP_PASSWORD=$(generate_password 32)
    REGISTRATION_SHARED_SECRET=$(generate_password 32)
    MACAROON_SECRET_KEY=$(generate_password 64)
    FORM_SECRET=$(generate_password 64)
    
    # 创建 .env 文件（尽可能覆盖 homeserver.yaml 和 docker-compose.yml 使用到的环境变量）
    cat > .env << EOF
# =============================================================================
# Synapse Matrix 服务器配置文件
# 域名: ${DOMAIN_NAME}
# Matrix 服务器: ${MATRIX_DOMAIN}
# Element Web: ${ELEMENT_DOMAIN}
# 生成时间: $(date '+%Y-%m-%d %H:%M:%S')
# =============================================================================

# Synapse服务器域名
SYNAPSE_SERVER_NAME=${MATRIX_DOMAIN}
ELEMENT_DOMAIN=${ELEMENT_DOMAIN}

# 管理员邮箱
SYNAPSE_ADMIN_EMAIL=admin@${DOMAIN_NAME}

# =============================================================================
# 数据库配置
# =============================================================================

# PostgreSQL数据库配置
POSTGRES_DB=synapse
POSTGRES_USER=synapse
POSTGRES_PASSWORD=${POSTGRES_PASSWORD}

# 数据库连接配置
POSTGRES_HOST=postgres
POSTGRES_PORT=5432

# =============================================================================
# 缓存配置
# =============================================================================

# Redis配置
REDIS_PASSWORD=${REDIS_PASSWORD}

# Redis连接配置
REDIS_HOST=redis
REDIS_PORT=6379

# =============================================================================
# 监控配置
# =============================================================================

# Grafana配置
GRAFANA_PASSWORD=${GRAFANA_PASSWORD}

# Prometheus配置
PROMETHEUS_RETENTION_TIME=15d
PROMETHEUS_STORAGE_RETENTION_SIZE=1GB

# 功能开关
ENABLE_METRICS=true
REPORT_STATS=no

# =============================================================================
# 网络配置
# =============================================================================

# 端口配置
SYNAPSE_HTTP_PORT=8008
SYNAPSE_HTTPS_PORT=8448
NGINX_HTTP_PORT=80
NGINX_HTTPS_PORT=443
GRAFANA_PORT=3000
PROMETHEUS_PORT=9090
JAEGER_PORT=16686

# =============================================================================
# SSL证书配置
# =============================================================================

# Let's Encrypt配置
LETSENCRYPT_EMAIL=admin@${DOMAIN_NAME}
LETSENCRYPT_DOMAIN=${MATRIX_DOMAIN}

# =============================================================================
# 邮件配置（当前开放注册不需要邮箱验证，可保留以便未来启用）
# =============================================================================

# SMTP服务器配置
SMTP_HOST=smtp.${DOMAIN_NAME}
SMTP_PORT=587
SMTP_USER=noreply@${DOMAIN_NAME}
SMTP_PASSWORD=${SMTP_PASSWORD}
SMTP_FROM=noreply@${DOMAIN_NAME}

# =============================================================================
# Synapse 特定配置
# =============================================================================

# 注册共享密钥（用于管理员生成注册token；当前配置为开放注册，暂不要求token）
REGISTRATION_SHARED_SECRET=${REGISTRATION_SHARED_SECRET}

# 其他密钥
MACAROON_SECRET_KEY=${MACAROON_SECRET_KEY}
FORM_SECRET=${FORM_SECRET}

# 性能及上传大小
SYNAPSE_MAX_UPLOAD_SIZE=50M
SYNAPSE_CACHE_FACTOR=0.5
SYNAPSE_EVENT_CACHE_SIZE=5K

# 好友功能（与 homeserver.yaml 保持一致）
FRIENDS_ENABLED=true
MAX_FRIENDS_PER_USER=500
FRIEND_REQUEST_TIMEOUT_DAYS=7
AUTO_ACCEPT_FRIENDS=false

# =============================================================================
# 资源限制配置（用于文档/外部脚本参考；docker-compose 已内置）
# =============================================================================

# 内存限制 (适用于1核2GB服务器)
SYNAPSE_MEMORY_LIMIT=1g
POSTGRES_MEMORY_LIMIT=512m
REDIS_MEMORY_LIMIT=256m
NGINX_MEMORY_LIMIT=128m
GRAFANA_MEMORY_LIMIT=256m
PROMETHEUS_MEMORY_LIMIT=512m

# CPU限制
SYNAPSE_CPU_LIMIT=0.8
POSTGRES_CPU_LIMIT=0.3
REDIS_CPU_LIMIT=0.2
NGINX_CPU_LIMIT=0.2
GRAFANA_CPU_LIMIT=0.2
PROMETHEUS_CPU_LIMIT=0.3
EOF

    log_success ".env 已生成（包含开放注册、无邮箱验证、无验证码所需配置）"
}

# 检查Docker和Docker Compose
check_dependencies() {
    log_info "检查依赖项..."
    
    if ! command -v docker &> /dev/null; then
        log_error "Docker未安装，请先安装Docker"
        exit 1
    fi
    
    if command -v docker-compose &> /dev/null; then
        COMPOSE_CMD="docker-compose"
    elif docker compose version &> /dev/null; then
        COMPOSE_CMD="docker compose"
    else
        log_error "Docker Compose未安装，请先安装Docker Compose"
        exit 1
    fi
    
    log_success "依赖项检查完成 (使用: ${COMPOSE_CMD})"
}

# 检查环境变量文件
check_env_file() {
    log_info "检查环境变量配置..."
    
    if [ ! -f ".env" ]; then
        log_warning ".env文件不存在，从模板创建..."
        cp .env.example .env
        log_warning "请编辑.env文件并设置正确的配置值"
        log_warning "特别注意修改以下配置："
        echo "  - SYNAPSE_SERVER_NAME (服务器域名)"
        echo "  - POSTGRES_PASSWORD (数据库密码)"
        echo "  - REDIS_PASSWORD (Redis密码)"
        echo "  - GRAFANA_PASSWORD (Grafana密码)"
        read -p "按Enter键继续，或Ctrl+C退出编辑.env文件..."
    fi
    
    # 检查关键配置
    source .env
    
    if [ "$SYNAPSE_SERVER_NAME" = "matrix.yourdomain.com" ]; then
        log_warning "请修改SYNAPSE_SERVER_NAME为您的实际域名"
    fi
    
    log_success "环境变量检查完成"
}

# 创建必要的目录
create_directories() {
    log_info "创建必要的目录..."
    
    mkdir -p ssl
    mkdir -p backups
    mkdir -p logs
    mkdir -p data/synapse
    mkdir -p data/postgres
    mkdir -p data/redis
    mkdir -p data/prometheus
    mkdir -p data/grafana
    
    # 设置权限
    chmod 755 ssl backups logs
    chmod 700 data/postgres
    
    log_success "目录创建完成"
}

# 生成SSL证书（自签名，用于测试）
generate_ssl_cert() {
    if [ ! -f "ssl/cert.pem" ] || [ ! -f "ssl/key.pem" ]; then
        log_info "生成自签名SSL证书（仅用于测试）..."
        
        openssl req -x509 -newkey rsa:4096 -keyout ssl/key.pem -out ssl/cert.pem -days 365 -nodes \
            -subj "/C=CN/ST=Beijing/L=Beijing/O=Matrix/OU=Synapse/CN=${SYNAPSE_SERVER_NAME:-localhost}"
        
        chmod 600 ssl/key.pem
        chmod 644 ssl/cert.pem
        
        log_warning "已生成自签名证书，生产环境请使用有效的SSL证书"
        log_success "SSL证书生成完成"
    else
        log_info "SSL证书已存在，跳过生成"
    fi
}

# 检查系统资源
check_system_resources() {
    log_info "检查系统资源..."
    
    # 检查内存
    total_mem=$(free -m | awk 'NR==2{printf "%.0f", $2}')
    if [ "$total_mem" -lt 1800 ]; then
        log_warning "系统内存少于1.8GB，可能影响性能"
        log_warning "当前内存: ${total_mem}MB"
    else
        log_success "内存检查通过: ${total_mem}MB"
    fi
    
    # 检查磁盘空间
    available_space=$(df -BG . | awk 'NR==2 {print $4}' | sed 's/G//')
    if [ "$available_space" -lt 10 ]; then
        log_warning "可用磁盘空间少于10GB，可能影响运行"
        log_warning "当前可用空间: ${available_space}GB"
    else
        log_success "磁盘空间检查通过: ${available_space}GB可用"
    fi
}

# 构建Docker镜像
build_images() {
    log_info "构建Synapse Docker镜像..."
    
    if ${COMPOSE_CMD} build synapse; then
        log_success "Synapse镜像构建完成"
    else
        log_error "Synapse镜像构建失败"
        exit 1
    fi
}

# 启动服务
start_services() {
    log_info "启动服务..."
    
    # 首先启动基础服务
    log_info "启动数据库和缓存服务..."
    ${COMPOSE_CMD} up -d postgres redis
    
    # 等待数据库就绪
    log_info "等待数据库就绪..."
    sleep 30
    
    # 检查数据库连接（使用TCP避免peer认证问题）
    if ! ${COMPOSE_CMD} exec -T postgres pg_isready -h localhost -p 5432; then
        log_error "数据库连接失败"
        log_info "PostgreSQL容器状态："
        ${COMPOSE_CMD} ps postgres || true
        log_info "最近的PostgreSQL日志（末尾200行）："
        ${COMPOSE_CMD} logs --tail 200 postgres || true
        exit 1
    fi
    
    log_success "数据库就绪"
    
    # 启动Synapse主服务
    log_info "启动Synapse服务..."
    ${COMPOSE_CMD} up -d synapse
    
    # 等待Synapse就绪
    log_info "等待Synapse服务就绪..."
    sleep 60
    
    # 启动其他服务
    log_info "启动Web服务器和监控服务..."
    ${COMPOSE_CMD} up -d nginx prometheus grafana
    
    # 启动监控导出器
    log_info "启动监控导出器..."
    ${COMPOSE_CMD} up -d node-exporter postgres-exporter redis-exporter nginx-exporter
    
    log_success "所有服务启动完成"
}

# 检查服务状态
check_services() {
    log_info "检查服务状态..."
    
    sleep 10
    
    # 检查容器状态
    if ${COMPOSE_CMD} ps | grep -q "Up"; then
        log_success "服务状态检查："
        ${COMPOSE_CMD} ps
    else
        log_error "部分服务启动失败"
        ${COMPOSE_CMD} ps
        exit 1
    fi
    
    # 检查Synapse健康状态
    log_info "检查Synapse健康状态..."
    
    max_attempts=30
    attempt=1
    
    while [ $attempt -le $max_attempts ]; do
        if curl -f http://localhost:8008/health &> /dev/null; then
            log_success "Synapse健康检查通过"
            break
        else
            log_info "等待Synapse就绪... (尝试 $attempt/$max_attempts)"
            sleep 10
            ((attempt++))
        fi
    done
    
    if [ $attempt -gt $max_attempts ]; then
        log_error "Synapse健康检查失败"
        log_error "请检查日志: ${COMPOSE_CMD} logs synapse"
        exit 1
    fi
}

# 显示访问信息
show_access_info() {
    log_success "=== Synapse Matrix服务器部署完成 ==="
    echo
    log_info "服务访问地址："
    echo "  Matrix客户端API: https://${SYNAPSE_SERVER_NAME:-localhost}:443"
    echo "  Matrix联邦API:   https://${SYNAPSE_SERVER_NAME:-localhost}:8448"
    echo "  Element Web:      https://${ELEMENT_DOMAIN:-element.localhost}"
    echo "  Grafana监控:     http://localhost:3000 (admin/${GRAFANA_PASSWORD:-admin_secure_password_2024})"
    echo "  Prometheus:      http://localhost:9091"
    echo
    log_info "注册与安全配置："
    echo "  开放注册: 已启用"
    echo "  邮箱验证: 已关闭"
    echo "  验证码:   已关闭"
    echo
    log_info "好友功能API端点："
    echo "  发送好友请求:    POST /_matrix/client/r0/friends/request"
    echo "  接受好友请求:    POST /_matrix/client/r0/friends/accept"
    echo "  拒绝好友请求:    POST /_matrix/client/r0/friends/reject"
    echo "  删除好友:        DELETE /_matrix/client/r0/friends/{user_id}"
    echo "  获取好友列表:    GET /_matrix/client/r0/friends"
    echo "  搜索用户:        GET /_matrix/client/r0/friends/search"
    echo
    log_info "管理命令："
    echo "  查看日志:        ${COMPOSE_CMD} logs -f [service_name]"
    echo "  重启服务:        ${COMPOSE_CMD} restart [service_name]"
    echo "  停止服务:        ${COMPOSE_CMD} down"
    echo "  更新服务:        ${COMPOSE_CMD} pull && ${COMPOSE_CMD} up -d"
    echo
    log_warning "注意事项："
    echo "  1. 首次启动可能需要几分钟时间"
    echo "  2. 生产环境请替换自签名证书为有效SSL证书"
    echo "  3. 定期备份数据库和配置文件"
    echo "  4. 监控系统资源使用情况"
    echo
}

# 主函数
main() {
    log_info "开始部署Synapse Matrix服务器（包含好友功能）"
    echo
    
    # 解析参数（可覆盖默认域名配置）
    parse_args "$@"

    # 固定工作目录到部署目录，避免误用仓库根目录的 docker-compose.yml
    log_info "将工作目录固定为部署目录: ${DEPLOY_DIR}"
    cd "${DEPLOY_DIR}"

    verify_project_structure
    
    check_dependencies
    
    # 如果 .env 文件不存在，自动生成
    if [ ! -f ".env" ]; then
        log_info "未找到 .env 文件，正在自动生成..."
        generate_env_file
        log_success ".env 文件已自动生成，包含随机密码和域名配置"
    fi
    
    check_env_file
    create_directories
    check_system_resources
    
    # 加载环境变量
    source .env
    
    generate_ssl_cert
    build_images
    start_services
    check_services
    show_access_info
    
    log_success "部署完成！"
}

# 错误处理
trap 'log_error "脚本执行失败，请检查错误信息"; exit 1' ERR

# 执行主函数
main "$@"