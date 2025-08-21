#!/bin/bash
# 代码上传至 GitHub 的自动化脚本
# 使用方式：
#   ./synapse-deployment/scripts/git-upload.sh -m "feat: update deploy configs" -b main [-r https://github.com/user/repo.git]
#
# 功能：
#  - 自动检测并进入 Git 仓库根目录
#  - 自动添加、提交、推送
#  - 首次运行可自动添加远程仓库（通过 -r 指定）
#  - 处理无变更、无上游分支、认证失败等常见场景

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info()    { echo -e "${BLUE}[INFO]${NC} $*"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $*"; }
log_warn()    { echo -e "${YELLOW}[WARN]${NC} $*"; }
log_error()   { echo -e "${RED}[ERROR]${NC} $*"; }

usage() {
  cat <<EOF
用法: $0 [-m 提交信息] [-b 分支名] [-r 远程仓库URL]
  -m  提交信息（默认：自动生成带时间戳的提交信息）
  -b  推送分支名（默认：当前分支；若无则使用 main）
  -r  远程仓库URL（若当前无 origin 时使用）
示例：
  $0 -m "chore: sync deploy configs" -b main
  $0 -m "fix: adjust env vars" -b main -r https://github.com/<user>/<repo>.git
EOF
}

COMMIT_MSG=""
BRANCH=""
REMOTE_URL=""

while getopts ":m:b:r:h" opt; do
  case "$opt" in
    m) COMMIT_MSG="$OPTARG" ;;
    b) BRANCH="$OPTARG" ;;
    r) REMOTE_URL="$OPTARG" ;;
    h) usage; exit 0 ;;
    *) usage; exit 1 ;;
  esac
done

if ! command -v git >/dev/null 2>&1; then
  log_error "未安装 git，请先安装后再运行。"
  exit 1
fi

# 进入仓库根目录
if git rev-parse --show-toplevel >/dev/null 2>&1; then
  REPO_ROOT=$(git rev-parse --show-toplevel)
  cd "$REPO_ROOT"
else
  log_error "当前目录不是 Git 仓库，请先执行 git init。"
  exit 1
fi

# 检查远程仓库
if ! git remote get-url origin >/dev/null 2>&1; then
  if [[ -n "${REMOTE_URL}" ]]; then
    log_info "未检测到 origin，正在添加远程仓库: ${REMOTE_URL}"
    git remote add origin "$REMOTE_URL"
  else
    log_warn "未检测到 origin，且未通过 -r 指定远程仓库。"
    log_warn "可执行: git remote add origin <your_repo_url> 后重试。"
    exit 1
  fi
fi

# 选择分支
if [[ -z "$BRANCH" ]]; then
  if git rev-parse --abbrev-ref HEAD >/dev/null 2>&1; then
    CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)
    if [[ "$CURRENT_BRANCH" == "HEAD" ]]; then
      BRANCH="main"
      log_info "当前为游离状态，使用分支：${BRANCH}"
      git checkout -B "$BRANCH"
    else
      BRANCH="$CURRENT_BRANCH"
    fi
  else
    BRANCH="main"
    git checkout -B "$BRANCH"
  fi
fi

# 生成默认提交信息
if [[ -z "$COMMIT_MSG" ]]; then
  TS=$(date '+%Y-%m-%d %H:%M:%S')
  COMMIT_MSG="chore: sync deployment configs (${TS})"
fi

log_info "分支: ${BRANCH}"
log_info "远程: $(git remote get-url origin)"

# 提交变更
git add -A
if git diff --cached --quiet; then
  log_warn "没有需要提交的变更。"
else
  git commit -m "$COMMIT_MSG" || true
fi

# 推送
set +e
OUTPUT=$(git push -u origin "$BRANCH" 2>&1)
STATUS=$?
set -e

if [[ $STATUS -ne 0 ]]; then
  echo "$OUTPUT"
  log_error "推送失败。可能的原因：网络问题/认证失败/权限不足。"
  log_info "可尝试："
  echo "  1) 设置代理后重试：  git config --global http.proxy http://127.0.0.1:7890"
  echo "  2) 切换为 SSH 方式：  git remote set-url origin git@github.com:<user>/<repo>.git"
  echo "  3) 设置上游分支：    git push -u origin ${BRANCH}"
  exit 1
fi

log_success "代码已成功推送到 GitHub: origin/${BRANCH}"