# 多阶段构建，优化镜像大小
FROM python:3.11-slim as builder

# 设置构建参数
ARG SYNAPSE_VERSION=v1.95.1
ARG DEBIAN_FRONTEND=noninteractive

# 安装构建依赖
RUN apt-get update && apt-get install -y \
    build-essential \
    libffi-dev \
    libjpeg-dev \
    libpq-dev \
    libssl-dev \
    libwebp-dev \
    libxml2-dev \
    libxslt1-dev \
    zlib1g-dev \
    git \
    curl \
    && rm -rf /var/lib/apt/lists/*

# 创建虚拟环境
RUN python -m venv /opt/venv
ENV PATH="/opt/venv/bin:$PATH"

# 升级pip和安装wheel
RUN pip install --no-cache-dir --upgrade pip setuptools wheel

# 复制requirements文件（注意：构建上下文为仓库根目录）
COPY synapse-deployment/requirements.txt /tmp/requirements.txt

# 安装Python依赖
RUN pip install --no-cache-dir -r /tmp/requirements.txt

# 克隆Synapse源码并安装
RUN git clone --depth 1 --branch ${SYNAPSE_VERSION} https://github.com/matrix-org/synapse.git /tmp/synapse
WORKDIR /tmp/synapse

# 复制好友功能扩展代码（仓库根目录存在 synapse/ 上游源码目录）
COPY ./synapse/ /tmp/synapse/

# 安装Synapse
RUN pip install --no-cache-dir -e .

# 运行时镜像
FROM python:3.11-slim as runtime

# 设置环境变量
ENV PYTHONPATH="/opt/venv/lib/python3.11/site-packages"
ENV PATH="/opt/venv/bin:$PATH"
ENV PYTHONUNBUFFERED=1
ENV PYTHONDONTWRITEBYTECODE=1

# 安装运行时依赖
RUN apt-get update && apt-get install -y \
    curl \
    gosu \
    libjpeg62-turbo \
    libpq5 \
    libwebp7 \
    libxml2 \
    libxslt1.1 \
    openssl \
    && rm -rf /var/lib/apt/lists/*

# 复制虚拟环境
COPY --from=builder /opt/venv /opt/venv

# 创建synapse用户
RUN groupadd -r synapse && useradd -r -g synapse synapse

# 创建必要的目录
RUN mkdir -p /data /media /logs && \
    chown -R synapse:synapse /data /media /logs

# 复制启动脚本（从 synapse-deployment/scripts/ 提供）
COPY synapse-deployment/scripts/docker-entrypoint.sh /usr/local/bin/
RUN chmod +x /usr/local/bin/docker-entrypoint.sh

# 暴露端口
EXPOSE 8008 8009 8448 9000

# 健康检查
HEALTHCHECK --interval=30s --timeout=10s --start-period=120s --retries=3 \
    CMD curl -f http://localhost:8008/_matrix/client/versions || exit 1

# 设置工作目录
WORKDIR /data

# 设置用户
USER synapse

# 启动命令
ENTRYPOINT ["/usr/local/bin/docker-entrypoint.sh"]
CMD ["python", "-m", "synapse.app.homeserver", "--config-path", "/data/homeserver.yaml"]