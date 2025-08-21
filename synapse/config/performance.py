#
# This file is licensed under the Affero General Public License (AGPL) version 3.
#
# Copyright (C) 2023 New Vector, Ltd
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as
# published by the Free Software Foundation, either version 3 of the
# License, or (at your option) any later version.
#
# See the GNU Affero General Public License for more details:
# <https://www.gnu.org/licenses/agpl-3.0.html>.
#
# 性能优化配置模块
# Performance optimization configuration module
#

import logging
from typing import Any, Dict, Optional

from synapse.types import JsonDict

from ._base import Config, ConfigError

logger = logging.getLogger(__name__)


class PerformanceConfig(Config):
    """性能优化配置类
    
    用于配置 Synapse 服务器的性能优化参数，特别针对低配置服务器环境。
    Performance optimization configuration class for Synapse server,
    especially optimized for low-resource server environments.
    """
    
    section = "performance"
    
    def read_config(self, config: JsonDict, **kwargs: Any) -> None:
        """读取性能配置
        
        Args:
            config: 配置字典
        """
        performance_config = config.get("performance", {})
        
        # 数据库连接池配置
        # Database connection pool configuration
        self.database_pool = performance_config.get("database_pool", {})
        self.max_connections = self.database_pool.get("max_connections", 10)
        self.min_connections = self.database_pool.get("min_connections", 2)
        self.connection_timeout = self.database_pool.get("connection_timeout", 30)
        self.idle_timeout = self.database_pool.get("idle_timeout", 300)
        
        # 内存优化配置
        # Memory optimization configuration
        self.memory_optimization = performance_config.get("memory_optimization", {})
        self.gc_threshold = self.memory_optimization.get("gc_threshold", [700, 10, 10])
        self.max_memory_usage = self.memory_optimization.get("max_memory_usage", "1.5GB")
        self.memory_warning_threshold = self.memory_optimization.get("memory_warning_threshold", 0.8)
        
        # 网络优化配置
        # Network optimization configuration
        self.network_optimization = performance_config.get("network_optimization", {})
        self.tcp_keepalive = self.network_optimization.get("tcp_keepalive", True)
        self.tcp_nodelay = self.network_optimization.get("tcp_nodelay", True)
        self.max_request_size = self.network_optimization.get("max_request_size", "50MB")
        self.request_timeout = self.network_optimization.get("request_timeout", 60)
        
        # 并发控制配置
        # Concurrency control configuration
        self.concurrency = performance_config.get("concurrency", {})
        self.max_concurrent_requests = self.concurrency.get("max_concurrent_requests", 100)
        self.max_worker_threads = self.concurrency.get("max_worker_threads", 4)
        self.thread_pool_size = self.concurrency.get("thread_pool_size", 10)
        
        # 缓存优化配置
        # Cache optimization configuration
        self.cache_optimization = performance_config.get("cache_optimization", {})
        self.enable_redis_cache = self.cache_optimization.get("enable_redis_cache", True)
        self.redis_cache_ttl = self.cache_optimization.get("redis_cache_ttl", 3600)
        self.local_cache_size = self.cache_optimization.get("local_cache_size", "100MB")
        
        # 好友功能性能配置
        # Friends feature performance configuration
        self.friends_performance = performance_config.get("friends_performance", {})
        self.friends_cache_ttl = self.friends_performance.get("cache_ttl", 1800)
        self.friends_batch_size = self.friends_performance.get("batch_size", 50)
        self.friends_max_concurrent_requests = self.friends_performance.get("max_concurrent_requests", 20)
        
        # 数据库查询优化
        # Database query optimization
        self.query_optimization = performance_config.get("query_optimization", {})
        self.enable_query_cache = self.query_optimization.get("enable_query_cache", True)
        self.query_cache_size = self.query_optimization.get("query_cache_size", "50MB")
        self.slow_query_threshold = self.query_optimization.get("slow_query_threshold", 1.0)
        self.enable_query_logging = self.query_optimization.get("enable_query_logging", False)
        
        # 资源限制配置
        # Resource limits configuration
        self.resource_limits = performance_config.get("resource_limits", {})
        self.max_upload_size = self.resource_limits.get("max_upload_size", "10MB")
        self.max_avatar_size = self.resource_limits.get("max_avatar_size", "1MB")
        self.max_room_members = self.resource_limits.get("max_room_members", 1000)
        
        # 监控和指标配置
        # Monitoring and metrics configuration
        self.monitoring = performance_config.get("monitoring", {})
        self.enable_performance_metrics = self.monitoring.get("enable_performance_metrics", True)
        self.metrics_collection_interval = self.monitoring.get("metrics_collection_interval", 60)
        self.enable_slow_request_logging = self.monitoring.get("enable_slow_request_logging", True)
        self.slow_request_threshold = self.monitoring.get("slow_request_threshold", 2.0)
        
        # 验证配置值
        # Validate configuration values
        self._validate_config()
        
    def _validate_config(self) -> None:
        """验证配置参数的有效性
        
        Validate the configuration parameters.
        """
        # 验证数据库连接池配置
        if self.max_connections < self.min_connections:
            raise ConfigError(
                "performance.database_pool.max_connections must be >= min_connections"
            )
            
        if self.max_connections > 50:
            logger.warning(
                "performance.database_pool.max_connections is set to %d, "
                "which may be too high for low-resource servers",
                self.max_connections
            )
            
        # 验证内存配置
        if self.memory_warning_threshold < 0.5 or self.memory_warning_threshold > 1.0:
            raise ConfigError(
                "performance.memory_optimization.memory_warning_threshold must be between 0.5 and 1.0"
            )
            
        # 验证并发配置
        if self.max_concurrent_requests < 1:
            raise ConfigError(
                "performance.concurrency.max_concurrent_requests must be >= 1"
            )
            
        if self.max_worker_threads < 1:
            raise ConfigError(
                "performance.concurrency.max_worker_threads must be >= 1"
            )
            
        # 验证好友功能配置
        if self.friends_batch_size < 1 or self.friends_batch_size > 1000:
            raise ConfigError(
                "performance.friends_performance.batch_size must be between 1 and 1000"
            )
            
    def get_database_pool_config(self) -> Dict[str, Any]:
        """获取数据库连接池配置
        
        Returns:
            数据库连接池配置字典
        """
        return {
            "max_connections": self.max_connections,
            "min_connections": self.min_connections,
            "connection_timeout": self.connection_timeout,
            "idle_timeout": self.idle_timeout,
        }
        
    def get_memory_config(self) -> Dict[str, Any]:
        """获取内存优化配置
        
        Returns:
            内存优化配置字典
        """
        return {
            "gc_threshold": self.gc_threshold,
            "max_memory_usage": self.max_memory_usage,
            "memory_warning_threshold": self.memory_warning_threshold,
        }
        
    def get_friends_performance_config(self) -> Dict[str, Any]:
        """获取好友功能性能配置
        
        Returns:
            好友功能性能配置字典
        """
        return {
            "cache_ttl": self.friends_cache_ttl,
            "batch_size": self.friends_batch_size,
            "max_concurrent_requests": self.friends_max_concurrent_requests,
        }
        
    def is_low_resource_mode(self) -> bool:
        """判断是否为低资源模式
        
        Returns:
            如果配置为低资源模式则返回 True
        """
        # 基于配置参数判断是否为低资源模式
        return (
            self.max_connections <= 10 and
            self.max_worker_threads <= 4 and
            self.max_concurrent_requests <= 100
        )