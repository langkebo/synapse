#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Synapse 告警管理器 (Synapse Alert Manager)

用于管理和发送 Synapse 服务的告警通知
Used to manage and send alert notifications for Synapse service

功能 (Features):
- 多种告警通道 (Multiple alert channels)
- 告警级别管理 (Alert level management)
- 告警去重和聚合 (Alert deduplication and aggregation)
- 告警历史记录 (Alert history tracking)
- 自定义告警规则 (Custom alert rules)
"""

import os
import sys
import time
import json
import logging
import hashlib
import argparse
import requests
import smtplib
from datetime import datetime, timedelta
from typing import Dict, List, Any, Optional, Set
from email.mime.text import MIMEText
from email.mime.multipart import MIMEMultipart
from dataclasses import dataclass, asdict
from pathlib import Path


@dataclass
class Alert:
    """告警数据类 (Alert data class)"""
    id: str
    level: str  # info, warning, critical
    title: str
    message: str
    source: str
    timestamp: float
    details: Dict[str, Any]
    resolved: bool = False
    resolved_at: Optional[float] = None
    
    def to_dict(self) -> Dict[str, Any]:
        """转换为字典 (Convert to dictionary)"""
        return asdict(self)
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'Alert':
        """从字典创建 (Create from dictionary)"""
        return cls(**data)
    
    def get_hash(self) -> str:
        """获取告警哈希值用于去重 (Get alert hash for deduplication)"""
        content = f"{self.source}:{self.title}:{self.level}"
        return hashlib.md5(content.encode()).hexdigest()


class AlertManager:
    """告警管理器类 (Alert Manager Class)"""
    
    def __init__(self, config: Dict[str, Any]):
        """初始化告警管理器 (Initialize alert manager)"""
        self.config = config
        self.logger = self._setup_logger()
        self.alerts_history: List[Alert] = []
        self.active_alerts: Dict[str, Alert] = {}
        self.alert_counts: Dict[str, int] = {}
        self.last_cleanup = time.time()
        
        # 加载历史告警 (Load alert history)
        self._load_alert_history()
    
    def _setup_logger(self) -> logging.Logger:
        """设置日志记录器 (Setup logger)"""
        logger = logging.getLogger('alert_manager')
        logger.setLevel(logging.INFO)
        
        if not logger.handlers:
            # 控制台处理器 (Console handler)
            console_handler = logging.StreamHandler()
            console_formatter = logging.Formatter(
                '%(asctime)s - %(name)s - %(levelname)s - %(message)s'
            )
            console_handler.setFormatter(console_formatter)
            logger.addHandler(console_handler)
            
            # 文件处理器 (File handler)
            log_config = self.config.get('logging', {})
            if log_config.get('file'):
                try:
                    log_dir = Path(log_config['file']).parent
                    log_dir.mkdir(parents=True, exist_ok=True)
                    
                    file_handler = logging.FileHandler(log_config['file'])
                    file_formatter = logging.Formatter(
                        '%(asctime)s - %(name)s - %(levelname)s - %(message)s'
                    )
                    file_handler.setFormatter(file_formatter)
                    logger.addHandler(file_handler)
                except Exception as e:
                    logger.warning(f"无法设置文件日志: {e} (Cannot setup file logging: {e})")
        
        return logger
    
    def _load_alert_history(self):
        """加载告警历史 (Load alert history)"""
        history_file = self.config.get('history_file', '/data/alerts_history.json')
        
        try:
            if os.path.exists(history_file):
                with open(history_file, 'r', encoding='utf-8') as f:
                    history_data = json.load(f)
                    self.alerts_history = [
                        Alert.from_dict(alert_data) 
                        for alert_data in history_data
                    ]
                    
                    # 恢复活跃告警 (Restore active alerts)
                    for alert in self.alerts_history:
                        if not alert.resolved:
                            self.active_alerts[alert.get_hash()] = alert
                    
                    self.logger.info(f"加载了 {len(self.alerts_history)} 条历史告警 (Loaded {len(self.alerts_history)} historical alerts)")
        except Exception as e:
            self.logger.warning(f"加载告警历史失败: {e} (Failed to load alert history: {e})")
    
    def _save_alert_history(self):
        """保存告警历史 (Save alert history)"""
        history_file = self.config.get('history_file', '/data/alerts_history.json')
        
        try:
            # 创建目录 (Create directory)
            Path(history_file).parent.mkdir(parents=True, exist_ok=True)
            
            # 清理过期历史 (Clean expired history)
            retention_days = self.config.get('retention_days', 30)
            cutoff_time = time.time() - (retention_days * 24 * 3600)
            
            self.alerts_history = [
                alert for alert in self.alerts_history
                if alert.timestamp > cutoff_time
            ]
            
            # 保存到文件 (Save to file)
            with open(history_file, 'w', encoding='utf-8') as f:
                json.dump(
                    [alert.to_dict() for alert in self.alerts_history],
                    f,
                    indent=2,
                    ensure_ascii=False
                )
        except Exception as e:
            self.logger.error(f"保存告警历史失败: {e} (Failed to save alert history: {e})")
    
    def create_alert(
        self,
        level: str,
        title: str,
        message: str,
        source: str,
        details: Optional[Dict[str, Any]] = None
    ) -> Alert:
        """创建新告警 (Create new alert)"""
        alert = Alert(
            id=f"{source}_{int(time.time() * 1000)}",
            level=level.lower(),
            title=title,
            message=message,
            source=source,
            timestamp=time.time(),
            details=details or {}
        )
        
        return alert
    
    def process_alert(self, alert: Alert) -> bool:
        """处理告警 (Process alert)"""
        alert_hash = alert.get_hash()
        
        # 检查是否为重复告警 (Check for duplicate alerts)
        if alert_hash in self.active_alerts:
            # 更新计数 (Update count)
            self.alert_counts[alert_hash] = self.alert_counts.get(alert_hash, 0) + 1
            
            # 如果是严重告警或计数达到阈值，重新发送 (Resend if critical or count threshold reached)
            if alert.level == 'critical' or self.alert_counts[alert_hash] % 10 == 0:
                self.logger.info(f"重复告警达到阈值，重新发送: {alert.title} (Duplicate alert reached threshold, resending: {alert.title})")
                return self._send_alert(alert)
            else:
                self.logger.debug(f"忽略重复告警: {alert.title} (Ignoring duplicate alert: {alert.title})")
                return True
        
        # 新告警处理 (New alert processing)
        self.active_alerts[alert_hash] = alert
        self.alert_counts[alert_hash] = 1
        self.alerts_history.append(alert)
        
        self.logger.info(f"处理新告警: [{alert.level.upper()}] {alert.title} (Processing new alert: [{alert.level.upper()}] {alert.title})")
        
        # 发送告警 (Send alert)
        success = self._send_alert(alert)
        
        # 保存历史 (Save history)
        self._save_alert_history()
        
        return success
    
    def resolve_alert(self, alert_hash: str, message: str = "") -> bool:
        """解决告警 (Resolve alert)"""
        if alert_hash in self.active_alerts:
            alert = self.active_alerts[alert_hash]
            alert.resolved = True
            alert.resolved_at = time.time()
            
            if message:
                alert.details['resolution_message'] = message
            
            # 从活跃告警中移除 (Remove from active alerts)
            del self.active_alerts[alert_hash]
            
            # 发送解决通知 (Send resolution notification)
            resolution_alert = self.create_alert(
                level='info',
                title=f"告警已解决: {alert.title} (Alert Resolved: {alert.title})",
                message=f"告警已解决: {message} (Alert resolved: {message})",
                source=alert.source,
                details={'original_alert_id': alert.id, 'resolution_message': message}
            )
            
            self.logger.info(f"告警已解决: {alert.title} (Alert resolved: {alert.title})")
            
            # 保存历史 (Save history)
            self._save_alert_history()
            
            return self._send_alert(resolution_alert)
        
        return False
    
    def _send_alert(self, alert: Alert) -> bool:
        """发送告警 (Send alert)"""
        success = True
        channels = self.config.get('channels', {})
        
        # 检查告警级别过滤 (Check alert level filtering)
        min_level = self.config.get('min_level', 'info')
        level_priority = {'info': 0, 'warning': 1, 'critical': 2}
        
        if level_priority.get(alert.level, 0) < level_priority.get(min_level, 0):
            self.logger.debug(f"告警级别过低，跳过发送: {alert.title} (Alert level too low, skipping: {alert.title})")
            return True
        
        # 日志通道 (Log channel)
        if channels.get('log', {}).get('enabled', True):
            self._send_to_log(alert)
        
        # 文件通道 (File channel)
        if channels.get('file', {}).get('enabled', False):
            success &= self._send_to_file(alert)
        
        # Webhook 通道 (Webhook channel)
        if channels.get('webhook', {}).get('enabled', False):
            success &= self._send_to_webhook(alert)
        
        # 邮件通道 (Email channel)
        if channels.get('email', {}).get('enabled', False):
            success &= self._send_to_email(alert)
        
        return success
    
    def _send_to_log(self, alert: Alert):
        """发送到日志 (Send to log)"""
        level_map = {
            'info': logging.INFO,
            'warning': logging.WARNING,
            'critical': logging.CRITICAL
        }
        
        log_level = level_map.get(alert.level, logging.INFO)
        self.logger.log(log_level, f"[{alert.source}] {alert.title}: {alert.message}")
    
    def _send_to_file(self, alert: Alert) -> bool:
        """发送到文件 (Send to file)"""
        try:
            file_config = self.config.get('channels', {}).get('file', {})
            file_path = file_config.get('path', '/logs/alerts.log')
            
            # 创建目录 (Create directory)
            Path(file_path).parent.mkdir(parents=True, exist_ok=True)
            
            # 格式化告警信息 (Format alert message)
            timestamp = datetime.fromtimestamp(alert.timestamp).strftime('%Y-%m-%d %H:%M:%S')
            alert_line = f"[{timestamp}] [{alert.level.upper()}] [{alert.source}] {alert.title}: {alert.message}\n"
            
            # 写入文件 (Write to file)
            with open(file_path, 'a', encoding='utf-8') as f:
                f.write(alert_line)
            
            return True
        except Exception as e:
            self.logger.error(f"发送告警到文件失败: {e} (Failed to send alert to file: {e})")
            return False
    
    def _send_to_webhook(self, alert: Alert) -> bool:
        """发送到 Webhook (Send to webhook)"""
        try:
            webhook_config = self.config.get('channels', {}).get('webhook', {})
            url = webhook_config.get('url')
            timeout = webhook_config.get('timeout', 10)
            headers = webhook_config.get('headers', {'Content-Type': 'application/json'})
            
            if not url:
                self.logger.warning("Webhook URL 未配置 (Webhook URL not configured)")
                return False
            
            # 构建 Webhook 负载 (Build webhook payload)
            payload = {
                'alert': alert.to_dict(),
                'timestamp': datetime.fromtimestamp(alert.timestamp).isoformat(),
                'formatted_message': f"[{alert.level.upper()}] {alert.title}: {alert.message}"
            }
            
            # 发送请求 (Send request)
            response = requests.post(
                url,
                json=payload,
                headers=headers,
                timeout=timeout
            )
            
            if response.status_code == 200:
                self.logger.info(f"告警已发送到 Webhook: {alert.title} (Alert sent to webhook: {alert.title})")
                return True
            else:
                self.logger.error(f"Webhook 返回错误状态码: {response.status_code} (Webhook returned error status code: {response.status_code})")
                return False
        
        except Exception as e:
            self.logger.error(f"发送告警到 Webhook 失败: {e} (Failed to send alert to webhook: {e})")
            return False
    
    def _send_to_email(self, alert: Alert) -> bool:
        """发送到邮件 (Send to email)"""
        try:
            email_config = self.config.get('channels', {}).get('email', {})
            
            smtp_server = email_config.get('smtp_server')
            smtp_port = email_config.get('smtp_port', 587)
            username = email_config.get('username')
            password = email_config.get('password')
            from_addr = email_config.get('from_addr')
            to_addrs = email_config.get('to_addrs', [])
            
            if not all([smtp_server, username, password, from_addr, to_addrs]):
                self.logger.warning("邮件配置不完整 (Email configuration incomplete)")
                return False
            
            # 构建邮件 (Build email)
            msg = MIMEMultipart()
            msg['From'] = from_addr
            msg['To'] = ', '.join(to_addrs)
            msg['Subject'] = f"[Synapse Alert] [{alert.level.upper()}] {alert.title}"
            
            # 邮件正文 (Email body)
            body = f"""
Synapse 告警通知 (Synapse Alert Notification)

告警级别 (Alert Level): {alert.level.upper()}
告警来源 (Alert Source): {alert.source}
告警标题 (Alert Title): {alert.title}
告警消息 (Alert Message): {alert.message}
发生时间 (Timestamp): {datetime.fromtimestamp(alert.timestamp).strftime('%Y-%m-%d %H:%M:%S')}

详细信息 (Details):
{json.dumps(alert.details, indent=2, ensure_ascii=False)}

---
Synapse 监控系统 (Synapse Monitoring System)
"""
            
            msg.attach(MIMEText(body, 'plain', 'utf-8'))
            
            # 发送邮件 (Send email)
            with smtplib.SMTP(smtp_server, smtp_port) as server:
                server.starttls()
                server.login(username, password)
                server.send_message(msg)
            
            self.logger.info(f"告警邮件已发送: {alert.title} (Alert email sent: {alert.title})")
            return True
        
        except Exception as e:
            self.logger.error(f"发送告警邮件失败: {e} (Failed to send alert email: {e})")
            return False
    
    def cleanup_old_alerts(self):
        """清理过期告警 (Cleanup old alerts)"""
        current_time = time.time()
        
        # 每小时清理一次 (Cleanup every hour)
        if current_time - self.last_cleanup < 3600:
            return
        
        self.last_cleanup = current_time
        
        # 清理过期的活跃告警 (Cleanup expired active alerts)
        alert_timeout = self.config.get('alert_timeout', 24 * 3600)  # 24小时
        expired_hashes = [
            alert_hash for alert_hash, alert in self.active_alerts.items()
            if current_time - alert.timestamp > alert_timeout
        ]
        
        for alert_hash in expired_hashes:
            alert = self.active_alerts[alert_hash]
            self.logger.info(f"自动解决过期告警: {alert.title} (Auto-resolving expired alert: {alert.title})")
            self.resolve_alert(alert_hash, "告警超时自动解决 (Auto-resolved due to timeout)")
        
        self.logger.info(f"清理了 {len(expired_hashes)} 个过期告警 (Cleaned up {len(expired_hashes)} expired alerts)")
    
    def get_active_alerts(self) -> List[Alert]:
        """获取活跃告警列表 (Get active alerts list)"""
        return list(self.active_alerts.values())
    
    def get_alert_statistics(self) -> Dict[str, Any]:
        """获取告警统计信息 (Get alert statistics)"""
        current_time = time.time()
        
        # 最近24小时的告警 (Alerts in last 24 hours)
        recent_alerts = [
            alert for alert in self.alerts_history
            if current_time - alert.timestamp < 24 * 3600
        ]
        
        # 按级别统计 (Statistics by level)
        level_counts = {}
        for alert in recent_alerts:
            level_counts[alert.level] = level_counts.get(alert.level, 0) + 1
        
        # 按来源统计 (Statistics by source)
        source_counts = {}
        for alert in recent_alerts:
            source_counts[alert.source] = source_counts.get(alert.source, 0) + 1
        
        return {
            'total_alerts': len(self.alerts_history),
            'active_alerts': len(self.active_alerts),
            'recent_24h_alerts': len(recent_alerts),
            'level_counts': level_counts,
            'source_counts': source_counts,
            'last_alert_time': max([alert.timestamp for alert in self.alerts_history]) if self.alerts_history else None
        }


def load_config(config_path: Optional[str] = None) -> Dict[str, Any]:
    """加载配置文件 (Load configuration file)"""
    default_config = {
        'min_level': 'info',
        'retention_days': 30,
        'alert_timeout': 24 * 3600,
        'history_file': '/data/alerts_history.json',
        'logging': {
            'file': '/logs/alert_manager.log'
        },
        'channels': {
            'log': {
                'enabled': True
            },
            'file': {
                'enabled': True,
                'path': '/logs/alerts.log'
            },
            'webhook': {
                'enabled': False,
                'url': '',
                'timeout': 10,
                'headers': {
                    'Content-Type': 'application/json'
                }
            },
            'email': {
                'enabled': False,
                'smtp_server': '',
                'smtp_port': 587,
                'username': '',
                'password': '',
                'from_addr': '',
                'to_addrs': []
            }
        }
    }
    
    if config_path and os.path.exists(config_path):
        try:
            import yaml
            with open(config_path, 'r', encoding='utf-8') as f:
                file_config = yaml.safe_load(f)
                # 深度合并配置 (Deep merge configuration)
                def deep_merge(base, override):
                    for key, value in override.items():
                        if isinstance(value, dict) and key in base and isinstance(base[key], dict):
                            deep_merge(base[key], value)
                        else:
                            base[key] = value
                
                deep_merge(default_config, file_config)
        except Exception as e:
            print(f"警告: 无法加载配置文件 {config_path}: {e} (Warning: Cannot load config file {config_path}: {e})")
    
    return default_config


def main():
    """主函数 (Main function)"""
    parser = argparse.ArgumentParser(
        description='Synapse 告警管理器 (Synapse Alert Manager)'
    )
    parser.add_argument(
        '--config', '-c',
        help='配置文件路径 (Configuration file path)'
    )
    parser.add_argument(
        '--test-alert',
        action='store_true',
        help='发送测试告警 (Send test alert)'
    )
    parser.add_argument(
        '--stats',
        action='store_true',
        help='显示告警统计信息 (Show alert statistics)'
    )
    
    args = parser.parse_args()
    
    # 加载配置 (Load configuration)
    config = load_config(args.config)
    
    # 创建告警管理器 (Create alert manager)
    alert_manager = AlertManager(config)
    
    if args.test_alert:
        # 发送测试告警 (Send test alert)
        test_alert = alert_manager.create_alert(
            level='warning',
            title='测试告警 (Test Alert)',
            message='这是一个测试告警消息 (This is a test alert message)',
            source='alert_manager',
            details={'test': True, 'timestamp': time.time()}
        )
        
        success = alert_manager.process_alert(test_alert)
        print(f"测试告警发送{'成功' if success else '失败'} (Test alert {'successful' if success else 'failed'})")
    
    elif args.stats:
        # 显示统计信息 (Show statistics)
        stats = alert_manager.get_alert_statistics()
        print(json.dumps(stats, indent=2, ensure_ascii=False))
    
    else:
        print("请使用 --test-alert 或 --stats 参数 (Please use --test-alert or --stats parameter)")


if __name__ == '__main__':
    main()