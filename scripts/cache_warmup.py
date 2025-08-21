#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Synapse 缓存预热脚本
Synapse Cache Warmup Script

用于在 Synapse 启动时预热关键缓存数据，包括：
- 用户资料缓存
- 好友关系缓存
- 房间状态缓存
- 常用查询结果缓存

Used to warm up critical cache data when Synapse starts, including:
- User profile cache
- Friends relationship cache
- Room state cache
- Common query result cache
"""

import argparse
import asyncio
import json
import logging
import sys
import time
from datetime import datetime
from typing import Dict, List, Optional, Any, Tuple

import redis
import psycopg2
from psycopg2.extras import RealDictCursor

# 配置日志
# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
    handlers=[
        logging.FileHandler('/var/log/synapse/cache_warmup.log'),
        logging.StreamHandler(sys.stdout)
    ]
)
logger = logging.getLogger(__name__)


class CacheWarmup:
    """缓存预热器类
    
    Cache warmup class
    """
    
    def __init__(self, config_path: str = "/data/homeserver.yaml"):
        """初始化缓存预热器
        
        Initialize cache warmup
        
        Args:
            config_path: Synapse 配置文件路径 (Synapse config file path)
        """
        self.config_path = config_path
        self.config = self._load_config()
        self.redis_client = None
        self.db_connection = None
        self.warmup_stats = {
            'start_time': None,
            'end_time': None,
            'total_keys_warmed': 0,
            'strategies_executed': [],
            'errors': []
        }
        
    def _load_config(self) -> Dict[str, Any]:
        """加载 Synapse 配置文件
        
        Load Synapse configuration file
        """
        try:
            import yaml
            with open(self.config_path, 'r', encoding='utf-8') as f:
                return yaml.safe_load(f)
        except Exception as e:
            logger.error(f"加载配置文件失败 (Failed to load config file): {e}")
            return {}
    
    async def initialize_connections(self):
        """初始化数据库和 Redis 连接
        
        Initialize database and Redis connections
        """
        try:
            # 初始化 Redis 连接
            # Initialize Redis connection
            redis_config = self.config.get('cache_strategy', {}).get('redis', {})
            if redis_config.get('enabled', False):
                self.redis_client = redis.Redis(
                    host=redis_config.get('host', 'localhost'),
                    port=redis_config.get('port', 6379),
                    password=redis_config.get('password'),
                    db=redis_config.get('db', 0),
                    decode_responses=True
                )
                # 测试连接
                # Test connection
                self.redis_client.ping()
                logger.info("Redis 连接成功 (Redis connection successful)")
            else:
                logger.warning("Redis 未启用，跳过缓存预热 (Redis not enabled, skipping cache warmup)")
                return False
            
            # 初始化数据库连接
            # Initialize database connection
            db_config = self.config.get('database', {}).get('args', {})
            if db_config:
                self.db_connection = psycopg2.connect(
                    host=db_config.get('host', 'localhost'),
                    port=db_config.get('port', 5432),
                    database=db_config.get('database', 'synapse'),
                    user=db_config.get('user', 'synapse_user'),
                    password=db_config.get('password', ''),
                    cursor_factory=RealDictCursor
                )
                logger.info("数据库连接成功 (Database connection successful)")
            else:
                logger.error("数据库配置未找到 (Database configuration not found)")
                return False
                
            return True
                
        except Exception as e:
            logger.error(f"初始化连接失败 (Failed to initialize connections): {e}")
            return False
    
    async def warmup_user_profiles(self, batch_size: int = 100) -> int:
        """预热用户资料缓存
        
        Warm up user profile cache
        
        Args:
            batch_size: 批处理大小 (Batch size)
            
        Returns:
            预热的缓存键数量 (Number of cache keys warmed)
        """
        logger.info("开始预热用户资料缓存 (Starting to warm up user profile cache)")
        
        try:
            with self.db_connection.cursor() as cursor:
                # 获取活跃用户列表
                # Get active users list
                cursor.execute("""
                    SELECT user_id, displayname, avatar_url, creation_ts
                    FROM users
                    WHERE deactivated = 0
                    ORDER BY creation_ts DESC
                    LIMIT %s
                """, (batch_size * 10,))
                
                users = cursor.fetchall()
                keys_warmed = 0
                
                # 批量预热用户资料
                # Batch warm up user profiles
                for i in range(0, len(users), batch_size):
                    batch = users[i:i + batch_size]
                    pipe = self.redis_client.pipeline()
                    
                    for user in batch:
                        user_id = user['user_id']
                        profile_data = {
                            'displayname': user['displayname'],
                            'avatar_url': user['avatar_url'],
                            'creation_ts': user['creation_ts']
                        }
                        
                        # 缓存用户资料
                        # Cache user profile
                        cache_key = f"user_profile:{user_id}"
                        pipe.setex(cache_key, 3600, json.dumps(profile_data, default=str))
                        keys_warmed += 1
                    
                    pipe.execute()
                    
                    # 避免过载
                    # Avoid overload
                    await asyncio.sleep(0.1)
                
                logger.info(f"用户资料缓存预热完成，预热了 {keys_warmed} 个键 (User profile cache warmup completed, warmed {keys_warmed} keys)")
                return keys_warmed
                
        except Exception as e:
            logger.error(f"预热用户资料缓存失败 (Failed to warm up user profile cache): {e}")
            self.warmup_stats['errors'].append(f"用户资料缓存预热失败: {e}")
            return 0
    
    async def warmup_friends_relationships(self, batch_size: int = 100) -> int:
        """预热好友关系缓存
        
        Warm up friends relationship cache
        
        Args:
            batch_size: 批处理大小 (Batch size)
            
        Returns:
            预热的缓存键数量 (Number of cache keys warmed)
        """
        logger.info("开始预热好友关系缓存 (Starting to warm up friends relationship cache)")
        
        try:
            with self.db_connection.cursor() as cursor:
                # 获取有好友的用户列表
                # Get users with friends list
                cursor.execute("""
                    SELECT DISTINCT user_id
                    FROM friends
                    ORDER BY created_at DESC
                    LIMIT %s
                """, (batch_size * 5,))
                
                users_with_friends = cursor.fetchall()
                keys_warmed = 0
                
                # 批量预热好友关系
                # Batch warm up friends relationships
                for i in range(0, len(users_with_friends), batch_size):
                    batch = users_with_friends[i:i + batch_size]
                    
                    for user_row in batch:
                        user_id = user_row['user_id']
                        
                        # 获取用户的好友列表
                        # Get user's friends list
                        cursor.execute("""
                            SELECT friend_id, status, created_at
                            FROM friends
                            WHERE user_id = %s AND status = 'accepted'
                        """, (user_id,))
                        
                        friends = cursor.fetchall()
                        
                        if friends:
                            friends_data = [
                                {
                                    'friend_id': friend['friend_id'],
                                    'status': friend['status'],
                                    'created_at': friend['created_at']
                                }
                                for friend in friends
                            ]
                            
                            # 缓存好友关系
                            # Cache friends relationship
                            cache_key = f"friends:relationships:{user_id}"
                            self.redis_client.setex(
                                cache_key, 
                                3600, 
                                json.dumps(friends_data, default=str)
                            )
                            keys_warmed += 1
                    
                    # 避免过载
                    # Avoid overload
                    await asyncio.sleep(0.1)
                
                logger.info(f"好友关系缓存预热完成，预热了 {keys_warmed} 个键 (Friends relationship cache warmup completed, warmed {keys_warmed} keys)")
                return keys_warmed
                
        except Exception as e:
            logger.error(f"预热好友关系缓存失败 (Failed to warm up friends relationship cache): {e}")
            self.warmup_stats['errors'].append(f"好友关系缓存预热失败: {e}")
            return 0
    
    async def warmup_friends_requests(self, batch_size: int = 100) -> int:
        """预热好友请求缓存
        
        Warm up friends request cache
        
        Args:
            batch_size: 批处理大小 (Batch size)
            
        Returns:
            预热的缓存键数量 (Number of cache keys warmed)
        """
        logger.info("开始预热好友请求缓存 (Starting to warm up friends request cache)")
        
        try:
            with self.db_connection.cursor() as cursor:
                # 获取有待处理好友请求的用户
                # Get users with pending friend requests
                cursor.execute("""
                    SELECT DISTINCT target_user_id as user_id
                    FROM friend_requests
                    WHERE status = 'pending'
                    ORDER BY created_at DESC
                    LIMIT %s
                """, (batch_size * 3,))
                
                users_with_requests = cursor.fetchall()
                keys_warmed = 0
                
                # 批量预热好友请求
                # Batch warm up friends requests
                for i in range(0, len(users_with_requests), batch_size):
                    batch = users_with_requests[i:i + batch_size]
                    
                    for user_row in batch:
                        user_id = user_row['user_id']
                        
                        # 获取用户的待处理好友请求
                        # Get user's pending friend requests
                        cursor.execute("""
                            SELECT requester_user_id, message, created_at
                            FROM friend_requests
                            WHERE target_user_id = %s AND status = 'pending'
                            ORDER BY created_at DESC
                        """, (user_id,))
                        
                        requests = cursor.fetchall()
                        
                        if requests:
                            requests_data = [
                                {
                                    'requester_user_id': req['requester_user_id'],
                                    'message': req['message'],
                                    'created_at': req['created_at']
                                }
                                for req in requests
                            ]
                            
                            # 缓存好友请求
                            # Cache friends requests
                            cache_key = f"friends:requests:{user_id}"
                            self.redis_client.setex(
                                cache_key, 
                                1800, 
                                json.dumps(requests_data, default=str)
                            )
                            keys_warmed += 1
                    
                    # 避免过载
                    # Avoid overload
                    await asyncio.sleep(0.1)
                
                logger.info(f"好友请求缓存预热完成，预热了 {keys_warmed} 个键 (Friends request cache warmup completed, warmed {keys_warmed} keys)")
                return keys_warmed
                
        except Exception as e:
            logger.error(f"预热好友请求缓存失败 (Failed to warm up friends request cache): {e}")
            self.warmup_stats['errors'].append(f"好友请求缓存预热失败: {e}")
            return 0
    
    async def warmup_room_states(self, batch_size: int = 50) -> int:
        """预热房间状态缓存
        
        Warm up room state cache
        
        Args:
            batch_size: 批处理大小 (Batch size)
            
        Returns:
            预热的缓存键数量 (Number of cache keys warmed)
        """
        logger.info("开始预热房间状态缓存 (Starting to warm up room state cache)")
        
        try:
            with self.db_connection.cursor() as cursor:
                # 获取活跃房间列表
                # Get active rooms list
                cursor.execute("""
                    SELECT DISTINCT room_id
                    FROM current_state_events
                    WHERE type = 'm.room.member'
                    ORDER BY stream_ordering DESC
                    LIMIT %s
                """, (batch_size * 2,))
                
                active_rooms = cursor.fetchall()
                keys_warmed = 0
                
                # 批量预热房间状态
                # Batch warm up room states
                for i in range(0, len(active_rooms), batch_size):
                    batch = active_rooms[i:i + batch_size]
                    
                    for room_row in batch:
                        room_id = room_row['room_id']
                        
                        # 获取房间成员数
                        # Get room member count
                        cursor.execute("""
                            SELECT count(*) as member_count
                            FROM current_state_events
                            WHERE room_id = %s AND type = 'm.room.member'
                            AND membership = 'join'
                        """, (room_id,))
                        
                        member_count_result = cursor.fetchone()
                        member_count = member_count_result['member_count'] if member_count_result else 0
                        
                        # 获取房间名称和主题
                        # Get room name and topic
                        cursor.execute("""
                            SELECT type, content
                            FROM current_state_events
                            WHERE room_id = %s AND type IN ('m.room.name', 'm.room.topic')
                        """, (room_id,))
                        
                        room_info = cursor.fetchall()
                        
                        room_data = {
                            'room_id': room_id,
                            'member_count': member_count,
                            'name': None,
                            'topic': None
                        }
                        
                        for info in room_info:
                            if info['type'] == 'm.room.name':
                                content = json.loads(info['content']) if info['content'] else {}
                                room_data['name'] = content.get('name')
                            elif info['type'] == 'm.room.topic':
                                content = json.loads(info['content']) if info['content'] else {}
                                room_data['topic'] = content.get('topic')
                        
                        # 缓存房间状态
                        # Cache room state
                        cache_key = f"room:state:{room_id}"
                        self.redis_client.setex(
                            cache_key, 
                            1800, 
                            json.dumps(room_data, default=str)
                        )
                        keys_warmed += 1
                    
                    # 避免过载
                    # Avoid overload
                    await asyncio.sleep(0.1)
                
                logger.info(f"房间状态缓存预热完成，预热了 {keys_warmed} 个键 (Room state cache warmup completed, warmed {keys_warmed} keys)")
                return keys_warmed
                
        except Exception as e:
            logger.error(f"预热房间状态缓存失败 (Failed to warm up room state cache): {e}")
            self.warmup_stats['errors'].append(f"房间状态缓存预热失败: {e}")
            return 0
    
    async def warmup_friends_presence(self, batch_size: int = 100) -> int:
        """预热好友在线状态缓存
        
        Warm up friends presence cache
        
        Args:
            batch_size: 批处理大小 (Batch size)
            
        Returns:
            预热的缓存键数量 (Number of cache keys warmed)
        """
        logger.info("开始预热好友在线状态缓存 (Starting to warm up friends presence cache)")
        
        try:
            with self.db_connection.cursor() as cursor:
                # 获取最近活跃的用户
                # Get recently active users
                cursor.execute("""
                    SELECT user_id, last_seen, currently_active
                    FROM user_ips
                    WHERE last_seen > extract(epoch from now() - interval '1 day') * 1000
                    GROUP BY user_id, last_seen, currently_active
                    ORDER BY last_seen DESC
                    LIMIT %s
                """, (batch_size * 5,))
                
                active_users = cursor.fetchall()
                keys_warmed = 0
                
                # 批量预热在线状态
                # Batch warm up presence
                for i in range(0, len(active_users), batch_size):
                    batch = active_users[i:i + batch_size]
                    pipe = self.redis_client.pipeline()
                    
                    for user in batch:
                        user_id = user['user_id']
                        presence_data = {
                            'user_id': user_id,
                            'last_seen': user['last_seen'],
                            'currently_active': user['currently_active'],
                            'status': 'online' if user['currently_active'] else 'offline'
                        }
                        
                        # 缓存在线状态
                        # Cache presence
                        cache_key = f"friends:presence:{user_id}"
                        pipe.setex(cache_key, 300, json.dumps(presence_data, default=str))
                        keys_warmed += 1
                    
                    pipe.execute()
                    
                    # 避免过载
                    # Avoid overload
                    await asyncio.sleep(0.1)
                
                logger.info(f"好友在线状态缓存预热完成，预热了 {keys_warmed} 个键 (Friends presence cache warmup completed, warmed {keys_warmed} keys)")
                return keys_warmed
                
        except Exception as e:
            logger.error(f"预热好友在线状态缓存失败 (Failed to warm up friends presence cache): {e}")
            self.warmup_stats['errors'].append(f"好友在线状态缓存预热失败: {e}")
            return 0
    
    async def execute_warmup_strategies(self, strategies: List[str], batch_size: int = 100):
        """执行缓存预热策略
        
        Execute cache warmup strategies
        
        Args:
            strategies: 预热策略列表 (Warmup strategies list)
            batch_size: 批处理大小 (Batch size)
        """
        self.warmup_stats['start_time'] = datetime.now()
        
        strategy_map = {
            'user_profiles': self.warmup_user_profiles,
            'friends_relationships': self.warmup_friends_relationships,
            'friends_requests': self.warmup_friends_requests,
            'room_states': self.warmup_room_states,
            'friends_presence': self.warmup_friends_presence
        }
        
        for strategy in strategies:
            if strategy in strategy_map:
                logger.info(f"执行预热策略: {strategy} (Executing warmup strategy: {strategy})")
                try:
                    keys_warmed = await strategy_map[strategy](batch_size)
                    self.warmup_stats['total_keys_warmed'] += keys_warmed
                    self.warmup_stats['strategies_executed'].append({
                        'strategy': strategy,
                        'keys_warmed': keys_warmed,
                        'success': True
                    })
                except Exception as e:
                    logger.error(f"预热策略 {strategy} 执行失败 (Warmup strategy {strategy} failed): {e}")
                    self.warmup_stats['strategies_executed'].append({
                        'strategy': strategy,
                        'keys_warmed': 0,
                        'success': False,
                        'error': str(e)
                    })
            else:
                logger.warning(f"未知的预热策略: {strategy} (Unknown warmup strategy: {strategy})")
        
        self.warmup_stats['end_time'] = datetime.now()
        duration = (self.warmup_stats['end_time'] - self.warmup_stats['start_time']).total_seconds()
        
        logger.info(f"缓存预热完成，总共预热了 {self.warmup_stats['total_keys_warmed']} 个键，耗时 {duration:.2f} 秒 (Cache warmup completed, warmed {self.warmup_stats['total_keys_warmed']} keys in {duration:.2f} seconds)")
    
    def print_warmup_summary(self):
        """打印预热摘要
        
        Print warmup summary
        """
        print("\n" + "="*60)
        print("Synapse 缓存预热报告 (Synapse Cache Warmup Report)")
        print("="*60)
        
        if self.warmup_stats['start_time'] and self.warmup_stats['end_time']:
            duration = (self.warmup_stats['end_time'] - self.warmup_stats['start_time']).total_seconds()
            print(f"开始时间 (Start Time): {self.warmup_stats['start_time']}")
            print(f"结束时间 (End Time): {self.warmup_stats['end_time']}")
            print(f"总耗时 (Total Duration): {duration:.2f} 秒 (seconds)")
        
        print(f"总预热键数 (Total Keys Warmed): {self.warmup_stats['total_keys_warmed']}")
        
        print(f"\n执行的策略 (Executed Strategies):")
        for strategy in self.warmup_stats['strategies_executed']:
            status = "✅ 成功" if strategy['success'] else "❌ 失败"
            print(f"  {strategy['strategy']}: {status} ({strategy['keys_warmed']} 键)")
            if not strategy['success'] and 'error' in strategy:
                print(f"    错误 (Error): {strategy['error']}")
        
        if self.warmup_stats['errors']:
            print(f"\n错误信息 (Errors):")
            for error in self.warmup_stats['errors']:
                print(f"  ❌ {error}")
        
        print("="*60)
    
    async def cleanup(self):
        """清理资源
        
        Cleanup resources
        """
        if self.redis_client:
            self.redis_client.close()
        if self.db_connection:
            self.db_connection.close()


async def main():
    """主函数
    
    Main function
    """
    parser = argparse.ArgumentParser(description='Synapse 缓存预热脚本 (Synapse Cache Warmup Script)')
    parser.add_argument('--config', '-c', default='/data/homeserver.yaml',
                       help='Synapse 配置文件路径 (Synapse config file path)')
    parser.add_argument('--strategies', '-s', nargs='+', 
                       default=['user_profiles', 'friends_relationships', 'friends_requests', 'friends_presence'],
                       help='预热策略列表 (Warmup strategies list)')
    parser.add_argument('--batch-size', '-b', type=int, default=100,
                       help='批处理大小 (Batch size)')
    parser.add_argument('--quiet', '-q', action='store_true',
                       help='静默模式，不打印摘要 (Quiet mode, do not print summary)')
    
    args = parser.parse_args()
    
    warmup = CacheWarmup(args.config)
    
    try:
        if not await warmup.initialize_connections():
            logger.error("初始化连接失败，退出 (Failed to initialize connections, exiting)")
            return
        
        await warmup.execute_warmup_strategies(args.strategies, args.batch_size)
        
        if not args.quiet:
            warmup.print_warmup_summary()
    
    except Exception as e:
        logger.error(f"缓存预热过程中发生错误 (Error during cache warmup): {e}")
    finally:
        await warmup.cleanup()


if __name__ == '__main__':
    asyncio.run(main())