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

"""好友功能测试用例"""

import json
from typing import Dict, Any
from unittest.mock import Mock, patch

from synapse.api.errors import SynapseError, Codes
from synapse.handlers.friends import FriendsHandler
from synapse.rest.client.friends import (
    FriendsListServlet,
    FriendRequestServlet,
    FriendActionServlet,
    UserSearchServlet,
    BlockedUsersServlet,
    BlockedUserActionServlet,
)
from synapse.server import HomeServer
from synapse.storage.friends import FriendsStore
from synapse.types import UserID
from synapse.util import Clock

from tests import unittest
from tests.server import make_request
from tests.utils import MockClock
from twisted.internet.testing import MemoryReactor


class FriendsTestCase(unittest.HomeserverTestCase):
    """好友功能基础测试类"""
    
    servlets = [
        FriendsListServlet.register,
        FriendRequestServlet.register,
        FriendActionServlet.register,
        UserSearchServlet.register,
        BlockedUsersServlet.register,
        BlockedUserActionServlet.register,
    ]
    
    def prepare(self, reactor: MemoryReactor, clock: Clock, hs: HomeServer) -> None:
        self.hs = hs
        self.store = hs.get_datastore()
        self.friends_handler = hs.get_friends_handler()
        
        # 创建测试用户
        self.user1_id = "@user1:test"
        self.user2_id = "@user2:test"
        self.user3_id = "@user3:test"
        
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
        
        self.get_success(
            self.store.register_user(
                user_id=self.user3_id,
                password_hash=None,
                was_guest=False,
                make_guest=False,
                appservice_id=None,
                create_profile_with_displayname="User 3",
                admin=False,
                user_type=None,
                shadow_banned=False,
            )
        )
    
    async def test_send_friend_request(self):
        """测试发送好友请求"""
        result = await self.friends_handler.send_friend_request(
            "@user1:example.com", "@user2:example.com", "Hello!"
        )
        
        assert result["request_id"] is not None
        assert result["status"] == "pending"
        
        # 验证存储调用
        self.mock_store.create_friend_request.assert_called_once()
    
    def test_accept_friend_request(self) -> None:
        """测试接受好友请求"""
        # 发送好友请求
        request_id = self.get_success(
            self.friends_handler.send_friend_request(
                requester_id=self.user1_id,
                target_id=self.user2_id,
                message="Hello!"
            )
        )
        
        # 接受好友请求
        self.get_success(
            self.friends_handler.accept_friend_request(
                user_id=self.user2_id,
                request_id=request_id
            )
        )
        
        # 验证好友关系已建立
        is_friend = self.get_success(
            self.store.are_friends(self.user1_id, self.user2_id)
        )
        self.assertTrue(is_friend)
        
        # 验证请求状态已更新
        request = self.get_success(
            self.store.get_friend_request(request_id)
        )
        self.assertEqual(request["status"], "accepted")
    
    def test_reject_friend_request(self) -> None:
        """测试拒绝好友请求"""
        # 发送好友请求
        request_id = self.get_success(
            self.friends_handler.send_friend_request(
                requester_id=self.user1_id,
                target_id=self.user2_id,
                message="Hello!"
            )
        )
        
        # 拒绝好友请求
        self.get_success(
            self.friends_handler.reject_friend_request(
                user_id=self.user2_id,
                request_id=request_id
            )
        )
        
        # 验证好友关系未建立
        is_friend = self.get_success(
            self.store.are_friends(self.user1_id, self.user2_id)
        )
        self.assertFalse(is_friend)
        
        # 验证请求状态已更新
        request = self.get_success(
            self.store.get_friend_request(request_id)
        )
        self.assertEqual(request["status"], "rejected")
    
    def test_remove_friend(self) -> None:
        """测试删除好友"""
        # 建立好友关系
        self.get_success(
            self.store.add_friend(self.user1_id, self.user2_id)
        )
        
        # 验证好友关系存在
        is_friend = self.get_success(
            self.store.are_friends(self.user1_id, self.user2_id)
        )
        self.assertTrue(is_friend)
        
        # 删除好友
        self.get_success(
            self.friends_handler.remove_friend(
                user_id=self.user1_id,
                friend_id=self.user2_id
            )
        )
        
        # 验证好友关系已删除
        is_friend = self.get_success(
            self.store.are_friends(self.user1_id, self.user2_id)
        )
        self.assertFalse(is_friend)
    
    def test_get_friends_list(self) -> None:
        """测试获取好友列表"""
        # 建立好友关系
        self.get_success(
            self.store.add_friend(self.user1_id, self.user2_id)
        )
        self.get_success(
            self.store.add_friend(self.user1_id, self.user3_id)
        )
        
        # 获取好友列表
        friends = self.get_success(
            self.friends_handler.get_friends_list(self.user1_id)
        )
        
        self.assertEqual(len(friends), 2)
        friend_ids = [friend["user_id"] for friend in friends]
        self.assertIn(self.user2_id, friend_ids)
        self.assertIn(self.user3_id, friend_ids)
    
    def test_search_users(self) -> None:
        """测试搜索用户"""
        # 搜索用户
        results = self.get_success(
            self.friends_handler.search_users(
                searcher_id=self.user1_id,
                query="User",
                limit=10
            )
        )
        
        self.assertGreaterEqual(len(results), 2)
        user_ids = [user["user_id"] for user in results]
        self.assertIn(self.user2_id, user_ids)
        self.assertIn(self.user3_id, user_ids)
    
    def test_duplicate_friend_request(self) -> None:
        """测试重复发送好友请求"""
        # 发送第一个好友请求
        request_id1 = self.get_success(
            self.friends_handler.send_friend_request(
                requester_id=self.user1_id,
                target_id=self.user2_id,
                message="Hello!"
            )
        )
        
        # 尝试发送重复的好友请求
        with self.assertRaises(SynapseError) as cm:
            self.get_success(
                self.friends_handler.send_friend_request(
                    requester_id=self.user1_id,
                    target_id=self.user2_id,
                    message="Hello again!"
                )
            )
        
        self.assertEqual(cm.exception.code, 400)
        self.assertEqual(cm.exception.errcode, Codes.DUPLICATE_ANNOTATION)
    
    def test_self_friend_request(self) -> None:
        """测试向自己发送好友请求"""
        with self.assertRaises(SynapseError) as cm:
            self.get_success(
                self.friends_handler.send_friend_request(
                    requester_id=self.user1_id,
                    target_id=self.user1_id,
                    message="Hello myself!"
                )
            )
        
        self.assertEqual(cm.exception.code, 400)
        self.assertEqual(cm.exception.errcode, Codes.INVALID_PARAM)
    
    def test_friend_request_to_existing_friend(self) -> None:
        """测试向已经是好友的用户发送好友请求"""
        # 建立好友关系
        self.get_success(
            self.store.add_friend(self.user1_id, self.user2_id)
        )
        
        # 尝试发送好友请求
        with self.assertRaises(SynapseError) as cm:
            self.get_success(
                self.friends_handler.send_friend_request(
                    requester_id=self.user1_id,
                    target_id=self.user2_id,
                    message="We're already friends!"
                )
            )
        
        self.assertEqual(cm.exception.code, 400)
        self.assertEqual(cm.exception.errcode, Codes.DUPLICATE_ANNOTATION)


class FriendsRestTestCase(unittest.HomeserverTestCase):
    """好友功能 REST API 测试类"""
    
    servlets = [
        FriendsListServlet.register,
        FriendRequestServlet.register,
        FriendActionServlet.register,
        UserSearchServlet.register,
        BlockedUsersServlet.register,
        BlockedUserActionServlet.register,
    ]
    
    def prepare(self, reactor: MemoryReactor, clock: Clock, hs: HomeServer) -> None:
        self.hs = hs
        self.store = hs.get_datastore()
        
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
        
        # 创建访问令牌
        self.user1_token = self.login("user1", "password")
        self.user2_token = self.login("user2", "password")
    
    def test_get_friends_list_api(self) -> None:
        """测试获取好友列表 API"""
        # 建立好友关系
        self.get_success(
            self.store.add_friend(self.user1_id, self.user2_id)
        )
        
        # 调用 API
        channel = self.make_request(
            "GET",
            "/_matrix/client/v1/friends",
            access_token=self.user1_token,
        )
        
        self.assertEqual(channel.code, 200)
        
        response = channel.json_body
        self.assertIn("friends", response)
        self.assertEqual(len(response["friends"]), 1)
        self.assertEqual(response["friends"][0]["user_id"], self.user2_id)
    
    def test_send_friend_request_api(self) -> None:
        """测试发送好友请求 API"""
        # 调用 API
        channel = self.make_request(
            "POST",
            "/_matrix/client/v1/friends/request",
            content={
                "target_id": self.user2_id,
                "message": "Hello, let's be friends!"
            },
            access_token=self.user1_token,
        )
        
        self.assertEqual(channel.code, 200)
        
        response = channel.json_body
        self.assertIn("request_id", response)
        
        # 验证请求已创建
        request = self.get_success(
            self.store.get_friend_request(response["request_id"])
        )
        self.assertEqual(request["requester_id"], self.user1_id)
        self.assertEqual(request["target_id"], self.user2_id)
    
    def test_accept_friend_request_api(self) -> None:
        """测试接受好友请求 API"""
        # 发送好友请求
        request_id = self.get_success(
            self.hs.get_friends_handler().send_friend_request(
                requester_id=self.user1_id,
                target_id=self.user2_id,
                message="Hello!"
            )
        )
        
        # 调用 API 接受请求
        channel = self.make_request(
            "POST",
            f"/_matrix/client/v1/friends/request/{request_id}/accept",
            access_token=self.user2_token,
        )
        
        self.assertEqual(channel.code, 200)
        
        # 验证好友关系已建立
        is_friend = self.get_success(
            self.store.are_friends(self.user1_id, self.user2_id)
        )
        self.assertTrue(is_friend)
    
    def test_search_users_api(self) -> None:
        """测试搜索用户 API"""
        # 调用 API
        channel = self.make_request(
            "GET",
            "/_matrix/client/v1/friends/search?q=User&limit=10",
            access_token=self.user1_token,
        )
        
        self.assertEqual(channel.code, 200)
        
        response = channel.json_body
        self.assertIn("users", response)
        self.assertGreaterEqual(len(response["users"]), 1)
        
        # 验证搜索结果包含用户信息
        user_ids = [user["user_id"] for user in response["users"]]
        self.assertIn(self.user2_id, user_ids)
    
    def test_unauthorized_access(self) -> None:
        """测试未授权访问"""
        # 不提供访问令牌
        channel = self.make_request(
            "GET",
            "/_matrix/client/v1/friends",
        )
        
        self.assertEqual(channel.code, 401)
    
    def test_invalid_request_format(self) -> None:
        """测试无效的请求格式"""
        # 发送无效的请求体
        channel = self.make_request(
            "POST",
            "/_matrix/client/v1/friends/request",
            content={
                "invalid_field": "value"
            },
            access_token=self.user1_token,
        )
        
        self.assertEqual(channel.code, 400)


class FriendsStoreTestCase(unittest.HomeserverTestCase):
    """好友功能存储层测试类"""
    
    def prepare(self, reactor: MemoryReactor, clock: Clock, hs: HomeServer) -> None:
        self.store = hs.get_datastore()
        
        # 创建测试用户
        self.user1_id = "@user1:test"
        self.user2_id = "@user2:test"
        self.user3_id = "@user3:test"
    
    def test_add_friend(self) -> None:
        """测试添加好友"""
        # 添加好友关系
        self.get_success(
            self.store.add_friend(self.user1_id, self.user2_id)
        )
        
        # 验证好友关系
        is_friend = self.get_success(
            self.store.are_friends(self.user1_id, self.user2_id)
        )
        self.assertTrue(is_friend)
        
        # 验证双向关系
        is_friend_reverse = self.get_success(
            self.store.are_friends(self.user2_id, self.user1_id)
        )
        self.assertTrue(is_friend_reverse)
    
    def test_remove_friend(self) -> None:
        """测试删除好友"""
        # 添加好友关系
        self.get_success(
            self.store.add_friend(self.user1_id, self.user2_id)
        )
        
        # 删除好友关系
        self.get_success(
            self.store.remove_friend(self.user1_id, self.user2_id)
        )
        
        # 验证好友关系已删除
        is_friend = self.get_success(
            self.store.are_friends(self.user1_id, self.user2_id)
        )
        self.assertFalse(is_friend)
    
    def test_get_friends_list(self) -> None:
        """测试获取好友列表"""
        # 添加多个好友
        self.get_success(
            self.store.add_friend(self.user1_id, self.user2_id)
        )
        self.get_success(
            self.store.add_friend(self.user1_id, self.user3_id)
        )
        
        # 获取好友列表
        friends = self.get_success(
            self.store.get_friends_list(self.user1_id)
        )
        
        self.assertEqual(len(friends), 2)
        friend_ids = [friend["user_id"] for friend in friends]
        self.assertIn(self.user2_id, friend_ids)
        self.assertIn(self.user3_id, friend_ids)
    
    def test_create_friend_request(self) -> None:
        """测试创建好友请求"""
        # 创建好友请求
        request_id = self.get_success(
            self.store.create_friend_request(
                requester_id=self.user1_id,
                target_id=self.user2_id,
                message="Hello!"
            )
        )
        
        self.assertIsNotNone(request_id)
        
        # 获取请求详情
        request = self.get_success(
            self.store.get_friend_request(request_id)
        )
        
        self.assertEqual(request["requester_id"], self.user1_id)
        self.assertEqual(request["target_id"], self.user2_id)
        self.assertEqual(request["message"], "Hello!")
        self.assertEqual(request["status"], "pending")
    
    def test_update_friend_request_status(self) -> None:
        """测试更新好友请求状态"""
        # 创建好友请求
        request_id = self.get_success(
            self.store.create_friend_request(
                requester_id=self.user1_id,
                target_id=self.user2_id,
                message="Hello!"
            )
        )
        
        # 更新请求状态
        self.get_success(
            self.store.update_friend_request_status(
                request_id=request_id,
                status="accepted"
            )
        )
        
        # 验证状态已更新
        request = self.get_success(
            self.store.get_friend_request(request_id)
        )
        self.assertEqual(request["status"], "accepted")
    
    def test_get_pending_friend_requests(self) -> None:
        """测试获取待处理的好友请求"""
        # 创建多个好友请求
        request_id1 = self.get_success(
            self.store.create_friend_request(
                requester_id=self.user1_id,
                target_id=self.user2_id,
                message="Hello!"
            )
        )
        
        request_id2 = self.get_success(
            self.store.create_friend_request(
                requester_id=self.user3_id,
                target_id=self.user2_id,
                message="Hi there!"
            )
        )
        
        # 获取待处理请求
        requests = self.get_success(
            self.store.get_pending_friend_requests(self.user2_id)
        )
        
        self.assertEqual(len(requests), 2)
        request_ids = [req["request_id"] for req in requests]
        self.assertIn(request_id1, request_ids)
        self.assertIn(request_id2, request_ids)


class FriendsCacheTestCase(unittest.HomeserverTestCase):
    """好友功能缓存测试类"""
    
    def prepare(self, reactor: MemoryReactor, clock: Clock, hs: HomeServer) -> None:
        self.hs = hs
        self.store = hs.get_datastore()
        self.cache_manager = hs.get_cache_manager()
        
        # 创建测试用户
        self.user1_id = "@user1:test"
        self.user2_id = "@user2:test"
    
    @patch('synapse.util.caches.cache_manager.CacheManager.get')
    @patch('synapse.util.caches.cache_manager.CacheManager.set')
    def test_friends_list_cache(self, mock_set, mock_get) -> None:
        """测试好友列表缓存"""
        # 模拟缓存未命中
        mock_get.return_value = None
        
        # 添加好友关系
        self.get_success(
            self.store.add_friend(self.user1_id, self.user2_id)
        )
        
        # 获取好友列表（应该触发缓存设置）
        friends = self.get_success(
            self.store.get_friends_list(self.user1_id)
        )
        
        # 验证缓存操作被调用
        mock_get.assert_called()
        mock_set.assert_called()
    
    def test_cache_invalidation_on_friend_add(self) -> None:
        """测试添加好友时的缓存失效"""
        # 获取初始好友列表（建立缓存）
        friends1 = self.get_success(
            self.store.get_friends_list(self.user1_id)
        )
        self.assertEqual(len(friends1), 0)
        
        # 添加好友
        self.get_success(
            self.store.add_friend(self.user1_id, self.user2_id)
        )
        
        # 再次获取好友列表（应该反映新的好友关系）
        friends2 = self.get_success(
            self.store.get_friends_list(self.user1_id)
        )
        self.assertEqual(len(friends2), 1)
        self.assertEqual(friends2[0]["user_id"], self.user2_id)
    
    def test_cache_invalidation_on_friend_remove(self) -> None:
        """测试删除好友时的缓存失效"""
        # 添加好友
        self.get_success(
            self.store.add_friend(self.user1_id, self.user2_id)
        )
        
        # 获取好友列表（建立缓存）
        friends1 = self.get_success(
            self.store.get_friends_list(self.user1_id)
        )
        self.assertEqual(len(friends1), 1)
        
        # 删除好友
        self.get_success(
            self.store.remove_friend(self.user1_id, self.user2_id)
        )
        
        # 再次获取好友列表（应该反映删除操作）
        friends2 = self.get_success(
            self.store.get_friends_list(self.user1_id)
        )
        self.assertEqual(len(friends2), 0)