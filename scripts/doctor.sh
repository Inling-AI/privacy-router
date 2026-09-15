#!/usr/bin/env bash
# 校验开发环境是否满足构建与运行要求。
#
# 本脚本不复制任何版本号或校验和：它们分别来自 rust-toolchain.toml、
# .fvmrc 与 privacy-model 的模型描述，脚本只做读取与比对。
set -uo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.." || exit 1

failures=0
report() { printf '  %-26s %s\n' "$1" "$2"; }
pass() { report "$1" "ok — $2"; }
fail() { report "$1" "FAIL — $2"; failures=$((failures + 1)); }

echo "环境校验"

# ---------------------------------------------------------------------------
# Rust：版本事实来自 rust-toolchain.toml，由 rustup 自动选择
# ---------------------------------------------------------------------------

expected_rust="$(sed -n 's/^channel[[:space:]]*=[[:space:]]*"\(.*\)".*/\1/p' rust-toolchain.toml)"
actual_rust="$(rustc --version 2>/dev/null | awk '{print $2}')"

if [ -z "$actual_rust" ]; then
  fail "rustc" "未找到 rustc（需要安装 rustup）"
elif [ "$actual_rust" != "$expected_rust" ]; then
  fail "rustc" "期望 $expected_rust，实际 $actual_rust"
else
  pass "rustc" "$actual_rust"
fi

for component in rustfmt clippy; do
  if rustup component list --installed 2>/dev/null | grep -q "^${component}"; then
    pass "$component" "已安装"
  else
    fail "$component" "缺少组件，运行 mise run toolchain"
  fi
done

# edition 2024 是 workspace 的硬前提（rustc 1.85+），单独探测避免版本号比较写死。
if cargo metadata --no-deps --format-version 1 >/dev/null 2>&1; then
  pass "cargo metadata" "workspace 可解析"
else
  fail "cargo metadata" "workspace 解析失败"
fi

# ---------------------------------------------------------------------------
# Flutter：版本事实来自 .fvmrc
# ---------------------------------------------------------------------------

expected_flutter="$(sed -n 's/.*"flutter"[[:space:]]*:[[:space:]]*"\(.*\)".*/\1/p' .fvmrc)"
actual_flutter="$(fvm flutter --version 2>/dev/null | head -1 | awk '{print $2}')"

if ! command -v fvm >/dev/null 2>&1; then
  fail "fvm" "未找到 fvm，运行 mise install"
elif [ -z "$actual_flutter" ]; then
  fail "flutter" "无法执行 fvm flutter，运行 mise run flutter:install"
elif [ "$actual_flutter" != "$expected_flutter" ]; then
  fail "flutter" "期望 ${expected_flutter}，实际 ${actual_flutter}"
else
  pass "flutter" "${actual_flutter}（项目内固定）"
fi

# wasm 是控制台的构建前提，直接探测工具能力而不是推断。
if fvm flutter build web --help 2>/dev/null | grep -q -- '--wasm'; then
  pass "flutter web wasm" "工具支持 --wasm"
else
  fail "flutter web wasm" "当前 SDK 不支持 --wasm"
fi

# ---------------------------------------------------------------------------
# 模型权重：校验和事实由 privacy-model 提供，复用同一个离线验证入口。
# ---------------------------------------------------------------------------

if model_output="$(cargo run --quiet -p privacy-router -- model verify 2>&1)"; then
  pass "模型权重" "固定模型文件校验通过"
else
  fail "模型权重" "${model_output}（运行 mise run model:download）"
fi

# ---------------------------------------------------------------------------
# 构建依赖：内嵌 SQLite 需要 C 编译器；刻意不依赖系统 libsqlite3
# ---------------------------------------------------------------------------

if command -v cc >/dev/null 2>&1; then
  pass "C 编译器" "$(cc --version 2>/dev/null | head -1)"
else
  fail "C 编译器" "缺少 cc，sqlx 的 sqlite-bundled 需要编译 SQLite"
fi

# ---------------------------------------------------------------------------

echo
if [ "$failures" -eq 0 ]; then
  echo "全部通过。"
else
  echo "$failures 项未通过。"
fi
exit "$([ "$failures" -eq 0 ] && echo 0 || echo 1)"
