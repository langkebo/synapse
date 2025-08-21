# -*- coding: utf-8 -*-
"""
Synapse 中文错误处理模块

提供本地化的错误消息处理功能，支持多语言错误消息和格式化。
"""

import logging
import yaml
from typing import Dict, Any, Optional, Union
from pathlib import Path

logger = logging.getLogger(__name__)


class LocalizationError(Exception):
    """本地化相关错误"""
    pass


class ChineseErrorHandler:
    """
    中文错误处理器
    
    负责加载和管理本地化错误消息，提供错误消息的格式化和翻译功能。
    """
    
    def __init__(self, localization_dir: str = "/data/localization", default_language: str = "zh_CN"):
        """
        初始化错误处理器
        
        Args:
            localization_dir: 本地化文件目录
            default_language: 默认语言
        """
        self.localization_dir = Path(localization_dir)
        self.default_language = default_language
        self.fallback_language = "en_US"
        
        # 语言数据缓存
        self._language_cache: Dict[str, Dict[str, Any]] = {}
        
        # 错误代码映射
        self._error_code_mapping: Dict[str, str] = {}
        
        # 初始化
        self._load_error_mappings()
        self._load_default_language()
    
    def _load_error_mappings(self) -> None:
        """
        加载错误代码映射配置
        """
        try:
            config_file = self.localization_dir.parent / "localization_config.yaml"
            if config_file.exists():
                with open(config_file, 'r', encoding='utf-8') as f:
                    config = yaml.safe_load(f)
                    
                error_config = config.get('error_localization', {})
                self._error_code_mapping = error_config.get('error_code_mapping', {})
                
                logger.info(f"已加载 {len(self._error_code_mapping)} 个错误代码映射")
            else:
                logger.warning(f"本地化配置文件不存在: {config_file}")
                
        except Exception as e:
            logger.error(f"加载错误代码映射失败: {e}")
    
    def _load_default_language(self) -> None:
        """
        预加载默认语言
        """
        try:
            self._load_language(self.default_language)
            if self.default_language != self.fallback_language:
                self._load_language(self.fallback_language)
        except Exception as e:
            logger.error(f"加载默认语言失败: {e}")
    
    def _load_language(self, language: str) -> Dict[str, Any]:
        """
        加载指定语言的本地化数据
        
        Args:
            language: 语言代码
            
        Returns:
            语言数据字典
        """
        if language in self._language_cache:
            return self._language_cache[language]
        
        language_file = self.localization_dir / f"{language}.yaml"
        
        if not language_file.exists():
            if language == self.fallback_language:
                logger.error(f"回退语言文件不存在: {language_file}")
                return {}
            else:
                logger.warning(f"语言文件不存在: {language_file}，使用回退语言")
                return self._load_language(self.fallback_language)
        
        try:
            with open(language_file, 'r', encoding='utf-8') as f:
                language_data = yaml.safe_load(f) or {}
                self._language_cache[language] = language_data
                logger.debug(f"已加载语言文件: {language_file}")
                return language_data
                
        except Exception as e:
            logger.error(f"加载语言文件失败 {language_file}: {e}")
            if language != self.fallback_language:
                return self._load_language(self.fallback_language)
            return {}
    
    def _get_nested_value(self, data: Dict[str, Any], key_path: str) -> Optional[str]:
        """
        获取嵌套字典中的值
        
        Args:
            data: 数据字典
            key_path: 键路径，如 'friends.errors.not_found'
            
        Returns:
            对应的值，如果不存在返回 None
        """
        keys = key_path.split('.')
        current = data
        
        for key in keys:
            if isinstance(current, dict) and key in current:
                current = current[key]
            else:
                return None
        
        return current if isinstance(current, str) else None
    
    def get_error_message(
        self, 
        error_code: Union[str, int], 
        language: Optional[str] = None, 
        **kwargs
    ) -> str:
        """
        获取本地化的错误消息
        
        Args:
            error_code: 错误代码
            language: 语言代码，默认使用默认语言
            **kwargs: 消息格式化参数
            
        Returns:
            本地化的错误消息
        """
        if language is None:
            language = self.default_language
        
        # 获取语言数据
        language_data = self._load_language(language)
        
        # 查找错误消息键
        error_key = self._error_code_mapping.get(str(error_code))
        
        if not error_key:
            # 如果没有映射，尝试直接使用错误代码作为键
            error_key = f"errors.{error_code}"
        
        # 获取错误消息
        message = self._get_nested_value(language_data, error_key)
        
        if not message:
            # 尝试从回退语言获取
            if language != self.fallback_language:
                fallback_data = self._load_language(self.fallback_language)
                message = self._get_nested_value(fallback_data, error_key)
            
            # 如果仍然没有找到，使用默认消息
            if not message:
                message = f"未知错误 (代码: {error_code})"
                logger.warning(f"未找到错误消息: {error_code} -> {error_key}")
        
        # 格式化消息
        try:
            if kwargs:
                message = message.format(**kwargs)
        except (KeyError, ValueError) as e:
            logger.warning(f"错误消息格式化失败: {e}")
        
        return message
    
    def get_success_message(
        self, 
        message_key: str, 
        language: Optional[str] = None, 
        **kwargs
    ) -> str:
        """
        获取成功消息
        
        Args:
            message_key: 消息键
            language: 语言代码
            **kwargs: 格式化参数
            
        Returns:
            本地化的成功消息
        """
        if language is None:
            language = self.default_language
        
        language_data = self._load_language(language)
        message = self._get_nested_value(language_data, f"friends.messages.{message_key}")
        
        if not message and language != self.fallback_language:
            fallback_data = self._load_language(self.fallback_language)
            message = self._get_nested_value(fallback_data, f"friends.messages.{message_key}")
        
        if not message:
            message = message_key.replace('_', ' ').title()
        
        try:
            if kwargs:
                message = message.format(**kwargs)
        except (KeyError, ValueError) as e:
            logger.warning(f"成功消息格式化失败: {e}")
        
        return message
    
    def get_label(
        self, 
        label_key: str, 
        language: Optional[str] = None
    ) -> str:
        """
        获取界面标签
        
        Args:
            label_key: 标签键
            language: 语言代码
            
        Returns:
            本地化的标签
        """
        if language is None:
            language = self.default_language
        
        language_data = self._load_language(language)
        label = self._get_nested_value(language_data, f"friends.labels.{label_key}")
        
        if not label and language != self.fallback_language:
            fallback_data = self._load_language(self.fallback_language)
            label = self._get_nested_value(fallback_data, f"friends.labels.{label_key}")
        
        if not label:
            label = label_key.replace('_', ' ').title()
        
        return label
    
    def format_time_ago(self, seconds: int, language: Optional[str] = None) -> str:
        """
        格式化时间差显示
        
        Args:
            seconds: 秒数
            language: 语言代码
            
        Returns:
            格式化的时间显示
        """
        if language is None:
            language = self.default_language
        
        language_data = self._load_language(language)
        
        if seconds < 60:
            return self._get_nested_value(language_data, "friends.labels.just_now") or "刚刚"
        elif seconds < 3600:
            minutes = seconds // 60
            template = self._get_nested_value(language_data, "friends.labels.minutes_ago") or "{minutes}分钟前"
            return template.format(minutes=minutes)
        elif seconds < 86400:
            hours = seconds // 3600
            template = self._get_nested_value(language_data, "friends.labels.hours_ago") or "{hours}小时前"
            return template.format(hours=hours)
        elif seconds < 604800:
            days = seconds // 86400
            template = self._get_nested_value(language_data, "friends.labels.days_ago") or "{days}天前"
            return template.format(days=days)
        elif seconds < 2592000:
            weeks = seconds // 604800
            template = self._get_nested_value(language_data, "friends.labels.weeks_ago") or "{weeks}周前"
            return template.format(weeks=weeks)
        else:
            months = seconds // 2592000
            template = self._get_nested_value(language_data, "friends.labels.months_ago") or "{months}个月前"
            return template.format(months=months)
    
    def create_error_response(
        self, 
        error_code: Union[str, int], 
        http_status: int = 400,
        language: Optional[str] = None,
        **kwargs
    ) -> Dict[str, Any]:
        """
        创建标准化的错误响应
        
        Args:
            error_code: 错误代码
            http_status: HTTP状态码
            language: 语言代码
            **kwargs: 格式化参数
            
        Returns:
            错误响应字典
        """
        message = self.get_error_message(error_code, language, **kwargs)
        
        response = {
            "errcode": str(error_code),
            "error": message,
            "success": False
        }
        
        # 添加额外信息
        if kwargs:
            response["details"] = kwargs
        
        return response
    
    def create_success_response(
        self, 
        message_key: str, 
        data: Optional[Dict[str, Any]] = None,
        language: Optional[str] = None,
        **kwargs
    ) -> Dict[str, Any]:
        """
        创建标准化的成功响应
        
        Args:
            message_key: 消息键
            data: 响应数据
            language: 语言代码
            **kwargs: 格式化参数
            
        Returns:
            成功响应字典
        """
        message = self.get_success_message(message_key, language, **kwargs)
        
        response = {
            "message": message,
            "success": True
        }
        
        if data is not None:
            response["data"] = data
        
        return response
    
    def reload_languages(self) -> None:
        """
        重新加载所有语言文件
        """
        self._language_cache.clear()
        self._load_error_mappings()
        self._load_default_language()
        logger.info("已重新加载所有语言文件")
    
    def get_supported_languages(self) -> list:
        """
        获取支持的语言列表
        
        Returns:
            支持的语言代码列表
        """
        languages = []
        
        if self.localization_dir.exists():
            for file_path in self.localization_dir.glob("*.yaml"):
                if file_path.stem not in ['config', 'template']:
                    languages.append(file_path.stem)
        
        return sorted(languages)
    
    def validate_language_file(self, language: str) -> Dict[str, Any]:
        """
        验证语言文件的完整性
        
        Args:
            language: 语言代码
            
        Returns:
            验证结果
        """
        result = {
            "language": language,
            "valid": False,
            "errors": [],
            "warnings": [],
            "stats": {}
        }
        
        try:
            language_data = self._load_language(language)
            
            if not language_data:
                result["errors"].append("语言文件为空或无法加载")
                return result
            
            # 检查必需的部分
            required_sections = ['friends', 'system', 'api']
            for section in required_sections:
                if section not in language_data:
                    result["warnings"].append(f"缺少必需部分: {section}")
            
            # 统计翻译数量
            def count_translations(data, prefix=""):
                count = 0
                if isinstance(data, dict):
                    for key, value in data.items():
                        if isinstance(value, str):
                            count += 1
                        elif isinstance(value, dict):
                            count += count_translations(value, f"{prefix}.{key}" if prefix else key)
                return count
            
            result["stats"]["total_translations"] = count_translations(language_data)
            result["valid"] = len(result["errors"]) == 0
            
        except Exception as e:
            result["errors"].append(f"验证失败: {str(e)}")
        
        return result


# 全局错误处理器实例
_error_handler: Optional[ChineseErrorHandler] = None


def get_error_handler() -> ChineseErrorHandler:
    """
    获取全局错误处理器实例
    
    Returns:
        错误处理器实例
    """
    global _error_handler
    
    if _error_handler is None:
        _error_handler = ChineseErrorHandler()
    
    return _error_handler


def init_error_handler(localization_dir: str, default_language: str = "zh_CN") -> ChineseErrorHandler:
    """
    初始化全局错误处理器
    
    Args:
        localization_dir: 本地化文件目录
        default_language: 默认语言
        
    Returns:
        错误处理器实例
    """
    global _error_handler
    
    _error_handler = ChineseErrorHandler(localization_dir, default_language)
    return _error_handler


# 便捷函数
def get_error_message(error_code: Union[str, int], language: Optional[str] = None, **kwargs) -> str:
    """
    获取错误消息的便捷函数
    """
    return get_error_handler().get_error_message(error_code, language, **kwargs)


def get_success_message(message_key: str, language: Optional[str] = None, **kwargs) -> str:
    """
    获取成功消息的便捷函数
    """
    return get_error_handler().get_success_message(message_key, language, **kwargs)


def create_error_response(error_code: Union[str, int], http_status: int = 400, language: Optional[str] = None, **kwargs) -> Dict[str, Any]:
    """
    创建错误响应的便捷函数
    """
    return get_error_handler().create_error_response(error_code, http_status, language, **kwargs)


def create_success_response(message_key: str, data: Optional[Dict[str, Any]] = None, language: Optional[str] = None, **kwargs) -> Dict[str, Any]:
    """
    创建成功响应的便捷函数
    """
    return get_error_handler().create_success_response(message_key, data, language, **kwargs)