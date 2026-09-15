#!/usr/bin/env bash
#
# 裁剪 Flutter web 发布产物。
#
# `flutter build web --wasm` 会同时产出两份可运行的构建：dart2wasm + skwasm，以及
# dart2js + canvaskit 回退。flutter_tools 没有任何开关能只留前者——它无条件地把两份
# 都编出来、把整套 canvaskit 变体都拷进 build/web。而这个产物会被 rust-embed 逐字节
# 内嵌进发布二进制，所以每个不需要的文件都要在这里显式删掉。
#
# 我们只支持支持 WasmGC 的浏览器（Chrome/Edge 119+、Firefox 120+、Safari 18.2+），
# 因此 dart2js 回退与其 canvaskit 运行时、以及仅供 DevTools 符号化的 .symbols 文件
# 都不参与运行。
#
# 依赖这份裁剪的另外两处，改这里时必须一起看：
#   - apps/console/web/flutter_bootstrap.js 显式把 renderer 钉成 skwasm，让不支持
#     WasmGC 的浏览器在加载前就报错，而不是回退到已被删掉的 main.dart.js。
#   - 下面的必需文件清单会在裁剪后校验，Flutter 升级改了文件名时立刻失败。
set -euo pipefail

web_dir="${1:-build/web}"

if [[ ! -d "$web_dir" ]]; then
  echo "prune_web_release: 目录不存在：$web_dir" >&2
  exit 1
fi

# dart2js 回退入口与 canvaskit 运行时。
rm -f \
  "$web_dir/main.dart.js" \
  "$web_dir/main.dart.js_1.part.js"

rm -rf \
  "$web_dir/canvaskit/canvaskit.js" \
  "$web_dir/canvaskit/canvaskit.wasm" \
  "$web_dir/canvaskit/wimp.js" \
  "$web_dir/canvaskit/wimp.wasm" \
  "$web_dir/canvaskit/chromium" \
  "$web_dir/canvaskit/webparagraph"

# DevTools 符号化数据，只在浏览器开发者工具里被读取。
find "$web_dir" -name '*.symbols' -delete

# flutter.js 的内容已经由 flutter_bootstrap.js 内联；service worker 与 version.json
# 服务于「缓存陈旧前端」的加载策略，而我们刻意不注册 service worker。
rm -f \
  "$web_dir/flutter.js" \
  "$web_dir/flutter_service_worker.js" \
  "$web_dir/version.json"

required=(
  index.html
  flutter_bootstrap.js
  manifest.json
  main.dart.mjs
  main.dart.wasm
  canvaskit/skwasm.js
  canvaskit/skwasm.wasm
  canvaskit/skwasm_heavy.js
  canvaskit/skwasm_heavy.wasm
)

missing=()
for path in "${required[@]}"; do
  [[ -f "$web_dir/$path" ]] || missing+=("$path")
done

if (( ${#missing[@]} > 0 )); then
  printf 'prune_web_release: 裁剪后缺少运行必需的文件：%s\n' "${missing[*]}" >&2
  exit 1
fi
