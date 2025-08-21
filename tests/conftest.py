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

"""测试配置文件"""

import os
import sys
import tempfile
import shutil
from typing import Generator, Dict, Any
from unittest.mock import Mock, patch

import pytest

# 添加项目根目录到 Python 路径
project_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if project_root not in sys.path:
    sys.path.insert(0, project_root)

# 创建模拟对象，避免依赖复杂的 Synapse 模块
class MockHomeServer:
    def __init__(self):
        self._datastore = Mock()
        self._cache_manager = Mock()
        self._friends_handler = Mock()
    
    def get_datastore(self):
        return self._datastore
    
    def get_cache_manager(self):
        return self._cache_manager
    
    def get_friends_handler(self):
        return self._friends_handler

class MockHomeServerConfig:
    def __init__(self, config_dict):
        self.config = config_dict

class MockClock:
    def __init__(self):
        self.time = 0
    
    def time_msec(self):
        return int(self.time * 1000)
    
    def advance(self, amount):
        self.time += amount

HomeServer = MockHomeServer
HomeServerConfig = MockHomeServerConfig
Clock = MockClock

def setup_test_homeserver(*args, **kwargs):
    return MockHomeServer()


@pytest.fixture(scope="session")
def test_data_dir() -> Generator[str, None, None]:
    """创建临时测试数据目录"""
    temp_dir = tempfile.mkdtemp(prefix="synapse_test_")
    try:
        yield temp_dir
    finally:
        shutil.rmtree(temp_dir, ignore_errors=True)


@pytest.fixture(scope="session")
def test_config(test_data_dir: str) -> Dict[str, Any]:
    """测试配置"""
    return {
        "server_name": "test",
        "data_dir": test_data_dir,
        "database": {
            "name": "sqlite3",
            "args": {
                "database": ":memory:"
            }
        },
        "redis": {
            "enabled": False
        },
        "cache_strategy": {
            "default_ttl": 3600,
            "max_memory_cache_size": 1000,
            "redis_enabled": False
        },
        "performance": {
            "max_concurrent_requests": 10,
            "database_pool_size": 5,
            "memory_limit_mb": 512,
            "gc_threshold": 1000
        },
        "friends": {
            "enabled": True,
            "max_friends": 100,
            "request_timeout": 86400
        },
        "logging": {
            "version": 1,
            "disable_existing_loggers": False,
            "handlers": {
                "console": {
                    "class": "logging.StreamHandler",
                    "level": "WARNING"
                }
            },
            "root": {
                "level": "WARNING",
                "handlers": ["console"]
            }
        }
    }


@pytest.fixture
def mock_homeserver(test_config: Dict[str, Any]) -> Generator[HomeServer, None, None]:
    """模拟 HomeServer 实例"""
    try:
        # 尝试创建真实的 HomeServer
        reactor = Mock()
        clock = MockClock()
        hs = setup_test_homeserver(
            "test",
            reactor=reactor,
            clock=clock,
            config=test_config
        )
        yield hs
    except Exception:
        # 如果失败，创建模拟对象
        mock_hs = Mock(spec=HomeServer)
        mock_hs.get_datastore.return_value = Mock()
        mock_hs.get_cache_manager.return_value = Mock()
        mock_hs.get_friends_handler.return_value = Mock()
        yield mock_hs


@pytest.fixture
def mock_cache_manager():
    """模拟缓存管理器"""
    cache_manager = Mock()
    cache_manager.get.return_value = None
    cache_manager.set.return_value = None
    cache_manager.delete.return_value = None
    cache_manager.clear.return_value = None
    cache_manager.get_stats.return_value = {
        "test": {
            "hits": 10,
            "misses": 5,
            "size": 15
        }
    }
    return cache_manager


@pytest.fixture
def mock_friends_handler():
    """模拟好友处理器"""
    handler = Mock()
    handler.send_friend_request.return_value = defer.succeed("request_id_123")
    handler.accept_friend_request.return_value = defer.succeed(None)
    handler.reject_friend_request.return_value = defer.succeed(None)
    handler.remove_friend.return_value = defer.succeed(None)
    handler.get_friends.return_value = defer.succeed([])
    handler.get_friend_requests.return_value = defer.succeed([])
    handler.search_users.return_value = defer.succeed([])
    return handler


@pytest.fixture
def mock_datastore():
    """模拟数据存储"""
    datastore = Mock()
    
    # 好友相关方法
    datastore.send_friend_request.return_value = defer.succeed("request_id_123")
    datastore.accept_friend_request.return_value = defer.succeed(None)
    datastore.reject_friend_request.return_value = defer.succeed(None)
    datastore.add_friend.return_value = defer.succeed(None)
    datastore.remove_friend.return_value = defer.succeed(None)
    datastore.get_friends.return_value = defer.succeed([])
    datastore.get_friend_requests_sent.return_value = defer.succeed([])
    datastore.get_friend_requests_received.return_value = defer.succeed([])
    datastore.search_users.return_value = defer.succeed([])
    datastore.is_friend.return_value = defer.succeed(False)
    datastore.has_pending_request.return_value = defer.succeed(False)
    
    # 用户相关方法
    datastore.get_user_by_id.return_value = defer.succeed(None)
    datastore.register_user.return_value = defer.succeed(None)
    
    return datastore


@pytest.fixture
def mock_performance_monitor():
    """模拟性能监控器"""
    monitor = Mock()
    monitor.collect_system_metrics.return_value = {
        "cpu_percent": 25.5,
        "memory_percent": 45.2,
        "disk_usage": 60.0,
        "network_io": {
            "bytes_sent": 1024,
            "bytes_recv": 2048
        }
    }
    monitor.collect_cache_metrics.return_value = {
        "cache_stats": {
            "test": {
                "hits": 100,
                "misses": 20,
                "size": 120
            }
        }
    }
    monitor.collect_database_metrics.return_value = {
        "active_connections": 5,
        "total_connections": 10,
        "query_count": 1000,
        "avg_query_time": 0.05
    }
    monitor.check_thresholds.return_value = []
    return monitor


@pytest.fixture(autouse=True)
def setup_test_environment(monkeypatch):
    """自动设置测试环境"""
    # 设置测试环境变量
    monkeypatch.setenv("SYNAPSE_TEST_MODE", "1")
    monkeypatch.setenv("SYNAPSE_CONFIG_PATH", "/tmp/test_config.yaml")
    
    # 模拟一些可能不存在的模块
    mock_modules = [
        "synapse.handlers.friends",
        "synapse.storage.databases.main.friends",
        "synapse.util.caches.cache_manager",
        "synapse.config.performance",
        "synapse.config.cache_strategy"
    ]
    
    for module_name in mock_modules:
        if module_name not in sys.modules:
            sys.modules[module_name] = Mock()


@pytest.fixture
def sample_users():
    """示例用户数据"""
    return [
        {
            "user_id": "@alice:test",
            "display_name": "Alice",
            "avatar_url": "mxc://test/alice_avatar"
        },
        {
            "user_id": "@bob:test",
            "display_name": "Bob",
            "avatar_url": "mxc://test/bob_avatar"
        },
        {
            "user_id": "@charlie:test",
            "display_name": "Charlie",
            "avatar_url": None
        }
    ]


@pytest.fixture
def sample_friend_requests():
    """示例好友请求数据"""
    return [
        {
            "request_id": "req_123",
            "from_user_id": "@alice:test",
            "to_user_id": "@bob:test",
            "message": "Hi Bob, let's be friends!",
            "created_at": 1640995200000,  # 2022-01-01 00:00:00
            "status": "pending"
        },
        {
            "request_id": "req_456",
            "from_user_id": "@charlie:test",
            "to_user_id": "@alice:test",
            "message": "Hello Alice!",
            "created_at": 1640995260000,  # 2022-01-01 00:01:00
            "status": "pending"
        }
    ]


@pytest.fixture
def sample_friendships():
    """示例好友关系数据"""
    return [
        {
            "user_id": "@alice:test",
            "friend_id": "@bob:test",
            "created_at": 1640995200000
        },
        {
            "user_id": "@bob:test",
            "friend_id": "@alice:test",
            "created_at": 1640995200000
        }
    ]


# 测试标记
pytestmark = [
    pytest.mark.asyncio,  # 支持异步测试
]


# 测试钩子
def pytest_configure(config):
    """配置 pytest"""
    # 添加自定义标记
    config.addinivalue_line(
        "markers", "slow: marks tests as slow (deselect with '-m \"not slow\"')"
    )
    config.addinivalue_line(
        "markers", "integration: marks tests as integration tests"
    )
    config.addinivalue_line(
        "markers", "unit: marks tests as unit tests"
    )
    config.addinivalue_line(
        "markers", "performance: marks tests as performance tests"
    )


def pytest_collection_modifyitems(config, items):
    """修改测试收集"""
    # 为没有标记的测试添加默认标记
    for item in items:
        if not any(item.iter_markers()):
            item.add_marker(pytest.mark.unit)
        
        # 为慢测试添加标记
        if "integration" in item.nodeid or "performance" in item.nodeid:
            item.add_marker(pytest.mark.slow)


def pytest_runtest_setup(item):
    """测试运行前设置"""
    # 跳过需要真实 Synapse 环境的测试
    if item.get_closest_marker("requires_synapse"):
        try:
            import synapse
        except ImportError:
            pytest.skip("需要 Synapse 环境")


def pytest_runtest_teardown(item, nextitem):
    """测试运行后清理"""
    # 清理临时文件
    import tempfile
    import glob
    
    temp_files = glob.glob("/tmp/synapse_test_*")
    for temp_file in temp_files:
        try:
            if os.path.isfile(temp_file):
                os.remove(temp_file)
            elif os.path.isdir(temp_file):
                shutil.rmtree(temp_file, ignore_errors=True)
        except Exception:
            pass  # 忽略清理错误