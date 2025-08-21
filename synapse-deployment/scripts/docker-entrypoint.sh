#!/usr/bin/env sh
set -eu

CONFIG_PATH="${SYNAPSE_CONFIG_PATH:-/data/homeserver.yaml}"

# 打印简要启动信息（可通过环境变量禁用）
if [ "${ENTRYPOINT_VERBOSE:-1}" = "1" ]; then
  echo "[entrypoint] Starting Synapse with config: ${CONFIG_PATH}"
fi

exec python -m synapse.app.homeserver --config-path "${CONFIG_PATH}"