// Flutter 的默认 bootstrap 会注册 service worker，并把它缓存的资源留给下一次访问。
// 单二进制内嵌的控制台不接受这种陈旧缓存：升级二进制后浏览器仍会加载旧前端，
// 表现为「界面没有变化」而实际服务端已经更新。
//
// 因此这里显式提供一份 bootstrap，并且**不传** serviceWorkerSettings —— Flutter 只在
// 该字段存在时才注册 service worker。这样无需依赖已被弃用的 --pwa-strategy 选项。
//
// renderer 钉成 skwasm 同理：发布产物只保留 dart2wasm 那一份构建（见
// scripts/prune_web_release.sh），不支持 WasmGC 的浏览器应当在选择构建时就失败并
// 说明原因，而不是回退到一个已经被裁剪掉的 main.dart.js 上。
//
// 这两个占位符由 flutter build web 在构建时替换，不要删除。
{{flutter_js}}
{{flutter_build_config}}

_flutter.loader.load({
  config: { renderer: 'skwasm' },
});
