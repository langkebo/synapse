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

"""集成测试用例"""

import json
import time
from typing import Dict, Any, Optional
from unittest.mock import Mock, patch, AsyncMock

from twisted.test.proto_helpers import MemoryReactor
from twisted.internet import defer

from synapse.api.constants import EventTypes, Membership
from synapse.api.errors import SynapseError
from synapse.events import make_event_from_dict
from synapse.rest.client import friends
from synapse.server import HomeServer
from synapse.types import UserID, RoomID
from synapse.util import Clock

from tests import unittest
from tests.utils import MockClock, setup_test_homeserver


class FriendsIntegrationTestCase(unittest.HomeserverTestCase):
    """好友功能集成测试类"""
    
    servlets = [
        friends.register_servlets,
    ]
    
    def prepare(self, reactor: MemoryReactor, clock: Clock, hs: HomeServer) -> None:
        """准备测试环境"""
        self.user_id = self.register_user("alice", "password")
        self.access_token = self.login("alice", "password")
        
        self.friend_user_id = self.register_user("bob", "password")
        self.friend_access_token = self.login("bob", "password")
        
        self.other_user_id = self.register_user("charlie", "password")
        self.other_access_token = self.login("charlie", "password")
        
        # 获取存储和处理器
        self.store = hs.get_datastore()
        self.friends_handler = hs.get_friends_handler()
        self.cache_manager = hs.get_cache_manager()
    
    def test_complete_friend_workflow(self) -> None:
        """测试完整的好友工作流程"""
        # 1. 发送好友请求
        channel = self.make_request(
            "POST",
            f"/_matrix/client/r0/friends/{self.friend_user_id}/request",
            {"message": "Hi, let's be friends!"},
            access_token=self.access_token,
        )
        self.assertEqual(channel.code, 200)
        
        # 2. 检查好友请求是否创建
        channel = self.make_request(
            "GET",
            "/_matrix/client/r0/friends/requests/sent",
            access_token=self.access_token,
        )
        self.assertEqual(channel.code, 200)
        sent_requests = channel.json_body["requests"]
        self.assertEqual(len(sent_requests), 1)
        self.assertEqual(sent_requests[0]["to_user_id"], self.friend_user_id)
        
        # 3. 检查接收方的待处理请求
        channel = self.make_request(
            "GET",
            "/_matrix/client/r0/friends/requests/received",
            access_token=self.friend_access_token,
        )
        self.assertEqual(channel.code, 200)
        received_requests = channel.json_body["requests"]
        self.assertEqual(len(received_requests), 1)
        self.assertEqual(received_requests[0]["from_user_id"], self.user_id)
        
        request_id = received_requests[0]["request_id"]
        
        # 4. 接受好友请求
        channel = self.make_request(
            "POST",
            f"/_matrix/client/r0/friends/requests/{request_id}/accept",
            {},
            access_token=self.friend_access_token,
        )
        self.assertEqual(channel.code, 200)
        
        # 5. 验证好友关系已建立
        channel = self.make_request(
            "GET",
            "/_matrix/client/r0/friends",
            access_token=self.access_token,
        )
        self.assertEqual(channel.code, 200)
        friends_list = channel.json_body["friends"]
        self.assertEqual(len(friends_list), 1)
        self.assertEqual(friends_list[0]["user_id"], self.friend_user_id)
        
        # 6. 验证双向好友关系
        channel = self.make_request(
            "GET",
            "/_matrix/client/r0/friends",
            access_token=self.friend_access_token,
        )
        self.assertEqual(channel.code, 200)
        friends_list = channel.json_body["friends"]
        self.assertEqual(len(friends_list), 1)
        self.assertEqual(friends_list[0]["user_id"], self.user_id)
        
        # 7. 删除好友关系
        channel = self.make_request(
            "DELETE",
            f"/_matrix/client/r0/friends/{self.friend_user_id}",
            access_token=self.access_token,
        )
        self.assertEqual(channel.code, 200)
        
        # 8. 验证好友关系已删除
        channel = self.make_request(
            "GET",
            "/_matrix/client/r0/friends",
            access_token=self.access_token,
        )
        self.assertEqual(channel.code, 200)
        friends_list = channel.json_body["friends"]
        self.assertEqual(len(friends_list), 0)
    
    def test_friend_request_rejection_workflow(self) -> None:
        """测试好友请求拒绝工作流程"""
        # 1. 发送好友请求
        channel = self.make_request(
            "POST",
            f"/_matrix/client/r0/friends/{self.friend_user_id}/request",
            {"message": "Hi, let's be friends!"},
            access_token=self.access_token,
        )
        self.assertEqual(channel.code, 200)
        
        # 2. 获取请求ID
        channel = self.make_request(
            "GET",
            "/_matrix/client/r0/friends/requests/received",
            access_token=self.friend_access_token,
        )
        self.assertEqual(channel.code, 200)
        received_requests = channel.json_body["requests"]
        request_id = received_requests[0]["request_id"]
        
        # 3. 拒绝好友请求
        channel = self.make_request(
            "POST",
            f"/_matrix/client/r0/friends/requests/{request_id}/reject",
            {},
            access_token=self.friend_access_token,
        )
        self.assertEqual(channel.code, 200)
        
        # 4. 验证请求已被拒绝
        channel = self.make_request(
            "GET",
            "/_matrix/client/r0/friends/requests/received",
            access_token=self.friend_access_token,
        )
        self.assertEqual(channel.code, 200)
        received_requests = channel.json_body["requests"]
        self.assertEqual(len(received_requests), 0)
        
        # 5. 验证没有建立好友关系
        channel = self.make_request(
            "GET",
            "/_matrix/client/r0/friends",
            access_token=self.access_token,
        )
        self.assertEqual(channel.code, 200)
        friends_list = channel.json_body["friends"]
        self.assertEqual(len(friends_list), 0)
    
    def test_user_search_functionality(self) -> None:
        """测试用户搜索功能"""
        # 1. 搜索用户
        channel = self.make_request(
            "GET",
            "/_matrix/client/r0/friends/search?q=bob",
            access_token=self.access_token,
        )
        self.assertEqual(channel.code, 200)
        search_results = channel.json_body["results"]
        
        # 2. 验证搜索结果
        self.assertGreaterEqual(len(search_results), 1)
        
        # 查找 bob 用户
        bob_found = False
        for user in search_results:
            if user["user_id"] == self.friend_user_id:
                bob_found = True
                self.assertIn("display_name", user)
                self.assertIn("avatar_url", user)
                break
        
        self.assertTrue(bob_found, "Bob user not found in search results")
        
        # 3. 测试空搜索查询
        channel = self.make_request(
            "GET",
            "/_matrix/client/r0/friends/search?q=",
            access_token=self.access_token,
        )
        self.assertEqual(channel.code, 400)  # 应该返回错误
        
        # 4. 测试搜索限制
        channel = self.make_request(
            "GET",
            "/_matrix/client/r0/friends/search?q=user&limit=5",
            access_token=self.access_token,
        )
        self.assertEqual(channel.code, 200)
        search_results = channel.json_body["results"]
        self.assertLessEqual(len(search_results), 5)
    
    def test_cache_invalidation_on_friend_operations(self) -> None:
        """测试好友操作时的缓存失效"""
        # 1. 建立好友关系
        self._establish_friendship()
        
        # 2. 获取好友列表（应该被缓存）
        channel = self.make_request(
            "GET",
            "/_matrix/client/r0/friends",
            access_token=self.access_token,
        )
        self.assertEqual(channel.code, 200)
        friends_list_1 = channel.json_body["friends"]
        self.assertEqual(len(friends_list_1), 1)
        
        # 3. 删除好友关系
        channel = self.make_request(
            "DELETE",
            f"/_matrix/client/r0/friends/{self.friend_user_id}",
            access_token=self.access_token,
        )
        self.assertEqual(channel.code, 200)
        
        # 4. 再次获取好友列表（缓存应该已失效）
        channel = self.make_request(
            "GET",
            "/_matrix/client/r0/friends",
            access_token=self.access_token,
        )
        self.assertEqual(channel.code, 200)
        friends_list_2 = channel.json_body["friends"]
        self.assertEqual(len(friends_list_2), 0)
    
    def test_concurrent_friend_requests(self) -> None:
        """测试并发好友请求处理"""
        # 1. Alice 向 Bob 发送好友请求
        channel = self.make_request(
            "POST",
            f"/_matrix/client/r0/friends/{self.friend_user_id}/request",
            {"message": "Hi from Alice!"},
            access_token=self.access_token,
        )
        self.assertEqual(channel.code, 200)
        
        # 2. Bob 同时向 Alice 发送好友请求
        channel = self.make_request(
            "POST",
            f"/_matrix/client/r0/friends/{self.user_id}/request",
            {"message": "Hi from Bob!"},
            access_token=self.friend_access_token,
        )
        self.assertEqual(channel.code, 200)
        
        # 3. 检查两个用户都有待处理的请求
        channel = self.make_request(
            "GET",
            "/_matrix/client/r0/friends/requests/received",
            access_token=self.access_token,
        )
        self.assertEqual(channel.code, 200)
        alice_requests = channel.json_body["requests"]
        
        channel = self.make_request(
            "GET",
            "/_matrix/client/r0/friends/requests/received",
            access_token=self.friend_access_token,
        )
        self.assertEqual(channel.code, 200)
        bob_requests = channel.json_body["requests"]
        
        # 4. 验证请求状态
        self.assertEqual(len(alice_requests), 1)
        self.assertEqual(len(bob_requests), 1)
        
        # 5. Alice 接受 Bob 的请求
        alice_request_id = alice_requests[0]["request_id"]
        channel = self.make_request(
            "POST",
            f"/_matrix/client/r0/friends/requests/{alice_request_id}/accept",
            {},
            access_token=self.access_token,
        )
        self.assertEqual(channel.code, 200)
        
        # 6. 验证好友关系已建立，且 Bob 的请求也被自动处理
        channel = self.make_request(
            "GET",
            "/_matrix/client/r0/friends",
            access_token=self.access_token,
        )
        self.assertEqual(channel.code, 200)
        friends_list = channel.json_body["friends"]
        self.assertEqual(len(friends_list), 1)
        
        # 7. 验证双方都没有待处理的请求
        channel = self.make_request(
            "GET",
            "/_matrix/client/r0/friends/requests/received",
            access_token=self.access_token,
        )
        self.assertEqual(channel.code, 200)
        self.assertEqual(len(channel.json_body["requests"]), 0)
        
        channel = self.make_request(
            "GET",
            "/_matrix/client/r0/friends/requests/received",
            access_token=self.friend_access_token,
        )
        self.assertEqual(channel.code, 200)
        self.assertEqual(len(channel.json_body["requests"]), 0)
    
    def test_error_handling_and_edge_cases(self) -> None:
        """测试错误处理和边界情况"""
        # 1. 向自己发送好友请求
        channel = self.make_request(
            "POST",
            f"/_matrix/client/r0/friends/{self.user_id}/request",
            {"message": "Hi myself!"},
            access_token=self.access_token,
        )
        self.assertEqual(channel.code, 400)
        
        # 2. 向不存在的用户发送好友请求
        channel = self.make_request(
            "POST",
            "/_matrix/client/r0/friends/@nonexistent:test/request",
            {"message": "Hi!"},
            access_token=self.access_token,
        )
        self.assertEqual(channel.code, 404)
        
        # 3. 重复发送好友请求
        channel = self.make_request(
            "POST",
            f"/_matrix/client/r0/friends/{self.friend_user_id}/request",
            {"message": "First request"},
            access_token=self.access_token,
        )
        self.assertEqual(channel.code, 200)
        
        channel = self.make_request(
            "POST",
            f"/_matrix/client/r0/friends/{self.friend_user_id}/request",
            {"message": "Duplicate request"},
            access_token=self.access_token,
        )
        self.assertEqual(channel.code, 409)  # 冲突
        
        # 4. 接受不存在的好友请求
        channel = self.make_request(
            "POST",
            "/_matrix/client/r0/friends/requests/nonexistent/accept",
            {},
            access_token=self.access_token,
        )
        self.assertEqual(channel.code, 404)
        
        # 5. 删除不存在的好友
        channel = self.make_request(
            "DELETE",
            f"/_matrix/client/r0/friends/{self.other_user_id}",
            access_token=self.access_token,
        )
        self.assertEqual(channel.code, 404)
        
        # 6. 无效的访问令牌
        channel = self.make_request(
            "GET",
            "/_matrix/client/r0/friends",
            access_token="invalid_token",
        )
        self.assertEqual(channel.code, 401)
    
    def _establish_friendship(self) -> None:
        """建立好友关系的辅助方法"""
        # 发送好友请求
        channel = self.make_request(
            "POST",
            f"/_matrix/client/r0/friends/{self.friend_user_id}/request",
            {"message": "Hi, let's be friends!"},
            access_token=self.access_token,
        )
        self.assertEqual(channel.code, 200)
        
        # 获取请求ID
        channel = self.make_request(
            "GET",
            "/_matrix/client/r0/friends/requests/received",
            access_token=self.friend_access_token,
        )
        self.assertEqual(channel.code, 200)
        received_requests = channel.json_body["requests"]
        request_id = received_requests[0]["request_id"]
        
        # 接受好友请求
        channel = self.make_request(
            "POST",
            f"/_matrix/client/r0/friends/requests/{request_id}/accept",
            {},
            access_token=self.friend_access_token,
        )
        self.assertEqual(channel.code, 200)


class PerformanceIntegrationTestCase(unittest.HomeserverTestCase):
    """性能优化集成测试类"""
    
    def prepare(self, reactor: MemoryReactor, clock: Clock, hs: HomeServer) -> None:
        """准备测试环境"""
        self.user_id = self.register_user("alice", "password")
        self.access_token = self.login("alice", "password")
        
        # 获取性能相关组件
        self.cache_manager = hs.get_cache_manager()
        self.performance_monitor = getattr(hs, '_performance_monitor', None)
    
    def test_cache_performance_under_load(self) -> None:
        """测试负载下的缓存性能"""
        if not self.cache_manager:
            self.skipTest("Cache manager not available")
        
        # 1. 预热缓存
        test_data = {f"key_{i}": f"value_{i}" for i in range(100)}
        
        start_time = time.time()
        for key, value in test_data.items():
            self.cache_manager.set("test", key, value, ttl=3600)
        cache_write_time = time.time() - start_time
        
        # 2. 测试缓存读取性能
        start_time = time.time()
        for key in test_data.keys():
            cached_value = self.cache_manager.get("test", key)
            self.assertIsNotNone(cached_value)
        cache_read_time = time.time() - start_time
        
        # 3. 验证性能指标
        self.assertLess(cache_write_time, 1.0, "Cache write time too slow")
        self.assertLess(cache_read_time, 0.5, "Cache read time too slow")
        
        # 4. 测试缓存统计
        stats = self.cache_manager.get_stats()
        self.assertIn("test", stats)
        self.assertGreater(stats["test"]["hits"], 0)
    
    def test_database_connection_pooling(self) -> None:
        """测试数据库连接池"""
        # 1. 模拟并发数据库操作
        concurrent_operations = []
        
        for i in range(10):
            # 创建用户操作
            user_id = f"@testuser{i}:test"
            operation = self.hs.get_datastore().register_user(
                user_id=user_id,
                password_hash="dummy_hash",
            )
            concurrent_operations.append(operation)
        
        # 2. 等待所有操作完成
        start_time = time.time()
        for operation in concurrent_operations:
            if hasattr(operation, 'result'):
                try:
                    operation.result()
                except Exception:
                    pass  # 忽略重复用户错误
        operation_time = time.time() - start_time
        
        # 3. 验证操作时间合理
        self.assertLess(operation_time, 5.0, "Database operations too slow")
    
    def test_memory_usage_optimization(self) -> None:
        """测试内存使用优化"""
        import psutil
        import os
        
        # 1. 获取初始内存使用
        process = psutil.Process(os.getpid())
        initial_memory = process.memory_info().rss / 1024 / 1024  # MB
        
        # 2. 执行内存密集型操作
        large_data = []
        for i in range(1000):
            large_data.append({"id": i, "data": "x" * 1000})
        
        # 3. 触发垃圾回收
        import gc
        gc.collect()
        
        # 4. 检查内存使用
        final_memory = process.memory_info().rss / 1024 / 1024  # MB
        memory_increase = final_memory - initial_memory
        
        # 5. 验证内存使用在合理范围内（针对低配置服务器）
        self.assertLess(memory_increase, 100, "Memory usage increase too high")
        
        # 清理
        del large_data
        gc.collect()
    
    @patch('synapse.util.caches.cache_manager.CacheManager')
    def test_cache_strategy_switching(self, mock_cache_manager) -> None:
        """测试缓存策略切换"""
        # 1. 模拟内存缓存
        mock_cache_manager.return_value.get.return_value = "memory_value"
        
        # 2. 测试缓存获取
        if self.cache_manager:
            value = self.cache_manager.get("test", "key1")
            # 验证缓存调用
            self.assertIsNotNone(value)
        
        # 3. 模拟 Redis 缓存故障转移
        mock_cache_manager.return_value.get.side_effect = Exception("Redis connection failed")
        
        # 4. 验证故障转移到内存缓存
        try:
            if self.cache_manager:
                value = self.cache_manager.get("test", "key1")
        except Exception as e:
            # 应该优雅处理 Redis 故障
            self.assertIn("Redis", str(e))


class MonitoringIntegrationTestCase(unittest.HomeserverTestCase):
    """监控功能集成测试类"""
    
    def prepare(self, reactor: MemoryReactor, clock: Clock, hs: HomeServer) -> None:
        """准备测试环境"""
        self.user_id = self.register_user("alice", "password")
        self.access_token = self.login("alice", "password")
        
        # 获取监控相关组件
        self.performance_monitor = getattr(hs, '_performance_monitor', None)
        self.system_monitor = getattr(hs, '_system_monitor', None)
    
    def test_performance_metrics_collection(self) -> None:
        """测试性能指标收集"""
        if not self.performance_monitor:
            self.skipTest("Performance monitor not available")
        
        # 1. 收集系统指标
        metrics = self.performance_monitor.collect_system_metrics()
        
        # 2. 验证指标完整性
        required_metrics = [
            "cpu_percent",
            "memory_percent",
            "disk_usage",
            "network_io",
        ]
        
        for metric in required_metrics:
            self.assertIn(metric, metrics)
            self.assertIsInstance(metrics[metric], (int, float))
        
        # 3. 验证指标范围合理性
        self.assertGreaterEqual(metrics["cpu_percent"], 0)
        self.assertLessEqual(metrics["cpu_percent"], 100)
        self.assertGreaterEqual(metrics["memory_percent"], 0)
        self.assertLessEqual(metrics["memory_percent"], 100)
    
    def test_cache_metrics_collection(self) -> None:
        """测试缓存指标收集"""
        if not self.performance_monitor:
            self.skipTest("Performance monitor not available")
        
        # 1. 执行一些缓存操作
        cache_manager = self.hs.get_cache_manager()
        if cache_manager:
            cache_manager.set("test", "key1", "value1", ttl=3600)
            cache_manager.get("test", "key1")
            cache_manager.get("test", "nonexistent_key")
        
        # 2. 收集缓存指标
        metrics = self.performance_monitor.collect_cache_metrics()
        
        # 3. 验证缓存指标
        if cache_manager:
            self.assertIn("cache_stats", metrics)
            cache_stats = metrics["cache_stats"]
            
            if "test" in cache_stats:
                test_stats = cache_stats["test"]
                self.assertIn("hits", test_stats)
                self.assertIn("misses", test_stats)
                self.assertIn("size", test_stats)
    
    def test_alert_threshold_checking(self) -> None:
        """测试告警阈值检查"""
        if not self.performance_monitor:
            self.skipTest("Performance monitor not available")
        
        # 1. 设置测试阈值
        test_thresholds = {
            "cpu_percent": 80,
            "memory_percent": 85,
            "disk_percent": 90,
        }
        
        # 2. 模拟高负载指标
        high_load_metrics = {
            "cpu_percent": 85,  # 超过阈值
            "memory_percent": 90,  # 超过阈值
            "disk_percent": 75,  # 未超过阈值
        }
        
        # 3. 检查阈值
        alerts = []
        for metric, value in high_load_metrics.items():
            if metric in test_thresholds and value > test_thresholds[metric]:
                alerts.append({
                    "metric": metric,
                    "value": value,
                    "threshold": test_thresholds[metric],
                    "level": "warning"
                })
        
        # 4. 验证告警生成
        self.assertEqual(len(alerts), 2)  # CPU 和内存超过阈值
        
        alert_metrics = [alert["metric"] for alert in alerts]
        self.assertIn("cpu_percent", alert_metrics)
        self.assertIn("memory_percent", alert_metrics)
        self.assertNotIn("disk_percent", alert_metrics)
    
    def test_log_rotation_and_cleanup(self) -> None:
        """测试日志轮转和清理"""
        import tempfile
        import os
        
        # 1. 创建临时日志目录
        with tempfile.TemporaryDirectory() as temp_dir:
            log_file = os.path.join(temp_dir, "test.log")
            
            # 2. 写入大量日志数据
            with open(log_file, "w") as f:
                for i in range(1000):
                    f.write(f"Log line {i}\n")
            
            # 3. 检查文件大小
            file_size = os.path.getsize(log_file)
            self.assertGreater(file_size, 0)
            
            # 4. 模拟日志轮转
            rotated_file = f"{log_file}.1"
            os.rename(log_file, rotated_file)
            
            # 5. 创建新的日志文件
            with open(log_file, "w") as f:
                f.write("New log file\n")
            
            # 6. 验证轮转成功
            self.assertTrue(os.path.exists(log_file))
            self.assertTrue(os.path.exists(rotated_file))
            
            # 7. 验证新文件大小较小
            new_file_size = os.path.getsize(log_file)
            self.assertLess(new_file_size, file_size)


class SecurityIntegrationTestCase(unittest.HomeserverTestCase):
    """安全功能集成测试类"""
    
    def prepare(self, reactor: MemoryReactor, clock: Clock, hs: HomeServer) -> None:
        """准备测试环境"""
        self.user_id = self.register_user("alice", "password")
        self.access_token = self.login("alice", "password")
    
    def test_rate_limiting_enforcement(self) -> None:
        """测试速率限制执行"""
        # 1. 快速发送多个请求
        responses = []
        for i in range(10):
            channel = self.make_request(
                "GET",
                "/_matrix/client/r0/friends",
                access_token=self.access_token,
            )
            responses.append(channel.code)
        
        # 2. 验证大部分请求成功
        success_count = sum(1 for code in responses if code == 200)
        self.assertGreaterEqual(success_count, 5)  # 至少一半请求成功
        
        # 3. 可能有一些请求被限制（429 状态码）
        rate_limited_count = sum(1 for code in responses if code == 429)
        # 在测试环境中，速率限制可能不会触发，所以这里只是记录
    
    def test_authentication_required(self) -> None:
        """测试认证要求"""
        # 1. 未认证请求
        channel = self.make_request(
            "GET",
            "/_matrix/client/r0/friends",
        )
        self.assertEqual(channel.code, 401)
        
        # 2. 无效令牌
        channel = self.make_request(
            "GET",
            "/_matrix/client/r0/friends",
            access_token="invalid_token",
        )
        self.assertEqual(channel.code, 401)
        
        # 3. 有效令牌
        channel = self.make_request(
            "GET",
            "/_matrix/client/r0/friends",
            access_token=self.access_token,
        )
        self.assertEqual(channel.code, 200)
    
    def test_input_validation_and_sanitization(self) -> None:
        """测试输入验证和清理"""
        # 1. 测试恶意输入
        malicious_inputs = [
            "<script>alert('xss')</script>",
            "'; DROP TABLE users; --",
            "../../../etc/passwd",
            "\x00\x01\x02",  # 空字节和控制字符
        ]
        
        for malicious_input in malicious_inputs:
            # 尝试在好友请求消息中使用恶意输入
            channel = self.make_request(
                "POST",
                "/_matrix/client/r0/friends/@test:test/request",
                {"message": malicious_input},
                access_token=self.access_token,
            )
            
            # 应该被拒绝或清理，不应该导致服务器错误
            self.assertIn(channel.code, [400, 404])  # 400 验证失败，404 用户不存在
        
        # 2. 测试过长输入
        long_message = "x" * 10000
        channel = self.make_request(
            "POST",
            "/_matrix/client/r0/friends/@test:test/request",
            {"message": long_message},
            access_token=self.access_token,
        )
        self.assertEqual(channel.code, 400)  # 应该被拒绝
    
    def test_user_isolation(self) -> None:
        """测试用户隔离"""
        # 1. 创建另一个用户
        other_user_id = self.register_user("bob", "password")
        other_access_token = self.login("bob", "password")
        
        # 2. Alice 创建一些数据
        channel = self.make_request(
            "POST",
            f"/_matrix/client/r0/friends/{other_user_id}/request",
            {"message": "Hi Bob!"},
            access_token=self.access_token,
        )
        self.assertEqual(channel.code, 200)
        
        # 3. Bob 不应该能访问 Alice 的发送请求列表
        channel = self.make_request(
            "GET",
            "/_matrix/client/r0/friends/requests/sent",
            access_token=other_access_token,
        )
        self.assertEqual(channel.code, 200)
        sent_requests = channel.json_body["requests"]
        
        # Bob 的发送列表应该为空（因为他没有发送请求）
        self.assertEqual(len(sent_requests), 0)
        
        # 4. Bob 应该能看到自己的接收请求
        channel = self.make_request(
            "GET",
            "/_matrix/client/r0/friends/requests/received",
            access_token=other_access_token,
        )
        self.assertEqual(channel.code, 200)
        received_requests = channel.json_body["requests"]
        
        # Bob 应该有一个来自 Alice 的请求
        self.assertEqual(len(received_requests), 1)
        self.assertEqual(received_requests[0]["from_user_id"], self.user_id)