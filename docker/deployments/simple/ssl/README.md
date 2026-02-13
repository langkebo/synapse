# SSL 证书配置说明

此目录用于存放 SSL/TLS 证书文件，用于 HTTPS 加密通信。

## 📁 所需文件

部署前请确保证书文件存在：

```
ssl/
├── fullchain.pem    # 完整证书链 (包含服务器证书 + 中间证书)
└── privkey.pem      # 私钥文件
```

## 🔐 获取证书

### 方式一: Let's Encrypt (推荐)

Let's Encrypt 提供免费的 SSL 证书，有效期 90 天，可自动续期。

```bash
# 1. 安装 certbot
sudo apt update
sudo apt install certbot

# 2. 停止占用 80 端口的服务 (如果有)
sudo systemctl stop nginx  # 或 docker compose down

# 3. 获取证书
sudo certbot certonly --standalone \
  -d your-domain.com \
  -d matrix.your-domain.com

# 4. 复制证书到 ssl 目录
sudo cp /etc/letsencrypt/live/your-domain.com/fullchain.pem ./
sudo cp /etc/letsencrypt/live/your-domain.com/privkey.pem ./
sudo chown $USER:$USER fullchain.pem privkey.pem

# 5. 设置自动续期
sudo crontab -e
# 添加以下行:
0 0,12 * * * certbot renew --quiet --post-hook "docker compose -f /opt/synapse/simple/docker-compose.yml restart nginx"
```

### 方式二: 自签名证书 (仅开发环境)

⚠️ 自签名证书不被浏览器信任，仅用于开发测试。

```bash
# 生成私钥
openssl genrsa -out privkey.pem 2048

# 生成证书签名请求 (CSR)
openssl req -new -key privkey.pem -out server.csr \
  -subj "/C=CN/ST=Beijing/L=Beijing/O=YourOrg/CN=your-domain.com"

# 生成自签名证书 (有效期 365 天)
openssl x509 -req -days 365 -in server.csr -signkey privkey.pem -out fullchain.pem

# 清理临时文件
rm server.csr
```

### 方式三: 商业证书

如果从证书颁发机构 (CA) 购买了证书：

```bash
# 1. 将收到的证书文件合并为 fullchain.pem
cat your_domain.crt intermediate.crt > fullchain.pem

# 2. 使用生成的私钥
cp your_private.key privkey.pem

# 3. 设置权限
chmod 644 fullchain.pem
chmod 600 privkey.pem
```

## 🔒 文件权限

确保证书文件权限正确：

```bash
# 证书文件权限
chmod 644 fullchain.pem

# 私钥文件权限 (重要!)
chmod 600 privkey.pem
```

## ✅ 验证证书

```bash
# 检查证书内容
openssl x509 -in fullchain.pem -text -noout

# 检查私钥是否匹配
openssl x509 -noout -modulus -in fullchain.pem | openssl md5
openssl rsa -noout -modulus -in privkey.pem | openssl md5
# 两个 MD5 值应该相同

# 测试证书链
openssl s_client -connect your-domain.com:443 -showcerts
```

## 🔄 更新证书

证书过期前需要更新：

```bash
# Let's Encrypt 自动续期
sudo certbot renew

# 手动更新后复制证书
sudo cp /etc/letsencrypt/live/your-domain.com/fullchain.pem ./
sudo cp /etc/letsencrypt/live/your-domain.com/privkey.pem ./

# 重启 Nginx 使证书生效
docker compose restart nginx
```

## ⚠️ 安全提醒

1. **私钥文件 (privkey.pem) 必须保密**，不要提交到版本控制
2. 定期检查证书有效期，及时续期
3. 生产环境不要使用自签名证书
4. 使用强加密算法 (RSA 2048+ 或 ECDSA)
5. 启用 HSTS 强制 HTTPS

## 📋 证书信息

| 项目 | 说明 |
|------|------|
| 证书类型 | Let's Encrypt / 商业证书 |
| 有效期 | 90 天 (Let's Encrypt) |
| 续期方式 | 自动 (certbot renew) |
| 端口 | 443 (HTTPS), 8448 (Federation) |
