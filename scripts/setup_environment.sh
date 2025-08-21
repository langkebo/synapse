#!/bin/bash
# -*- coding: utf-8 -*-

# Synapse2 环境设置脚本
# 用于设置开发和部署环境

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

# 检查系统要求
check_system_requirements() {
    log_info "检查系统要求..."
    
    # 检查操作系统
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        OS="linux"
        log_info "检测到 Linux 系统"
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        OS="macos"
        log_info "检测到 macOS 系统"
    else
        log_error "不支持的操作系统: $OSTYPE"
        exit 1
    fi
    
    # 检查 Python
    if command -v python3 &> /dev/null; then
        PYTHON_VERSION=$(python3 --version | cut -d' ' -f2)
        log_success "Python 3 已安装: $PYTHON_VERSION"
    else
        log_error "Python 3 未安装"
        exit 1
    fi
    
    # 检查 Docker
    if command -v docker &> /dev/null; then
        DOCKER_VERSION=$(docker --version | cut -d' ' -f3 | cut -d',' -f1)
        log_success "Docker 已安装: $DOCKER_VERSION"
    else
        log_warning "Docker 未安装，将跳过 Docker 相关设置"
    fi
    
    # 检查 Docker Compose
    if command -v docker-compose &> /dev/null; then
        COMPOSE_VERSION=$(docker-compose --version | cut -d' ' -f3 | cut -d',' -f1)
        log_success "Docker Compose 已安装: $COMPOSE_VERSION"
    else
        log_warning "Docker Compose 未安装，将跳过 Docker 相关设置"
    fi
}

# 安装 Python 依赖
install_python_dependencies() {
    log_info "安装 Python 依赖..."
    
    # 检查是否有 requirements.txt
    if [ -f "requirements.txt" ]; then
        log_info "安装主要依赖..."
        pip3 install -r requirements.txt
    else
        log_warning "未找到 requirements.txt，跳过主要依赖安装"
    fi
    
    # 安装测试依赖
    if [ -f "tests/requirements.txt" ]; then
        log_info "安装测试依赖..."
        pip3 install -r tests/requirements.txt
    else
        log_warning "未找到测试依赖文件，跳过测试依赖安装"
    fi
    
    log_success "Python 依赖安装完成"
}

# 设置配置文件
setup_config_files() {
    log_info "设置配置文件..."
    
    # 创建必要的目录
    mkdir -p data logs
    
    # 复制示例配置文件
    if [ ! -f "homeserver.yaml" ] && [ -f "homeserver.yaml.example" ]; then
        cp homeserver.yaml.example homeserver.yaml
        log_success "创建 homeserver.yaml 配置文件"
    fi
    
    # 设置环境变量文件
    if [ ! -f ".env" ]; then
        cat > .env << EOF
# Synapse2 环境变量配置

# 数据库配置
POSTGRES_USER=synapse_user
POSTGRES_PASSWORD=synapse_password
POSTGRES_DB=synapse
POSTGRES_HOST=localhost
POSTGRES_PORT=5432

# Redis 配置
REDIS_HOST=localhost
REDIS_PORT=6379
REDIS_PASSWORD=

# Synapse 配置
SYNAPSE_SERVER_NAME=localhost
SYNAPSE_REPORT_STATS=no
SYNAPSE_LOG_LEVEL=INFO

# 性能配置
SYNAPSE_CACHE_SIZE=256M
SYNAPSE_MAX_UPLOAD_SIZE=50M
SYNAPSE_FEDERATION_TIMEOUT=60

# 好友功能配置
FRIENDS_ENABLED=true
FRIENDS_MAX_REQUESTS_PER_USER=100
FRIENDS_REQUEST_TIMEOUT=7d

# 监控配置
MONITORING_ENABLED=true
METRICS_PORT=9090
EOF
        log_success "创建 .env 环境变量文件"
    fi
    
    log_success "配置文件设置完成"
}

# 初始化数据库
init_database() {
    log_info "初始化数据库..."
    
    # 检查是否有数据库迁移文件
    if [ -d "migrations" ] && [ "$(ls -A migrations)" ]; then
        log_info "发现数据库迁移文件"
        
        # 这里可以添加数据库迁移逻辑
        # 例如使用 Alembic 或其他迁移工具
        
        log_success "数据库迁移准备完成"
    else
        log_warning "未找到数据库迁移文件"
    fi
}

# 设置 Docker 环境
setup_docker_environment() {
    if ! command -v docker &> /dev/null; then
        log_warning "Docker 未安装，跳过 Docker 环境设置"
        return
    fi
    
    log_info "设置 Docker 环境..."
    
    # 创建 Docker 网络
    if ! docker network ls | grep -q "synapse-network"; then
        docker network create synapse-network
        log_success "创建 Docker 网络: synapse-network"
    fi
    
    # 拉取必要的镜像
    log_info "拉取 Docker 镜像..."
    docker pull postgres:13-alpine
    docker pull redis:7-alpine
    
    log_success "Docker 环境设置完成"
}

# 运行测试
run_tests() {
    log_info "运行测试..."
    
    if [ -f "scripts/run_tests.py" ]; then
        python3 scripts/run_tests.py --verbose
        log_success "测试运行完成"
    else
        log_warning "未找到测试脚本"
    fi
}

# 验证部署
validate_deployment() {
    log_info "验证部署配置..."
    
    if [ -f "scripts/validate_deployment.py" ]; then
        python3 scripts/validate_deployment.py --save-report
        log_success "部署验证完成"
    else
        log_warning "未找到部署验证脚本"
    fi
}

# 显示帮助信息
show_help() {
    echo "Synapse2 环境设置脚本"
    echo ""
    echo "用法: $0 [选项]"
    echo ""
    echo "选项:"
    echo "  --help, -h          显示此帮助信息"
    echo "  --check-only        仅检查系统要求"
    echo "  --skip-deps         跳过依赖安装"
    echo "  --skip-docker       跳过 Docker 设置"
    echo "  --skip-tests        跳过测试运行"
    echo "  --skip-validation   跳过部署验证"
    echo "  --dev               开发环境设置"
    echo "  --prod              生产环境设置"
    echo ""
}

# 主函数
main() {
    local CHECK_ONLY=false
    local SKIP_DEPS=false
    local SKIP_DOCKER=false
    local SKIP_TESTS=false
    local SKIP_VALIDATION=false
    local ENV_TYPE="dev"
    
    # 解析命令行参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            --help|-h)
                show_help
                exit 0
                ;;
            --check-only)
                CHECK_ONLY=true
                shift
                ;;
            --skip-deps)
                SKIP_DEPS=true
                shift
                ;;
            --skip-docker)
                SKIP_DOCKER=true
                shift
                ;;
            --skip-tests)
                SKIP_TESTS=true
                shift
                ;;
            --skip-validation)
                SKIP_VALIDATION=true
                shift
                ;;
            --dev)
                ENV_TYPE="dev"
                shift
                ;;
            --prod)
                ENV_TYPE="prod"
                shift
                ;;
            *)
                log_error "未知选项: $1"
                show_help
                exit 1
                ;;
        esac
    done
    
    log_info "开始 Synapse2 环境设置 (环境类型: $ENV_TYPE)..."
    
    # 检查系统要求
    check_system_requirements
    
    if [ "$CHECK_ONLY" = true ]; then
        log_success "系统要求检查完成"
        exit 0
    fi
    
    # 安装依赖
    if [ "$SKIP_DEPS" = false ]; then
        install_python_dependencies
    fi
    
    # 设置配置文件
    setup_config_files
    
    # 初始化数据库
    init_database
    
    # 设置 Docker 环境
    if [ "$SKIP_DOCKER" = false ]; then
        setup_docker_environment
    fi
    
    # 运行测试
    if [ "$SKIP_TESTS" = false ]; then
        run_tests
    fi
    
    # 验证部署
    if [ "$SKIP_VALIDATION" = false ]; then
        validate_deployment
    fi
    
    log_success "Synapse2 环境设置完成！"
    
    # 显示下一步操作
    echo ""
    log_info "下一步操作:"
    echo "  1. 编辑 homeserver.yaml 配置文件"
    echo "  2. 编辑 .env 环境变量文件"
    echo "  3. 运行 ./scripts/quick_deploy.sh 开始部署"
    echo ""
}

# 运行主函数
main "$@"