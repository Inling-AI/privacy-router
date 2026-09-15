/// 品牌标识。
///
/// 「哪个亮度用哪张图」只在这里成立一次：侧栏词标按 [BrandMark.of] 取图，
/// 不各自去猜文件名。Web 侧（`web/favicon*.png`）是另一套构建产物，由 HTML 的
/// `prefers-color-scheme` 决定，不经过这里。
library;

import 'package:cupertino_ui/cupertino_ui.dart';

abstract final class BrandMark {
  static const String _light = 'assets/brand/pr_mark_light.png';
  static const String _dark = 'assets/brand/pr_mark_dark.png';

  /// 该亮度下应显示的标识资源路径。
  static String of(Brightness brightness) =>
      brightness == Brightness.dark ? _dark : _light;
}
