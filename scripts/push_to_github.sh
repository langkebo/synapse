#!/usr/bin/env bash
# Simple helper to stage, commit and push changes to GitHub.
# Usage:
#   scripts/push_to_github.sh -m "commit message" [-b branch] [-r remote]
# Environment fallback (if remote is missing):
#   GITHUB_REPO   # e.g. "owner/repo" or full https URL
#   GITHUB_TOKEN  # Personal Access Token for https remote (optional)

set -euo pipefail

usage() {
  cat <<EOF
Usage: $0 -m "commit message" [-b branch] [-r remote]

Options:
  -m  Commit message (required)
  -b  Branch to push (default: current branch)
  -r  Remote name to push to (default: origin)
  -h  Show this help

Env fallbacks when remote is missing:
  GITHUB_REPO   owner/repo or full https URL
  GITHUB_TOKEN  token used for https remote (optional)
EOF
}

msg=""
branch=""
remote="origin"

while getopts ":m:b:r:h" opt; do
  case $opt in
    m) msg="$OPTARG" ;;
    b) branch="$OPTARG" ;;
    r) remote="$OPTARG" ;;
    h) usage; exit 0 ;;
    :) echo "Option -$OPTARG requires an argument." >&2; usage; exit 1 ;;
    \?) echo "Invalid option: -$OPTARG" >&2; usage; exit 1 ;;
  esac
done

if [[ -z "$msg" ]]; then
  echo "Error: commit message is required (-m)." >&2
  usage
  exit 1
fi

# Ensure we're in a git repository
if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "Error: not inside a git repository." >&2
  exit 1
fi

# Determine current branch if not provided
if [[ -z "$branch" ]]; then
  branch=$(git rev-parse --abbrev-ref HEAD)
fi

echo "[push_to_github] Remote: $remote | Branch: $branch"

echo "[push_to_github] Staging changes..."
git add -A

# If nothing to commit, exit gracefully
if git diff --cached --quiet; then
  echo "[push_to_github] No staged changes to commit. Skipping commit."
else
  echo "[push_to_github] Committing..."
  git commit -m "$msg" || true
fi

# If still no changes (e.g., nothing new after commit), still attempt push to sync

# Ensure remote exists, otherwise optionally configure from env
if ! git remote get-url "$remote" >/dev/null 2>&1; then
  echo "[push_to_github] Remote '$remote' not found. Attempting to configure from env..."
  if [[ -n "${GITHUB_REPO:-}" ]]; then
    if [[ "$GITHUB_REPO" =~ ^https?:// ]]; then
      remote_url="$GITHUB_REPO"
    else
      # Expect owner/repo format; build https URL
      if [[ -n "${GITHUB_TOKEN:-}" ]]; then
        remote_url="https://${GITHUB_TOKEN}@github.com/${GITHUB_REPO}.git"
      else
        remote_url="https://github.com/${GITHUB_REPO}.git"
      fi
    fi
    echo "[push_to_github] Adding remote '$remote' -> $remote_url"
    git remote add "$remote" "$remote_url"
  else
    echo "Error: remote '$remote' missing and GITHUB_REPO not set. Please set remote manually." >&2
    exit 1
  fi
fi

# Show remote for visibility (safe; token may be redacted by credential helper)
echo "[push_to_github] Remote URL: $(git remote get-url "$remote")"

echo "[push_to_github] Pushing..."
git push "$remote" "$branch"

echo "[push_to_github] Done."