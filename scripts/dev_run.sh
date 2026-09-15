#!/usr/bin/env bash
# 开发期热重载回路。
#
# ## 为什么不走单二进制内嵌
#
# 内嵌是**发布形态**的约束：单文件交付时前端必须躺在二进制里。开发时它不是约束而是枷锁——
# 每改一行 UI 都要 `flutter build web` + `cargo build --release` + 重启进程，一轮两分钟；
# 热重载是一秒。所以开发期让 Dart 开发服务器托管前端、API 直连本机代理进程。
#
# ## 跨源
#
# 托管前端在 5173，API 在 8787，两者不同源。代理进程用 `--dev-origin` 只对**控制台 API**
# 放开这一个来源；代理端点 `/v1/*` 不挂 CORS 层，任何来源都拿不到跨源许可。发布形态下控制台
# 与 API 同源，`--dev-origin` 根本不需要传。
#
# ## 为什么是 `-d chrome`
#
# 热重载靠 Dart VM 服务：只有浏览器真的连上调试服务，`flutter run` 才注册 SIGUSR1/SIGUSR2
# 并写出 pid 文件。`-d web-server` 只是托管文件，在浏览器接上之前拿不到这个能力。
# 本机没有 Chrome，但有同样是 Chromium 的 Edge，用 CHROME_EXECUTABLE 指过去即可
# （Flutter 的 edge 设备只在 Windows 上注册）。
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONSOLE="${ROOT}/apps/console"
RUN_DIR="${PRIVACY_ROUTER_DEV_DIR:-/tmp/privacy-router-dev}"
API="${PRIVACY_ROUTER_API:-http://127.0.0.1:8787}"
PORT="${PRIVACY_ROUTER_WEB_PORT:-5173}"

mkdir -p "${RUN_DIR}"
PIDFILE="${RUN_DIR}/flutter.pid"
LOGFILE="${RUN_DIR}/flutter.log"

if [[ -f "${PIDFILE}" ]] && kill -0 "$(cat "${PIDFILE}")" 2>/dev/null; then
  echo "已在运行：pid=$(cat "${PIDFILE}")"
  exit 0
fi

# 没有 Chrome 时退回 Edge；两者都是 Chromium，热重载路径完全相同。
if [[ -z "${CHROME_EXECUTABLE:-}" && ! -d "/Applications/Google Chrome.app" ]]; then
  for candidate in \
    "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge" \
    "/Applications/Chromium.app/Contents/MacOS/Chromium" \
    "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser"; do
    if [[ -x "${candidate}" ]]; then
      export CHROME_EXECUTABLE="${candidate}"
      break
    fi
  done
fi
if [[ -n "${CHROME_EXECUTABLE:-}" ]]; then
  echo "浏览器：${CHROME_EXECUTABLE}"
fi

cd "${CONSOLE}"
nohup fvm flutter run \
  -d chrome \
  --web-port "${PORT}" \
  --web-hostname 127.0.0.1 \
  --pid-file "${PIDFILE}" \
  --dart-define="PRIVACY_ROUTER_API=${API}" \
  >"${LOGFILE}" 2>&1 &

echo "启动中；日志 ${LOGFILE}"
