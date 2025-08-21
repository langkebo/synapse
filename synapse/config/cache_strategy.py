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
# 缓存策略配置模块
# Cache strategy configuration module
#

import logging
from typing import Any, Dict, List, Optional, Union

from synapse.types import JsonDict

from ._base import Config, ConfigError

logger = logging.getLogger(__name__)


class CacheStrategyConfig(Config):
    """缓存策略配置类
    
    用于配置 Synapse 服务器的缓存策略，包括 Redis 缓存、内存缓存和好友功能专用缓存。
    Cache strategy configuration class for Synapse server,
    including Redis cache, memory cache, and friends feature specific cache.
    """
    
    section = "cache_strategy"
    
    def read_config(self, config: JsonDict, **kwargs: Any) -> None:
        """读取缓存策略配置
        
        Args:
            config: 配置字典
        """
        cache_strategy_config = config.get("cache_strategy", {})
        
        # Redis 缓存配置
        # Redis cache configuration
        self.redis_config = cache_strategy_config.get("redis", {})
        self.redis_enabled = self.redis_config.get("enabled", True)
        self.redis_host = self.redis_config.get("host", "localhost")
        self.redis_port = self.redis_config.get("port", 6379)
        self.redis_password = self.redis_config.get("password")
        self.redis_db = self.redis_config.get("db", 0)
        self.redis_max_connections = self.redis_config.get("max_connections", 20)
        self.redis_connection_timeout = self.redis_config.get("connection_timeout", 5)
        self.redis_socket_timeout = self.redis_config.get("socket_timeout", 5)
        
        # Redis 连接池配置
        # Redis connection pool configuration
        self.redis_pool_config = self.redis_config.get("pool", {})
        self.redis_pool_max_connections = self.redis_pool_config.get("max_connections", 50)
        self.redis_pool_retry_on_timeout = self.redis_pool_config.get("retry_on_timeout", True)
        self.redis_pool_health_check_interval = self.redis_pool_config.get("health_check_interval", 30)
        
        # 内存缓存配置
        # Memory cache configuration
        self.memory_cache_config = cache_strategy_config.get("memory_cache", {})
        self.memory_cache_enabled = self.memory_cache_config.get("enabled", True)
        self.memory_cache_max_size = self.memory_cache_config.get("max_size", "200MB")
        self.memory_cache_ttl = self.memory_cache_config.get("ttl", 1800)  # 30分钟
        self.memory_cache_cleanup_interval = self.memory_cache_config.get("cleanup_interval", 300)  # 5分钟
        
        # 缓存层级配置
        # Cache tier configuration
        self.cache_tiers = cache_strategy_config.get("tiers", {})
        self.l1_cache_size = self.cache_tiers.get("l1_cache_size", "50MB")  # L1: 内存缓存
        self.l2_cache_size = self.cache_tiers.get("l2_cache_size", "500MB")  # L2: Redis缓存
        self.cache_promotion_threshold = self.cache_tiers.get("promotion_threshold", 3)  # 访问次数阈值
        
        # 好友功能缓存配置
        # Friends feature cache configuration
        self.friends_cache_config = cache_strategy_config.get("friends_cache", {})
        self.friends_cache_enabled = self.friends_cache_config.get("enabled", True)
        
        # 好友关系缓存
        self.friends_relationships_cache = self.friends_cache_config.get("relationships", {})
        self.friends_relationships_ttl = self.friends_relationships_cache.get("ttl", 3600)  # 1小时
        self.friends_relationships_max_size = self.friends_relationships_cache.get("max_size", 10000)
        
        # 好友请求缓存
        self.friends_requests_cache = self.friends_cache_config.get("requests", {})
        self.friends_requests_ttl = self.friends_requests_cache.get("ttl", 1800)  # 30分钟
        self.friends_requests_max_size = self.friends_requests_cache.get("max_size", 5000)
        
        # 好友在线状态缓存
        self.friends_presence_cache = self.friends_cache_config.get("presence", {})
        self.friends_presence_ttl = self.friends_presence_cache.get("ttl", 300)  # 5分钟
        self.friends_presence_max_size = self.friends_presence_cache.get("max_size", 20000)
        
        # 好友推荐缓存
        self.friends_recommendations_cache = self.friends_cache_config.get("recommendations", {})
        self.friends_recommendations_ttl = self.friends_recommendations_cache.get("ttl", 7200)  # 2小时
        self.friends_recommendations_max_size = self.friends_recommendations_cache.get("max_size", 1000)
        
        # 缓存预热配置
        # Cache warming configuration
        self.cache_warming = cache_strategy_config.get("cache_warming", {})
        self.cache_warming_enabled = self.cache_warming.get("enabled", True)
        self.cache_warming_strategies = self.cache_warming.get("strategies", ["friends_relationships", "user_profiles"])
        self.cache_warming_batch_size = self.cache_warming.get("batch_size", 100)
        self.cache_warming_interval = self.cache_warming.get("interval", 3600)  # 1小时
        
        # 缓存淘汰策略
        # Cache eviction policy
        self.eviction_policy = cache_strategy_config.get("eviction_policy", {})
        self.default_eviction_policy = self.eviction_policy.get("default", "lru")  # LRU, LFU, FIFO
        self.memory_pressure_threshold = self.eviction_policy.get("memory_pressure_threshold", 0.8)
        self.aggressive_eviction_threshold = self.eviction_policy.get("aggressive_eviction_threshold", 0.9)
        
        # 缓存压缩配置
        # Cache compression configuration
        self.compression = cache_strategy_config.get("compression", {})
        self.compression_enabled = self.compression.get("enabled", True)
        self.compression_algorithm = self.compression.get("algorithm", "gzip")  # gzip, lz4, snappy
        self.compression_threshold = self.compression.get("threshold", 1024)  # 1KB
        
        # 缓存监控配置
        # Cache monitoring configuration
        self.monitoring = cache_strategy_config.get("monitoring", {})
        self.monitoring_enabled = self.monitoring.get("enabled", True)
        self.hit_rate_threshold = self.monitoring.get("hit_rate_threshold", 0.8)
        self.monitoring_interval = self.monitoring.get("interval", 60)
        self.alert_on_low_hit_rate = self.monitoring.get("alert_on_low_hit_rate", True)
        
        # 验证配置
        self._validate_config()
        
    def _validate_config(self) -> None:
        """验证缓存策略配置的有效性
        
        Validate the cache strategy configuration.
        """
        # 验证 Redis 配置
        if self.redis_enabled:
            if not isinstance(self.redis_port, int) or self.redis_port <= 0:
                raise ConfigError("cache_strategy.redis.port must be a positive integer")
                
            if self.redis_max_connections <= 0:
                raise ConfigError("cache_strategy.redis.max_connections must be positive")
                
        # 验证内存缓存配置
        if self.memory_cache_ttl <= 0:
            raise ConfigError("cache_strategy.memory_cache.ttl must be positive")
            
        # 验证好友缓存配置
        if self.friends_cache_enabled:
            if self.friends_relationships_ttl <= 0:
                raise ConfigError("cache_strategy.friends_cache.relationships.ttl must be positive")
                
            if self.friends_requests_ttl <= 0:
                raise ConfigError("cache_strategy.friends_cache.requests.ttl must be positive")
                
        # 验证淘汰策略
        valid_policies = ["lru", "lfu", "fifo"]
        if self.default_eviction_policy not in valid_policies:
            raise ConfigError(
                f"cache_strategy.eviction_policy.default must be one of {valid_policies}"
            )
            
        # 验证压缩算法
        valid_algorithms = ["gzip", "lz4", "snappy"]
        if self.compression_algorithm not in valid_algorithms:
            raise ConfigError(
                f"cache_strategy.compression.algorithm must be one of {valid_algorithms}"
            )
            
    def get_redis_config(self) -> Dict[str, Any]:
        """获取 Redis 配置
        
        Returns:
            Redis 配置字典
        """
        return {
            "enabled": self.redis_enabled,
            "host": self.redis_host,
            "port": self.redis_port,
            "password": self.redis_password,
            "db": self.redis_db,
            "max_connections": self.redis_max_connections,
            "connection_timeout": self.redis_connection_timeout,
            "socket_timeout": self.redis_socket_timeout,
            "pool": {
                "max_connections": self.redis_pool_max_connections,
                "retry_on_timeout": self.redis_pool_retry_on_timeout,
                "health_check_interval": self.redis_pool_health_check_interval,
            }
        }
        
    def get_friends_cache_config(self) -> Dict[str, Any]:
        """获取好友功能缓存配置
        
        Returns:
            好友功能缓存配置字典
        """
        return {
            "enabled": self.friends_cache_enabled,
            "relationships": {
                "ttl": self.friends_relationships_ttl,
                "max_size": self.friends_relationships_max_size,
            },
            "requests": {
                "ttl": self.friends_requests_ttl,
                "max_size": self.friends_requests_max_size,
            },
            "presence": {
                "ttl": self.friends_presence_ttl,
                "max_size": self.friends_presence_max_size,
            },
            "recommendations": {
                "ttl": self.friends_recommendations_ttl,
                "max_size": self.friends_recommendations_max_size,
            }
        }
        
    def get_cache_warming_config(self) -> Dict[str, Any]:
        """获取缓存预热配置
        
        Returns:
            缓存预热配置字典
        """
        return {
            "enabled": self.cache_warming_enabled,
            "strategies": self.cache_warming_strategies,
            "batch_size": self.cache_warming_batch_size,
            "interval": self.cache_warming_interval,
        }
        
    def should_compress_cache_entry(self, data_size: int) -> bool:
        """判断是否应该压缩缓存条目
        
        Args:
            data_size: 数据大小（字节）
            
        Returns:
            如果应该压缩则返回 True
        """
        return self.compression_enabled and data_size >= self.compression_threshold
        
    def get_cache_key_prefix(self, cache_type: str) -> str:
        """获取缓存键前缀
        
        Args:
            cache_type: 缓存类型
            
        Returns:
            缓存键前缀
        """
        prefixes = {
            "friends_relationships": "fr:",
            "friends_requests": "freq:",
            "friends_presence": "fp:",
            "friends_recommendations": "frec:",
            "user_profiles": "up:",
            "room_state": "rs:",
            "events": "ev:",
        }
        return prefixes.get(cache_type, "cache:")