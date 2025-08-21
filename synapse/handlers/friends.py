# -*- coding: utf-8 -*-
# 好友功能业务逻辑处理器
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
from typing import TYPE_CHECKING, Dict, List, Optional, Tuple, Any

from synapse.api.errors import (
    AuthError,
    Codes,
    NotFoundError,
    RequestSendFailed,
    StoreError,
    SynapseError,
)
# BaseHandler import removed - not needed in current Synapse version
from synapse.types import JsonDict, UserID
from synapse.util.async_helpers import Linearizer
from synapse.util.caches.descriptors import cached
from synapse.util.metrics import measure_func

if TYPE_CHECKING:
    from synapse.server import HomeServer

logger = logging.getLogger(__name__)


class FriendsHandler:
    """好友功能业务逻辑处理器"""

    def __init__(self, hs: "HomeServer"):
        self.store = hs.get_datastores().main
        self.auth = hs.get_auth()
        self.clock = hs.get_clock()
        self.server_name = hs.hostname
        
        # 用于防止并发操作的线性化器
        self._friend_request_linearizer = Linearizer(name="friend_request")
        self._friendship_linearizer = Linearizer(name="friendship")
        
        # 配置参数 - 使用默认值，因为这些配置可能不存在
        self.max_friends_per_user = 1000
        self.max_pending_requests = 50
        self.enable_friend_search = True

    @measure_func("friends.send_friend_request")
    async def send_friend_request(
        self,
        requester_user_id: str,
        target_user_id: str,
        message: Optional[str] = None,
    ) -> Dict[str, Any]:
        """发送好友请求
        
        Args:
            requester_user_id: 请求者用户ID
            target_user_id: 目标用户ID
            message: 请求消息
            
        Returns:
            包含请求ID和状态的字典
            
        Raises:
            SynapseError: 各种业务逻辑错误
        """
        # 验证用户ID格式
        if not UserID.is_valid(requester_user_id):
            raise SynapseError(400, "无效的请求者用户ID", Codes.INVALID_PARAM)
        
        if not UserID.is_valid(target_user_id):
            raise SynapseError(400, "无效的目标用户ID", Codes.INVALID_PARAM)
        
        # 不能向自己发送好友请求
        if requester_user_id == target_user_id:
            raise SynapseError(400, "不能向自己发送好友请求", Codes.INVALID_PARAM)
        
        # 检查目标用户是否存在
        target_user = await self.store.get_user_by_id(target_user_id)
        if not target_user:
            raise NotFoundError("目标用户不存在")
        
        # 使用线性化器防止并发请求
        async with self._friend_request_linearizer.queue(
            (requester_user_id, target_user_id)
        ):
            # 检查是否已经是好友
            friendship_status = await self.store.get_friendship_status(
                requester_user_id, target_user_id
            )
            if friendship_status == "active":
                raise SynapseError(400, "用户已经是好友", Codes.INVALID_PARAM)
            
            # 检查是否被屏蔽
            is_blocked = await self.store.is_user_blocked(
                target_user_id, requester_user_id
            )
            if is_blocked:
                raise SynapseError(403, "无法向该用户发送好友请求", Codes.FORBIDDEN)
            
            # 检查待处理请求数量限制
            pending_requests = await self.store.get_friend_requests(
                requester_user_id, "sent", self.max_pending_requests + 1
            )
            pending_count = len([r for r in pending_requests if r["status"] == "pending"])
            if pending_count >= self.max_pending_requests:
                raise SynapseError(
                    429, 
                    f"待处理的好友请求过多，最多允许{self.max_pending_requests}个",
                    Codes.LIMIT_EXCEEDED
                )
            
            # 检查好友数量限制
            friends_list = await self.store.get_friends_list(
                requester_user_id, self.max_friends_per_user + 1
            )
            if len(friends_list) >= self.max_friends_per_user:
                raise SynapseError(
                    429,
                    f"好友数量已达上限，最多允许{self.max_friends_per_user}个好友",
                    Codes.LIMIT_EXCEEDED
                )
            
            # 创建好友请求
            try:
                request_id = await self.store.create_friend_request(
                    requester_user_id, target_user_id, message
                )
                
                logger.info(
                    "用户 %s 向用户 %s 发送了好友请求，请求ID: %s",
                    requester_user_id,
                    target_user_id,
                    request_id,
                )
                
                return {
                    "request_id": request_id,
                    "status": "pending",
                    "message": "好友请求已发送",
                }
                
            except StoreError as e:
                if e.code == 400:
                    raise SynapseError(400, str(e), Codes.INVALID_PARAM)
                elif e.code == 403:
                    raise SynapseError(403, str(e), Codes.FORBIDDEN)
                else:
                    raise

    @measure_func("friends.respond_to_friend_request")
    async def respond_to_friend_request(
        self,
        user_id: str,
        request_id: str,
        response: str,
    ) -> Dict[str, Any]:
        """响应好友请求
        
        Args:
            user_id: 响应者用户ID
            request_id: 请求ID
            response: 响应类型 ('accept' 或 'reject')
            
        Returns:
            包含响应结果的字典
        """
        if response not in ("accept", "reject"):
            raise SynapseError(400, "无效的响应类型", Codes.INVALID_PARAM)
        
        status = "accepted" if response == "accept" else "rejected"
        
        async with self._friend_request_linearizer.queue(request_id):
            success = await self.store.update_friend_request_status(
                request_id, status, user_id
            )
            
            if not success:
                raise NotFoundError("好友请求不存在或无权限操作")
            
            action = "接受" if response == "accept" else "拒绝"
            logger.info(
                "用户 %s %s了好友请求，请求ID: %s",
                user_id,
                action,
                request_id,
            )
            
            return {
                "request_id": request_id,
                "status": status,
                "message": f"已{action}好友请求",
            }

    @measure_func("friends.cancel_friend_request")
    async def cancel_friend_request(
        self,
        user_id: str,
        request_id: str,
    ) -> Dict[str, Any]:
        """取消好友请求
        
        Args:
            user_id: 取消者用户ID
            request_id: 请求ID
            
        Returns:
            包含取消结果的字典
        """
        async with self._friend_request_linearizer.queue(request_id):
            success = await self.store.update_friend_request_status(
                request_id, "cancelled", user_id
            )
            
            if not success:
                raise NotFoundError("好友请求不存在或无权限操作")
            
            logger.info(
                "用户 %s 取消了好友请求，请求ID: %s",
                user_id,
                request_id,
            )
            
            return {
                "request_id": request_id,
                "status": "cancelled",
                "message": "已取消好友请求",
            }

    @measure_func("friends.get_friends_list")
    async def get_friends_list(
        self,
        user_id: str,
        limit: int = 100,
        offset: int = 0,
    ) -> Dict[str, Any]:
        """获取好友列表
        
        Args:
            user_id: 用户ID
            limit: 返回数量限制
            offset: 偏移量
            
        Returns:
            包含好友列表的字典
        """
        friends = await self.store.get_friends_list(user_id, limit, offset)
        
        # 获取好友的详细信息
        friends_with_info = []
        for friend in friends:
            friend_id = friend["friend_id"]
            user_info = await self.store.get_user_by_id(friend_id)
            
            if user_info:
                # Get profile information separately
                profile_info = await self.store.get_profileinfo(user_info.user_id)
                friends_with_info.append({
                    "user_id": friend_id,
                    "displayname": profile_info.display_name,
                    "avatar_url": profile_info.avatar_url,
                    "status": friend["status"],
                    "created_ts": friend["created_ts"],
                })
        
        return {
            "friends": friends_with_info,
            "total_count": len(friends_with_info),
            "has_more": len(friends) == limit,
        }

    @measure_func("friends.get_friend_requests")
    async def get_friend_requests(
        self,
        user_id: str,
        request_type: str = "received",
        limit: int = 50,
    ) -> Dict[str, Any]:
        """获取好友请求列表
        
        Args:
            user_id: 用户ID
            request_type: 请求类型 ('sent' 或 'received')
            limit: 返回数量限制
            
        Returns:
            包含好友请求列表的字典
        """
        if request_type not in ("sent", "received"):
            raise SynapseError(400, "无效的请求类型", Codes.INVALID_PARAM)
        
        requests = await self.store.get_friend_requests(user_id, request_type, limit)
        
        # 获取相关用户的详细信息
        requests_with_info = []
        for request in requests:
            other_user_id = (
                request["target_user_id"]
                if request_type == "sent"
                else request["sender_user_id"]
            )
            
            user_info = await self.store.get_user_by_id(other_user_id)
            
            if user_info:
                # Get profile information separately
                profile_info = await self.store.get_profileinfo(user_info.user_id)
                requests_with_info.append({
                    "request_id": request["request_id"],
                    "user_id": other_user_id,
                    "displayname": profile_info.display_name,
                    "avatar_url": profile_info.avatar_url,
                    "message": request["message"],
                    "status": request["status"],
                    "created_ts": request["created_ts"],
                    "updated_ts": request["updated_ts"],
                })
        
        return {
            "requests": requests_with_info,
            "total_count": len(requests_with_info),
            "request_type": request_type,
        }

    @measure_func("friends.remove_friend")
    async def remove_friend(
        self,
        user_id: str,
        friend_user_id: str,
    ) -> Dict[str, Any]:
        """删除好友
        
        Args:
            user_id: 用户ID
            friend_user_id: 好友用户ID
            
        Returns:
            包含删除结果的字典
        """
        if user_id == friend_user_id:
            raise SynapseError(400, "不能删除自己", Codes.INVALID_PARAM)
        
        async with self._friendship_linearizer.queue((user_id, friend_user_id)):
            # 检查是否是好友关系
            friendship_status = await self.store.get_friendship_status(
                user_id, friend_user_id
            )
            if friendship_status != "active":
                raise NotFoundError("好友关系不存在")
            
            success = await self.store.remove_friendship(user_id, friend_user_id)
            
            if success:
                logger.info(
                    "用户 %s 删除了好友 %s",
                    user_id,
                    friend_user_id,
                )
                
                return {
                    "message": "已删除好友",
                    "friend_user_id": friend_user_id,
                }
            else:
                raise SynapseError(500, "删除好友失败", Codes.UNKNOWN)

    @measure_func("friends.block_user")
    async def block_user(
        self,
        user_id: str,
        blocked_user_id: str,
        reason: Optional[str] = None,
    ) -> Dict[str, Any]:
        """屏蔽用户
        
        Args:
            user_id: 用户ID
            blocked_user_id: 被屏蔽用户ID
            reason: 屏蔽原因
            
        Returns:
            包含屏蔽结果的字典
        """
        if user_id == blocked_user_id:
            raise SynapseError(400, "不能屏蔽自己", Codes.INVALID_PARAM)
        
        # 检查被屏蔽用户是否存在
        blocked_user = await self.store.get_user_by_id(blocked_user_id)
        if not blocked_user:
            raise NotFoundError("用户不存在")
        
        async with self._friendship_linearizer.queue((user_id, blocked_user_id)):
            success = await self.store.block_user(user_id, blocked_user_id, reason)
            
            if success:
                logger.info(
                    "用户 %s 屏蔽了用户 %s，原因: %s",
                    user_id,
                    blocked_user_id,
                    reason or "未提供",
                )
                
                return {
                    "message": "已屏蔽用户",
                    "blocked_user_id": blocked_user_id,
                }
            else:
                raise SynapseError(500, "屏蔽用户失败", Codes.UNKNOWN)

    @measure_func("friends.unblock_user")
    async def unblock_user(
        self,
        user_id: str,
        blocked_user_id: str,
    ) -> Dict[str, Any]:
        """取消屏蔽用户
        
        Args:
            user_id: 用户ID
            blocked_user_id: 被屏蔽用户ID
            
        Returns:
            包含取消屏蔽结果的字典
        """
        async with self._friendship_linearizer.queue((user_id, blocked_user_id)):
            success = await self.store.unblock_user(user_id, blocked_user_id)
            
            if not success:
                raise NotFoundError("屏蔽关系不存在")
            
            logger.info(
                "用户 %s 取消屏蔽了用户 %s",
                user_id,
                blocked_user_id,
            )
            
            return {
                "message": "已取消屏蔽用户",
                "unblocked_user_id": blocked_user_id,
            }

    @measure_func("friends.get_blocked_users")
    async def get_blocked_users(
        self,
        user_id: str,
        limit: int = 100,
    ) -> Dict[str, Any]:
        """获取屏蔽用户列表
        
        Args:
            user_id: 用户ID
            limit: 返回数量限制
            
        Returns:
            包含屏蔽用户列表的字典
        """
        blocked_users = await self.store.get_blocked_users(user_id, limit)
        
        # 获取被屏蔽用户的详细信息
        blocked_users_with_info = []
        for blocked in blocked_users:
            blocked_user_id = blocked["blocked_user_id"]
            user_info = await self.store.get_user_by_id(blocked_user_id)
            
            if user_info:
                # Get profile information separately
                profile_info = await self.store.get_profileinfo(user_info.user_id)
                blocked_users_with_info.append({
                    "user_id": blocked_user_id,
                    "displayname": profile_info.display_name,
                    "avatar_url": profile_info.avatar_url,
                    "created_ts": blocked["created_ts"],
                    "reason": blocked["reason"],
                })
        
        return {
            "blocked_users": blocked_users_with_info,
            "total_count": len(blocked_users_with_info),
        }

    @measure_func("friends.search_users")
    async def search_users(
        self,
        user_id: str,
        search_term: str,
        limit: int = 20,
    ) -> Dict[str, Any]:
        """搜索用户（用于添加好友）
        
        Args:
            user_id: 搜索者用户ID
            search_term: 搜索关键词
            limit: 返回数量限制
            
        Returns:
            包含搜索结果的字典
        """
        if not self.enable_friend_search:
            raise SynapseError(403, "用户搜索功能已禁用", Codes.FORBIDDEN)
        
        if not search_term or len(search_term.strip()) < 2:
            raise SynapseError(400, "搜索关键词至少需要2个字符", Codes.INVALID_PARAM)
        
        search_term = search_term.strip()
        
        # 限制搜索频率（可以考虑添加速率限制）
        users = await self.store.search_users(search_term)
        
        return {
            "users": users,
            "search_term": search_term,
            "total_count": len(users),
        }

    async def get_friendship_status(
        self,
        user1_id: str,
        user2_id: str,
    ) -> Optional[str]:
        """获取两个用户之间的好友关系状态
        
        Args:
            user1_id: 用户1的ID
            user2_id: 用户2的ID
            
        Returns:
            好友关系状态或None
        """
        return await self.store.get_friendship_status(user1_id, user2_id)