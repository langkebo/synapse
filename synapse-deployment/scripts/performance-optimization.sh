#!/bin/bash

# 性能优化脚本
# 针对Synapse Matrix服务器和好友功能的性能优化
# 适用于1核2GB服务器环境
# 作者: Synapse开发团队
# 版本: 1.0.0
# 日期: 2024年

set -euo pipefail

# ============================================================================
# 全局变量和配置
# ============================================================================

# 脚本信息
SCRIPT_NAME="Synapse性能优化脚本"
SCRIPT_VERSION="1.0.0"
SCRIPT_DATE="2024年"

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# 日志文件
LOG_FILE="/var/log/synapse-optimization.log"
ERROR_LOG="/var/log/synapse-optimization-error.log"

# 配置目录
CONFIG_DIR="/opt/synapse/config"
BACKUP_DIR="/opt/synapse/backup"
SCRIPTS_DIR="/opt/synapse/scripts"

# 系统信息
CPU_CORES=$(nproc)
TOTAL_MEMORY=$(free -m | awk 'NR==2{printf "%.0f", $2}')
AVAILABLE_MEMORY=$(free -m | awk 'NR==2{printf "%.0f", $7}')

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
            echo -e "${GREEN}[INFO]${NC} ${timestamp} - $message" | tee -a "$LOG_FILE"
            ;;
        "WARN")
            echo -e "${YELLOW}[WARN]${NC} ${timestamp} - $message" | tee -a "$LOG_FILE"
            ;;
        "ERROR")
            echo -e "${RED}[ERROR]${NC} ${timestamp} - $message" | tee -a "$LOG_FILE" | tee -a "$ERROR_LOG"
            ;;
        "DEBUG")
            if [[ "${DEBUG:-false}" == "true" ]]; then
                echo -e "${BLUE}[DEBUG]${NC} ${timestamp} - $message" | tee -a "$LOG_FILE"
            fi
            ;;
        "SUCCESS")
            echo -e "${GREEN}[SUCCESS]${NC} ${timestamp} - $message" | tee -a "$LOG_FILE"
            ;;
    esac
}

# 错误处理函数
error_exit() {
    log "ERROR" "$1"
    exit 1
}

# 检查命令是否存在
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# 检查文件是否存在
file_exists() {
    [[ -f "$1" ]]
}

# 检查目录是否存在
dir_exists() {
    [[ -d "$1" ]]
}

# 创建目录
create_dir() {
    local dir="$1"
    if ! dir_exists "$dir"; then
        mkdir -p "$dir"
        log "INFO" "创建目录: $dir"
    fi
}

# 备份文件
backup_file() {
    local file="$1"
    local backup_name="$2"
    
    if file_exists "$file"; then
        local backup_path="$BACKUP_DIR/$backup_name.$(date +%Y%m%d_%H%M%S).bak"
        cp "$file" "$backup_path"
        log "INFO" "备份文件: $file -> $backup_path"
    fi
}

# 检查是否为root用户
check_root() {
    if [[ $EUID -ne 0 ]]; then
        error_exit "此脚本需要root权限运行"
    fi
}

# 检查系统资源
check_system_resources() {
    log "INFO" "检查系统资源..."
    
    log "INFO" "CPU核心数: $CPU_CORES"
    log "INFO" "总内存: ${TOTAL_MEMORY}MB"
    log "INFO" "可用内存: ${AVAILABLE_MEMORY}MB"
    
    # 检查内存是否足够
    if [[ $TOTAL_MEMORY -lt 1800 ]]; then
        log "WARN" "系统内存不足2GB，可能影响性能"
    fi
    
    # 检查磁盘空间
    local disk_usage=$(df / | awk 'NR==2 {print $5}' | sed 's/%//')
    if [[ $disk_usage -gt 80 ]]; then
        log "WARN" "磁盘使用率超过80%: ${disk_usage}%"
    fi
    
    # 检查负载
    local load_avg=$(uptime | awk -F'load average:' '{print $2}' | awk '{print $1}' | sed 's/,//')
    log "INFO" "系统负载: $load_avg"
}

# ============================================================================
# 系统内核参数优化
# ============================================================================

optimize_kernel_parameters() {
    log "INFO" "优化系统内核参数..."
    
    # 备份原始配置
    backup_file "/etc/sysctl.conf" "sysctl.conf"
    
    # 创建Synapse专用的sysctl配置
    cat > "/etc/sysctl.d/99-synapse-optimization.conf" << 'EOF'
# Synapse Matrix服务器性能优化配置
# 适用于1核2GB服务器环境

# ============================================================================
# 网络优化
# ============================================================================

# TCP连接优化
net.core.somaxconn = 1024
net.core.netdev_max_backlog = 2000
net.ipv4.tcp_max_syn_backlog = 2048
net.ipv4.tcp_syncookies = 1
net.ipv4.tcp_tw_reuse = 1
net.ipv4.tcp_fin_timeout = 30
net.ipv4.tcp_keepalive_time = 1200
net.ipv4.tcp_keepalive_probes = 3
net.ipv4.tcp_keepalive_intvl = 15

# TCP缓冲区优化
net.core.rmem_default = 262144
net.core.rmem_max = 16777216
net.core.wmem_default = 262144
net.core.wmem_max = 16777216
net.ipv4.tcp_rmem = 4096 65536 16777216
net.ipv4.tcp_wmem = 4096 65536 16777216

# TCP拥塞控制
net.ipv4.tcp_congestion_control = bbr
net.core.default_qdisc = fq

# 连接跟踪优化
net.netfilter.nf_conntrack_max = 65536
net.netfilter.nf_conntrack_tcp_timeout_established = 1200

# ============================================================================
# 内存管理优化
# ============================================================================

# 虚拟内存优化
vm.swappiness = 10
vm.dirty_ratio = 15
vm.dirty_background_ratio = 5
vm.vfs_cache_pressure = 50
vm.min_free_kbytes = 65536

# 内存过量分配
vm.overcommit_memory = 1
vm.overcommit_ratio = 50

# 透明大页优化
vm.nr_hugepages = 0

# ============================================================================
# 文件系统优化
# ============================================================================

# 文件描述符限制
fs.file-max = 65536
fs.nr_open = 65536

# inotify优化
fs.inotify.max_user_watches = 524288
fs.inotify.max_user_instances = 256

# ============================================================================
# 进程和线程优化
# ============================================================================

# 进程限制
kernel.pid_max = 32768
kernel.threads-max = 16384

# 共享内存优化
kernel.shmmax = 268435456
kernel.shmall = 65536

# ============================================================================
# 安全优化
# ============================================================================

# 地址空间随机化
kernel.randomize_va_space = 2

# 核心转储限制
kernel.core_pattern = /tmp/core_%e_%p_%t
kernel.core_uses_pid = 1

EOF

    # 应用内核参数
    sysctl -p /etc/sysctl.d/99-synapse-optimization.conf
    
    log "SUCCESS" "内核参数优化完成"
}

# ============================================================================
# 系统限制优化
# ============================================================================

optimize_system_limits() {
    log "INFO" "优化系统限制..."
    
    # 备份原始配置
    backup_file "/etc/security/limits.conf" "limits.conf"
    
    # 创建Synapse专用的limits配置
    cat > "/etc/security/limits.d/99-synapse.conf" << 'EOF'
# Synapse Matrix服务器系统限制优化
# 适用于1核2GB服务器环境

# 文件描述符限制
* soft nofile 65536
* hard nofile 65536
root soft nofile 65536
root hard nofile 65536

# 进程数限制
* soft nproc 16384
* hard nproc 16384
root soft nproc 16384
root hard nproc 16384

# 内存锁定限制
* soft memlock unlimited
* hard memlock unlimited

# 栈大小限制
* soft stack 8192
* hard stack 8192

# 核心转储大小限制
* soft core 0
* hard core 0

# CPU时间限制（秒）
* soft cpu unlimited
* hard cpu unlimited

# 数据段大小限制
* soft data unlimited
* hard data unlimited

# 文件大小限制
* soft fsize unlimited
* hard fsize unlimited

# 虚拟内存限制
* soft as unlimited
* hard as unlimited

EOF

    # 更新PAM配置以启用limits
    if ! grep -q "pam_limits.so" /etc/pam.d/common-session; then
        echo "session required pam_limits.so" >> /etc/pam.d/common-session
    fi
    
    log "SUCCESS" "系统限制优化完成"
}

# ============================================================================
# Docker优化
# ============================================================================

optimize_docker() {
    log "INFO" "优化Docker配置..."
    
    if ! command_exists docker; then
        log "WARN" "Docker未安装，跳过Docker优化"
        return
    fi
    
    # 创建Docker配置目录
    create_dir "/etc/docker"
    
    # 备份原始配置
    backup_file "/etc/docker/daemon.json" "docker-daemon.json"
    
    # 创建Docker优化配置
    cat > "/etc/docker/daemon.json" << 'EOF'
{
  "log-driver": "json-file",
  "log-opts": {
    "max-size": "10m",
    "max-file": "3"
  },
  "storage-driver": "overlay2",
  "storage-opts": [
    "overlay2.override_kernel_check=true"
  ],
  "default-ulimits": {
    "nofile": {
      "Name": "nofile",
      "Hard": 65536,
      "Soft": 65536
    },
    "nproc": {
      "Name": "nproc",
      "Hard": 16384,
      "Soft": 16384
    }
  },
  "max-concurrent-downloads": 3,
  "max-concurrent-uploads": 3,
  "default-shm-size": "128M",
  "userland-proxy": false,
  "experimental": false,
  "metrics-addr": "127.0.0.1:9323",
  "live-restore": true,
  "cgroup-parent": "docker.slice",
  "default-runtime": "runc",
  "runtimes": {
    "runc": {
      "path": "runc"
    }
  },
  "exec-opts": ["native.cgroupdriver=systemd"],
  "bridge": "docker0",
  "fixed-cidr": "172.17.0.0/16",
  "default-gateway": "172.17.0.1",
  "ip-forward": true,
  "ip-masq": true,
  "iptables": true,
  "ipv6": false,
  "mtu": 1500,
  "registry-mirrors": [
    "https://docker.mirrors.ustc.edu.cn",
    "https://hub-mirror.c.163.com"
  ],
  "insecure-registries": [],
  "debug": false,
  "hosts": ["unix:///var/run/docker.sock"]
}
EOF

    # 重启Docker服务
    systemctl daemon-reload
    systemctl restart docker
    
    log "SUCCESS" "Docker优化完成"
}

# ============================================================================
# PostgreSQL优化
# ============================================================================

optimize_postgresql() {
    log "INFO" "优化PostgreSQL配置..."
    
    # PostgreSQL配置文件路径
    local pg_config="/opt/synapse/postgres/postgresql.conf"
    
    if ! file_exists "$pg_config"; then
        log "WARN" "PostgreSQL配置文件不存在，跳过优化"
        return
    fi
    
    # 备份原始配置
    backup_file "$pg_config" "postgresql.conf"
    
    # 应用性能优化配置
    cat >> "$pg_config" << 'EOF'

# ============================================================================
# Synapse性能优化配置
# ============================================================================

# 连接和认证优化
max_connections = 100
superuser_reserved_connections = 3
tcp_keepalives_idle = 600
tcp_keepalives_interval = 30
tcp_keepalives_count = 3

# 内存优化
shared_buffers = 256MB
effective_cache_size = 1GB
work_mem = 4MB
maintenance_work_mem = 64MB
temp_buffers = 8MB
max_stack_depth = 2MB

# 检查点优化
checkpoint_timeout = 15min
checkpoint_completion_target = 0.9
checkpoint_warning = 30s
max_wal_size = 1GB
min_wal_size = 80MB

# 查询优化
random_page_cost = 1.1
seq_page_cost = 1.0
cpu_tuple_cost = 0.01
cpu_index_tuple_cost = 0.005
cpu_operator_cost = 0.0025
effective_io_concurrency = 2

# 并行查询优化
max_worker_processes = 2
max_parallel_workers_per_gather = 1
max_parallel_workers = 2
max_parallel_maintenance_workers = 1

# 自动清理优化
autovacuum = on
autovacuum_max_workers = 2
autovacuum_naptime = 30s
autovacuum_vacuum_threshold = 50
autovacuum_analyze_threshold = 50
autovacuum_vacuum_scale_factor = 0.1
autovacuum_analyze_scale_factor = 0.05
autovacuum_vacuum_cost_delay = 10ms
autovacuum_vacuum_cost_limit = 200

# 统计信息优化
default_statistics_target = 100
track_activities = on
track_counts = on
track_io_timing = on
track_functions = all

# JIT编译优化
jit = on
jit_above_cost = 100000
jit_inline_above_cost = 500000
jit_optimize_above_cost = 500000

EOF

    log "SUCCESS" "PostgreSQL优化完成"
}

# ============================================================================
# Redis优化
# ============================================================================

optimize_redis() {
    log "INFO" "优化Redis配置..."
    
    # Redis配置文件路径
    local redis_config="/opt/synapse/redis/redis.conf"
    
    if ! file_exists "$redis_config"; then
        log "WARN" "Redis配置文件不存在，跳过优化"
        return
    fi
    
    # 备份原始配置
    backup_file "$redis_config" "redis.conf"
    
    # 应用性能优化配置
    cat >> "$redis_config" << 'EOF'

# ============================================================================
# Synapse性能优化配置
# ============================================================================

# 内存优化
maxmemory 512mb
maxmemory-policy allkeys-lru
maxmemory-samples 5

# 持久化优化
save 900 1
save 300 10
save 60 10000
stop-writes-on-bgsave-error yes
rdbcompression yes
rdbchecksum yes

# 网络优化
tcp-keepalive 300
timeout 300
tcp-backlog 511

# 客户端优化
maxclients 1000

# 慢日志优化
slowlog-log-slower-than 10000
slowlog-max-len 128

# 延迟监控
latency-monitor-threshold 100

# 线程I/O优化
io-threads 2
io-threads-do-reads yes

# 内存碎片整理
activedefrag yes
active-defrag-ignore-bytes 100mb
active-defrag-threshold-lower 10
active-defrag-threshold-upper 100
active-defrag-cycle-min 5
active-defrag-cycle-max 75

EOF

    log "SUCCESS" "Redis优化完成"
}

# ============================================================================
# Nginx优化
# ============================================================================

optimize_nginx() {
    log "INFO" "优化Nginx配置..."
    
    # Nginx配置文件路径
    local nginx_config="/opt/synapse/nginx/nginx.conf"
    
    if ! file_exists "$nginx_config"; then
        log "WARN" "Nginx配置文件不存在，跳过优化"
        return
    fi
    
    # 备份原始配置
    backup_file "$nginx_config" "nginx.conf"
    
    # 创建性能优化配置片段
    cat > "/opt/synapse/nginx/conf.d/performance.conf" << 'EOF'
# Nginx性能优化配置
# 适用于1核2GB服务器环境

# 工作进程优化
worker_processes auto;
worker_cpu_affinity auto;
worker_rlimit_nofile 65535;

# 事件处理优化
events {
    worker_connections 1024;
    use epoll;
    multi_accept on;
    accept_mutex off;
}

# HTTP优化
http {
    # 基础优化
    sendfile on;
    tcp_nopush on;
    tcp_nodelay on;
    keepalive_timeout 65;
    keepalive_requests 100;
    
    # 缓冲区优化
    client_body_buffer_size 128k;
    client_header_buffer_size 1k;
    large_client_header_buffers 4 4k;
    output_buffers 1 32k;
    postpone_output 1460;
    
    # 超时优化
    client_header_timeout 3m;
    client_body_timeout 3m;
    send_timeout 3m;
    
    # Gzip压缩优化
    gzip on;
    gzip_vary on;
    gzip_min_length 1024;
    gzip_proxied any;
    gzip_comp_level 6;
    gzip_types
        text/plain
        text/css
        text/xml
        text/javascript
        application/json
        application/javascript
        application/xml+rss
        application/atom+xml
        image/svg+xml;
    
    # 缓存优化
    open_file_cache max=1000 inactive=20s;
    open_file_cache_valid 30s;
    open_file_cache_min_uses 2;
    open_file_cache_errors on;
    
    # 连接限制
    limit_conn_zone $binary_remote_addr zone=conn_limit_per_ip:10m;
    limit_req_zone $binary_remote_addr zone=req_limit_per_ip:10m rate=5r/s;
}
EOF

    log "SUCCESS" "Nginx优化完成"
}

# ============================================================================
# 系统服务优化
# ============================================================================

optimize_system_services() {
    log "INFO" "优化系统服务..."
    
    # 禁用不必要的服务
    local services_to_disable=(
        "bluetooth"
        "cups"
        "avahi-daemon"
        "ModemManager"
        "whoopsie"
        "apport"
    )
    
    for service in "${services_to_disable[@]}"; do
        if systemctl is-enabled "$service" >/dev/null 2>&1; then
            systemctl disable "$service"
            systemctl stop "$service"
            log "INFO" "禁用服务: $service"
        fi
    done
    
    # 优化systemd配置
    cat > "/etc/systemd/system.conf.d/99-synapse.conf" << 'EOF'
[Manager]
# 默认超时时间
DefaultTimeoutStartSec=30s
DefaultTimeoutStopSec=30s

# 默认重启设置
DefaultRestartSec=5s

# 默认限制
DefaultLimitNOFILE=65536
DefaultLimitNPROC=16384
DefaultLimitCORE=0

# 日志设置
LogLevel=info
LogTarget=journal

EOF

    # 重新加载systemd配置
    systemctl daemon-reload
    
    log "SUCCESS" "系统服务优化完成"
}

# ============================================================================
# 磁盘I/O优化
# ============================================================================

optimize_disk_io() {
    log "INFO" "优化磁盘I/O..."
    
    # 获取主要磁盘设备
    local main_disk=$(lsblk -no PKNAME $(df / | tail -1 | awk '{print $1}') | head -1)
    
    if [[ -n "$main_disk" ]]; then
        # 设置I/O调度器
        echo "mq-deadline" > "/sys/block/$main_disk/queue/scheduler"
        
        # 优化读取预读
        echo 256 > "/sys/block/$main_disk/queue/read_ahead_kb"
        
        # 优化队列深度
        echo 32 > "/sys/block/$main_disk/queue/nr_requests"
        
        log "INFO" "磁盘I/O优化完成: $main_disk"
    fi
    
    # 创建持久化配置
    cat > "/etc/udev/rules.d/99-synapse-disk-optimization.rules" << 'EOF'
# Synapse磁盘I/O优化规则
ACTION=="add|change", KERNEL=="sd[a-z]", ATTR{queue/scheduler}="mq-deadline"
ACTION=="add|change", KERNEL=="sd[a-z]", ATTR{queue/read_ahead_kb}="256"
ACTION=="add|change", KERNEL=="sd[a-z]", ATTR{queue/nr_requests}="32"
EOF

    log "SUCCESS" "磁盘I/O优化完成"
}

# ============================================================================
# 网络优化
# ============================================================================

optimize_network() {
    log "INFO" "优化网络配置..."
    
    # 启用BBR拥塞控制
    if ! lsmod | grep -q tcp_bbr; then
        modprobe tcp_bbr
        echo "tcp_bbr" >> /etc/modules-load.d/modules.conf
    fi
    
    # 优化网络接口
    local main_interface=$(ip route | grep default | awk '{print $5}' | head -1)
    
    if [[ -n "$main_interface" ]]; then
        # 优化网络接口队列
        ethtool -G "$main_interface" rx 512 tx 512 2>/dev/null || true
        
        # 启用网络接口特性
        ethtool -K "$main_interface" gso on gro on tso on 2>/dev/null || true
        
        log "INFO" "网络接口优化完成: $main_interface"
    fi
    
    # 优化防火墙
    if command_exists ufw; then
        # 优化UFW配置
        ufw --force reset
        ufw default deny incoming
        ufw default allow outgoing
        
        # 允许必要端口
        ufw allow 22/tcp comment 'SSH'
        ufw allow 80/tcp comment 'HTTP'
        ufw allow 443/tcp comment 'HTTPS'
        ufw allow 8008/tcp comment 'Synapse'
        ufw allow 8448/tcp comment 'Synapse Federation'
        
        ufw --force enable
        
        log "INFO" "防火墙优化完成"
    fi
    
    log "SUCCESS" "网络优化完成"
}

# ============================================================================
# 内存优化
# ============================================================================

optimize_memory() {
    log "INFO" "优化内存配置..."
    
    # 禁用透明大页
    echo never > /sys/kernel/mm/transparent_hugepage/enabled
    echo never > /sys/kernel/mm/transparent_hugepage/defrag
    
    # 创建持久化配置
    cat > "/etc/systemd/system/disable-thp.service" << 'EOF'
[Unit]
Description=Disable Transparent Huge Pages (THP)
DefaultDependencies=no
After=sysinit.target local-fs.target
Before=basic.target

[Service]
Type=oneshot
ExecStart=/bin/sh -c 'echo never > /sys/kernel/mm/transparent_hugepage/enabled'
ExecStart=/bin/sh -c 'echo never > /sys/kernel/mm/transparent_hugepage/defrag'
RemainAfterExit=yes

[Install]
WantedBy=basic.target
EOF

    systemctl enable disable-thp.service
    systemctl start disable-thp.service
    
    # 优化NUMA
    if [[ -f /proc/sys/kernel/numa_balancing ]]; then
        echo 0 > /proc/sys/kernel/numa_balancing
    fi
    
    # 清理内存缓存
    sync
    echo 1 > /proc/sys/vm/drop_caches
    
    log "SUCCESS" "内存优化完成"
}

# ============================================================================
# 监控和日志优化
# ============================================================================

optimize_monitoring() {
    log "INFO" "优化监控和日志..."
    
    # 配置日志轮转
    cat > "/etc/logrotate.d/synapse" << 'EOF'
/var/log/synapse/*.log {
    daily
    missingok
    rotate 7
    compress
    delaycompress
    notifempty
    create 644 synapse synapse
    postrotate
        systemctl reload synapse || true
    endscript
}

/var/log/synapse-optimization*.log {
    weekly
    missingok
    rotate 4
    compress
    delaycompress
    notifempty
    create 644 root root
}
EOF

    # 优化journald配置
    cat > "/etc/systemd/journald.conf.d/99-synapse.conf" << 'EOF'
[Journal]
SystemMaxUse=100M
SystemMaxFileSize=10M
SystemMaxFiles=10
MaxRetentionSec=1week
Compress=yes
Seal=yes
SplitMode=uid
RateLimitInterval=30s
RateLimitBurst=1000
EOF

    systemctl restart systemd-journald
    
    log "SUCCESS" "监控和日志优化完成"
}

# ============================================================================
# 性能监控脚本
# ============================================================================

create_performance_monitor() {
    log "INFO" "创建性能监控脚本..."
    
    cat > "/opt/synapse/scripts/performance-monitor.sh" << 'EOF'
#!/bin/bash

# Synapse性能监控脚本
# 定期检查系统性能指标

LOG_FILE="/var/log/synapse-performance.log"
TIMESTAMP=$(date '+%Y-%m-%d %H:%M:%S')

# 获取系统指标
CPU_USAGE=$(top -bn1 | grep "Cpu(s)" | awk '{print $2}' | sed 's/%us,//')
MEMORY_USAGE=$(free | grep Mem | awk '{printf "%.2f", $3/$2 * 100.0}')
DISK_USAGE=$(df / | awk 'NR==2 {print $5}' | sed 's/%//')
LOAD_AVG=$(uptime | awk -F'load average:' '{print $2}' | awk '{print $1}' | sed 's/,//')

# 获取网络连接数
CONN_COUNT=$(ss -tun | wc -l)

# 获取Docker容器状态
if command -v docker >/dev/null 2>&1; then
    DOCKER_CONTAINERS=$(docker ps --format "table {{.Names}}\t{{.Status}}" | tail -n +2)
fi

# 记录性能指标
echo "[$TIMESTAMP] CPU: ${CPU_USAGE}%, Memory: ${MEMORY_USAGE}%, Disk: ${DISK_USAGE}%, Load: ${LOAD_AVG}, Connections: ${CONN_COUNT}" >> "$LOG_FILE"

# 检查告警条件
if (( $(echo "$CPU_USAGE > 80" | bc -l) )); then
    echo "[$TIMESTAMP] ALERT: High CPU usage: ${CPU_USAGE}%" >> "$LOG_FILE"
fi

if (( $(echo "$MEMORY_USAGE > 85" | bc -l) )); then
    echo "[$TIMESTAMP] ALERT: High memory usage: ${MEMORY_USAGE}%" >> "$LOG_FILE"
fi

if [[ $DISK_USAGE -gt 85 ]]; then
    echo "[$TIMESTAMP] ALERT: High disk usage: ${DISK_USAGE}%" >> "$LOG_FILE"
fi

if (( $(echo "$LOAD_AVG > 2.0" | bc -l) )); then
    echo "[$TIMESTAMP] ALERT: High system load: ${LOAD_AVG}" >> "$LOG_FILE"
fi
EOF

    chmod +x "/opt/synapse/scripts/performance-monitor.sh"
    
    # 创建cron任务
    cat > "/etc/cron.d/synapse-performance" << 'EOF'
# Synapse性能监控定时任务
*/5 * * * * root /opt/synapse/scripts/performance-monitor.sh
EOF

    log "SUCCESS" "性能监控脚本创建完成"
}

# ============================================================================
# 系统清理和优化
# ============================================================================

cleanup_system() {
    log "INFO" "清理系统..."
    
    # 清理包缓存
    if command_exists apt; then
        apt autoremove -y
        apt autoclean
        apt clean
    fi
    
    # 清理日志文件
    journalctl --vacuum-time=1week
    
    # 清理临时文件
    find /tmp -type f -atime +7 -delete 2>/dev/null || true
    find /var/tmp -type f -atime +7 -delete 2>/dev/null || true
    
    # 清理缓存
    sync
    echo 1 > /proc/sys/vm/drop_caches
    
    log "SUCCESS" "系统清理完成"
}

# ============================================================================
# 验证优化效果
# ============================================================================

validate_optimization() {
    log "INFO" "验证优化效果..."
    
    # 检查内核参数
    log "INFO" "检查内核参数..."
    sysctl net.core.somaxconn
    sysctl vm.swappiness
    sysctl net.ipv4.tcp_congestion_control
    
    # 检查系统限制
    log "INFO" "检查系统限制..."
    ulimit -n
    ulimit -u
    
    # 检查服务状态
    log "INFO" "检查服务状态..."
    systemctl is-active docker || true
    systemctl is-active postgresql || true
    systemctl is-active redis || true
    systemctl is-active nginx || true
    
    # 检查性能指标
    log "INFO" "当前性能指标:"
    log "INFO" "CPU核心数: $(nproc)"
    log "INFO" "总内存: $(free -h | awk 'NR==2{print $2}')"
    log "INFO" "可用内存: $(free -h | awk 'NR==2{print $7}')"
    log "INFO" "磁盘使用率: $(df -h / | awk 'NR==2{print $5}')"
    log "INFO" "系统负载: $(uptime | awk -F'load average:' '{print $2}')"
    
    log "SUCCESS" "优化验证完成"
}

# ============================================================================
# 主函数
# ============================================================================

main() {
    log "INFO" "开始执行$SCRIPT_NAME v$SCRIPT_VERSION"
    log "INFO" "适用于1核2GB服务器环境的Synapse Matrix服务器优化"
    
    # 检查运行环境
    check_root
    
    # 创建必要目录
    create_dir "$CONFIG_DIR"
    create_dir "$BACKUP_DIR"
    create_dir "$SCRIPTS_DIR"
    
    # 检查系统资源
    check_system_resources
    
    # 执行优化
    optimize_kernel_parameters
    optimize_system_limits
    optimize_docker
    optimize_postgresql
    optimize_redis
    optimize_nginx
    optimize_system_services
    optimize_disk_io
    optimize_network
    optimize_memory
    optimize_monitoring
    
    # 创建监控脚本
    create_performance_monitor
    
    # 清理系统
    cleanup_system
    
    # 验证优化效果
    validate_optimization
    
    log "SUCCESS" "$SCRIPT_NAME 执行完成！"
    log "INFO" "建议重启系统以确保所有优化生效"
    log "INFO" "性能监控日志: /var/log/synapse-performance.log"
    log "INFO" "优化日志: $LOG_FILE"
}

# ============================================================================
# 脚本入口
# ============================================================================

# 处理命令行参数
while [[ $# -gt 0 ]]; do
    case $1 in
        --debug)
            DEBUG=true
            shift
            ;;
        --help|-h)
            echo "用法: $0 [选项]"
            echo "选项:"
            echo "  --debug    启用调试模式"
            echo "  --help     显示帮助信息"
            exit 0
            ;;
        *)
            error_exit "未知参数: $1"
            ;;
    esac
done

# 执行主函数
main "$@"

# ============================================================================
# 脚本结束
# ============================================================================