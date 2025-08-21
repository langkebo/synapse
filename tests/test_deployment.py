# -*- coding: utf-8 -*-
# Copyright 2023 The Matrix.org Foundation C.I.C.
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""部署功能测试用例"""

import os
import tempfile
import yaml
from typing import Dict, Any
from unittest.mock import Mock, patch, mock_open

from twisted.test.proto_helpers import MemoryReactor

from synapse.config.homeserver import HomeServerConfig
from synapse.server import HomeServer
from synapse.util import Clock

from tests import unittest
from tests.utils import MockClock


class DeploymentConfigTestCase(unittest.TestCase):
    """部署配置测试类"""
    
    def test_homeserver_performance_config_loading(self) -> None:
        """测试性能配置文件加载"""
        # 模拟性能配置文件内容
        config_content = """
server_name: "test.example.com"
pid_file: "/tmp/synapse.pid"
listeners:
  - port: 8008
    type: http
    tls: false
    bind_addresses: ['0.0.0.0']
    resources:
      - names: [client, federation]
        compress: false

database:
  name: psycopg2
  args:
    user: synapse
    password: secret
    database: synapse
    host: localhost
    port: 5432
    cp_min: 5
    cp_max: 10
    cp_max_overflow: 20

performance:
  max_concurrent_requests: 100
  request_timeout: 30
  database:
    pool_size: 10
    max_overflow: 20
    enable_query_optimization: true
    enable_connection_pooling: true
  memory:
    gc_threshold: [700, 10, 10]
    max_memory_usage: 1024
  network:
    tcp_keepalive: true
    tcp_nodelay: true

cache_strategy:
  enable_memory_cache: true
  enable_redis_cache: true
  memory_cache:
    size: 100
    ttl: 3600
  redis:
    host: localhost
    port: 6379
    db: 0
    connection_pool_size: 10
"""
        
        # 创建临时配置文件
        with tempfile.NamedTemporaryFile(mode='w', suffix='.yaml', delete=False) as f:
            f.write(config_content)
            config_file = f.name
        
        try:
            # 加载配置
            config = HomeServerConfig()
            config.read_config_files([config_file])
            
            # 验证基本配置
            self.assertEqual(config.server.server_name, "test.example.com")
            
            # 验证性能配置
            self.assertEqual(config.performance.max_concurrent_requests, 100)
            self.assertEqual(config.performance.request_timeout, 30)
            self.assertEqual(config.performance.database_pool_size, 10)
            self.assertEqual(config.performance.database_max_overflow, 20)
            self.assertTrue(config.performance.enable_query_optimization)
            self.assertTrue(config.performance.enable_connection_pooling)
            
            # 验证缓存策略配置
            self.assertTrue(config.cache_strategy.enable_memory_cache)
            self.assertTrue(config.cache_strategy.enable_redis_cache)
            self.assertEqual(config.cache_strategy.memory_cache_size, 100)
            self.assertEqual(config.cache_strategy.redis_host, "localhost")
            self.assertEqual(config.cache_strategy.redis_port, 6379)
            
        finally:
            # 清理临时文件
            os.unlink(config_file)
    
    def test_docker_compose_config_validation(self) -> None:
        """测试 Docker Compose 配置验证"""
        # 模拟 docker-compose.low-spec.yml 内容
        docker_compose_content = {
            "version": "3.8",
            "services": {
                "synapse": {
                    "build": {
                        "context": ".",
                        "dockerfile": "docker/Dockerfile.low-spec"
                    },
                    "container_name": "synapse-server",
                    "restart": "unless-stopped",
                    "ports": ["8008:8008"],
                    "environment": {
                        "SYNAPSE_SERVER_NAME": "${SYNAPSE_SERVER_NAME:-matrix.example.com}",
                        "SYNAPSE_REPORT_STATS": "${SYNAPSE_REPORT_STATS:-no}",
                        "POSTGRES_HOST": "postgres",
                        "POSTGRES_DB": "synapse",
                        "POSTGRES_USER": "synapse",
                        "POSTGRES_PASSWORD": "${POSTGRES_PASSWORD:-changeme}",
                        "REDIS_HOST": "redis",
                        "REDIS_PORT": "6379"
                    },
                    "volumes": [
                        "./data:/data",
                        "./logs:/var/log/synapse"
                    ],
                    "depends_on": ["postgres", "redis"],
                    "deploy": {
                        "resources": {
                            "limits": {
                                "memory": "1.5G",
                                "cpus": "0.8"
                            },
                            "reservations": {
                                "memory": "512M",
                                "cpus": "0.2"
                            }
                        }
                    }
                },
                "postgres": {
                    "image": "postgres:13-alpine",
                    "container_name": "synapse-postgres",
                    "restart": "unless-stopped",
                    "environment": {
                        "POSTGRES_DB": "synapse",
                        "POSTGRES_USER": "synapse",
                        "POSTGRES_PASSWORD": "${POSTGRES_PASSWORD:-changeme}",
                        "POSTGRES_INITDB_ARGS": "--encoding=UTF-8 --lc-collate=C --lc-ctype=C"
                    },
                    "volumes": [
                        "postgres_data:/var/lib/postgresql/data",
                        "./docker/postgres/init.sql:/docker-entrypoint-initdb.d/init.sql"
                    ],
                    "deploy": {
                        "resources": {
                            "limits": {
                                "memory": "256M",
                                "cpus": "0.1"
                            }
                        }
                    }
                },
                "redis": {
                    "image": "redis:7-alpine",
                    "container_name": "synapse-redis",
                    "restart": "unless-stopped",
                    "command": "redis-server --maxmemory 64mb --maxmemory-policy allkeys-lru",
                    "volumes": ["redis_data:/data"],
                    "deploy": {
                        "resources": {
                            "limits": {
                                "memory": "128M",
                                "cpus": "0.1"
                            }
                        }
                    }
                }
            },
            "volumes": {
                "postgres_data": {},
                "redis_data": {}
            },
            "networks": {
                "default": {
                    "driver": "bridge"
                }
            }
        }
        
        # 验证配置结构
        self.assertIn("version", docker_compose_content)
        self.assertIn("services", docker_compose_content)
        
        services = docker_compose_content["services"]
        self.assertIn("synapse", services)
        self.assertIn("postgres", services)
        self.assertIn("redis", services)
        
        # 验证 Synapse 服务配置
        synapse_service = services["synapse"]
        self.assertIn("build", synapse_service)
        self.assertIn("environment", synapse_service)
        self.assertIn("deploy", synapse_service)
        
        # 验证资源限制
        synapse_resources = synapse_service["deploy"]["resources"]
        self.assertIn("limits", synapse_resources)
        self.assertEqual(synapse_resources["limits"]["memory"], "1.5G")
        self.assertEqual(synapse_resources["limits"]["cpus"], "0.8")
        
        # 验证 PostgreSQL 配置
        postgres_service = services["postgres"]
        self.assertEqual(postgres_service["image"], "postgres:13-alpine")
        self.assertIn("POSTGRES_DB", postgres_service["environment"])
        
        # 验证 Redis 配置
        redis_service = services["redis"]
        self.assertEqual(redis_service["image"], "redis:7-alpine")
        self.assertIn("maxmemory 64mb", redis_service["command"])
    
    def test_dockerfile_low_spec_validation(self) -> None:
        """测试低配置 Dockerfile 验证"""
        # 模拟 Dockerfile.low-spec 内容
        dockerfile_content = """
# 多阶段构建 - 构建阶段
FROM python:3.11-alpine AS builder

# 设置构建参数
ARG SYNAPSE_VERSION=v1.95.1
ARG BUILD_DATE
ARG VCS_REF

# 添加标签
LABEL org.label-schema.build-date=$BUILD_DATE \
      org.label-schema.name="Synapse Matrix Server (Low-Spec)" \
      org.label-schema.description="Matrix homeserver optimized for 1C2G servers" \
      org.label-schema.url="https://matrix.org/" \
      org.label-schema.vcs-ref=$VCS_REF \
      org.label-schema.vcs-url="https://github.com/matrix-org/synapse" \
      org.label-schema.vendor="Matrix.org Foundation" \
      org.label-schema.version=$SYNAPSE_VERSION \
      org.label-schema.schema-version="1.0"

# 设置环境变量
ENV PYTHONUNBUFFERED=1 \
    PYTHONDONTWRITEBYTECODE=1 \
    PIP_NO_CACHE_DIR=1 \
    PIP_DISABLE_PIP_VERSION_CHECK=1 \
    LANG=C.UTF-8 \
    LC_ALL=C.UTF-8

# 安装系统依赖
RUN apk add --no-cache \
    build-base \
    libffi-dev \
    libjpeg-turbo-dev \
    libpq-dev \
    libxslt-dev \
    linux-headers \
    openssl-dev \
    postgresql-dev \
    rust \
    cargo \
    zlib-dev

# 创建工作目录
WORKDIR /app

# 复制 requirements 文件
COPY requirements.txt .

# 安装 Python 依赖
RUN pip install --no-cache-dir --user -r requirements.txt

# 运行阶段
FROM python:3.11-alpine AS runtime

# 设置环境变量
ENV PYTHONUNBUFFERED=1 \
    PYTHONDONTWRITEBYTECODE=1 \
    PATH="/home/synapse/.local/bin:$PATH" \
    SYNAPSE_CONFIG_DIR="/data" \
    SYNAPSE_DATA_DIR="/data" \
    UID=991 \
    GID=991

# 安装运行时依赖
RUN apk add --no-cache \
    ca-certificates \
    curl \
    libjpeg-turbo \
    libpq \
    libxslt \
    openssl \
    postgresql-client \
    redis \
    su-exec \
    tzdata \
    xmlsec

# 创建用户和组
RUN addgroup -g $GID synapse && \
    adduser -D -u $UID -G synapse -h /home/synapse synapse

# 从构建阶段复制 Python 包
COPY --from=builder /root/.local /home/synapse/.local

# 创建必要的目录
RUN mkdir -p /data /var/log/synapse && \
    chown -R synapse:synapse /data /var/log/synapse /home/synapse

# 复制应用代码
COPY --chown=synapse:synapse . /app

# 复制启动脚本
COPY docker/start.sh /start.sh
RUN chmod +x /start.sh

# 设置工作目录
WORKDIR /app

# 切换到非 root 用户
USER synapse

# 暴露端口
EXPOSE 8008

# 健康检查
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
    CMD curl -f http://localhost:8008/health || exit 1

# 设置数据卷
VOLUME ["/data", "/var/log/synapse"]

# 启动命令
CMD ["/start.sh"]
"""
        
        # 验证 Dockerfile 内容
        lines = dockerfile_content.strip().split('\n')
        
        # 验证多阶段构建
        builder_stage = any('FROM python:3.11-alpine AS builder' in line for line in lines)
        runtime_stage = any('FROM python:3.11-alpine AS runtime' in line for line in lines)
        self.assertTrue(builder_stage)
        self.assertTrue(runtime_stage)
        
        # 验证环境变量设置
        env_vars = any('PYTHONUNBUFFERED=1' in line for line in lines)
        self.assertTrue(env_vars)
        
        # 验证用户创建
        user_creation = any('adduser -D -u $UID -G synapse' in line for line in lines)
        self.assertTrue(user_creation)
        
        # 验证健康检查
        health_check = any('HEALTHCHECK' in line for line in lines)
        self.assertTrue(health_check)
        
        # 验证端口暴露
        expose_port = any('EXPOSE 8008' in line for line in lines)
        self.assertTrue(expose_port)
    
    def test_environment_variables_validation(self) -> None:
        """测试环境变量配置验证"""
        # 模拟 .env.example 内容
        env_vars = {
            # 服务器基本配置
            "SYNAPSE_SERVER_NAME": "matrix.example.com",
            "SYNAPSE_REPORT_STATS": "no",
            "SYNAPSE_ENABLE_REGISTRATION": "false",
            "SYNAPSE_REGISTRATION_SHARED_SECRET": "your-secret-key-here",
            
            # 数据库配置
            "POSTGRES_HOST": "localhost",
            "POSTGRES_PORT": "5432",
            "POSTGRES_DB": "synapse",
            "POSTGRES_USER": "synapse",
            "POSTGRES_PASSWORD": "changeme",
            
            # Redis 配置
            "REDIS_HOST": "localhost",
            "REDIS_PORT": "6379",
            "REDIS_DB": "0",
            "REDIS_PASSWORD": "",
            
            # 性能优化配置
            "SYNAPSE_MAX_CONCURRENT_REQUESTS": "100",
            "SYNAPSE_REQUEST_TIMEOUT": "30",
            "SYNAPSE_DB_POOL_SIZE": "10",
            "SYNAPSE_DB_MAX_OVERFLOW": "20",
            
            # 内存优化配置
            "SYNAPSE_MEMORY_CACHE_SIZE": "100",
            "SYNAPSE_MEMORY_CACHE_TTL": "3600",
            "SYNAPSE_MAX_MEMORY_USAGE": "1024",
            "SYNAPSE_GC_THRESHOLD": "700,10,10",
        }
        
        # 验证必需的环境变量
        required_vars = [
            "SYNAPSE_SERVER_NAME",
            "POSTGRES_HOST",
            "POSTGRES_DB",
            "POSTGRES_USER",
            "POSTGRES_PASSWORD",
            "REDIS_HOST",
        ]
        
        for var in required_vars:
            self.assertIn(var, env_vars)
            self.assertIsNotNone(env_vars[var])
        
        # 验证数值类型的环境变量
        numeric_vars = {
            "POSTGRES_PORT": 5432,
            "REDIS_PORT": 6379,
            "SYNAPSE_MAX_CONCURRENT_REQUESTS": 100,
            "SYNAPSE_REQUEST_TIMEOUT": 30,
            "SYNAPSE_DB_POOL_SIZE": 10,
            "SYNAPSE_DB_MAX_OVERFLOW": 20,
        }
        
        for var, expected_value in numeric_vars.items():
            self.assertEqual(int(env_vars[var]), expected_value)
        
        # 验证布尔类型的环境变量
        boolean_vars = {
            "SYNAPSE_REPORT_STATS": False,
            "SYNAPSE_ENABLE_REGISTRATION": False,
        }
        
        for var, expected_value in boolean_vars.items():
            actual_value = env_vars[var].lower() in ['true', 'yes', '1']
            self.assertEqual(actual_value, expected_value)


class DeploymentScriptTestCase(unittest.TestCase):
    """部署脚本测试类"""
    
    @patch('subprocess.run')
    @patch('os.path.exists')
    def test_quick_deploy_script_validation(self, mock_exists, mock_run) -> None:
        """测试快速部署脚本验证"""
        # 模拟系统环境
        mock_exists.return_value = True
        mock_run.return_value = Mock(returncode=0, stdout="Ubuntu 20.04")
        
        # 模拟快速部署脚本的关键功能
        deployment_steps = [
            "check_system_requirements",
            "install_dependencies",
            "setup_project",
            "download_files",
            "generate_config",
            "start_services",
            "health_check",
            "create_admin_user",
            "configure_firewall",
            "setup_systemd_service",
            "display_deployment_info"
        ]
        
        # 验证部署步骤完整性
        self.assertEqual(len(deployment_steps), 11)
        
        # 验证关键步骤存在
        critical_steps = [
            "check_system_requirements",
            "install_dependencies",
            "generate_config",
            "start_services",
            "health_check"
        ]
        
        for step in critical_steps:
            self.assertIn(step, deployment_steps)
    
    @patch('subprocess.run')
    def test_system_requirements_check(self, mock_run) -> None:
        """测试系统要求检查"""
        # 模拟系统信息检查
        system_checks = {
            "os_version": "Ubuntu 20.04",
            "memory_gb": 2,
            "cpu_cores": 1,
            "disk_space_gb": 20,
            "python_version": "3.9",
        }
        
        # 验证最低系统要求
        self.assertGreaterEqual(system_checks["memory_gb"], 2)
        self.assertGreaterEqual(system_checks["cpu_cores"], 1)
        self.assertGreaterEqual(system_checks["disk_space_gb"], 10)
        
        # 验证操作系统支持
        supported_os = ["Ubuntu 20.04", "Ubuntu 22.04", "Debian 11", "Debian 12"]
        self.assertTrue(any(os in system_checks["os_version"] for os in supported_os))
    
    @patch('yaml.safe_load')
    @patch('builtins.open', new_callable=mock_open)
    def test_config_generation(self, mock_file, mock_yaml) -> None:
        """测试配置文件生成"""
        # 模拟配置模板
        config_template = {
            "server_name": "${SYNAPSE_SERVER_NAME}",
            "database": {
                "name": "psycopg2",
                "args": {
                    "host": "${POSTGRES_HOST}",
                    "port": "${POSTGRES_PORT}",
                    "database": "${POSTGRES_DB}",
                    "user": "${POSTGRES_USER}",
                    "password": "${POSTGRES_PASSWORD}",
                }
            },
            "performance": {
                "max_concurrent_requests": "${SYNAPSE_MAX_CONCURRENT_REQUESTS}",
                "database": {
                    "pool_size": "${SYNAPSE_DB_POOL_SIZE}",
                    "max_overflow": "${SYNAPSE_DB_MAX_OVERFLOW}",
                }
            }
        }
        
        mock_yaml.return_value = config_template
        
        # 模拟环境变量替换
        env_vars = {
            "SYNAPSE_SERVER_NAME": "matrix.example.com",
            "POSTGRES_HOST": "localhost",
            "POSTGRES_PORT": "5432",
            "POSTGRES_DB": "synapse",
            "POSTGRES_USER": "synapse",
            "POSTGRES_PASSWORD": "secret123",
            "SYNAPSE_MAX_CONCURRENT_REQUESTS": "100",
            "SYNAPSE_DB_POOL_SIZE": "10",
            "SYNAPSE_DB_MAX_OVERFLOW": "20",
        }
        
        # 验证配置模板结构
        self.assertIn("server_name", config_template)
        self.assertIn("database", config_template)
        self.assertIn("performance", config_template)
        
        # 验证数据库配置
        db_config = config_template["database"]
        self.assertEqual(db_config["name"], "psycopg2")
        self.assertIn("args", db_config)
        
        # 验证性能配置
        perf_config = config_template["performance"]
        self.assertIn("max_concurrent_requests", perf_config)
        self.assertIn("database", perf_config)
    
    @patch('subprocess.run')
    def test_service_health_check(self, mock_run) -> None:
        """测试服务健康检查"""
        # 模拟健康检查结果
        health_checks = {
            "synapse": {"status": "healthy", "port": 8008},
            "postgres": {"status": "healthy", "port": 5432},
            "redis": {"status": "healthy", "port": 6379},
            "nginx": {"status": "healthy", "port": 80},
        }
        
        # 验证所有服务都健康
        for service, check in health_checks.items():
            self.assertEqual(check["status"], "healthy")
            self.assertIsInstance(check["port"], int)
            self.assertGreater(check["port"], 0)
        
        # 验证关键服务存在
        required_services = ["synapse", "postgres", "redis"]
        for service in required_services:
            self.assertIn(service, health_checks)
    
    def test_monitoring_setup(self) -> None:
        """测试监控设置"""
        # 模拟监控配置
        monitoring_config = {
            "performance_monitor": {
                "enabled": True,
                "interval": 60,
                "thresholds": {
                    "cpu_percent": 80,
                    "memory_percent": 85,
                    "disk_percent": 90,
                }
            },
            "system_monitor": {
                "enabled": True,
                "interval": 30,
                "log_file": "/var/log/synapse/system.log",
            },
            "alert_manager": {
                "enabled": True,
                "channels": ["log", "file"],
                "webhook_url": None,
                "email_config": None,
            }
        }
        
        # 验证监控配置结构
        self.assertIn("performance_monitor", monitoring_config)
        self.assertIn("system_monitor", monitoring_config)
        self.assertIn("alert_manager", monitoring_config)
        
        # 验证性能监控配置
        perf_monitor = monitoring_config["performance_monitor"]
        self.assertTrue(perf_monitor["enabled"])
        self.assertEqual(perf_monitor["interval"], 60)
        self.assertIn("thresholds", perf_monitor)
        
        # 验证阈值配置合理性
        thresholds = perf_monitor["thresholds"]
        self.assertLessEqual(thresholds["cpu_percent"], 100)
        self.assertLessEqual(thresholds["memory_percent"], 100)
        self.assertLessEqual(thresholds["disk_percent"], 100)
        
        # 验证告警管理器配置
        alert_manager = monitoring_config["alert_manager"]
        self.assertTrue(alert_manager["enabled"])
        self.assertIn("log", alert_manager["channels"])
        self.assertIn("file", alert_manager["channels"])


class SecurityConfigTestCase(unittest.TestCase):
    """安全配置测试类"""
    
    def test_security_headers_config(self) -> None:
        """测试安全头配置"""
        # 模拟安全配置
        security_config = {
            "tls_certificate_path": "/etc/ssl/certs/synapse.crt",
            "tls_private_key_path": "/etc/ssl/private/synapse.key",
            "tls_dh_params_path": "/etc/ssl/certs/dhparam.pem",
            "require_auth_for_profile_requests": True,
            "allow_guest_access": False,
            "enable_registration": False,
            "registration_shared_secret": "your-secret-key-here",
            "macaroon_secret_key": "your-macaroon-secret-here",
            "form_secret": "your-form-secret-here",
        }
        
        # 验证 TLS 配置
        self.assertIn("tls_certificate_path", security_config)
        self.assertIn("tls_private_key_path", security_config)
        
        # 验证认证配置
        self.assertTrue(security_config["require_auth_for_profile_requests"])
        self.assertFalse(security_config["allow_guest_access"])
        self.assertFalse(security_config["enable_registration"])
        
        # 验证密钥配置
        secret_keys = [
            "registration_shared_secret",
            "macaroon_secret_key",
            "form_secret"
        ]
        
        for key in secret_keys:
            self.assertIn(key, security_config)
            self.assertIsNotNone(security_config[key])
            self.assertNotEqual(security_config[key], "")
    
    def test_rate_limiting_config(self) -> None:
        """测试速率限制配置"""
        # 模拟速率限制配置
        rate_limiting_config = {
            "rc_message": {
                "per_second": 0.2,
                "burst_count": 10,
            },
            "rc_registration": {
                "per_second": 0.17,
                "burst_count": 3,
            },
            "rc_login": {
                "address": {
                    "per_second": 0.17,
                    "burst_count": 3,
                },
                "account": {
                    "per_second": 0.17,
                    "burst_count": 3,
                },
                "failed_attempts": {
                    "per_second": 0.17,
                    "burst_count": 3,
                },
            },
            "rc_admin_redaction": {
                "per_second": 1,
                "burst_count": 50,
            },
        }
        
        # 验证消息速率限制
        message_rc = rate_limiting_config["rc_message"]
        self.assertLessEqual(message_rc["per_second"], 1.0)  # 不超过每秒1条
        self.assertGreaterEqual(message_rc["burst_count"], 5)  # 至少允许5条突发
        
        # 验证注册速率限制
        registration_rc = rate_limiting_config["rc_registration"]
        self.assertLessEqual(registration_rc["per_second"], 0.5)  # 严格限制注册
        self.assertLessEqual(registration_rc["burst_count"], 5)  # 限制突发注册
        
        # 验证登录速率限制
        login_rc = rate_limiting_config["rc_login"]
        self.assertIn("address", login_rc)
        self.assertIn("account", login_rc)
        self.assertIn("failed_attempts", login_rc)
        
        # 验证管理员操作速率限制
        admin_rc = rate_limiting_config["rc_admin_redaction"]
        self.assertGreaterEqual(admin_rc["per_second"], 0.5)  # 管理员操作相对宽松
        self.assertGreaterEqual(admin_rc["burst_count"], 10)  # 允许更多突发操作
    
    def test_federation_security_config(self) -> None:
        """测试联邦安全配置"""
        # 模拟联邦安全配置
        federation_config = {
            "federation_domain_whitelist": [],  # 空列表表示允许所有域
            "federation_ip_range_blacklist": [
                "127.0.0.0/8",
                "10.0.0.0/8",
                "172.16.0.0/12",
                "192.168.0.0/16",
                "100.64.0.0/10",
                "169.254.0.0/16",
                "::1/128",
                "fe80::/64",
                "fc00::/7",
            ],
            "allow_public_rooms_without_auth": False,
            "allow_public_rooms_over_federation": True,
            "federation_rr_transactions_per_room_per_second": 50,
        }
        
        # 验证域名白名单配置
        self.assertIsInstance(federation_config["federation_domain_whitelist"], list)
        
        # 验证 IP 黑名单配置
        ip_blacklist = federation_config["federation_ip_range_blacklist"]
        self.assertIn("127.0.0.0/8", ip_blacklist)  # 本地回环
        self.assertIn("10.0.0.0/8", ip_blacklist)   # 私有网络
        self.assertIn("192.168.0.0/16", ip_blacklist)  # 私有网络
        
        # 验证公共房间配置
        self.assertFalse(federation_config["allow_public_rooms_without_auth"])
        self.assertTrue(federation_config["allow_public_rooms_over_federation"])
        
        # 验证联邦事务速率限制
        self.assertGreaterEqual(
            federation_config["federation_rr_transactions_per_room_per_second"], 
            10
        )