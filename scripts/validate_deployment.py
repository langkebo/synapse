#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Synapse2 部署验证脚本
用于验证部署配置和功能的完整性
"""

import os
import sys
import json
import yaml
import subprocess
from pathlib import Path
from typing import Dict, List, Any, Optional
import logging

# 配置日志
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s',
    handlers=[
        logging.StreamHandler(),
        logging.FileHandler('deployment_validation.log')
    ]
)
logger = logging.getLogger(__name__)


class DeploymentValidator:
    """部署验证器"""
    
    def __init__(self, project_root: str):
        self.project_root = Path(project_root)
        self.validation_results = []
    
    def validate_project_structure(self) -> bool:
        """验证项目结构"""
        logger.info("验证项目结构...")
        
        required_files = [
            "homeserver.yaml",
            "docker-compose.yml",
            "docker-compose.low-spec.yml",
            "Dockerfile.low-spec",
            "scripts/quick_deploy.sh",
            "scripts/setup_environment.sh",
            "config/performance.yaml",
            "config/logging.yaml",
            "config/chinese_localization.yaml",
            "synapse/handlers/friends.py",
            "synapse/storage/databases/main/friends.py",
            "synapse/rest/client/friends.py"
        ]
        
        required_dirs = [
            "synapse/handlers",
            "synapse/storage/databases/main",
            "synapse/rest/client",
            "config",
            "scripts",
            "docker",
            "migrations",
            "tests"
        ]
        
        missing_files = []
        missing_dirs = []
        
        # 检查文件
        for file_path in required_files:
            full_path = self.project_root / file_path
            if not full_path.exists():
                missing_files.append(file_path)
            else:
                logger.info(f"✓ 找到文件: {file_path}")
        
        # 检查目录
        for dir_path in required_dirs:
            full_path = self.project_root / dir_path
            if not full_path.exists():
                missing_dirs.append(dir_path)
            else:
                logger.info(f"✓ 找到目录: {dir_path}")
        
        if missing_files:
            logger.error(f"缺少文件: {missing_files}")
        
        if missing_dirs:
            logger.error(f"缺少目录: {missing_dirs}")
        
        success = len(missing_files) == 0 and len(missing_dirs) == 0
        self.validation_results.append({
            "test": "project_structure",
            "success": success,
            "missing_files": missing_files,
            "missing_dirs": missing_dirs
        })
        
        return success
    
    def validate_docker_configs(self) -> bool:
        """验证 Docker 配置"""
        logger.info("验证 Docker 配置...")
        
        configs_to_check = [
            "docker-compose.yml",
            "docker-compose.low-spec.yml",
            "Dockerfile.low-spec"
        ]
        
        valid_configs = []
        invalid_configs = []
        
        for config_file in configs_to_check:
            config_path = self.project_root / config_file
            
            if not config_path.exists():
                invalid_configs.append(f"{config_file}: 文件不存在")
                continue
            
            try:
                if config_file.endswith('.yml'):
                    with open(config_path, 'r', encoding='utf-8') as f:
                        yaml.safe_load(f)
                    logger.info(f"✓ {config_file} YAML 格式有效")
                    valid_configs.append(config_file)
                else:
                    # 检查 Dockerfile 基本语法
                    with open(config_path, 'r', encoding='utf-8') as f:
                        content = f.read()
                        if 'FROM' in content and 'RUN' in content:
                            logger.info(f"✓ {config_file} Dockerfile 格式有效")
                            valid_configs.append(config_file)
                        else:
                            invalid_configs.append(f"{config_file}: Dockerfile 格式无效")
            
            except Exception as e:
                invalid_configs.append(f"{config_file}: {str(e)}")
        
        success = len(invalid_configs) == 0
        self.validation_results.append({
            "test": "docker_configs",
            "success": success,
            "valid_configs": valid_configs,
            "invalid_configs": invalid_configs
        })
        
        return success
    
    def validate_yaml_configs(self) -> bool:
        """验证 YAML 配置文件"""
        logger.info("验证 YAML 配置文件...")
        
        yaml_files = [
            "homeserver.yaml",
            "config/performance.yaml",
            "config/logging.yaml",
            "config/chinese_localization.yaml"
        ]
        
        valid_yamls = []
        invalid_yamls = []
        
        for yaml_file in yaml_files:
            yaml_path = self.project_root / yaml_file
            
            if not yaml_path.exists():
                invalid_yamls.append(f"{yaml_file}: 文件不存在")
                continue
            
            try:
                with open(yaml_path, 'r', encoding='utf-8') as f:
                    config = yaml.safe_load(f)
                    
                    if config is None:
                        invalid_yamls.append(f"{yaml_file}: 空配置文件")
                        continue
                    
                    # 特定配置验证
                    if yaml_file == "homeserver.yaml":
                        required_keys = ['server_name', 'database', 'listeners']
                        missing_keys = [key for key in required_keys if key not in config]
                        if missing_keys:
                            invalid_yamls.append(f"{yaml_file}: 缺少必需键: {missing_keys}")
                            continue
                    
                    logger.info(f"✓ {yaml_file} 配置有效")
                    valid_yamls.append(yaml_file)
            
            except Exception as e:
                invalid_yamls.append(f"{yaml_file}: {str(e)}")
        
        success = len(invalid_yamls) == 0
        self.validation_results.append({
            "test": "yaml_configs",
            "success": success,
            "valid_yamls": valid_yamls,
            "invalid_yamls": invalid_yamls
        })
        
        return success
    
    def validate_python_syntax(self) -> bool:
        """验证 Python 代码语法"""
        logger.info("验证 Python 代码语法...")
        
        python_files = []
        
        # 查找所有 Python 文件
        for pattern in ['synapse/**/*.py', 'scripts/**/*.py', 'tests/**/*.py']:
            python_files.extend(self.project_root.glob(pattern))
        
        valid_files = []
        invalid_files = []
        
        for py_file in python_files:
            try:
                with open(py_file, 'r', encoding='utf-8') as f:
                    content = f.read()
                
                # 编译检查语法
                compile(content, str(py_file), 'exec')
                valid_files.append(str(py_file.relative_to(self.project_root)))
                
            except SyntaxError as e:
                invalid_files.append(f"{py_file.relative_to(self.project_root)}: {str(e)}")
            except Exception as e:
                # 忽略编码等其他错误，只关注语法
                pass
        
        logger.info(f"✓ 验证了 {len(valid_files)} 个 Python 文件")
        
        if invalid_files:
            logger.error(f"语法错误的文件: {invalid_files}")
        
        success = len(invalid_files) == 0
        self.validation_results.append({
            "test": "python_syntax",
            "success": success,
            "valid_files_count": len(valid_files),
            "invalid_files": invalid_files
        })
        
        return success
    
    def validate_scripts_executable(self) -> bool:
        """验证脚本可执行性"""
        logger.info("验证脚本可执行性...")
        
        script_files = [
            "scripts/quick_deploy.sh",
            "scripts/setup_environment.sh",
            "scripts/run_tests.py"
        ]
        
        executable_scripts = []
        non_executable_scripts = []
        
        for script_file in script_files:
            script_path = self.project_root / script_file
            
            if not script_path.exists():
                non_executable_scripts.append(f"{script_file}: 文件不存在")
                continue
            
            if os.access(script_path, os.X_OK):
                logger.info(f"✓ {script_file} 可执行")
                executable_scripts.append(script_file)
            else:
                non_executable_scripts.append(f"{script_file}: 不可执行")
        
        success = len(non_executable_scripts) == 0
        self.validation_results.append({
            "test": "scripts_executable",
            "success": success,
            "executable_scripts": executable_scripts,
            "non_executable_scripts": non_executable_scripts
        })
        
        return success
    
    def validate_database_migrations(self) -> bool:
        """验证数据库迁移文件"""
        logger.info("验证数据库迁移文件...")
        
        migrations_dir = self.project_root / "migrations"
        
        if not migrations_dir.exists():
            logger.error("migrations 目录不存在")
            self.validation_results.append({
                "test": "database_migrations",
                "success": False,
                "error": "migrations 目录不存在"
            })
            return False
        
        sql_files = list(migrations_dir.glob("*.sql"))
        
        if not sql_files:
            logger.warning("没有找到 SQL 迁移文件")
        
        valid_migrations = []
        invalid_migrations = []
        
        for sql_file in sql_files:
            try:
                with open(sql_file, 'r', encoding='utf-8') as f:
                    content = f.read().strip()
                    
                    if not content:
                        invalid_migrations.append(f"{sql_file.name}: 空文件")
                        continue
                    
                    # 基本 SQL 语法检查
                    if any(keyword in content.upper() for keyword in ['CREATE', 'ALTER', 'INSERT', 'UPDATE']):
                        logger.info(f"✓ {sql_file.name} 包含有效的 SQL 语句")
                        valid_migrations.append(sql_file.name)
                    else:
                        invalid_migrations.append(f"{sql_file.name}: 不包含有效的 SQL 语句")
            
            except Exception as e:
                invalid_migrations.append(f"{sql_file.name}: {str(e)}")
        
        success = len(invalid_migrations) == 0
        self.validation_results.append({
            "test": "database_migrations",
            "success": success,
            "valid_migrations": valid_migrations,
            "invalid_migrations": invalid_migrations,
            "total_migrations": len(sql_files)
        })
        
        return success
    
    def run_all_validations(self) -> Dict[str, Any]:
        """运行所有验证"""
        logger.info("开始部署验证...")
        
        validations = [
            ("项目结构", self.validate_project_structure),
            ("Docker 配置", self.validate_docker_configs),
            ("YAML 配置", self.validate_yaml_configs),
            ("Python 语法", self.validate_python_syntax),
            ("脚本可执行性", self.validate_scripts_executable),
            ("数据库迁移", self.validate_database_migrations)
        ]
        
        passed_tests = 0
        total_tests = len(validations)
        
        for test_name, validation_func in validations:
            logger.info(f"\n{'='*50}")
            logger.info(f"运行验证: {test_name}")
            logger.info(f"{'='*50}")
            
            try:
                if validation_func():
                    logger.info(f"✓ {test_name} 验证通过")
                    passed_tests += 1
                else:
                    logger.error(f"✗ {test_name} 验证失败")
            except Exception as e:
                logger.error(f"✗ {test_name} 验证出错: {str(e)}")
        
        # 生成总结报告
        success_rate = (passed_tests / total_tests) * 100
        
        summary = {
            "total_tests": total_tests,
            "passed_tests": passed_tests,
            "failed_tests": total_tests - passed_tests,
            "success_rate": success_rate,
            "overall_success": passed_tests == total_tests,
            "detailed_results": self.validation_results
        }
        
        logger.info(f"\n{'='*60}")
        logger.info("验证总结")
        logger.info(f"{'='*60}")
        logger.info(f"总测试数: {total_tests}")
        logger.info(f"通过测试: {passed_tests}")
        logger.info(f"失败测试: {total_tests - passed_tests}")
        logger.info(f"成功率: {success_rate:.1f}%")
        
        if summary["overall_success"]:
            logger.info("🎉 所有验证都通过了！部署配置看起来很好。")
        else:
            logger.warning("⚠️  有一些验证失败，请检查上面的详细信息。")
        
        return summary
    
    def save_report(self, filename: str = "deployment_validation_report.json"):
        """保存验证报告"""
        report_path = self.project_root / filename
        
        summary = {
            "validation_timestamp": str(Path().cwd()),
            "project_root": str(self.project_root),
            "results": self.validation_results
        }
        
        with open(report_path, 'w', encoding='utf-8') as f:
            json.dump(summary, f, indent=2, ensure_ascii=False)
        
        logger.info(f"验证报告已保存到: {report_path}")


def main():
    """主函数"""
    import argparse
    
    parser = argparse.ArgumentParser(description="Synapse2 部署验证工具")
    parser.add_argument(
        "--project-root",
        default=".",
        help="项目根目录路径 (默认: 当前目录)"
    )
    parser.add_argument(
        "--save-report",
        action="store_true",
        help="保存验证报告到 JSON 文件"
    )
    parser.add_argument(
        "--report-file",
        default="deployment_validation_report.json",
        help="报告文件名 (默认: deployment_validation_report.json)"
    )
    
    args = parser.parse_args()
    
    # 创建验证器
    validator = DeploymentValidator(args.project_root)
    
    # 运行验证
    summary = validator.run_all_validations()
    
    # 保存报告
    if args.save_report:
        validator.save_report(args.report_file)
    
    # 退出码
    sys.exit(0 if summary["overall_success"] else 1)


if __name__ == "__main__":
    main()