# -*- coding: utf-8 -*-
# 好友功能REST API接口
# Copyright 2024 The Matrix.org Foundation C.I.C.
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

import logging
from typing import TYPE_CHECKING, Tuple

from synapse.api.errors import Codes, SynapseError
from synapse.http.servlet import (
    RestServlet,
    parse_json_object_from_request,
    parse_string,
    parse_integer,
)
from synapse.http.site import SynapseRequest
from synapse.rest.client._base import client_patterns
from synapse.types import JsonDict

if TYPE_CHECKING:
    from synapse.server import HomeServer

logger = logging.getLogger(__name__)


class FriendRequestServlet(RestServlet):
    """好友请求相关API"""
    
    PATTERNS = client_patterns("/friends/requests$", releases=("r0",))
    
    def __init__(self, hs: "HomeServer"):
        super().__init__()
        self.auth = hs.get_auth()
        self.friends_handler = hs.get_friends_handler()
    
    async def on_POST(self, request: SynapseRequest) -> Tuple[int, JsonDict]:
        """发送好友请求
        
        POST /friends/requests
        {
            "target_user_id": "@user:example.com",
            "message": "想和你成为好友"
        }
        """
        requester = await self.auth.get_user_by_req(request)
        user_id = requester.user.to_string()
        
        content = parse_json_object_from_request(request)
        
        target_user_id = content.get("target_user_id")
        if not target_user_id:
            raise SynapseError(400, "缺少目标用户ID", Codes.MISSING_PARAM)
        
        message = content.get("message")
        if message and len(message) > 500:
            raise SynapseError(400, "消息长度不能超过500字符", Codes.INVALID_PARAM)
        
        result = await self.friends_handler.send_friend_request(
            user_id, target_user_id, message
        )
        
        return 200, result
    
    async def on_GET(self, request: SynapseRequest) -> Tuple[int, JsonDict]:
        """获取好友请求列表
        
        GET /friends/requests?type=received&limit=50
        """
        requester = await self.auth.get_user_by_req(request)
        user_id = requester.user.to_string()
        
        request_type = parse_string(request, "type", default="received")
        limit = parse_integer(request, "limit", default=50)
        
        if limit > 100:
            limit = 100
        
        result = await self.friends_handler.get_friend_requests(
            user_id, request_type, limit
        )
        
        return 200, result


class FriendRequestActionServlet(RestServlet):
    """好友请求操作API"""
    
    PATTERNS = client_patterns("/friends/requests/(?P<request_id>[^/]+)$", releases=("r0",))
    
    def __init__(self, hs: "HomeServer"):
        super().__init__()
        self.auth = hs.get_auth()
        self.friends_handler = hs.get_friends_handler()
    
    async def on_PUT(self, request: SynapseRequest, request_id: str) -> Tuple[int, JsonDict]:
        """响应好友请求
        
        PUT /friends/requests/{request_id}
        {
            "action": "accept"  // 或 "reject"
        }
        """
        requester = await self.auth.get_user_by_req(request)
        user_id = requester.user.to_string()
        
        content = parse_json_object_from_request(request)
        action = content.get("action")
        
        if action not in ("accept", "reject"):
            raise SynapseError(400, "无效的操作类型", Codes.INVALID_PARAM)
        
        result = await self.friends_handler.respond_to_friend_request(
            user_id, request_id, action
        )
        
        return 200, result
    
    async def on_DELETE(self, request: SynapseRequest, request_id: str) -> Tuple[int, JsonDict]:
        """取消好友请求
        
        DELETE /friends/requests/{request_id}
        """
        requester = await self.auth.get_user_by_req(request)
        user_id = requester.user.to_string()
        
        result = await self.friends_handler.cancel_friend_request(
            user_id, request_id
        )
        
        return 200, result


class FriendsListServlet(RestServlet):
    """好友列表API"""
    
    PATTERNS = client_patterns("/friends$", releases=("r0",))
    
    def __init__(self, hs: "HomeServer"):
        super().__init__()
        self.auth = hs.get_auth()
        self.friends_handler = hs.get_friends_handler()
    
    async def on_GET(self, request: SynapseRequest) -> Tuple[int, JsonDict]:
        """获取好友列表
        
        GET /friends?limit=100&offset=0
        """
        requester = await self.auth.get_user_by_req(request)
        user_id = requester.user.to_string()
        
        limit = parse_integer(request, "limit", default=100)
        offset = parse_integer(request, "offset", default=0)
        
        if limit > 200:
            limit = 200
        
        result = await self.friends_handler.get_friends_list(
            user_id, limit, offset
        )
        
        return 200, result


class FriendActionServlet(RestServlet):
    """好友操作API"""
    
    PATTERNS = client_patterns("/friends/(?P<friend_user_id>[^/]+)$", releases=("r0",))
    
    def __init__(self, hs: "HomeServer"):
        super().__init__()
        self.auth = hs.get_auth()
        self.friends_handler = hs.get_friends_handler()
    
    async def on_DELETE(self, request: SynapseRequest, friend_user_id: str) -> Tuple[int, JsonDict]:
        """删除好友
        
        DELETE /friends/{friend_user_id}
        """
        requester = await self.auth.get_user_by_req(request)
        user_id = requester.user.to_string()
        
        result = await self.friends_handler.remove_friend(
            user_id, friend_user_id
        )
        
        return 200, result
    
    async def on_GET(self, request: SynapseRequest, friend_user_id: str) -> Tuple[int, JsonDict]:
        """获取好友关系状态
        
        GET /friends/{friend_user_id}
        """
        requester = await self.auth.get_user_by_req(request)
        user_id = requester.user.to_string()
        
        status = await self.friends_handler.get_friendship_status(
            user_id, friend_user_id
        )
        
        return 200, {
            "user_id": friend_user_id,
            "status": status,
        }


class BlockedUsersServlet(RestServlet):
    """用户屏蔽API"""
    
    PATTERNS = client_patterns("/friends/blocked$", releases=("r0",))
    
    def __init__(self, hs: "HomeServer"):
        super().__init__()
        self.auth = hs.get_auth()
        self.friends_handler = hs.get_friends_handler()
    
    async def on_GET(self, request: SynapseRequest) -> Tuple[int, JsonDict]:
        """获取屏蔽用户列表
        
        GET /friends/blocked?limit=100
        """
        requester = await self.auth.get_user_by_req(request)
        user_id = requester.user.to_string()
        
        limit = parse_integer(request, "limit", default=100)
        if limit > 200:
            limit = 200
        
        result = await self.friends_handler.get_blocked_users(
            user_id, limit
        )
        
        return 200, result
    
    async def on_POST(self, request: SynapseRequest) -> Tuple[int, JsonDict]:
        """屏蔽用户
        
        POST /friends/blocked
        {
            "user_id": "@user:example.com",
            "reason": "骚扰"
        }
        """
        requester = await self.auth.get_user_by_req(request)
        user_id = requester.user.to_string()
        
        content = parse_json_object_from_request(request)
        
        blocked_user_id = content.get("user_id")
        if not blocked_user_id:
            raise SynapseError(400, "缺少用户ID", Codes.MISSING_PARAM)
        
        reason = content.get("reason")
        if reason and len(reason) > 200:
            raise SynapseError(400, "屏蔽原因长度不能超过200字符", Codes.INVALID_PARAM)
        
        result = await self.friends_handler.block_user(
            user_id, blocked_user_id, reason
        )
        
        return 200, result


class BlockedUserActionServlet(RestServlet):
    """屏蔽用户操作API"""
    
    PATTERNS = client_patterns("/friends/blocked/(?P<blocked_user_id>[^/]+)$", releases=("r0",))
    
    def __init__(self, hs: "HomeServer"):
        super().__init__()
        self.auth = hs.get_auth()
        self.friends_handler = hs.get_friends_handler()
    
    async def on_DELETE(self, request: SynapseRequest, blocked_user_id: str) -> Tuple[int, JsonDict]:
        """取消屏蔽用户
        
        DELETE /friends/blocked/{blocked_user_id}
        """
        requester = await self.auth.get_user_by_req(request)
        user_id = requester.user.to_string()
        
        result = await self.friends_handler.unblock_user(
            user_id, blocked_user_id
        )
        
        return 200, result


class UserSearchServlet(RestServlet):
    """用户搜索API"""
    
    PATTERNS = client_patterns("/friends/search$", releases=("r0",))
    
    def __init__(self, hs: "HomeServer"):
        super().__init__()
        self.auth = hs.get_auth()
        self.friends_handler = hs.get_friends_handler()
    
    async def on_GET(self, request: SynapseRequest) -> Tuple[int, JsonDict]:
        """搜索用户
        
        GET /friends/search?q=search_term&limit=20
        """
        requester = await self.auth.get_user_by_req(request)
        user_id = requester.user.to_string()
        
        search_term = parse_string(request, "q")
        if not search_term:
            raise SynapseError(400, "缺少搜索关键词", Codes.MISSING_PARAM)
        
        limit = parse_integer(request, "limit", default=20)
        if limit > 50:
            limit = 50
        
        result = await self.friends_handler.search_users(
            user_id, search_term, limit
        )
        
        return 200, result


def register_servlets(hs: "HomeServer", http_server) -> None:
    """注册好友功能相关的REST API端点"""
    FriendRequestServlet(hs).register(http_server)
    FriendRequestActionServlet(hs).register(http_server)
    FriendsListServlet(hs).register(http_server)
    FriendActionServlet(hs).register(http_server)
    BlockedUsersServlet(hs).register(http_server)
    BlockedUserActionServlet(hs).register(http_server)
    UserSearchServlet(hs).register(http_server)