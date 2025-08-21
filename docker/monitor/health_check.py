#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Synapse 健康检查脚本 (Synapse Health Check Script)

用于检查 Synapse 服务和相关组件的健康状态
Used to check the health status of Synapse service and related components

功能 (Features):
- Synapse 服务健康检查 (Synapse service health check)
- 数据库连接检查 (Database connection check)
- Redis 连接检查 (Redis connection check)
- 系统资源检查 (System resource check)
- 详细的错误报告 (Detailed error reporting)
"""

import sys
import time
import json
import logging
import argparse
import requests
import psycopg2
import redis
import psutil
from typing import Dict, Any, Optional
from urllib.parse import urljoin


class HealthChecker:
    """健康检查器类 (Health Checker Class)"""
    
    def __init__(self, config: Dict[str, Any]):
        """初始化健康检查器 (Initialize health checker)"""
        self.config = config
        self.logger = self._setup_logger()
        self.results = {
            'timestamp': int(time.time()),
            'overall_status': 'unknown',
            'checks': {}
        }
    
    def _setup_logger(self) -> logging.Logger:
        """设置日志记录器 (Setup logger)"""
        logger = logging.getLogger('health_checker')
        logger.setLevel(logging.INFO)
        
        if not logger.handlers:
            handler = logging.StreamHandler()
            formatter = logging.Formatter(
                '%(asctime)s - %(name)s - %(levelname)s - %(message)s'
            )
            handler.setFormatter(formatter)
            logger.addHandler(handler)
        
        return logger
    
    def check_synapse_health(self) -> Dict[str, Any]:
        """检查 Synapse 服务健康状态 (Check Synapse service health)"""
        check_name = 'synapse_health'
        result = {
            'status': 'unknown',
            'message': '',
            'details': {},
            'response_time': 0
        }
        
        try:
            synapse_config = self.config.get('synapse', {})
            base_url = synapse_config.get('url', 'http://localhost:8008')
            health_endpoint = synapse_config.get('health_endpoint', '/health')
            timeout = synapse_config.get('timeout', 30)
            
            url = urljoin(base_url, health_endpoint)
            
            self.logger.info(f"检查 Synapse 健康状态: {url} (Checking Synapse health: {url})")
            
            start_time = time.time()
            response = requests.get(url, timeout=timeout)
            response_time = time.time() - start_time
            
            result['response_time'] = round(response_time * 1000, 2)  # ms
            
            if response.status_code == 200:
                result['status'] = 'healthy'
                result['message'] = 'Synapse 服务运行正常 (Synapse service is running normally)'
                
                # 尝试解析响应内容 (Try to parse response content)
                try:
                    health_data = response.json()
                    result['details'] = health_data
                except json.JSONDecodeError:
                    result['details'] = {'raw_response': response.text[:500]}
            else:
                result['status'] = 'unhealthy'
                result['message'] = f'Synapse 服务返回错误状态码: {response.status_code} (Synapse service returned error status code: {response.status_code})'
                result['details'] = {
                    'status_code': response.status_code,
                    'response': response.text[:500]
                }
        
        except requests.exceptions.ConnectionError as e:
            result['status'] = 'unhealthy'
            result['message'] = f'无法连接到 Synapse 服务 (Cannot connect to Synapse service): {str(e)}'
            result['details'] = {'error': str(e)}
        
        except requests.exceptions.Timeout as e:
            result['status'] = 'unhealthy'
            result['message'] = f'Synapse 服务响应超时 (Synapse service response timeout): {str(e)}'
            result['details'] = {'error': str(e)}
        
        except Exception as e:
            result['status'] = 'error'
            result['message'] = f'检查 Synapse 健康状态时发生错误 (Error occurred while checking Synapse health): {str(e)}'
            result['details'] = {'error': str(e)}
        
        self.results['checks'][check_name] = result
        return result
    
    def check_database_health(self) -> Dict[str, Any]:
        """检查数据库健康状态 (Check database health)"""
        check_name = 'database_health'
        result = {
            'status': 'unknown',
            'message': '',
            'details': {},
            'response_time': 0
        }
        
        try:
            db_config = self.config.get('database', {})
            if not db_config.get('enabled', True):
                result['status'] = 'skipped'
                result['message'] = '数据库检查已禁用 (Database check is disabled)'
                self.results['checks'][check_name] = result
                return result
            
            host = db_config.get('host', 'localhost')
            port = db_config.get('port', 5432)
            database = db_config.get('database', 'synapse')
            user = db_config.get('user', 'synapse_user')
            password = db_config.get('password', '')
            timeout = db_config.get('timeout', 30)
            
            self.logger.info(f"检查数据库连接: {host}:{port}/{database} (Checking database connection: {host}:{port}/{database})")
            
            start_time = time.time()
            
            # 建立数据库连接 (Establish database connection)
            conn = psycopg2.connect(
                host=host,
                port=port,
                database=database,
                user=user,
                password=password,
                connect_timeout=timeout
            )
            
            # 执行简单查询 (Execute simple query)
            with conn.cursor() as cursor:
                cursor.execute('SELECT version();')
                version = cursor.fetchone()[0]
                
                cursor.execute('SELECT COUNT(*) FROM pg_stat_activity;')
                active_connections = cursor.fetchone()[0]
            
            response_time = time.time() - start_time
            result['response_time'] = round(response_time * 1000, 2)  # ms
            
            conn.close()
            
            result['status'] = 'healthy'
            result['message'] = '数据库连接正常 (Database connection is normal)'
            result['details'] = {
                'version': version,
                'active_connections': active_connections
            }
        
        except psycopg2.OperationalError as e:
            result['status'] = 'unhealthy'
            result['message'] = f'数据库连接失败 (Database connection failed): {str(e)}'
            result['details'] = {'error': str(e)}
        
        except Exception as e:
            result['status'] = 'error'
            result['message'] = f'检查数据库健康状态时发生错误 (Error occurred while checking database health): {str(e)}'
            result['details'] = {'error': str(e)}
        
        self.results['checks'][check_name] = result
        return result
    
    def check_redis_health(self) -> Dict[str, Any]:
        """检查 Redis 健康状态 (Check Redis health)"""
        check_name = 'redis_health'
        result = {
            'status': 'unknown',
            'message': '',
            'details': {},
            'response_time': 0
        }
        
        try:
            redis_config = self.config.get('redis', {})
            if not redis_config.get('enabled', True):
                result['status'] = 'skipped'
                result['message'] = 'Redis 检查已禁用 (Redis check is disabled)'
                self.results['checks'][check_name] = result
                return result
            
            host = redis_config.get('host', 'localhost')
            port = redis_config.get('port', 6379)
            password = redis_config.get('password', '')
            timeout = redis_config.get('timeout', 30)
            
            self.logger.info(f"检查 Redis 连接: {host}:{port} (Checking Redis connection: {host}:{port})")
            
            start_time = time.time()
            
            # 建立 Redis 连接 (Establish Redis connection)
            r = redis.Redis(
                host=host,
                port=port,
                password=password if password else None,
                socket_timeout=timeout,
                decode_responses=True
            )
            
            # 执行 PING 命令 (Execute PING command)
            ping_result = r.ping()
            
            # 获取 Redis 信息 (Get Redis info)
            info = r.info()
            
            response_time = time.time() - start_time
            result['response_time'] = round(response_time * 1000, 2)  # ms
            
            if ping_result:
                result['status'] = 'healthy'
                result['message'] = 'Redis 连接正常 (Redis connection is normal)'
                result['details'] = {
                    'version': info.get('redis_version', 'unknown'),
                    'connected_clients': info.get('connected_clients', 0),
                    'used_memory_human': info.get('used_memory_human', 'unknown'),
                    'uptime_in_seconds': info.get('uptime_in_seconds', 0)
                }
            else:
                result['status'] = 'unhealthy'
                result['message'] = 'Redis PING 失败 (Redis PING failed)'
        
        except redis.ConnectionError as e:
            result['status'] = 'unhealthy'
            result['message'] = f'Redis 连接失败 (Redis connection failed): {str(e)}'
            result['details'] = {'error': str(e)}
        
        except redis.TimeoutError as e:
            result['status'] = 'unhealthy'
            result['message'] = f'Redis 连接超时 (Redis connection timeout): {str(e)}'
            result['details'] = {'error': str(e)}
        
        except Exception as e:
            result['status'] = 'error'
            result['message'] = f'检查 Redis 健康状态时发生错误 (Error occurred while checking Redis health): {str(e)}'
            result['details'] = {'error': str(e)}
        
        self.results['checks'][check_name] = result
        return result
    
    def check_system_resources(self) -> Dict[str, Any]:
        """检查系统资源 (Check system resources)"""
        check_name = 'system_resources'
        result = {
            'status': 'unknown',
            'message': '',
            'details': {},
            'response_time': 0
        }
        
        try:
            start_time = time.time()
            
            # 获取系统资源信息 (Get system resource information)
            cpu_percent = psutil.cpu_percent(interval=1)
            memory = psutil.virtual_memory()
            disk = psutil.disk_usage('/')
            load_avg = psutil.getloadavg()
            
            response_time = time.time() - start_time
            result['response_time'] = round(response_time * 1000, 2)  # ms
            
            # 检查资源使用情况 (Check resource usage)
            alerts = []
            
            if cpu_percent > 90:
                alerts.append(f'CPU 使用率过高: {cpu_percent:.1f}% (High CPU usage: {cpu_percent:.1f}%)')
            
            if memory.percent > 90:
                alerts.append(f'内存使用率过高: {memory.percent:.1f}% (High memory usage: {memory.percent:.1f}%)')
            
            if disk.percent > 90:
                alerts.append(f'磁盘使用率过高: {disk.percent:.1f}% (High disk usage: {disk.percent:.1f}%)')
            
            if load_avg[0] > psutil.cpu_count() * 2:
                alerts.append(f'系统负载过高: {load_avg[0]:.2f} (High system load: {load_avg[0]:.2f})')
            
            result['details'] = {
                'cpu_percent': round(cpu_percent, 2),
                'memory_percent': round(memory.percent, 2),
                'memory_available_gb': round(memory.available / (1024**3), 2),
                'disk_percent': round(disk.percent, 2),
                'disk_free_gb': round(disk.free / (1024**3), 2),
                'load_average': [round(x, 2) for x in load_avg],
                'cpu_count': psutil.cpu_count()
            }
            
            if alerts:
                result['status'] = 'warning'
                result['message'] = f'系统资源告警 (System resource alerts): {"; ".join(alerts)}'
            else:
                result['status'] = 'healthy'
                result['message'] = '系统资源正常 (System resources are normal)'
        
        except Exception as e:
            result['status'] = 'error'
            result['message'] = f'检查系统资源时发生错误 (Error occurred while checking system resources): {str(e)}'
            result['details'] = {'error': str(e)}
        
        self.results['checks'][check_name] = result
        return result
    
    def run_all_checks(self) -> Dict[str, Any]:
        """运行所有健康检查 (Run all health checks)"""
        self.logger.info("开始运行健康检查 (Starting health checks)")
        
        # 运行各项检查 (Run individual checks)
        self.check_synapse_health()
        self.check_database_health()
        self.check_redis_health()
        self.check_system_resources()
        
        # 计算总体状态 (Calculate overall status)
        statuses = [check['status'] for check in self.results['checks'].values()]
        
        if 'error' in statuses or 'unhealthy' in statuses:
            self.results['overall_status'] = 'unhealthy'
        elif 'warning' in statuses:
            self.results['overall_status'] = 'warning'
        elif all(status in ['healthy', 'skipped'] for status in statuses):
            self.results['overall_status'] = 'healthy'
        else:
            self.results['overall_status'] = 'unknown'
        
        self.logger.info(f"健康检查完成，总体状态: {self.results['overall_status']} (Health checks completed, overall status: {self.results['overall_status']})")
        
        return self.results


def load_config(config_path: Optional[str] = None) -> Dict[str, Any]:
    """加载配置文件 (Load configuration file)"""
    default_config = {
        'synapse': {
            'url': 'http://synapse:8008',
            'health_endpoint': '/health',
            'timeout': 30
        },
        'database': {
            'enabled': True,
            'host': 'postgres',
            'port': 5432,
            'database': 'synapse',
            'user': 'synapse_user',
            'password': '',
            'timeout': 30
        },
        'redis': {
            'enabled': True,
            'host': 'redis',
            'port': 6379,
            'password': '',
            'timeout': 30
        }
    }
    
    if config_path:
        try:
            import yaml
            with open(config_path, 'r', encoding='utf-8') as f:
                file_config = yaml.safe_load(f)
                # 合并配置 (Merge configuration)
                for key, value in file_config.items():
                    if isinstance(value, dict) and key in default_config:
                        default_config[key].update(value)
                    else:
                        default_config[key] = value
        except Exception as e:
            print(f"警告: 无法加载配置文件 {config_path}: {e} (Warning: Cannot load config file {config_path}: {e})")
    
    return default_config


def main():
    """主函数 (Main function)"""
    parser = argparse.ArgumentParser(
        description='Synapse 健康检查工具 (Synapse Health Check Tool)'
    )
    parser.add_argument(
        '--config', '-c',
        help='配置文件路径 (Configuration file path)'
    )
    parser.add_argument(
        '--output', '-o',
        choices=['json', 'text'],
        default='json',
        help='输出格式 (Output format)'
    )
    parser.add_argument(
        '--exit-code',
        action='store_true',
        help='根据健康状态设置退出码 (Set exit code based on health status)'
    )
    
    args = parser.parse_args()
    
    # 加载配置 (Load configuration)
    config = load_config(args.config)
    
    # 运行健康检查 (Run health checks)
    checker = HealthChecker(config)
    results = checker.run_all_checks()
    
    # 输出结果 (Output results)
    if args.output == 'json':
        print(json.dumps(results, indent=2, ensure_ascii=False))
    else:
        print(f"\n=== Synapse 健康检查报告 (Synapse Health Check Report) ===")
        print(f"时间 (Time): {time.strftime('%Y-%m-%d %H:%M:%S', time.localtime(results['timestamp']))}")
        print(f"总体状态 (Overall Status): {results['overall_status']}")
        print("\n详细检查结果 (Detailed Check Results):")
        
        for check_name, check_result in results['checks'].items():
            status_icon = {
                'healthy': '✅',
                'warning': '⚠️',
                'unhealthy': '❌',
                'error': '💥',
                'skipped': '⏭️',
                'unknown': '❓'
            }.get(check_result['status'], '❓')
            
            print(f"\n{status_icon} {check_name}: {check_result['status']}")
            print(f"   消息 (Message): {check_result['message']}")
            if check_result['response_time'] > 0:
                print(f"   响应时间 (Response Time): {check_result['response_time']}ms")
            
            if check_result['details']:
                print(f"   详细信息 (Details): {json.dumps(check_result['details'], indent=4, ensure_ascii=False)}")
    
    # 设置退出码 (Set exit code)
    if args.exit_code:
        if results['overall_status'] == 'healthy':
            sys.exit(0)
        elif results['overall_status'] == 'warning':
            sys.exit(1)
        else:
            sys.exit(2)


if __name__ == '__main__':
    main()