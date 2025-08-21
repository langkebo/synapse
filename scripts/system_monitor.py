#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Synapse 系统资源监控脚本
Synapse System Resource Monitor Script

用于监控 Synapse 服务器的系统资源使用情况，包括：
- CPU 使用率
- 内存使用情况
- 磁盘空间和 I/O
- 网络流量
- 进程状态
- 数据库连接数

Used to monitor Synapse server system resource usage, including:
- CPU usage
- Memory usage
- Disk space and I/O
- Network traffic
- Process status
- Database connections
"""

import argparse
import json
import logging
import os
import psutil
import sys
import time
from datetime import datetime, timedelta
from typing import Dict, List, Optional, Any

import psycopg2
from psycopg2.extras import RealDictCursor

# 配置日志
# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
    handlers=[
        logging.FileHandler('/var/log/synapse/system_monitor.log'),
        logging.StreamHandler(sys.stdout)
    ]
)
logger = logging.getLogger(__name__)


class SystemMonitor:
    """系统资源监控器类
    
    System resource monitor class
    """
    
    def __init__(self, config_path: str = "/data/homeserver.yaml"):
        """初始化系统监控器
        
        Initialize system monitor
        
        Args:
            config_path: Synapse 配置文件路径 (Synapse config file path)
        """
        self.config_path = config_path
        self.config = self._load_config()
        self.db_connection = None
        self.monitoring_data = []
        
        # 阈值配置 (Threshold configuration)
        self.thresholds = {
            'cpu_usage': 80.0,  # CPU 使用率阈值 (CPU usage threshold)
            'memory_usage': 85.0,  # 内存使用率阈值 (Memory usage threshold)
            'disk_usage': 90.0,  # 磁盘使用率阈值 (Disk usage threshold)
            'disk_io_wait': 20.0,  # 磁盘 I/O 等待时间阈值 (Disk I/O wait threshold)
            'network_errors': 10,  # 网络错误数阈值 (Network errors threshold)
            'db_connections': 80,  # 数据库连接数阈值 (Database connections threshold)
            'load_average': 2.0  # 系统负载阈值 (System load threshold)
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
    
    def initialize_db_connection(self) -> bool:
        """初始化数据库连接
        
        Initialize database connection
        
        Returns:
            是否成功初始化 (Whether initialization was successful)
        """
        try:
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
                return True
            else:
                logger.warning("数据库配置未找到 (Database configuration not found)")
                return False
        except Exception as e:
            logger.error(f"初始化数据库连接失败 (Failed to initialize database connection): {e}")
            return False
    
    def get_cpu_info(self) -> Dict[str, Any]:
        """获取 CPU 信息
        
        Get CPU information
        
        Returns:
            CPU 信息字典 (CPU information dictionary)
        """
        try:
            cpu_percent = psutil.cpu_percent(interval=1)
            cpu_count = psutil.cpu_count()
            cpu_count_logical = psutil.cpu_count(logical=True)
            load_avg = os.getloadavg()
            
            # 获取每个核心的使用率
            # Get per-core usage
            cpu_per_core = psutil.cpu_percent(interval=1, percpu=True)
            
            return {
                'cpu_percent': cpu_percent,
                'cpu_count_physical': cpu_count,
                'cpu_count_logical': cpu_count_logical,
                'load_average_1m': load_avg[0],
                'load_average_5m': load_avg[1],
                'load_average_15m': load_avg[2],
                'cpu_per_core': cpu_per_core,
                'cpu_freq': psutil.cpu_freq()._asdict() if psutil.cpu_freq() else None
            }
        except Exception as e:
            logger.error(f"获取 CPU 信息失败 (Failed to get CPU info): {e}")
            return {}
    
    def get_memory_info(self) -> Dict[str, Any]:
        """获取内存信息
        
        Get memory information
        
        Returns:
            内存信息字典 (Memory information dictionary)
        """
        try:
            memory = psutil.virtual_memory()
            swap = psutil.swap_memory()
            
            return {
                'memory_total': memory.total,
                'memory_available': memory.available,
                'memory_used': memory.used,
                'memory_percent': memory.percent,
                'memory_free': memory.free,
                'memory_buffers': getattr(memory, 'buffers', 0),
                'memory_cached': getattr(memory, 'cached', 0),
                'swap_total': swap.total,
                'swap_used': swap.used,
                'swap_free': swap.free,
                'swap_percent': swap.percent
            }
        except Exception as e:
            logger.error(f"获取内存信息失败 (Failed to get memory info): {e}")
            return {}
    
    def get_disk_info(self) -> Dict[str, Any]:
        """获取磁盘信息
        
        Get disk information
        
        Returns:
            磁盘信息字典 (Disk information dictionary)
        """
        try:
            disk_usage = psutil.disk_usage('/')
            disk_io = psutil.disk_io_counters()
            
            # 获取所有挂载点的磁盘使用情况
            # Get disk usage for all mount points
            disk_partitions = []
            for partition in psutil.disk_partitions():
                try:
                    partition_usage = psutil.disk_usage(partition.mountpoint)
                    disk_partitions.append({
                        'device': partition.device,
                        'mountpoint': partition.mountpoint,
                        'fstype': partition.fstype,
                        'total': partition_usage.total,
                        'used': partition_usage.used,
                        'free': partition_usage.free,
                        'percent': (partition_usage.used / partition_usage.total) * 100
                    })
                except PermissionError:
                    continue
            
            result = {
                'disk_total': disk_usage.total,
                'disk_used': disk_usage.used,
                'disk_free': disk_usage.free,
                'disk_percent': (disk_usage.used / disk_usage.total) * 100,
                'disk_partitions': disk_partitions
            }
            
            if disk_io:
                result.update({
                    'disk_read_count': disk_io.read_count,
                    'disk_write_count': disk_io.write_count,
                    'disk_read_bytes': disk_io.read_bytes,
                    'disk_write_bytes': disk_io.write_bytes,
                    'disk_read_time': disk_io.read_time,
                    'disk_write_time': disk_io.write_time
                })
            
            return result
        except Exception as e:
            logger.error(f"获取磁盘信息失败 (Failed to get disk info): {e}")
            return {}
    
    def get_network_info(self) -> Dict[str, Any]:
        """获取网络信息
        
        Get network information
        
        Returns:
            网络信息字典 (Network information dictionary)
        """
        try:
            network_io = psutil.net_io_counters()
            network_connections = len(psutil.net_connections())
            
            # 获取每个网络接口的统计信息
            # Get statistics for each network interface
            network_interfaces = {}
            for interface, stats in psutil.net_io_counters(pernic=True).items():
                network_interfaces[interface] = {
                    'bytes_sent': stats.bytes_sent,
                    'bytes_recv': stats.bytes_recv,
                    'packets_sent': stats.packets_sent,
                    'packets_recv': stats.packets_recv,
                    'errin': stats.errin,
                    'errout': stats.errout,
                    'dropin': stats.dropin,
                    'dropout': stats.dropout
                }
            
            result = {
                'network_bytes_sent': network_io.bytes_sent,
                'network_bytes_recv': network_io.bytes_recv,
                'network_packets_sent': network_io.packets_sent,
                'network_packets_recv': network_io.packets_recv,
                'network_errin': network_io.errin,
                'network_errout': network_io.errout,
                'network_dropin': network_io.dropin,
                'network_dropout': network_io.dropout,
                'network_connections': network_connections,
                'network_interfaces': network_interfaces
            }
            
            return result
        except Exception as e:
            logger.error(f"获取网络信息失败 (Failed to get network info): {e}")
            return {}
    
    def get_process_info(self) -> Dict[str, Any]:
        """获取进程信息
        
        Get process information
        
        Returns:
            进程信息字典 (Process information dictionary)
        """
        try:
            synapse_processes = []
            total_processes = 0
            
            for proc in psutil.process_iter(['pid', 'name', 'cpu_percent', 'memory_percent', 'memory_info', 'status']):
                try:
                    total_processes += 1
                    if 'synapse' in proc.info['name'].lower() or 'python' in proc.info['name'].lower():
                        # 检查是否是 Synapse 相关进程
                        # Check if it's a Synapse-related process
                        try:
                            cmdline = proc.cmdline()
                            if any('synapse' in arg.lower() for arg in cmdline):
                                synapse_processes.append({
                                    'pid': proc.info['pid'],
                                    'name': proc.info['name'],
                                    'cpu_percent': proc.info['cpu_percent'],
                                    'memory_percent': proc.info['memory_percent'],
                                    'memory_rss': proc.info['memory_info'].rss if proc.info['memory_info'] else 0,
                                    'memory_vms': proc.info['memory_info'].vms if proc.info['memory_info'] else 0,
                                    'status': proc.info['status'],
                                    'cmdline': ' '.join(cmdline[:3])  # 只显示前3个参数
                                })
                        except (psutil.AccessDenied, psutil.NoSuchProcess):
                            continue
                except (psutil.NoSuchProcess, psutil.AccessDenied, psutil.ZombieProcess):
                    continue
            
            return {
                'total_processes': total_processes,
                'synapse_processes': synapse_processes,
                'synapse_process_count': len(synapse_processes)
            }
        except Exception as e:
            logger.error(f"获取进程信息失败 (Failed to get process info): {e}")
            return {}
    
    def get_database_info(self) -> Dict[str, Any]:
        """获取数据库信息
        
        Get database information
        
        Returns:
            数据库信息字典 (Database information dictionary)
        """
        if not self.db_connection:
            return {}
        
        try:
            with self.db_connection.cursor() as cursor:
                # 获取数据库连接数
                # Get database connection count
                cursor.execute("""
                    SELECT count(*) as connection_count
                    FROM pg_stat_activity
                    WHERE state = 'active'
                """)
                active_connections = cursor.fetchone()['connection_count']
                
                cursor.execute("""
                    SELECT count(*) as total_connections
                    FROM pg_stat_activity
                """)
                total_connections = cursor.fetchone()['total_connections']
                
                # 获取数据库大小
                # Get database size
                cursor.execute("""
                    SELECT pg_size_pretty(pg_database_size(current_database())) as db_size,
                           pg_database_size(current_database()) as db_size_bytes
                """)
                db_size_info = cursor.fetchone()
                
                # 获取表大小信息
                # Get table size information
                cursor.execute("""
                    SELECT 
                        schemaname,
                        tablename,
                        pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) as size,
                        pg_total_relation_size(schemaname||'.'||tablename) as size_bytes
                    FROM pg_tables 
                    WHERE schemaname = 'public'
                    ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC
                    LIMIT 10
                """)
                largest_tables = cursor.fetchall()
                
                # 获取慢查询信息
                # Get slow query information
                cursor.execute("""
                    SELECT 
                        query,
                        calls,
                        total_time,
                        mean_time,
                        rows
                    FROM pg_stat_statements 
                    WHERE mean_time > 100
                    ORDER BY mean_time DESC
                    LIMIT 5
                """)
                slow_queries = cursor.fetchall()
                
                return {
                    'active_connections': active_connections,
                    'total_connections': total_connections,
                    'db_size': db_size_info['db_size'],
                    'db_size_bytes': db_size_info['db_size_bytes'],
                    'largest_tables': [dict(table) for table in largest_tables],
                    'slow_queries': [dict(query) for query in slow_queries] if slow_queries else []
                }
        except Exception as e:
            logger.error(f"获取数据库信息失败 (Failed to get database info): {e}")
            return {}
    
    def check_thresholds(self, metrics: Dict[str, Any]) -> List[Dict[str, Any]]:
        """检查阈值并生成告警
        
        Check thresholds and generate alerts
        
        Args:
            metrics: 监控指标 (Monitoring metrics)
            
        Returns:
            告警列表 (Alert list)
        """
        alerts = []
        
        # CPU 使用率检查
        # CPU usage check
        if metrics.get('cpu_percent', 0) > self.thresholds['cpu_usage']:
            alerts.append({
                'type': 'cpu_high',
                'level': 'warning',
                'message': f"CPU 使用率过高: {metrics['cpu_percent']:.1f}% (阈值: {self.thresholds['cpu_usage']}%)",
                'value': metrics['cpu_percent'],
                'threshold': self.thresholds['cpu_usage']
            })
        
        # 内存使用率检查
        # Memory usage check
        if metrics.get('memory_percent', 0) > self.thresholds['memory_usage']:
            alerts.append({
                'type': 'memory_high',
                'level': 'warning',
                'message': f"内存使用率过高: {metrics['memory_percent']:.1f}% (阈值: {self.thresholds['memory_usage']}%)",
                'value': metrics['memory_percent'],
                'threshold': self.thresholds['memory_usage']
            })
        
        # 磁盘使用率检查
        # Disk usage check
        if metrics.get('disk_percent', 0) > self.thresholds['disk_usage']:
            alerts.append({
                'type': 'disk_high',
                'level': 'critical',
                'message': f"磁盘使用率过高: {metrics['disk_percent']:.1f}% (阈值: {self.thresholds['disk_usage']}%)",
                'value': metrics['disk_percent'],
                'threshold': self.thresholds['disk_usage']
            })
        
        # 系统负载检查
        # System load check
        if metrics.get('load_average_1m', 0) > self.thresholds['load_average']:
            alerts.append({
                'type': 'load_high',
                'level': 'warning',
                'message': f"系统负载过高: {metrics['load_average_1m']:.2f} (阈值: {self.thresholds['load_average']})",
                'value': metrics['load_average_1m'],
                'threshold': self.thresholds['load_average']
            })
        
        # 数据库连接数检查
        # Database connections check
        if metrics.get('active_connections', 0) > self.thresholds['db_connections']:
            alerts.append({
                'type': 'db_connections_high',
                'level': 'warning',
                'message': f"数据库活跃连接数过高: {metrics['active_connections']} (阈值: {self.thresholds['db_connections']})",
                'value': metrics['active_connections'],
                'threshold': self.thresholds['db_connections']
            })
        
        # 网络错误检查
        # Network errors check
        total_network_errors = metrics.get('network_errin', 0) + metrics.get('network_errout', 0)
        if total_network_errors > self.thresholds['network_errors']:
            alerts.append({
                'type': 'network_errors_high',
                'level': 'warning',
                'message': f"网络错误数过高: {total_network_errors} (阈值: {self.thresholds['network_errors']})",
                'value': total_network_errors,
                'threshold': self.thresholds['network_errors']
            })
        
        return alerts
    
    def collect_metrics(self) -> Dict[str, Any]:
        """收集所有监控指标
        
        Collect all monitoring metrics
        
        Returns:
            监控指标字典 (Monitoring metrics dictionary)
        """
        timestamp = datetime.now()
        
        metrics = {
            'timestamp': timestamp.isoformat(),
            'hostname': os.uname().nodename
        }
        
        # 收集各类指标
        # Collect various metrics
        metrics.update(self.get_cpu_info())
        metrics.update(self.get_memory_info())
        metrics.update(self.get_disk_info())
        metrics.update(self.get_network_info())
        metrics.update(self.get_process_info())
        metrics.update(self.get_database_info())
        
        # 检查阈值
        # Check thresholds
        alerts = self.check_thresholds(metrics)
        metrics['alerts'] = alerts
        
        return metrics
    
    def save_metrics(self, metrics: Dict[str, Any], output_file: str):
        """保存监控指标到文件
        
        Save monitoring metrics to file
        
        Args:
            metrics: 监控指标 (Monitoring metrics)
            output_file: 输出文件路径 (Output file path)
        """
        try:
            # 读取现有数据
            # Read existing data
            if os.path.exists(output_file):
                with open(output_file, 'r', encoding='utf-8') as f:
                    existing_data = json.load(f)
            else:
                existing_data = []
            
            # 添加新数据
            # Add new data
            existing_data.append(metrics)
            
            # 保留最近24小时的数据（假设每分钟收集一次）
            # Keep last 24 hours of data (assuming collection every minute)
            max_records = 24 * 60
            if len(existing_data) > max_records:
                existing_data = existing_data[-max_records:]
            
            # 保存数据
            # Save data
            with open(output_file, 'w', encoding='utf-8') as f:
                json.dump(existing_data, f, indent=2, ensure_ascii=False, default=str)
            
            logger.info(f"监控数据已保存到 {output_file} (Monitoring data saved to {output_file})")
        except Exception as e:
            logger.error(f"保存监控数据失败 (Failed to save monitoring data): {e}")
    
    def print_metrics_summary(self, metrics: Dict[str, Any]):
        """打印监控指标摘要
        
        Print monitoring metrics summary
        
        Args:
            metrics: 监控指标 (Monitoring metrics)
        """
        print("\n" + "="*60)
        print(f"Synapse 系统监控报告 (Synapse System Monitor Report)")
        print(f"时间 (Time): {metrics.get('timestamp', 'N/A')}")
        print(f"主机 (Hostname): {metrics.get('hostname', 'N/A')}")
        print("="*60)
        
        # CPU 信息
        # CPU information
        print(f"\n🖥️  CPU 信息 (CPU Information):")
        print(f"  使用率 (Usage): {metrics.get('cpu_percent', 0):.1f}%")
        print(f"  核心数 (Cores): {metrics.get('cpu_count_physical', 'N/A')} 物理 / {metrics.get('cpu_count_logical', 'N/A')} 逻辑")
        print(f"  负载 (Load): {metrics.get('load_average_1m', 0):.2f} / {metrics.get('load_average_5m', 0):.2f} / {metrics.get('load_average_15m', 0):.2f}")
        
        # 内存信息
        # Memory information
        print(f"\n💾 内存信息 (Memory Information):")
        memory_total_gb = metrics.get('memory_total', 0) / (1024**3)
        memory_used_gb = metrics.get('memory_used', 0) / (1024**3)
        memory_available_gb = metrics.get('memory_available', 0) / (1024**3)
        print(f"  使用率 (Usage): {metrics.get('memory_percent', 0):.1f}%")
        print(f"  总计 (Total): {memory_total_gb:.2f} GB")
        print(f"  已用 (Used): {memory_used_gb:.2f} GB")
        print(f"  可用 (Available): {memory_available_gb:.2f} GB")
        
        # 磁盘信息
        # Disk information
        print(f"\n💿 磁盘信息 (Disk Information):")
        disk_total_gb = metrics.get('disk_total', 0) / (1024**3)
        disk_used_gb = metrics.get('disk_used', 0) / (1024**3)
        disk_free_gb = metrics.get('disk_free', 0) / (1024**3)
        print(f"  使用率 (Usage): {metrics.get('disk_percent', 0):.1f}%")
        print(f"  总计 (Total): {disk_total_gb:.2f} GB")
        print(f"  已用 (Used): {disk_used_gb:.2f} GB")
        print(f"  可用 (Free): {disk_free_gb:.2f} GB")
        
        # 网络信息
        # Network information
        print(f"\n🌐 网络信息 (Network Information):")
        network_sent_mb = metrics.get('network_bytes_sent', 0) / (1024**2)
        network_recv_mb = metrics.get('network_bytes_recv', 0) / (1024**2)
        print(f"  发送 (Sent): {network_sent_mb:.2f} MB")
        print(f"  接收 (Received): {network_recv_mb:.2f} MB")
        print(f"  连接数 (Connections): {metrics.get('network_connections', 0)}")
        print(f"  错误 (Errors): 入 {metrics.get('network_errin', 0)} / 出 {metrics.get('network_errout', 0)}")
        
        # 进程信息
        # Process information
        print(f"\n⚙️  进程信息 (Process Information):")
        print(f"  总进程数 (Total Processes): {metrics.get('total_processes', 0)}")
        print(f"  Synapse 进程数 (Synapse Processes): {metrics.get('synapse_process_count', 0)}")
        
        # 数据库信息
        # Database information
        if metrics.get('active_connections') is not None:
            print(f"\n🗄️  数据库信息 (Database Information):")
            print(f"  活跃连接 (Active Connections): {metrics.get('active_connections', 0)}")
            print(f"  总连接 (Total Connections): {metrics.get('total_connections', 0)}")
            print(f"  数据库大小 (Database Size): {metrics.get('db_size', 'N/A')}")
        
        # 告警信息
        # Alert information
        alerts = metrics.get('alerts', [])
        if alerts:
            print(f"\n⚠️  告警信息 (Alerts):")
            for alert in alerts:
                level_emoji = "🔴" if alert['level'] == 'critical' else "🟡"
                print(f"  {level_emoji} {alert['message']}")
        else:
            print(f"\n✅ 无告警 (No Alerts)")
        
        print("="*60)
    
    def cleanup(self):
        """清理资源
        
        Cleanup resources
        """
        if self.db_connection:
            self.db_connection.close()


def main():
    """主函数
    
    Main function
    """
    parser = argparse.ArgumentParser(description='Synapse 系统资源监控脚本 (Synapse System Resource Monitor Script)')
    parser.add_argument('--config', '-c', default='/data/homeserver.yaml',
                       help='Synapse 配置文件路径 (Synapse config file path)')
    parser.add_argument('--output', '-o', default='/var/log/synapse/system_metrics.json',
                       help='输出文件路径 (Output file path)')
    parser.add_argument('--interval', '-i', type=int, default=60,
                       help='监控间隔（秒）(Monitoring interval in seconds)')
    parser.add_argument('--once', action='store_true',
                       help='只运行一次，不持续监控 (Run once, do not monitor continuously)')
    parser.add_argument('--quiet', '-q', action='store_true',
                       help='静默模式，不打印摘要 (Quiet mode, do not print summary)')
    
    args = parser.parse_args()
    
    monitor = SystemMonitor(args.config)
    
    try:
        # 初始化数据库连接（可选）
        # Initialize database connection (optional)
        monitor.initialize_db_connection()
        
        if args.once:
            # 单次运行
            # Single run
            metrics = monitor.collect_metrics()
            
            if not args.quiet:
                monitor.print_metrics_summary(metrics)
            
            monitor.save_metrics(metrics, args.output)
        else:
            # 持续监控
            # Continuous monitoring
            logger.info(f"开始持续监控，间隔 {args.interval} 秒 (Starting continuous monitoring with {args.interval} second interval)")
            
            while True:
                try:
                    metrics = monitor.collect_metrics()
                    
                    if not args.quiet:
                        monitor.print_metrics_summary(metrics)
                    
                    monitor.save_metrics(metrics, args.output)
                    
                    # 如果有告警，记录到日志
                    # Log alerts if any
                    alerts = metrics.get('alerts', [])
                    for alert in alerts:
                        if alert['level'] == 'critical':
                            logger.error(alert['message'])
                        else:
                            logger.warning(alert['message'])
                    
                    time.sleep(args.interval)
                    
                except KeyboardInterrupt:
                    logger.info("收到中断信号，停止监控 (Received interrupt signal, stopping monitoring)")
                    break
                except Exception as e:
                    logger.error(f"监控过程中发生错误 (Error during monitoring): {e}")
                    time.sleep(args.interval)
    
    except Exception as e:
        logger.error(f"系统监控过程中发生错误 (Error during system monitoring): {e}")
    finally:
        monitor.cleanup()


if __name__ == '__main__':
    main()