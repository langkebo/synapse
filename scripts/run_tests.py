#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Copyright 2023 The Matrix.org Foundation C.I.C.
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

"""Synapse2 测试运行脚本"""

import argparse
import os
import sys
import subprocess
import time
from typing import List, Dict, Any, Optional
import json
import logging
from pathlib import Path

# 设置日志
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)


class TestRunner:
    """测试运行器"""
    
    def __init__(self, project_root: str):
        self.project_root = Path(project_root)
        self.tests_dir = self.project_root / "tests"
        self.results = {
            "total": 0,
            "passed": 0,
            "failed": 0,
            "skipped": 0,
            "errors": [],
            "start_time": None,
            "end_time": None,
            "duration": 0
        }
    
    def discover_tests(self, pattern: str = "test_*.py") -> List[str]:
        """发现测试文件"""
        test_files = []
        
        if self.tests_dir.exists():
            for test_file in self.tests_dir.glob(pattern):
                if test_file.is_file():
                    test_files.append(str(test_file.relative_to(self.project_root)))
        
        logger.info(f"发现 {len(test_files)} 个测试文件")
        return test_files
    
    def run_test_file(self, test_file: str, verbose: bool = False) -> Dict[str, Any]:
        """运行单个测试文件"""
        logger.info(f"运行测试文件: {test_file}")
        
        cmd = [
            sys.executable, "-m", "pytest", 
            test_file,
            "-v" if verbose else "-q",
            "--tb=short"
        ]
        
        # 生成 JSON 报告（如果可用）
        try:
            import pytest_json_report
            cmd.extend(["--json-report", "--json-report-file=/tmp/test_report.json"])
        except ImportError:
            logger.warning("pytest-json-report 未安装，跳过 JSON 报告生成")
        
        try:
            result = subprocess.run(
                cmd,
                cwd=self.project_root,
                capture_output=True,
                text=True,
                timeout=300  # 5分钟超时
            )
            
            # 解析测试结果
            test_result = {
                "file": test_file,
                "returncode": result.returncode,
                "stdout": result.stdout,
                "stderr": result.stderr,
                "passed": 0,
                "failed": 0,
                "skipped": 0,
                "duration": 0
            }
            
            # 尝试解析 JSON 报告
            try:
                if os.path.exists("/tmp/test_report.json"):
                    with open("/tmp/test_report.json", "r") as f:
                        json_report = json.load(f)
                    
                    test_result["passed"] = json_report.get("summary", {}).get("passed", 0)
                    test_result["failed"] = json_report.get("summary", {}).get("failed", 0)
                    test_result["skipped"] = json_report.get("summary", {}).get("skipped", 0)
                    test_result["duration"] = json_report.get("summary", {}).get("duration", 0)
                    
                    # 清理临时文件
                    os.remove("/tmp/test_report.json")
            except Exception as e:
                logger.warning(f"无法解析测试报告: {e}")
            
            return test_result
            
        except subprocess.TimeoutExpired:
            logger.error(f"测试文件 {test_file} 执行超时")
            return {
                "file": test_file,
                "returncode": -1,
                "stdout": "",
                "stderr": "测试执行超时",
                "passed": 0,
                "failed": 1,
                "skipped": 0,
                "duration": 300
            }
        except Exception as e:
            logger.error(f"运行测试文件 {test_file} 时出错: {e}")
            return {
                "file": test_file,
                "returncode": -1,
                "stdout": "",
                "stderr": str(e),
                "passed": 0,
                "failed": 1,
                "skipped": 0,
                "duration": 0
            }
    
    def run_all_tests(self, pattern: str = "test_*.py", verbose: bool = False) -> Dict[str, Any]:
        """运行所有测试"""
        self.results["start_time"] = time.time()
        
        test_files = self.discover_tests(pattern)
        
        if not test_files:
            logger.warning("没有找到测试文件")
            return self.results
        
        logger.info(f"开始运行 {len(test_files)} 个测试文件")
        
        for test_file in test_files:
            result = self.run_test_file(test_file, verbose)
            
            # 更新总体结果
            self.results["total"] += result["passed"] + result["failed"] + result["skipped"]
            self.results["passed"] += result["passed"]
            self.results["failed"] += result["failed"]
            self.results["skipped"] += result["skipped"]
            
            if result["returncode"] != 0:
                self.results["errors"].append({
                    "file": test_file,
                    "stderr": result["stderr"],
                    "stdout": result["stdout"]
                })
            
            # 显示进度
            if verbose:
                logger.info(f"  {test_file}: {result['passed']} 通过, {result['failed']} 失败, {result['skipped']} 跳过")
        
        self.results["end_time"] = time.time()
        self.results["duration"] = self.results["end_time"] - self.results["start_time"]
        
        return self.results
    
    def run_specific_tests(self, test_names: List[str], verbose: bool = False) -> Dict[str, Any]:
        """运行特定的测试"""
        self.results["start_time"] = time.time()
        
        logger.info(f"运行指定的测试: {', '.join(test_names)}")
        
        for test_name in test_names:
            # 查找测试文件
            test_file = None
            if test_name.endswith(".py"):
                test_file = test_name
            else:
                # 尝试查找匹配的测试文件
                for pattern in [f"test_{test_name}.py", f"{test_name}.py", f"*{test_name}*.py"]:
                    matches = list(self.tests_dir.glob(pattern))
                    if matches:
                        test_file = str(matches[0].relative_to(self.project_root))
                        break
            
            if not test_file:
                logger.error(f"找不到测试文件: {test_name}")
                self.results["errors"].append({
                    "file": test_name,
                    "stderr": f"找不到测试文件: {test_name}",
                    "stdout": ""
                })
                continue
            
            result = self.run_test_file(test_file, verbose)
            
            # 更新总体结果
            self.results["total"] += result["passed"] + result["failed"] + result["skipped"]
            self.results["passed"] += result["passed"]
            self.results["failed"] += result["failed"]
            self.results["skipped"] += result["skipped"]
            
            if result["returncode"] != 0:
                self.results["errors"].append({
                    "file": test_file,
                    "stderr": result["stderr"],
                    "stdout": result["stdout"]
                })
        
        self.results["end_time"] = time.time()
        self.results["duration"] = self.results["end_time"] - self.results["start_time"]
        
        return self.results
    
    def print_summary(self) -> None:
        """打印测试摘要"""
        print("\n" + "=" * 60)
        print("测试结果摘要")
        print("=" * 60)
        
        print(f"总测试数: {self.results['total']}")
        print(f"通过: {self.results['passed']}")
        print(f"失败: {self.results['failed']}")
        print(f"跳过: {self.results['skipped']}")
        print(f"执行时间: {self.results['duration']:.2f} 秒")
        
        if self.results["failed"] > 0:
            print(f"\n失败率: {(self.results['failed'] / self.results['total'] * 100):.1f}%")
        else:
            print("\n所有测试通过! 🎉")
        
        # 显示错误详情
        if self.results["errors"]:
            print("\n错误详情:")
            print("-" * 40)
            for error in self.results["errors"]:
                print(f"\n文件: {error['file']}")
                if error["stderr"]:
                    print(f"错误: {error['stderr'][:500]}..." if len(error["stderr"]) > 500 else f"错误: {error['stderr']}")
        
        print("=" * 60)
    
    def save_report(self, output_file: str) -> None:
        """保存测试报告"""
        try:
            with open(output_file, "w", encoding="utf-8") as f:
                json.dump(self.results, f, indent=2, ensure_ascii=False)
            logger.info(f"测试报告已保存到: {output_file}")
        except Exception as e:
            logger.error(f"保存测试报告失败: {e}")


def check_dependencies() -> bool:
    """检查测试依赖"""
    required_packages = ["pytest"]
    optional_packages = ["pytest-json-report"]
    missing_packages = []
    missing_optional = []
    
    for package in required_packages:
        try:
            __import__(package.replace("-", "_"))
        except ImportError:
            missing_packages.append(package)
    
    for package in optional_packages:
        try:
            __import__(package.replace("-", "_"))
        except ImportError:
            missing_optional.append(package)
    
    if missing_packages:
        logger.error(f"缺少必需的测试依赖: {', '.join(missing_packages)}")
        logger.info(f"请运行: pip install {' '.join(missing_packages)}")
        return False
    
    if missing_optional:
        logger.warning(f"缺少可选的测试依赖: {', '.join(missing_optional)}")
        logger.info(f"建议运行: pip install {' '.join(missing_optional)}")
    
    logger.info("必需的测试依赖已安装")
    return True


def setup_test_environment() -> bool:
    """设置测试环境"""
    try:
        # 设置环境变量
        os.environ["SYNAPSE_TEST_MODE"] = "1"
        os.environ["PYTHONPATH"] = os.pathsep.join([
            os.getcwd(),
            os.path.join(os.getcwd(), "synapse"),
            os.environ.get("PYTHONPATH", "")
        ])
        
        # 创建测试数据目录
        test_data_dir = Path("test_data")
        test_data_dir.mkdir(exist_ok=True)
        
        return True
    except Exception as e:
        logger.error(f"设置测试环境失败: {e}")
        return False


def main():
    """主函数"""
    parser = argparse.ArgumentParser(description="Synapse2 测试运行器")
    parser.add_argument(
        "--pattern", "-p",
        default="test_*.py",
        help="测试文件匹配模式 (默认: test_*.py)"
    )
    parser.add_argument(
        "--verbose", "-v",
        action="store_true",
        help="详细输出"
    )
    parser.add_argument(
        "--output", "-o",
        help="测试报告输出文件"
    )
    parser.add_argument(
        "--tests", "-t",
        nargs="+",
        help="运行特定的测试文件或测试名称"
    )
    parser.add_argument(
        "--check-deps",
        action="store_true",
        help="检查测试依赖"
    )
    parser.add_argument(
        "--setup-env",
        action="store_true",
        help="设置测试环境"
    )
    
    args = parser.parse_args()
    
    # 检查依赖
    if args.check_deps:
        if check_dependencies():
            print("所有测试依赖已安装")
            return 0
        else:
            return 1
    
    # 设置环境
    if args.setup_env:
        if setup_test_environment():
            print("测试环境设置完成")
            return 0
        else:
            return 1
    
    # 检查依赖
    if not check_dependencies():
        return 1
    
    # 设置环境
    if not setup_test_environment():
        return 1
    
    # 创建测试运行器
    project_root = os.getcwd()
    runner = TestRunner(project_root)
    
    try:
        # 运行测试
        if args.tests:
            results = runner.run_specific_tests(args.tests, args.verbose)
        else:
            results = runner.run_all_tests(args.pattern, args.verbose)
        
        # 打印摘要
        runner.print_summary()
        
        # 保存报告
        if args.output:
            try:
                runner.save_report(args.output)
                logger.info(f"测试完成，报告保存在: {args.output}")
            except Exception as e:
                logger.warning(f"生成测试报告时出错: {e}")
                logger.info("测试完成，但报告生成失败")
        
        # 返回适当的退出码
        return 0 if results["failed"] == 0 else 1
        
    except KeyboardInterrupt:
        logger.info("\n测试被用户中断")
        return 130
    except Exception as e:
        logger.error(f"运行测试时出错: {e}")
        return 1


if __name__ == "__main__":
    sys.exit(main())