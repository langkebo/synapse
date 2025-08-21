# -*- coding: utf-8 -*-
# 好友功能数据存储层实现
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
import uuid

from synapse.storage._base import SQLBaseStore
from synapse.storage.database import DatabasePool, LoggingDatabaseConnection
from synapse.storage.engines import BaseDatabaseEngine
from synapse.util.caches.descriptors import cached
from synapse.api.errors import StoreError

if TYPE_CHECKING:
    from synapse.server import HomeServer

logger = logging.getLogger(__name__)


class FriendsWorkerStore(SQLBaseStore):
    """好友功能的只读数据存储层"""

    def __init__(
        self,
        database: DatabasePool,
        db_conn: LoggingDatabaseConnection,
        hs: "HomeServer",
    ) -> None:
        super().__init__(database, db_conn, hs)

    @cached()
    async def get_friendship_status(
        self, user1_id: str, user2_id: str
    ) -> Optional[str]:
        """获取两个用户之间的好友关系状态
        
        Args:
            user1_id: 用户1的ID
            user2_id: 用户2的ID
            
        Returns:
            好友关系状态: 'active', 'blocked' 或 None（无关系）
        """
        def _get_friendship_status_txn(txn) -> Optional[str]:
            # 查询双向好友关系
            sql = """
                SELECT status FROM user_friendships 
                WHERE (user1_id = ? AND user2_id = ?) 
                   OR (user1_id = ? AND user2_id = ?)
                LIMIT 1
            """
            txn.execute(sql, (user1_id, user2_id, user2_id, user1_id))
            row = txn.fetchone()
            return row[0] if row else None

        return await self.db_pool.runInteraction(
            "get_friendship_status", _get_friendship_status_txn
        )

    async def get_friends_list(
        self, user_id: str, limit: int = 100, offset: int = 0
    ) -> List[Dict[str, Any]]:
        """获取用户的好友列表
        
        Args:
            user_id: 用户ID
            limit: 返回数量限制
            offset: 偏移量
            
        Returns:
            好友列表，包含好友信息和关系状态
        """
        def _get_friends_list_txn(txn) -> List[Dict[str, Any]]:
            sql = """
                SELECT 
                    CASE 
                        WHEN uf.user1_id = ? THEN uf.user2_id 
                        ELSE uf.user1_id 
                    END as friend_id,
                    uf.status,
                    uf.created_ts,
                    uf.updated_ts
                FROM user_friendships uf
                WHERE (uf.user1_id = ? OR uf.user2_id = ?)
                  AND uf.status = 'active'
                ORDER BY uf.created_ts DESC
                LIMIT ? OFFSET ?
            """
            txn.execute(sql, (user_id, user_id, user_id, limit, offset))
            return [
                {
                    "friend_id": row[0],
                    "status": row[1],
                    "created_ts": row[2],
                    "updated_ts": row[3],
                }
                for row in txn.fetchall()
            ]

        return await self.db_pool.runInteraction(
            "get_friends_list", _get_friends_list_txn
        )

    async def get_friend_requests(
        self, user_id: str, request_type: str = "received", limit: int = 50
    ) -> List[Dict[str, Any]]:
        """获取好友请求列表
        
        Args:
            user_id: 用户ID
            request_type: 请求类型 'sent'(发送的) 或 'received'(接收的)
            limit: 返回数量限制
            
        Returns:
            好友请求列表
        """
        def _get_friend_requests_txn(txn) -> List[Dict[str, Any]]:
            if request_type == "sent":
                sql = """
                    SELECT request_id, sender_user_id, target_user_id, 
                           message, status, created_ts, updated_ts
                    FROM friend_requests 
                    WHERE sender_user_id = ? AND status IN ('pending', 'accepted', 'rejected')
                    ORDER BY created_ts DESC
                    LIMIT ?
                """
                txn.execute(sql, (user_id, limit))
            else:  # received
                sql = """
                    SELECT request_id, sender_user_id, target_user_id, 
                           message, status, created_ts, updated_ts
                    FROM friend_requests 
                    WHERE target_user_id = ? AND status = 'pending'
                    ORDER BY created_ts DESC
                    LIMIT ?
                """
                txn.execute(sql, (user_id, limit))
            
            return [
                {
                    "request_id": row[0],
                    "sender_user_id": row[1],
                    "target_user_id": row[2],
                    "message": row[3],
                    "status": row[4],
                    "created_ts": row[5],
                    "updated_ts": row[6],
                }
                for row in txn.fetchall()
            ]

        return await self.db_pool.runInteraction(
            "get_friend_requests", _get_friend_requests_txn
        )

    async def is_user_blocked(
        self, blocker_user_id: str, blocked_user_id: str
    ) -> bool:
        """检查用户是否被屏蔽
        
        Args:
            blocker_user_id: 屏蔽者用户ID
            blocked_user_id: 被屏蔽者用户ID
            
        Returns:
            是否被屏蔽
        """
        def _is_user_blocked_txn(txn) -> bool:
            sql = """
                SELECT 1 FROM user_blocks 
                WHERE blocker_user_id = ? AND blocked_user_id = ?
            """
            txn.execute(sql, (blocker_user_id, blocked_user_id))
            return txn.fetchone() is not None

        return await self.db_pool.runInteraction(
            "is_user_blocked", _is_user_blocked_txn
        )

    async def get_blocked_users(
        self, user_id: str, limit: int = 100
    ) -> List[Dict[str, Any]]:
        """获取用户屏蔽的用户列表
        
        Args:
            user_id: 用户ID
            limit: 返回数量限制
            
        Returns:
            被屏蔽用户列表
        """
        def _get_blocked_users_txn(txn) -> List[Dict[str, Any]]:
            sql = """
                SELECT blocked_user_id, created_ts, reason
                FROM user_blocks 
                WHERE blocker_user_id = ?
                ORDER BY created_ts DESC
                LIMIT ?
            """
            txn.execute(sql, (user_id, limit))
            return [
                {
                    "blocked_user_id": row[0],
                    "created_ts": row[1],
                    "reason": row[2],
                }
                for row in txn.fetchall()
            ]

        return await self.db_pool.runInteraction(
            "get_blocked_users", _get_blocked_users_txn
        )


class FriendsStore(FriendsWorkerStore):
    """好友功能的完整数据存储层（包含写操作）"""

    async def create_friend_request(
        self,
        sender_user_id: str,
        target_user_id: str,
        message: Optional[str] = None,
    ) -> str:
        """创建好友请求
        
        Args:
            sender_user_id: 发送者用户ID
            target_user_id: 目标用户ID
            message: 请求消息
            
        Returns:
            请求ID
            
        Raises:
            StoreError: 如果请求已存在或用户被屏蔽
        """
        # 检查是否已经是好友
        friendship_status = await self.get_friendship_status(
            sender_user_id, target_user_id
        )
        if friendship_status == "active":
            raise StoreError(400, "用户已经是好友")
        
        # 检查是否被屏蔽
        is_blocked = await self.is_user_blocked(target_user_id, sender_user_id)
        if is_blocked:
            raise StoreError(403, "无法向该用户发送好友请求")
        
        request_id = str(uuid.uuid4())
        current_ts = self.clock.time_msec()
        
        def _create_friend_request_txn(txn) -> None:
            # 检查是否已有待处理的请求
            sql = """
                SELECT request_id FROM friend_requests 
                WHERE sender_user_id = ? AND target_user_id = ? AND status = 'pending'
            """
            txn.execute(sql, (sender_user_id, target_user_id))
            if txn.fetchone():
                raise StoreError(400, "已有待处理的好友请求")
            
            # 插入新请求
            sql = """
                INSERT INTO friend_requests 
                (request_id, sender_user_id, target_user_id, message, status, created_ts, updated_ts)
                VALUES (?, ?, ?, ?, 'pending', ?, ?)
            """
            txn.execute(
                sql,
                (request_id, sender_user_id, target_user_id, message, current_ts, current_ts),
            )

        await self.db_pool.runInteraction(
            "create_friend_request", _create_friend_request_txn
        )
        
        return request_id

    async def update_friend_request_status(
        self, request_id: str, status: str, user_id: str
    ) -> bool:
        """更新好友请求状态
        
        Args:
            request_id: 请求ID
            status: 新状态 ('accepted', 'rejected', 'cancelled')
            user_id: 操作用户ID
            
        Returns:
            是否更新成功
        """
        current_ts = self.clock.time_msec()
        
        def _update_friend_request_status_txn(txn) -> bool:
            # 获取请求信息
            sql = """
                SELECT sender_user_id, target_user_id, status 
                FROM friend_requests 
                WHERE request_id = ?
            """
            txn.execute(sql, (request_id,))
            row = txn.fetchone()
            if not row:
                return False
            
            sender_id, target_id, current_status = row
            
            # 验证权限
            if status in ("accepted", "rejected") and user_id != target_id:
                return False
            if status == "cancelled" and user_id != sender_id:
                return False
            
            if current_status != "pending":
                return False
            
            # 更新请求状态
            sql = """
                UPDATE friend_requests 
                SET status = ?, updated_ts = ?
                WHERE request_id = ?
            """
            txn.execute(sql, (status, current_ts, request_id))
            
            # 如果接受请求，创建好友关系
            if status == "accepted":
                self._create_friendship_txn(txn, sender_id, target_id, current_ts)
            
            return True

        return await self.db_pool.runInteraction(
            "update_friend_request_status", _update_friend_request_status_txn
        )

    def _create_friendship_txn(
        self, txn, user1_id: str, user2_id: str, timestamp: int
    ) -> None:
        """在事务中创建好友关系"""
        # 确保user1_id < user2_id，保持一致性
        if user1_id > user2_id:
            user1_id, user2_id = user2_id, user1_id
        
        sql = """
            INSERT OR REPLACE INTO user_friendships 
            (user1_id, user2_id, status, created_ts, updated_ts)
            VALUES (?, ?, 'active', ?, ?)
        """
        txn.execute(sql, (user1_id, user2_id, timestamp, timestamp))
        
        # 清除缓存
        self.get_friendship_status.invalidate((user1_id, user2_id))
        self.get_friendship_status.invalidate((user2_id, user1_id))

    async def remove_friendship(
        self, user1_id: str, user2_id: str
    ) -> bool:
        """删除好友关系
        
        Args:
            user1_id: 用户1的ID
            user2_id: 用户2的ID
            
        Returns:
            是否删除成功
        """
        def _remove_friendship_txn(txn) -> bool:
            sql = """
                DELETE FROM user_friendships 
                WHERE (user1_id = ? AND user2_id = ?) 
                   OR (user1_id = ? AND user2_id = ?)
            """
            txn.execute(sql, (user1_id, user2_id, user2_id, user1_id))
            
            # 清除缓存
            self.get_friendship_status.invalidate((user1_id, user2_id))
            self.get_friendship_status.invalidate((user2_id, user1_id))
            
            return txn.rowcount > 0

        return await self.db_pool.runInteraction(
            "remove_friendship", _remove_friendship_txn
        )

    async def block_user(
        self, blocker_user_id: str, blocked_user_id: str, reason: Optional[str] = None
    ) -> bool:
        """屏蔽用户
        
        Args:
            blocker_user_id: 屏蔽者用户ID
            blocked_user_id: 被屏蔽者用户ID
            reason: 屏蔽原因
            
        Returns:
            是否屏蔽成功
        """
        current_ts = self.clock.time_msec()
        
        def _block_user_txn(txn) -> bool:
            # 先删除好友关系（如果存在）
            sql = """
                DELETE FROM user_friendships 
                WHERE (user1_id = ? AND user2_id = ?) 
                   OR (user1_id = ? AND user2_id = ?)
            """
            txn.execute(sql, (blocker_user_id, blocked_user_id, blocked_user_id, blocker_user_id))
            
            # 添加屏蔽关系
            sql = """
                INSERT OR REPLACE INTO user_blocks 
                (blocker_user_id, blocked_user_id, created_ts, reason)
                VALUES (?, ?, ?, ?)
            """
            txn.execute(sql, (blocker_user_id, blocked_user_id, current_ts, reason))
            
            # 清除缓存
            self.get_friendship_status.invalidate((blocker_user_id, blocked_user_id))
            self.get_friendship_status.invalidate((blocked_user_id, blocker_user_id))
            
            return True

        return await self.db_pool.runInteraction(
            "block_user", _block_user_txn
        )

    async def unblock_user(
        self, blocker_user_id: str, blocked_user_id: str
    ) -> bool:
        """取消屏蔽用户
        
        Args:
            blocker_user_id: 屏蔽者用户ID
            blocked_user_id: 被屏蔽者用户ID
            
        Returns:
            是否取消屏蔽成功
        """
        def _unblock_user_txn(txn) -> bool:
            sql = """
                DELETE FROM user_blocks 
                WHERE blocker_user_id = ? AND blocked_user_id = ?
            """
            txn.execute(sql, (blocker_user_id, blocked_user_id))
            return txn.rowcount > 0

        return await self.db_pool.runInteraction(
            "unblock_user", _unblock_user_txn
        )

    async def search_users(
        self, 
        searcher_user_id: str,
        search_term: str, 
        limit: int = 20
    ) -> List[Dict[str, Any]]:
        """搜索用户（用于添加好友）
        
        Args:
            searcher_user_id: 搜索者用户ID
            search_term: 搜索关键词
            limit: 返回数量限制
            
        Returns:
            用户搜索结果列表
        """
        def _search_users_txn(txn) -> List[Dict[str, Any]]:
            # 搜索用户名包含关键词的用户，排除已屏蔽和已是好友的用户
            sql = """
                SELECT u.name as user_id, u.displayname, u.avatar_url
                FROM users u
                WHERE (u.name LIKE ? OR u.displayname LIKE ?)
                  AND u.name != ?
                  AND u.name NOT IN (
                      SELECT blocked_user_id FROM user_blocks 
                      WHERE blocker_user_id = ?
                  )
                  AND u.name NOT IN (
                      SELECT CASE 
                          WHEN uf.user1_id = ? THEN uf.user2_id 
                          ELSE uf.user1_id 
                      END
                      FROM user_friendships uf
                      WHERE (uf.user1_id = ? OR uf.user2_id = ?)
                        AND uf.status = 'active'
                  )
                ORDER BY 
                    CASE WHEN u.name = ? THEN 0 ELSE 1 END,
                    CASE WHEN u.displayname = ? THEN 0 ELSE 1 END,
                    u.name
                LIMIT ?
            """
            search_pattern = f"%{search_term}%"
            txn.execute(
                sql,
                (
                    search_pattern, search_pattern, searcher_user_id,
                    searcher_user_id, searcher_user_id, searcher_user_id, searcher_user_id,
                    search_term, search_term, limit
                ),
            )
            
            return [
                {
                    "user_id": row[0],
                    "displayname": row[1],
                    "avatar_url": row[2],
                }
                for row in txn.fetchall()
            ]

        return await self.db_pool.runInteraction(
            "search_users", _search_users_txn
        )