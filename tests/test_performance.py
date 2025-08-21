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

"""性能优化和缓存功能测试用例"""

import time
from typing import Dict, Any, List
from unittest.mock import Mock, patch, AsyncMock

from twisted.test.proto_helpers import MemoryReactor

from synapse.config.cache_strategy import CacheStrategyConfig
from synapse.config.performance import PerformanceConfig
from synapse.server import HomeServer
from synapse.util import Clock
from synapse.util.caches.cache_manager import CacheManager

from tests import unittest
from tests.utils import MockClock


class CacheManagerTestCase(unittest.HomeserverTestCase):
    """缓存管理器测试类"""
    
    def prepare(self, reactor: MemoryReactor, clock: Clock, hs: HomeServer) -> None:
        self.hs = hs
        self.cache_manager = CacheManager(hs.config)
        
        # 模拟 Redis 连接
        self.mock_redis = Mock()
        self.cache_manager.redis_client = self.mock_redis
    
    def test_cache_manager_initialization(self) -> None:
        """测试缓存管理器初始化"""
        self.assertIsNotNone(self.cache_manager)
        self.assertIsNotNone(self.cache_manager.memory_cache)
        self.assertIsNotNone(self.cache_manager.cache_entries)
    
    def test_memory_cache_operations(self) -> None:
        """测试内存缓存操作"""
        # 设置缓存
        key = "test_key"
        value = {"data": "test_value"}
        
        self.get_success(
            self.cache_manager.set(key, value, cache_type="friends_list")
        )
        
        # 获取缓存
        cached_value = self.get_success(
            self.cache_manager.get(key, cache_type="friends_list")
        )
        
        self.assertEqual(cached_value, value)
    
    def test_redis_cache_operations(self) -> None:
        """测试 Redis 缓存操作"""
        # 模拟 Redis 操作
        self.mock_redis.get.return_value = b'{"data": "test_value"}'
        self.mock_redis.setex.return_value = True
        
        key = "test_redis_key"
        value = {"data": "test_value"}
        
        # 设置缓存
        self.get_success(
            self.cache_manager.set(key, value, cache_type="user_profile")
        )
        
        # 验证 Redis 调用
        self.mock_redis.setex.assert_called()
        
        # 获取缓存
        cached_value = self.get_success(
            self.cache_manager.get(key, cache_type="user_profile")
        )
        
        self.assertEqual(cached_value, value)
        self.mock_redis.get.assert_called()
    
    def test_cache_ttl(self) -> None:
        """测试缓存 TTL"""
        key = "ttl_test_key"
        value = {"data": "test_value"}
        
        # 设置短 TTL 缓存
        self.get_success(
            self.cache_manager.set(
                key, value, 
                cache_type="friend_request", 
                ttl=1  # 1秒 TTL
            )
        )
        
        # 立即获取应该成功
        cached_value = self.get_success(
            self.cache_manager.get(key, cache_type="friend_request")
        )
        self.assertEqual(cached_value, value)
        
        # 等待 TTL 过期
        self.reactor.advance(2)
        
        # 再次获取应该返回 None
        expired_value = self.get_success(
            self.cache_manager.get(key, cache_type="friend_request")
        )
        self.assertIsNone(expired_value)
    
    def test_cache_invalidation(self) -> None:
        """测试缓存失效"""
        key = "invalidation_test_key"
        value = {"data": "test_value"}
        
        # 设置缓存
        self.get_success(
            self.cache_manager.set(key, value, cache_type="friends_list")
        )
        
        # 验证缓存存在
        cached_value = self.get_success(
            self.cache_manager.get(key, cache_type="friends_list")
        )
        self.assertEqual(cached_value, value)
        
        # 删除缓存
        self.get_success(
            self.cache_manager.delete(key, cache_type="friends_list")
        )
        
        # 验证缓存已删除
        deleted_value = self.get_success(
            self.cache_manager.get(key, cache_type="friends_list")
        )
        self.assertIsNone(deleted_value)
    
    def test_cache_clear_by_type(self) -> None:
        """测试按类型清空缓存"""
        # 设置多个不同类型的缓存
        self.get_success(
            self.cache_manager.set(
                "key1", {"data": "value1"}, 
                cache_type="friends_list"
            )
        )
        self.get_success(
            self.cache_manager.set(
                "key2", {"data": "value2"}, 
                cache_type="friends_list"
            )
        )
        self.get_success(
            self.cache_manager.set(
                "key3", {"data": "value3"}, 
                cache_type="user_profile"
            )
        )
        
        # 清空特定类型的缓存
        self.get_success(
            self.cache_manager.clear_cache_type("friends_list")
        )
        
        # 验证指定类型的缓存已清空
        value1 = self.get_success(
            self.cache_manager.get("key1", cache_type="friends_list")
        )
        value2 = self.get_success(
            self.cache_manager.get("key2", cache_type="friends_list")
        )
        self.assertIsNone(value1)
        self.assertIsNone(value2)
        
        # 验证其他类型的缓存仍然存在
        value3 = self.get_success(
            self.cache_manager.get("key3", cache_type="user_profile")
        )
        self.assertEqual(value3, {"data": "value3"})
    
    def test_cache_statistics(self) -> None:
        """测试缓存统计信息"""
        # 执行一些缓存操作
        self.get_success(
            self.cache_manager.set(
                "stats_key1", {"data": "value1"}, 
                cache_type="friends_list"
            )
        )
        
        # 缓存命中
        self.get_success(
            self.cache_manager.get("stats_key1", cache_type="friends_list")
        )
        
        # 缓存未命中
        self.get_success(
            self.cache_manager.get("nonexistent_key", cache_type="friends_list")
        )
        
        # 获取统计信息
        stats = self.get_success(
            self.cache_manager.get_cache_stats()
        )
        
        self.assertIn("memory_cache", stats)
        self.assertIn("redis_cache", stats)
        self.assertIn("cache_types", stats)
        
        # 验证统计数据结构
        memory_stats = stats["memory_cache"]
        self.assertIn("hits", memory_stats)
        self.assertIn("misses", memory_stats)
        self.assertIn("size", memory_stats)


class PerformanceConfigTestCase(unittest.HomeserverTestCase):
    """性能配置测试类"""
    
    def test_performance_config_defaults(self) -> None:
        """测试性能配置默认值"""
        config = PerformanceConfig()
        config.read_config({}, "")
        
        # 验证默认配置
        self.assertEqual(config.max_concurrent_requests, 100)
        self.assertEqual(config.request_timeout, 30)
        self.assertEqual(config.database_pool_size, 10)
        self.assertEqual(config.database_max_overflow, 20)
        self.assertTrue(config.enable_query_optimization)
        self.assertTrue(config.enable_connection_pooling)
    
    def test_performance_config_custom_values(self) -> None:
        """测试自定义性能配置"""
        config_dict = {
            "performance": {
                "max_concurrent_requests": 50,
                "request_timeout": 60,
                "database": {
                    "pool_size": 5,
                    "max_overflow": 10,
                    "enable_query_optimization": False,
                    "enable_connection_pooling": False,
                },
                "memory": {
                    "gc_threshold": [500, 5, 5],
                    "max_memory_usage": 512,
                },
                "network": {
                    "tcp_keepalive": False,
                    "tcp_nodelay": False,
                },
            }
        }
        
        config = PerformanceConfig()
        config.read_config(config_dict, "")
        
        # 验证自定义配置
        self.assertEqual(config.max_concurrent_requests, 50)
        self.assertEqual(config.request_timeout, 60)
        self.assertEqual(config.database_pool_size, 5)
        self.assertEqual(config.database_max_overflow, 10)
        self.assertFalse(config.enable_query_optimization)
        self.assertFalse(config.enable_connection_pooling)
        self.assertEqual(config.gc_threshold, [500, 5, 5])
        self.assertEqual(config.max_memory_usage, 512)
        self.assertFalse(config.tcp_keepalive)
        self.assertFalse(config.tcp_nodelay)


class CacheStrategyConfigTestCase(unittest.HomeserverTestCase):
    """缓存策略配置测试类"""
    
    def test_cache_strategy_config_defaults(self) -> None:
        """测试缓存策略配置默认值"""
        config = CacheStrategyConfig()
        config.read_config({}, "")
        
        # 验证默认配置
        self.assertTrue(config.enable_memory_cache)
        self.assertTrue(config.enable_redis_cache)
        self.assertEqual(config.memory_cache_size, 100)
        self.assertEqual(config.redis_host, "localhost")
        self.assertEqual(config.redis_port, 6379)
        self.assertEqual(config.redis_db, 0)
    
    def test_cache_strategy_config_custom_values(self) -> None:
        """测试自定义缓存策略配置"""
        config_dict = {
            "cache_strategy": {
                "enable_memory_cache": False,
                "enable_redis_cache": True,
                "memory_cache": {
                    "size": 200,
                    "ttl": 1800,
                },
                "redis": {
                    "host": "redis.example.com",
                    "port": 6380,
                    "db": 1,
                    "password": "secret",
                    "connection_pool_size": 20,
                },
                "cache_entries": {
                    "friends_list": {
                        "memory_ttl": 600,
                        "redis_ttl": 3600,
                        "use_redis": True,
                    },
                },
            }
        }
        
        config = CacheStrategyConfig()
        config.read_config(config_dict, "")
        
        # 验证自定义配置
        self.assertFalse(config.enable_memory_cache)
        self.assertTrue(config.enable_redis_cache)
        self.assertEqual(config.memory_cache_size, 200)
        self.assertEqual(config.memory_cache_ttl, 1800)
        self.assertEqual(config.redis_host, "redis.example.com")
        self.assertEqual(config.redis_port, 6380)
        self.assertEqual(config.redis_db, 1)
        self.assertEqual(config.redis_password, "secret")
        self.assertEqual(config.redis_connection_pool_size, 20)
        
        # 验证缓存条目配置
        friends_list_config = config.cache_entries["friends_list"]
        self.assertEqual(friends_list_config["memory_ttl"], 600)
        self.assertEqual(friends_list_config["redis_ttl"], 3600)
        self.assertTrue(friends_list_config["use_redis"])


class PerformanceMonitorTestCase(unittest.HomeserverTestCase):
    """性能监控测试类"""
    
    def prepare(self, reactor: MemoryReactor, clock: Clock, hs: HomeServer) -> None:
        self.hs = hs
        self.clock = clock
    
    @patch('psutil.cpu_percent')
    @patch('psutil.virtual_memory')
    @patch('psutil.disk_usage')
    def test_system_metrics_collection(self, mock_disk, mock_memory, mock_cpu) -> None:
        """测试系统指标收集"""
        # 模拟系统指标
        mock_cpu.return_value = 45.5
        mock_memory.return_value = Mock(
            total=2147483648,  # 2GB
            available=1073741824,  # 1GB
            percent=50.0
        )
        mock_disk.return_value = Mock(
            total=107374182400,  # 100GB
            used=53687091200,   # 50GB
            free=53687091200,   # 50GB
            percent=50.0
        )
        
        # 导入性能监控模块
        from synapse.util.performance_monitor import collect_system_metrics
        
        # 收集系统指标
        metrics = collect_system_metrics()
        
        # 验证指标数据
        self.assertEqual(metrics["cpu_percent"], 45.5)
        self.assertEqual(metrics["memory_percent"], 50.0)
        self.assertEqual(metrics["disk_percent"], 50.0)
        self.assertEqual(metrics["memory_total"], 2147483648)
        self.assertEqual(metrics["memory_available"], 1073741824)
    
    def test_cache_metrics_collection(self) -> None:
        """测试缓存指标收集"""
        cache_manager = self.hs.get_cache_manager()
        
        # 执行一些缓存操作以生成指标
        self.get_success(
            cache_manager.set(
                "metrics_key", {"data": "value"}, 
                cache_type="friends_list"
            )
        )
        
        self.get_success(
            cache_manager.get("metrics_key", cache_type="friends_list")
        )
        
        self.get_success(
            cache_manager.get("nonexistent_key", cache_type="friends_list")
        )
        
        # 获取缓存统计信息
        stats = self.get_success(
            cache_manager.get_cache_stats()
        )
        
        # 验证统计信息结构
        self.assertIn("memory_cache", stats)
        self.assertIn("redis_cache", stats)
        
        memory_stats = stats["memory_cache"]
        self.assertIn("hits", memory_stats)
        self.assertIn("misses", memory_stats)
        self.assertIn("size", memory_stats)
    
    def test_database_metrics_collection(self) -> None:
        """测试数据库指标收集"""
        store = self.hs.get_datastore()
        
        # 执行一些数据库操作
        self.get_success(
            store.db_pool.simple_select_one(
                table="users",
                keyvalues={"name": "@test:example.com"},
                retcols=["name"],
                allow_none=True,
            )
        )
        
        # 获取数据库连接池统计信息
        pool_stats = store.db_pool.engine.pool.status()
        
        # 验证统计信息存在
        self.assertIsNotNone(pool_stats)
    
    def test_performance_thresholds(self) -> None:
        """测试性能阈值检查"""
        from synapse.util.performance_monitor import check_performance_thresholds
        
        # 测试正常指标
        normal_metrics = {
            "cpu_percent": 30.0,
            "memory_percent": 40.0,
            "disk_percent": 50.0,
        }
        
        alerts = check_performance_thresholds(normal_metrics)
        self.assertEqual(len(alerts), 0)
        
        # 测试超出阈值的指标
        high_metrics = {
            "cpu_percent": 95.0,
            "memory_percent": 90.0,
            "disk_percent": 85.0,
        }
        
        alerts = check_performance_thresholds(high_metrics)
        self.assertGreater(len(alerts), 0)
        
        # 验证告警内容
        alert_types = [alert["type"] for alert in alerts]
        self.assertIn("high_cpu", alert_types)
        self.assertIn("high_memory", alert_types)
        self.assertIn("high_disk", alert_types)


class CacheWarmupTestCase(unittest.HomeserverTestCase):
    """缓存预热测试类"""
    
    def prepare(self, reactor: MemoryReactor, clock: Clock, hs: HomeServer) -> None:
        self.hs = hs
        self.store = hs.get_datastore()
        self.cache_manager = hs.get_cache_manager()
        
        # 创建测试用户
        self.user1_id = "@user1:test"
        self.user2_id = "@user2:test"
        
        # 注册测试用户
        self.get_success(
            self.store.register_user(
                user_id=self.user1_id,
                password_hash=None,
                was_guest=False,
                make_guest=False,
                appservice_id=None,
                create_profile_with_displayname="User 1",
                admin=False,
                user_type=None,
                shadow_banned=False,
            )
        )
        
        self.get_success(
            self.store.register_user(
                user_id=self.user2_id,
                password_hash=None,
                was_guest=False,
                make_guest=False,
                appservice_id=None,
                create_profile_with_displayname="User 2",
                admin=False,
                user_type=None,
                shadow_banned=False,
            )
        )
    
    def test_friends_cache_warmup(self) -> None:
        """测试好友缓存预热"""
        # 添加好友关系
        self.get_success(
            self.store.add_friend(self.user1_id, self.user2_id)
        )
        
        # 导入缓存预热模块
        from synapse.util.cache_warmup import warmup_friends_cache
        
        # 执行好友缓存预热
        self.get_success(
            warmup_friends_cache(self.hs, [self.user1_id, self.user2_id])
        )
        
        # 验证缓存已预热
        cached_friends = self.get_success(
            self.cache_manager.get(
                f"friends_list:{self.user1_id}", 
                cache_type="friends_list"
            )
        )
        
        self.assertIsNotNone(cached_friends)
        self.assertEqual(len(cached_friends), 1)
        self.assertEqual(cached_friends[0]["user_id"], self.user2_id)
    
    def test_user_profile_cache_warmup(self) -> None:
        """测试用户资料缓存预热"""
        from synapse.util.cache_warmup import warmup_user_profiles_cache
        
        # 执行用户资料缓存预热
        self.get_success(
            warmup_user_profiles_cache(self.hs, [self.user1_id, self.user2_id])
        )
        
        # 验证缓存已预热
        cached_profile = self.get_success(
            self.cache_manager.get(
                f"user_profile:{self.user1_id}", 
                cache_type="user_profile"
            )
        )
        
        self.assertIsNotNone(cached_profile)
        self.assertEqual(cached_profile["user_id"], self.user1_id)
        self.assertEqual(cached_profile["displayname"], "User 1")
    
    def test_batch_cache_warmup(self) -> None:
        """测试批量缓存预热"""
        from synapse.util.cache_warmup import warmup_caches_batch
        
        # 执行批量缓存预热
        self.get_success(
            warmup_caches_batch(
                self.hs, 
                user_ids=[self.user1_id, self.user2_id],
                batch_size=1
            )
        )
        
        # 验证多种类型的缓存都已预热
        profile_cache = self.get_success(
            self.cache_manager.get(
                f"user_profile:{self.user1_id}", 
                cache_type="user_profile"
            )
        )
        
        friends_cache = self.get_success(
            self.cache_manager.get(
                f"friends_list:{self.user1_id}", 
                cache_type="friends_list"
            )
        )
        
        self.assertIsNotNone(profile_cache)
        self.assertIsNotNone(friends_cache)


class LowSpecOptimizationTestCase(unittest.HomeserverTestCase):
    """低配置服务器优化测试类"""
    
    def test_memory_optimization(self) -> None:
        """测试内存优化"""
        import gc
        
        # 获取初始内存使用情况
        initial_objects = len(gc.get_objects())
        
        # 执行一些操作
        data = []
        for i in range(1000):
            data.append({"id": i, "data": f"test_data_{i}"})
        
        # 清理数据
        del data
        gc.collect()
        
        # 验证内存已释放
        final_objects = len(gc.get_objects())
        self.assertLessEqual(final_objects, initial_objects + 100)  # 允许一些误差
    
    def test_connection_pool_optimization(self) -> None:
        """测试连接池优化"""
        store = self.hs.get_datastore()
        
        # 验证连接池配置
        pool = store.db_pool.engine.pool
        
        # 对于低配置服务器，连接池应该较小
        self.assertLessEqual(pool.size(), 20)  # 最大连接数不超过20
    
    def test_cache_size_optimization(self) -> None:
        """测试缓存大小优化"""
        cache_manager = self.hs.get_cache_manager()
        
        # 验证内存缓存大小配置合理
        memory_cache = cache_manager.memory_cache
        
        # 对于低配置服务器，缓存大小应该适中
        self.assertLessEqual(memory_cache.max_size, 1000)  # 最大缓存条目数
    
    def test_concurrent_request_limit(self) -> None:
        """测试并发请求限制"""
        config = self.hs.config.performance
        
        # 验证并发请求限制适合低配置服务器
        self.assertLessEqual(config.max_concurrent_requests, 200)
        self.assertGreaterEqual(config.max_concurrent_requests, 50)
    
    def test_gc_optimization(self) -> None:
        """测试垃圾回收优化"""
        import gc
        
        config = self.hs.config.performance
        
        # 验证 GC 阈值配置
        if hasattr(config, 'gc_threshold'):
            # GC 阈值应该针对低内存环境优化
            self.assertIsInstance(config.gc_threshold, list)
            self.assertEqual(len(config.gc_threshold), 3)
            
            # 第一代阈值应该较小，以便更频繁地清理
            self.assertLessEqual(config.gc_threshold[0], 1000)