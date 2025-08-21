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
# 缓存管理器模块
# Cache manager module
#

import asyncio
import gzip
import json
import logging
import time
from typing import Any, Dict, List, Optional, Union

import attr

from synapse.logging.context import make_deferred_yieldable
from synapse.metrics import LaterGauge
from synapse.util.async_helpers import Linearizer
from synapse.util.caches.lrucache import LruCache

try:
    import redis
    import redis.asyncio as aioredis
except ImportError:
    redis = None
    aioredis = None

logger = logging.getLogger(__name__)


@attr.s(slots=True, auto_attribs=True)
class CacheEntry:
    """缓存条目
    
    Cache entry containing data, metadata and expiration information.
    """
    data: Any
    created_at: float
    ttl: int
    access_count: int = 0
    last_accessed: float = attr.Factory(time.time)
    compressed: bool = False
    
    def is_expired(self) -> bool:
        """检查缓存条目是否已过期
        
        Returns:
            如果已过期则返回 True
        """
        return time.time() - self.created_at > self.ttl
        
    def touch(self) -> None:
        """更新访问时间和计数
        
        Update access time and count.
        """
        self.last_accessed = time.time()
        self.access_count += 1


class CacheManager:
    """缓存管理器
    
    Manages both Redis and in-memory caches with support for friends feature caching.
    """
    
    def __init__(self, config):
        """初始化缓存管理器
        
        Args:
            config: 缓存策略配置
        """
        self.config = config
        self._redis_client: Optional[aioredis.Redis] = None
        self._redis_pool: Optional[aioredis.ConnectionPool] = None
        
        # 内存缓存
        self._memory_caches: Dict[str, LruCache] = {}
        
        # 缓存统计
        self._cache_stats = {
            "hits": 0,
            "misses": 0,
            "sets": 0,
            "deletes": 0,
            "evictions": 0,
        }
        
        # 线性化器，用于防止缓存竞争
        self._linearizer = Linearizer(name="cache_manager")
        
        # 注册指标
        self._register_metrics()
        
        # 初始化内存缓存
        self._init_memory_caches()
        
    def _register_metrics(self) -> None:
        """注册缓存指标
        
        Register cache metrics for monitoring.
        """
        def get_cache_hits():
            return self._cache_stats["hits"]
            
        def get_cache_misses():
            return self._cache_stats["misses"]
            
        def get_cache_hit_rate():
            total = self._cache_stats["hits"] + self._cache_stats["misses"]
            if total == 0:
                return 0.0
            return self._cache_stats["hits"] / total
            
        LaterGauge("synapse_cache_hits_total", "", [], get_cache_hits)
        LaterGauge("synapse_cache_misses_total", "", [], get_cache_misses)
        LaterGauge("synapse_cache_hit_rate", "", [], get_cache_hit_rate)
        
    def _init_memory_caches(self) -> None:
        """初始化内存缓存
        
        Initialize memory caches for different data types.
        """
        # 好友关系缓存
        self._memory_caches["friends_relationships"] = LruCache(
            max_size=self.config.friends_relationships_max_size,
            cache_name="friends_relationships"
        )
        
        # 好友请求缓存
        self._memory_caches["friends_requests"] = LruCache(
            max_size=self.config.friends_requests_max_size,
            cache_name="friends_requests"
        )
        
        # 好友在线状态缓存
        self._memory_caches["friends_presence"] = LruCache(
            max_size=self.config.friends_presence_max_size,
            cache_name="friends_presence"
        )
        
        # 好友推荐缓存
        self._memory_caches["friends_recommendations"] = LruCache(
            max_size=self.config.friends_recommendations_max_size,
            cache_name="friends_recommendations"
        )
        
    async def start(self) -> None:
        """启动缓存管理器
        
        Start the cache manager and initialize Redis connection.
        """
        if self.config.redis_enabled and redis is not None:
            await self._init_redis()
            
        # 启动缓存预热
        if self.config.cache_warming_enabled:
            asyncio.create_task(self._cache_warming_loop())
            
        # 启动缓存清理
        asyncio.create_task(self._cache_cleanup_loop())
        
        logger.info("缓存管理器已启动 (Cache manager started)")
        
    async def stop(self) -> None:
        """停止缓存管理器
        
        Stop the cache manager and close Redis connection.
        """
        if self._redis_client:
            await self._redis_client.close()
            
        if self._redis_pool:
            await self._redis_pool.disconnect()
            
        logger.info("缓存管理器已停止 (Cache manager stopped)")
        
    async def _init_redis(self) -> None:
        """初始化 Redis 连接
        
        Initialize Redis connection with connection pooling.
        """
        try:
            # 创建连接池
            self._redis_pool = aioredis.ConnectionPool(
                host=self.config.redis_host,
                port=self.config.redis_port,
                password=self.config.redis_password,
                db=self.config.redis_db,
                max_connections=self.config.redis_pool_max_connections,
                retry_on_timeout=self.config.redis_pool_retry_on_timeout,
                socket_connect_timeout=self.config.redis_connection_timeout,
                socket_timeout=self.config.redis_socket_timeout,
            )
            
            # 创建 Redis 客户端
            self._redis_client = aioredis.Redis(
                connection_pool=self._redis_pool,
                decode_responses=False  # 我们手动处理编码
            )
            
            # 测试连接
            await self._redis_client.ping()
            logger.info(f"Redis 连接成功: {self.config.redis_host}:{self.config.redis_port}")
            
        except Exception as e:
            logger.error(f"Redis 连接失败: {e}")
            self._redis_client = None
            self._redis_pool = None
            
    async def get(self, cache_type: str, key: str) -> Optional[Any]:
        """获取缓存值
        
        Args:
            cache_type: 缓存类型
            key: 缓存键
            
        Returns:
            缓存值，如果不存在则返回 None
        """
        # 首先尝试从内存缓存获取
        memory_cache = self._memory_caches.get(cache_type)
        if memory_cache:
            entry = memory_cache.get(key)
            if entry and not entry.is_expired():
                entry.touch()
                self._cache_stats["hits"] += 1
                return entry.data
                
        # 然后尝试从 Redis 获取
        if self._redis_client:
            try:
                redis_key = self._build_redis_key(cache_type, key)
                data = await self._redis_client.get(redis_key)
                if data:
                    # 解压缩和反序列化
                    value = self._deserialize_data(data)
                    
                    # 将数据放入内存缓存
                    if memory_cache:
                        ttl = self._get_ttl_for_cache_type(cache_type)
                        entry = CacheEntry(
                            data=value,
                            created_at=time.time(),
                            ttl=ttl
                        )
                        memory_cache[key] = entry
                        
                    self._cache_stats["hits"] += 1
                    return value
                    
            except Exception as e:
                logger.warning(f"Redis 获取失败: {e}")
                
        self._cache_stats["misses"] += 1
        return None
        
    async def set(self, cache_type: str, key: str, value: Any, ttl: Optional[int] = None) -> None:
        """设置缓存值
        
        Args:
            cache_type: 缓存类型
            key: 缓存键
            value: 缓存值
            ttl: 过期时间（秒），如果为 None 则使用默认值
        """
        if ttl is None:
            ttl = self._get_ttl_for_cache_type(cache_type)
            
        # 设置内存缓存
        memory_cache = self._memory_caches.get(cache_type)
        if memory_cache:
            entry = CacheEntry(
                data=value,
                created_at=time.time(),
                ttl=ttl
            )
            memory_cache[key] = entry
            
        # 设置 Redis 缓存
        if self._redis_client:
            try:
                redis_key = self._build_redis_key(cache_type, key)
                serialized_data = self._serialize_data(value)
                await self._redis_client.setex(redis_key, ttl, serialized_data)
                
            except Exception as e:
                logger.warning(f"Redis 设置失败: {e}")
                
        self._cache_stats["sets"] += 1
        
    async def delete(self, cache_type: str, key: str) -> None:
        """删除缓存值
        
        Args:
            cache_type: 缓存类型
            key: 缓存键
        """
        # 从内存缓存删除
        memory_cache = self._memory_caches.get(cache_type)
        if memory_cache:
            memory_cache.pop(key, None)
            
        # 从 Redis 删除
        if self._redis_client:
            try:
                redis_key = self._build_redis_key(cache_type, key)
                await self._redis_client.delete(redis_key)
                
            except Exception as e:
                logger.warning(f"Redis 删除失败: {e}")
                
        self._cache_stats["deletes"] += 1
        
    async def clear_cache_type(self, cache_type: str) -> None:
        """清空指定类型的所有缓存
        
        Args:
            cache_type: 缓存类型
        """
        # 清空内存缓存
        memory_cache = self._memory_caches.get(cache_type)
        if memory_cache:
            memory_cache.clear()
            
        # 清空 Redis 缓存
        if self._redis_client:
            try:
                pattern = self._build_redis_key(cache_type, "*")
                keys = await self._redis_client.keys(pattern)
                if keys:
                    await self._redis_client.delete(*keys)
                    
            except Exception as e:
                logger.warning(f"Redis 清空失败: {e}")
                
    def _build_redis_key(self, cache_type: str, key: str) -> str:
        """构建 Redis 键
        
        Args:
            cache_type: 缓存类型
            key: 缓存键
            
        Returns:
            完整的 Redis 键
        """
        prefix = self.config.get_cache_key_prefix(cache_type)
        return f"{prefix}{key}"
        
    def _get_ttl_for_cache_type(self, cache_type: str) -> int:
        """获取缓存类型的默认 TTL
        
        Args:
            cache_type: 缓存类型
            
        Returns:
            TTL 值（秒）
        """
        ttl_mapping = {
            "friends_relationships": self.config.friends_relationships_ttl,
            "friends_requests": self.config.friends_requests_ttl,
            "friends_presence": self.config.friends_presence_ttl,
            "friends_recommendations": self.config.friends_recommendations_ttl,
        }
        return ttl_mapping.get(cache_type, 1800)  # 默认 30 分钟
        
    def _serialize_data(self, data: Any) -> bytes:
        """序列化数据
        
        Args:
            data: 要序列化的数据
            
        Returns:
            序列化后的字节数据
        """
        json_data = json.dumps(data, ensure_ascii=False).encode('utf-8')
        
        # 如果数据大小超过阈值，则压缩
        if self.config.should_compress_cache_entry(len(json_data)):
            return gzip.compress(json_data)
        else:
            return json_data
            
    def _deserialize_data(self, data: bytes) -> Any:
        """反序列化数据
        
        Args:
            data: 序列化的字节数据
            
        Returns:
            反序列化后的数据
        """
        try:
            # 尝试解压缩
            decompressed_data = gzip.decompress(data)
            return json.loads(decompressed_data.decode('utf-8'))
        except (gzip.BadGzipFile, OSError):
            # 如果不是压缩数据，直接解析
            return json.loads(data.decode('utf-8'))
            
    async def _cache_cleanup_loop(self) -> None:
        """缓存清理循环
        
        Periodic cleanup of expired cache entries.
        """
        while True:
            try:
                await asyncio.sleep(self.config.memory_cache_cleanup_interval)
                
                # 清理内存缓存中的过期条目
                for cache_name, cache in self._memory_caches.items():
                    expired_keys = []
                    for key, entry in cache.cache.items():
                        if entry.is_expired():
                            expired_keys.append(key)
                            
                    for key in expired_keys:
                        cache.pop(key, None)
                        self._cache_stats["evictions"] += 1
                        
                    if expired_keys:
                        logger.debug(f"清理 {cache_name} 缓存中的 {len(expired_keys)} 个过期条目")
                        
            except Exception as e:
                logger.error(f"缓存清理失败: {e}")
                
    async def _cache_warming_loop(self) -> None:
        """缓存预热循环
        
        Periodic cache warming for frequently accessed data.
        """
        while True:
            try:
                await asyncio.sleep(self.config.cache_warming_interval)
                
                # 这里可以实现具体的缓存预热逻辑
                # 例如预加载热门用户的好友关系等
                logger.debug("执行缓存预热")
                
            except Exception as e:
                logger.error(f"缓存预热失败: {e}")
                
    def get_cache_stats(self) -> Dict[str, Any]:
        """获取缓存统计信息
        
        Returns:
            缓存统计信息字典
        """
        total_requests = self._cache_stats["hits"] + self._cache_stats["misses"]
        hit_rate = self._cache_stats["hits"] / total_requests if total_requests > 0 else 0.0
        
        return {
            "hits": self._cache_stats["hits"],
            "misses": self._cache_stats["misses"],
            "hit_rate": hit_rate,
            "sets": self._cache_stats["sets"],
            "deletes": self._cache_stats["deletes"],
            "evictions": self._cache_stats["evictions"],
            "memory_caches": {
                name: {
                    "size": len(cache.cache),
                    "max_size": cache.max_size,
                }
                for name, cache in self._memory_caches.items()
            }
        }