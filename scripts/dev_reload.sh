#!/usr/bin/env bash
# 让正在运行的 flutter run 生效一次改动。
#
# `flutter run --pid-file` 注册了两个信号处理器，这里只是把名字写出来，避免每个调用点
# 各记一次信号编号：
#   reload  热重载，保留状态，改 UI 用这个
#   restart 热重启，丢弃状态，改 provider 结构或 main() 用这个
set -euo pipefail

RUN_DIR="${PRIVACY_ROUTER_DEV_DIR:-/tmp/privacy-router-dev}"
PIDFILE="${RUN_DIR}/flutter.pid"

if [[ ! -f "${PIDFILE}" ]]; then
  echo "没有正在运行的 flutter run（找不到 ${PIDFILE}）。先跑 mise run web:dev。" >&2
  exit 1
fi

pid="$(cat "${PIDFILE}")"
if ! kill -0 "${pid}" 2>/dev/null; then
  echo "flutter run 已退出（pid ${pid}）。" >&2
  exit 1
fi

case "${1:-reload}" in
  reload) signal=USR1 ;;
  restart) signal=USR2 ;;
  *)
    echo "用法：$0 [reload|restart]" >&2
    exit 1
    ;;
esac

kill "-${signal}" "${pid}"
echo "已向 pid ${pid} 发送 ${signal}"
