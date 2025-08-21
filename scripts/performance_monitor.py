#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Synapse 性能监控脚本
Synapse Performance Monitoring Script

用于监控 Synapse 服务器的性能指标，包括：
- 内存使用情况
- CPU 使用率
- 数据库连接状态
- 缓存命中率
- 网络连接状态
- 好友功能性能指标

Monitors Synapse server performance metrics including:
- Memory usage
- CPU utilization
- Database connection status
- Cache hit rates
- Network connection status
- Friends feature performance metrics
"""

import argparse
import asyncio
import json
import logging
import os
import sys
import time
from datetime import datetime
from typing import Dict, List, Optional, Any

import psutil
import redis
import psycopg2
from psycopg2.extras import RealDictCursor

# 配置日志
# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
    handlers=[
        logging.FileHandler('/var/log/synapse/performance_monitor.log'),
        logging.StreamHandler(sys.stdout)
    ]
)
logger = logging.getLogger(__name__)


class PerformanceMonitor:
    """性能监控器类
    
    Performance monitor class
    """
    
    def __init__(self, config_path: str = "/data/homeserver.yaml"):
        """初始化性能监控器
        
        Initialize performance monitor
        
        Args:
            config_path: Synapse 配置文件路径 (Synapse config file path)
        """
        self.config_path = config_path
        self.config = self._load_config()
        self.redis_client = None
        self.db_connection = None
        self.metrics = {
            'timestamp': None,
            'system': {},
            'database': {},
            'cache': {},
            'friends': {},
            'network': {},
            'alerts': []
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
                
        except Exception as e:
            logger.error(f"初始化连接失败 (Failed to initialize connections): {e}")
    
    def collect_system_metrics(self):
        """收集系统性能指标
        
        Collect system performance metrics
        """
        try:
            # CPU 使用率
            # CPU usage
            cpu_percent = psutil.cpu_percent(interval=1)
            cpu_count = psutil.cpu_count()
            
            # 内存使用情况
            # Memory usage
            memory = psutil.virtual_memory()
            swap = psutil.swap_memory()
            
            # 磁盘使用情况
            # Disk usage
            disk = psutil.disk_usage('/')
            
            # 网络统计
            # Network statistics
            network = psutil.net_io_counters()
            
            # 进程信息
            # Process information
            synapse_processes = []
            for proc in psutil.process_iter(['pid', 'name', 'cpu_percent', 'memory_percent']):
                try:
                    if 'synapse' in proc.info['name'].lower():
                        synapse_processes.append(proc.info)
                except (psutil.NoSuchProcess, psutil.AccessDenied):
                    pass
            
            self.metrics['system'] = {
                'cpu': {
                    'usage_percent': cpu_percent,
                    'count': cpu_count,
                    'load_average': os.getloadavg() if hasattr(os, 'getloadavg') else None
                },
                'memory': {
                    'total': memory.total,
                    'available': memory.available,
                    'used': memory.used,
                    'percent': memory.percent,
                    'swap_total': swap.total,
                    'swap_used': swap.used,
                    'swap_percent': swap.percent
                },
                'disk': {
                    'total': disk.total,
                    'used': disk.used,
                    'free': disk.free,
                    'percent': (disk.used / disk.total) * 100
                },
                'network': {
                    'bytes_sent': network.bytes_sent,
                    'bytes_recv': network.bytes_recv,
                    'packets_sent': network.packets_sent,
                    'packets_recv': network.packets_recv
                },
                'processes': synapse_processes
            }
            
            # 检查系统告警
            # Check system alerts
            self._check_system_alerts()
            
        except Exception as e:
            logger.error(f"收集系统指标失败 (Failed to collect system metrics): {e}")
    
    def collect_database_metrics(self):
        """收集数据库性能指标
        
        Collect database performance metrics
        """
        if not self.db_connection:
            return
            
        try:
            with self.db_connection.cursor() as cursor:
                # 数据库连接数
                # Database connections
                cursor.execute("""
                    SELECT count(*) as active_connections
                    FROM pg_stat_activity
                    WHERE state = 'active'
                """)
                active_connections = cursor.fetchone()['active_connections']
                
                cursor.execute("""
                    SELECT count(*) as total_connections
                    FROM pg_stat_activity
                """)
                total_connections = cursor.fetchone()['total_connections']
                
                # 数据库大小
                # Database size
                cursor.execute("""
                    SELECT pg_size_pretty(pg_database_size(current_database())) as db_size,
                           pg_database_size(current_database()) as db_size_bytes
                """)
                db_size_info = cursor.fetchone()
                
                # 慢查询统计
                # Slow query statistics
                cursor.execute("""
                    SELECT query, calls, total_time, mean_time
                    FROM pg_stat_statements
                    WHERE mean_time > 1000
                    ORDER BY mean_time DESC
                    LIMIT 10
                """)
                slow_queries = cursor.fetchall()
                
                # 表统计信息
                # Table statistics
                cursor.execute("""
                    SELECT schemaname, tablename, n_tup_ins, n_tup_upd, n_tup_del,
                           n_live_tup, n_dead_tup, last_vacuum, last_autovacuum
                    FROM pg_stat_user_tables
                    WHERE schemaname = 'public'
                    ORDER BY n_live_tup DESC
                    LIMIT 20
                """)
                table_stats = cursor.fetchall()
                
                # 好友功能相关表统计
                # Friends feature table statistics
                cursor.execute("""
                    SELECT 
                        (SELECT count(*) FROM friends) as total_friendships,
                        (SELECT count(*) FROM friend_requests WHERE status = 'pending') as pending_requests,
                        (SELECT count(DISTINCT user_id) FROM friends) as users_with_friends
                """)
                friends_stats = cursor.fetchone()
                
                self.metrics['database'] = {
                    'connections': {
                        'active': active_connections,
                        'total': total_connections
                    },
                    'size': {
                        'pretty': db_size_info['db_size'],
                        'bytes': db_size_info['db_size_bytes']
                    },
                    'slow_queries': slow_queries,
                    'table_stats': table_stats,
                    'friends_stats': dict(friends_stats) if friends_stats else {}
                }
                
                # 检查数据库告警
                # Check database alerts
                self._check_database_alerts()
                
        except Exception as e:
            logger.error(f"收集数据库指标失败 (Failed to collect database metrics): {e}")
    
    def collect_cache_metrics(self):
        """收集缓存性能指标
        
        Collect cache performance metrics
        """
        if not self.redis_client:
            return
            
        try:
            # Redis 信息
            # Redis information
            redis_info = self.redis_client.info()
            
            # 缓存统计
            # Cache statistics
            cache_stats = {
                'memory_usage': redis_info.get('used_memory', 0),
                'memory_usage_human': redis_info.get('used_memory_human', '0B'),
                'memory_peak': redis_info.get('used_memory_peak', 0),
                'memory_peak_human': redis_info.get('used_memory_peak_human', '0B'),
                'connected_clients': redis_info.get('connected_clients', 0),
                'total_commands_processed': redis_info.get('total_commands_processed', 0),
                'keyspace_hits': redis_info.get('keyspace_hits', 0),
                'keyspace_misses': redis_info.get('keyspace_misses', 0),
                'expired_keys': redis_info.get('expired_keys', 0),
                'evicted_keys': redis_info.get('evicted_keys', 0)
            }
            
            # 计算命中率
            # Calculate hit rate
            hits = cache_stats['keyspace_hits']
            misses = cache_stats['keyspace_misses']
            total_requests = hits + misses
            hit_rate = (hits / total_requests * 100) if total_requests > 0 else 0
            
            # 好友功能缓存统计
            # Friends feature cache statistics
            friends_cache_keys = self.redis_client.keys('friends:*')
            friends_cache_stats = {
                'total_keys': len(friends_cache_keys),
                'relationships_keys': len([k for k in friends_cache_keys if 'relationships' in k]),
                'requests_keys': len([k for k in friends_cache_keys if 'requests' in k]),
                'presence_keys': len([k for k in friends_cache_keys if 'presence' in k]),
                'recommendations_keys': len([k for k in friends_cache_keys if 'recommendations' in k])
            }
            
            self.metrics['cache'] = {
                'redis_stats': cache_stats,
                'hit_rate': hit_rate,
                'friends_cache': friends_cache_stats
            }
            
            # 检查缓存告警
            # Check cache alerts
            self._check_cache_alerts(hit_rate)
            
        except Exception as e:
            logger.error(f"收集缓存指标失败 (Failed to collect cache metrics): {e}")
    
    def collect_friends_metrics(self):
        """收集好友功能性能指标
        
        Collect friends feature performance metrics
        """
        if not self.db_connection:
            return
            
        try:
            with self.db_connection.cursor() as cursor:
                # 好友功能使用统计
                # Friends feature usage statistics
                cursor.execute("""
                    SELECT 
                        count(*) as total_friendships,
                        count(DISTINCT user_id) as users_with_friends,
                        count(DISTINCT friend_id) as unique_friends,
                        avg(extract(epoch from (now() - created_at))) as avg_friendship_age_seconds
                    FROM friends
                """)
                friendship_stats = cursor.fetchone()
                
                # 好友请求统计
                # Friend request statistics
                cursor.execute("""
                    SELECT 
                        status,
                        count(*) as count,
                        avg(extract(epoch from (now() - created_at))) as avg_age_seconds
                    FROM friend_requests
                    GROUP BY status
                """)
                request_stats = cursor.fetchall()
                
                # 最近活跃的好友功能使用
                # Recent friends feature activity
                cursor.execute("""
                    SELECT 
                        date_trunc('hour', created_at) as hour,
                        count(*) as new_friendships
                    FROM friends
                    WHERE created_at > now() - interval '24 hours'
                    GROUP BY hour
                    ORDER BY hour DESC
                """)
                recent_activity = cursor.fetchall()
                
                # 好友推荐性能
                # Friends recommendation performance
                cursor.execute("""
                    SELECT 
                        count(*) as total_recommendations,
                        count(DISTINCT user_id) as users_with_recommendations,
                        avg(score) as avg_recommendation_score
                    FROM friend_recommendations
                    WHERE created_at > now() - interval '7 days'
                """)
                recommendation_stats = cursor.fetchone()
                
                self.metrics['friends'] = {
                    'friendship_stats': dict(friendship_stats) if friendship_stats else {},
                    'request_stats': [dict(row) for row in request_stats],
                    'recent_activity': [dict(row) for row in recent_activity],
                    'recommendation_stats': dict(recommendation_stats) if recommendation_stats else {}
                }
                
        except Exception as e:
            logger.error(f"收集好友功能指标失败 (Failed to collect friends metrics): {e}")
    
    def collect_network_metrics(self):
        """收集网络性能指标
        
        Collect network performance metrics
        """
        try:
            # 网络连接统计
            # Network connection statistics
            connections = psutil.net_connections()
            
            connection_stats = {
                'total': len(connections),
                'established': len([c for c in connections if c.status == 'ESTABLISHED']),
                'listen': len([c for c in connections if c.status == 'LISTEN']),
                'time_wait': len([c for c in connections if c.status == 'TIME_WAIT']),
                'close_wait': len([c for c in connections if c.status == 'CLOSE_WAIT'])
            }
            
            # 端口使用情况
            # Port usage
            synapse_ports = []
            for conn in connections:
                if conn.laddr and conn.laddr.port in [8008, 8448]:  # Synapse 默认端口
                    synapse_ports.append({
                        'port': conn.laddr.port,
                        'status': conn.status,
                        'pid': conn.pid
                    })
            
            self.metrics['network'] = {
                'connections': connection_stats,
                'synapse_ports': synapse_ports
            }
            
        except Exception as e:
            logger.error(f"收集网络指标失败 (Failed to collect network metrics): {e}")
    
    def _check_system_alerts(self):
        """检查系统告警
        
        Check system alerts
        """
        system = self.metrics['system']
        
        # CPU 使用率告警
        # CPU usage alert
        if system['cpu']['usage_percent'] > 80:
            self.metrics['alerts'].append({
                'type': 'system',
                'level': 'warning',
                'message': f"CPU 使用率过高: {system['cpu']['usage_percent']:.1f}% (CPU usage too high)"
            })
        
        # 内存使用率告警
        # Memory usage alert
        if system['memory']['percent'] > 85:
            self.metrics['alerts'].append({
                'type': 'system',
                'level': 'warning',
                'message': f"内存使用率过高: {system['memory']['percent']:.1f}% (Memory usage too high)"
            })
        
        # 磁盘使用率告警
        # Disk usage alert
        if system['disk']['percent'] > 90:
            self.metrics['alerts'].append({
                'type': 'system',
                'level': 'critical',
                'message': f"磁盘使用率过高: {system['disk']['percent']:.1f}% (Disk usage too high)"
            })
    
    def _check_database_alerts(self):
        """检查数据库告警
        
        Check database alerts
        """
        database = self.metrics['database']
        
        # 数据库连接数告警
        # Database connection alert
        if database['connections']['active'] > 50:
            self.metrics['alerts'].append({
                'type': 'database',
                'level': 'warning',
                'message': f"数据库活跃连接数过多: {database['connections']['active']} (Too many active database connections)"
            })
    
    def _check_cache_alerts(self, hit_rate: float):
        """检查缓存告警
        
        Check cache alerts
        
        Args:
            hit_rate: 缓存命中率 (Cache hit rate)
        """
        # 缓存命中率告警
        # Cache hit rate alert
        if hit_rate < 80:
            self.metrics['alerts'].append({
                'type': 'cache',
                'level': 'warning',
                'message': f"缓存命中率过低: {hit_rate:.1f}% (Cache hit rate too low)"
            })
    
    async def collect_all_metrics(self):
        """收集所有性能指标
        
        Collect all performance metrics
        """
        self.metrics['timestamp'] = datetime.now().isoformat()
        self.metrics['alerts'] = []  # 重置告警列表 (Reset alerts list)
        
        logger.info("开始收集性能指标 (Starting to collect performance metrics)")
        
        # 并行收集指标
        # Collect metrics in parallel
        tasks = [
            asyncio.create_task(asyncio.to_thread(self.collect_system_metrics)),
            asyncio.create_task(asyncio.to_thread(self.collect_database_metrics)),
            asyncio.create_task(asyncio.to_thread(self.collect_cache_metrics)),
            asyncio.create_task(asyncio.to_thread(self.collect_friends_metrics)),
            asyncio.create_task(asyncio.to_thread(self.collect_network_metrics))
        ]
        
        await asyncio.gather(*tasks, return_exceptions=True)
        
        logger.info(f"性能指标收集完成，发现 {len(self.metrics['alerts'])} 个告警 (Performance metrics collection completed, found {len(self.metrics['alerts'])} alerts)")
    
    def save_metrics(self, output_file: str = None):
        """保存性能指标到文件
        
        Save performance metrics to file
        
        Args:
            output_file: 输出文件路径 (Output file path)
        """
        if not output_file:
            timestamp = datetime.now().strftime('%Y%m%d_%H%M%S')
            output_file = f"/var/log/synapse/performance_metrics_{timestamp}.json"
        
        try:
            os.makedirs(os.path.dirname(output_file), exist_ok=True)
            with open(output_file, 'w', encoding='utf-8') as f:
                json.dump(self.metrics, f, indent=2, ensure_ascii=False, default=str)
            logger.info(f"性能指标已保存到: {output_file} (Performance metrics saved to: {output_file})")
        except Exception as e:
            logger.error(f"保存性能指标失败 (Failed to save performance metrics): {e}")
    
    def print_summary(self):
        """打印性能指标摘要
        
        Print performance metrics summary
        """
        print("\n" + "="*60)
        print("Synapse 性能监控报告 (Synapse Performance Monitoring Report)")
        print("="*60)
        print(f"时间 (Time): {self.metrics['timestamp']}")
        
        # 系统指标摘要
        # System metrics summary
        if 'system' in self.metrics:
            system = self.metrics['system']
            print(f"\n系统指标 (System Metrics):")
            print(f"  CPU 使用率 (CPU Usage): {system['cpu']['usage_percent']:.1f}%")
            print(f"  内存使用率 (Memory Usage): {system['memory']['percent']:.1f}%")
            print(f"  磁盘使用率 (Disk Usage): {system['disk']['percent']:.1f}%")
        
        # 数据库指标摘要
        # Database metrics summary
        if 'database' in self.metrics:
            database = self.metrics['database']
            print(f"\n数据库指标 (Database Metrics):")
            print(f"  活跃连接数 (Active Connections): {database['connections']['active']}")
            print(f"  数据库大小 (Database Size): {database['size']['pretty']}")
        
        # 缓存指标摘要
        # Cache metrics summary
        if 'cache' in self.metrics:
            cache = self.metrics['cache']
            print(f"\n缓存指标 (Cache Metrics):")
            print(f"  命中率 (Hit Rate): {cache['hit_rate']:.1f}%")
            print(f"  内存使用 (Memory Usage): {cache['redis_stats']['memory_usage_human']}")
        
        # 好友功能指标摘要
        # Friends feature metrics summary
        if 'friends' in self.metrics:
            friends = self.metrics['friends']
            if friends.get('friendship_stats'):
                print(f"\n好友功能指标 (Friends Feature Metrics):")
                print(f"  总好友关系数 (Total Friendships): {friends['friendship_stats'].get('total_friendships', 0)}")
                print(f"  有好友的用户数 (Users with Friends): {friends['friendship_stats'].get('users_with_friends', 0)}")
        
        # 告警信息
        # Alert information
        if self.metrics['alerts']:
            print(f"\n告警信息 (Alerts):")
            for alert in self.metrics['alerts']:
                print(f"  [{alert['level'].upper()}] {alert['message']}")
        else:
            print(f"\n✅ 无告警 (No alerts)")
        
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
    parser = argparse.ArgumentParser(description='Synapse 性能监控脚本 (Synapse Performance Monitor)')
    parser.add_argument('--config', '-c', default='/data/homeserver.yaml',
                       help='Synapse 配置文件路径 (Synapse config file path)')
    parser.add_argument('--output', '-o', help='输出文件路径 (Output file path)')
    parser.add_argument('--interval', '-i', type=int, default=0,
                       help='监控间隔(秒)，0表示只运行一次 (Monitor interval in seconds, 0 means run once)')
    parser.add_argument('--quiet', '-q', action='store_true',
                       help='静默模式，不打印摘要 (Quiet mode, do not print summary)')
    
    args = parser.parse_args()
    
    monitor = PerformanceMonitor(args.config)
    
    try:
        await monitor.initialize_connections()
        
        if args.interval > 0:
            logger.info(f"开始持续监控，间隔 {args.interval} 秒 (Starting continuous monitoring with {args.interval} seconds interval)")
            while True:
                await monitor.collect_all_metrics()
                
                if not args.quiet:
                    monitor.print_summary()
                
                if args.output:
                    monitor.save_metrics(args.output)
                
                await asyncio.sleep(args.interval)
        else:
            await monitor.collect_all_metrics()
            
            if not args.quiet:
                monitor.print_summary()
            
            if args.output:
                monitor.save_metrics(args.output)
    
    except KeyboardInterrupt:
        logger.info("监控已停止 (Monitoring stopped)")
    except Exception as e:
        logger.error(f"监控过程中发生错误 (Error during monitoring): {e}")
    finally:
        await monitor.cleanup()


if __name__ == '__main__':
    asyncio.run(main())