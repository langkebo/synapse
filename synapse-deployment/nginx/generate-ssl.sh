#!/bin/bash

# SSL证书生成脚本
# 为Synapse Matrix服务器生成自签名SSL证书

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

# 配置变量
SSL_DIR="./ssl"
DOMAIN="matrix.cjystx.top"
COUNTRY="CN"
STATE="Beijing"
CITY="Beijing"
ORGANIZATION="Synapse Matrix Server"
ORGANIZATIONAL_UNIT="IT Department"
EMAIL="admin@cjystx.top"
VALIDITY_DAYS=365

# 检查OpenSSL
if ! command -v openssl &> /dev/null; then
    log_error "OpenSSL 未安装，请先安装 OpenSSL"
    exit 1
fi

# 创建SSL目录
log_info "创建SSL证书目录..."
mkdir -p "$SSL_DIR"

# 生成私钥
log_info "生成私钥..."
openssl genrsa -out "$SSL_DIR/matrix.key" 2048
chmod 600 "$SSL_DIR/matrix.key"

# 创建证书签名请求配置
log_info "创建证书配置文件..."
cat > "$SSL_DIR/matrix.conf" << EOF
[req]
distinguished_name = req_distinguished_name
req_extensions = v3_req
prompt = no

[req_distinguished_name]
C = $COUNTRY
ST = $STATE
L = $CITY
O = $ORGANIZATION
OU = $ORGANIZATIONAL_UNIT
CN = $DOMAIN
emailAddress = $EMAIL

[v3_req]
keyUsage = keyEncipherment, dataEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = $DOMAIN
DNS.2 = *.$DOMAIN
DNS.3 = localhost
IP.1 = 127.0.0.1
IP.2 = ::1
EOF

# 生成证书签名请求
log_info "生成证书签名请求..."
openssl req -new -key "$SSL_DIR/matrix.key" -out "$SSL_DIR/matrix.csr" -config "$SSL_DIR/matrix.conf"

# 生成自签名证书
log_info "生成自签名证书..."
openssl x509 -req -in "$SSL_DIR/matrix.csr" -signkey "$SSL_DIR/matrix.key" -out "$SSL_DIR/matrix.crt" -days $VALIDITY_DAYS -extensions v3_req -extfile "$SSL_DIR/matrix.conf"

# 生成DH参数（用于增强安全性）
log_info "生成DH参数（这可能需要几分钟）..."
openssl dhparam -out "$SSL_DIR/dhparam.pem" 2048

# 创建证书链文件
log_info "创建证书链文件..."
cat "$SSL_DIR/matrix.crt" > "$SSL_DIR/matrix-chain.crt"

# 设置文件权限
log_info "设置文件权限..."
chmod 644 "$SSL_DIR/matrix.crt"
chmod 644 "$SSL_DIR/matrix-chain.crt"
chmod 644 "$SSL_DIR/dhparam.pem"
chmod 600 "$SSL_DIR/matrix.key"
chmod 644 "$SSL_DIR/matrix.csr"
chmod 644 "$SSL_DIR/matrix.conf"

# 验证证书
log_info "验证生成的证书..."
if openssl x509 -in "$SSL_DIR/matrix.crt" -text -noout > /dev/null 2>&1; then
    log_success "证书验证成功"
else
    log_error "证书验证失败"
    exit 1
fi

# 显示证书信息
log_info "证书信息："
echo "----------------------------------------"
openssl x509 -in "$SSL_DIR/matrix.crt" -text -noout | grep -A 2 "Subject:"
openssl x509 -in "$SSL_DIR/matrix.crt" -text -noout | grep -A 2 "Validity"
openssl x509 -in "$SSL_DIR/matrix.crt" -text -noout | grep -A 5 "Subject Alternative Name"
echo "----------------------------------------"

# 创建Nginx SSL配置片段
log_info "创建Nginx SSL配置片段..."
cat > "$SSL_DIR/ssl-params.conf" << EOF
# SSL配置参数
ssl_protocols TLSv1.2 TLSv1.3;
ssl_ciphers ECDHE-RSA-AES128-GCM-SHA256:ECDHE-RSA-AES256-GCM-SHA384:ECDHE-RSA-AES128-SHA256:ECDHE-RSA-AES256-SHA384:ECDHE-RSA-AES256-SHA:ECDHE-RSA-AES128-SHA:DHE-RSA-AES256-SHA:DHE-RSA-AES128-SHA;
ssl_prefer_server_ciphers off;
ssl_dhparam /etc/nginx/ssl/dhparam.pem;

# SSL会话缓存
ssl_session_cache shared:SSL:10m;
ssl_session_timeout 10m;
ssl_session_tickets off;

# OCSP装订
ssl_stapling on;
ssl_stapling_verify on;

# 安全头
add_header Strict-Transport-Security "max-age=31536000; includeSubDomains; preload" always;
add_header X-Content-Type-Options nosniff always;
add_header X-Frame-Options DENY always;
add_header X-XSS-Protection "1; mode=block" always;
add_header Referrer-Policy "strict-origin-when-cross-origin" always;
EOF

# 创建证书更新脚本
log_info "创建证书更新脚本..."
cat > "$SSL_DIR/renew-cert.sh" << 'EOF'
#!/bin/bash

# 证书更新脚本
# 用于更新即将过期的SSL证书

SSL_DIR="$(dirname "$0")"
DOMAIN="matrix.cjystx.top"
VALIDITY_DAYS=365

# 检查证书是否即将过期（30天内）
if openssl x509 -checkend 2592000 -noout -in "$SSL_DIR/matrix.crt" > /dev/null 2>&1; then
    echo "证书仍然有效，无需更新"
    exit 0
fi

echo "证书即将过期，开始更新..."

# 备份旧证书
cp "$SSL_DIR/matrix.crt" "$SSL_DIR/matrix.crt.bak.$(date +%Y%m%d)"
cp "$SSL_DIR/matrix.key" "$SSL_DIR/matrix.key.bak.$(date +%Y%m%d)"

# 生成新证书
openssl x509 -req -in "$SSL_DIR/matrix.csr" -signkey "$SSL_DIR/matrix.key" -out "$SSL_DIR/matrix.crt" -days $VALIDITY_DAYS -extensions v3_req -extfile "$SSL_DIR/matrix.conf"

echo "证书更新完成"

# 重新加载Nginx（如果在Docker中运行）
if command -v docker &> /dev/null; then
    docker exec nginx nginx -s reload 2>/dev/null || echo "请手动重新加载Nginx配置"
fi
EOF

chmod +x "$SSL_DIR/renew-cert.sh"

# 创建证书监控脚本
log_info "创建证书监控脚本..."
cat > "$SSL_DIR/check-cert.sh" << 'EOF'
#!/bin/bash

# 证书监控脚本
# 检查证书状态和有效期

SSL_DIR="$(dirname "$0")"
CERT_FILE="$SSL_DIR/matrix.crt"

if [ ! -f "$CERT_FILE" ]; then
    echo "错误：证书文件不存在"
    exit 1
fi

echo "=== SSL证书状态检查 ==="
echo "证书文件：$CERT_FILE"
echo

# 检查证书有效期
echo "证书有效期："
openssl x509 -in "$CERT_FILE" -noout -dates
echo

# 检查证书主题
echo "证书主题："
openssl x509 -in "$CERT_FILE" -noout -subject
echo

# 检查证书颁发者
echo "证书颁发者："
openssl x509 -in "$CERT_FILE" -noout -issuer
echo

# 检查SAN
echo "主题备用名称："
openssl x509 -in "$CERT_FILE" -noout -text | grep -A 5 "Subject Alternative Name" || echo "无SAN扩展"
echo

# 检查证书是否即将过期
if openssl x509 -checkend 2592000 -noout -in "$CERT_FILE" > /dev/null 2>&1; then
    echo "状态：证书有效（30天内不会过期）"
else
    echo "警告：证书将在30天内过期！"
fi

# 检查证书指纹
echo
echo "证书指纹："
echo "SHA1: $(openssl x509 -in "$CERT_FILE" -noout -fingerprint -sha1 | cut -d= -f2)"
echo "SHA256: $(openssl x509 -in "$CERT_FILE" -noout -fingerprint -sha256 | cut -d= -f2)"
EOF

chmod +x "$SSL_DIR/check-cert.sh"

# 创建README文件
log_info "创建README文件..."
cat > "$SSL_DIR/README.md" << EOF
# SSL证书文件说明

## 文件列表

- \`matrix.key\` - 私钥文件（权限：600）
- \`matrix.crt\` - SSL证书文件
- \`matrix.csr\` - 证书签名请求文件
- \`matrix.conf\` - 证书配置文件
- \`matrix-chain.crt\` - 证书链文件
- \`dhparam.pem\` - DH参数文件
- \`ssl-params.conf\` - Nginx SSL配置片段
- \`renew-cert.sh\` - 证书更新脚本
- \`check-cert.sh\` - 证书状态检查脚本

## 使用说明

### 1. 在Nginx中使用

在Nginx配置文件中添加：

\`\`\`nginx
ssl_certificate /etc/nginx/ssl/matrix.crt;
ssl_certificate_key /etc/nginx/ssl/matrix.key;
include /etc/nginx/ssl/ssl-params.conf;
\`\`\`

### 2. 检查证书状态

\`\`\`bash
./check-cert.sh
\`\`\`

### 3. 更新证书

\`\`\`bash
./renew-cert.sh
\`\`\`

### 4. 生产环境建议

- 使用Let's Encrypt等免费CA颁发的证书
- 设置自动更新任务
- 定期备份私钥文件
- 监控证书过期时间

## 安全注意事项

1. 私钥文件(\`matrix.key\`)权限必须是600
2. 不要将私钥文件提交到版本控制系统
3. 定期更新证书
4. 使用强密码保护私钥（如果需要）

## 故障排除

### 证书验证失败

\`\`\`bash
# 验证证书格式
openssl x509 -in matrix.crt -text -noout

# 验证私钥格式
openssl rsa -in matrix.key -check

# 验证证书和私钥匹配
openssl x509 -noout -modulus -in matrix.crt | openssl md5
openssl rsa -noout -modulus -in matrix.key | openssl md5
\`\`\`

### 浏览器证书警告

自签名证书会在浏览器中显示安全警告，这是正常现象。生产环境建议使用CA颁发的证书。

## 域名配置

当前证书配置的域名：
- matrix.cjystx.top
- *.cjystx.top
- localhost

如需修改域名，请编辑生成脚本中的DOMAIN变量并重新生成证书。
EOF

# 显示完成信息
log_success "SSL证书生成完成！"
echo
echo "生成的文件："
ls -la "$SSL_DIR/"
echo
log_info "使用说明："
echo "1. 将SSL目录挂载到Nginx容器的 /etc/nginx/ssl/"
echo "2. 在Nginx配置中引用证书文件"
echo "3. 运行 ./ssl/check-cert.sh 检查证书状态"
echo "4. 生产环境请使用CA颁发的证书替换自签名证书"
echo
log_warning "注意：这是自签名证书，浏览器会显示安全警告"
log_warning "生产环境建议使用Let's Encrypt等免费CA证书"

log_success "脚本执行完成！"