#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_DIR="${PRIVACY_ROUTER_DEV_DIR:-/tmp/privacy-router-dev}"
mkdir -p "${RUN_DIR}"
API_PIDFILE="${RUN_DIR}/api.pid"
API_LOGFILE="${RUN_DIR}/api.log"

stop() {
  for pidfile in "${API_PIDFILE}" "${RUN_DIR}/flutter.pid"; do
    if [[ -f "${pidfile}" ]]; then
      pid="$(cat "${pidfile}")"
      kill "${pid}" 2>/dev/null || true
      rm -f "${pidfile}"
    fi
  done
}

if [[ "${1:-start}" == stop ]]; then
  stop
  echo "开发服务已停止。"
  exit 0
fi

if [[ -f "${API_PIDFILE}" ]] && kill -0 "$(cat "${API_PIDFILE}")" 2>/dev/null; then
  echo "API 已在运行：pid=$(cat "${API_PIDFILE}")"
else
  cd "${ROOT}"
  PRIVACY_ROUTER_DEV_ORIGIN="http://127.0.0.1:5173" \
    cargo run --release -p privacy-router -- \
    --bind "${PRIVACY_ROUTER_BIND:-127.0.0.1:8787}" \
    --database "${PRIVACY_ROUTER_DATABASE:-privacy-router.db}" \
    --log-filter "${PRIVACY_ROUTER_LOG_FILTER:-info,privacy_router=debug}" \
    >"${API_LOGFILE}" 2>&1 &
  echo $! >"${API_PIDFILE}"
  echo "API 启动中：日志 ${API_LOGFILE}"
fi

exec bash "${ROOT}/scripts/dev_run.sh"
