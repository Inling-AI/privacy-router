/// 响应式断点。
///
/// 组件库只提供手机形态的导航原语（`GlassTabBar` 的各个构造、`GlassAppBar`、`GlassToolbar`），
/// **没有**侧边栏或分栏原语。因此桌面布局由容器原语自行组合，断点也由本应用定义。
library;

import 'package:cupertino_ui/cupertino_ui.dart';

/// 布局形态。
enum LayoutClass {
  /// 手机：底部玻璃胶囊导航，详情用模态面板。
  compact,

  /// 平板：图标轨导航，详情用覆盖式面板。
  medium,

  /// 桌面：持久侧边栏 + 内容区 + 内联检查器。
  expanded;

  bool get isDesktop => this == LayoutClass.expanded;

  /// 详情是否以内联方式呈现（桌面），否则用面板。
  bool get hasInlineInspector => this == LayoutClass.expanded;
}

/// 断点。以宽度为准，同时考虑窗口高度——过矮的窗口即使很宽也不适合三栏。
class Breakpoints {
  const Breakpoints._();

  static const double medium = 700;
  static const double expanded = 1100;

  /// 三栏布局要求的最小宽度。
  static const double expandedWidth = 1100;

  /// 侧边栏宽度。
  static const double sidebarWidth = 320;

  /// 图标轨宽度。
  static const double railWidth = 68;

  /// 侧边栏收起/展开的中点。
  ///
  /// 窗格宽度跨过它之后内容才切到另一套布局，两侧用同一个值：窗格自己用它判断
  /// compact，内部控件（如外观菜单）用它判断展不展示文字。
  static const double sidebarCollapseMidpoint = (railWidth + sidebarWidth) / 2;

  static LayoutClass resolve(Size size) {
    // 高度不足时降级：矮窗口里三栏会把内容压扁。
    if (size.width >= expandedWidth && size.height >= 520) {
      return LayoutClass.expanded;
    }
    if (size.width >= medium) {
      return LayoutClass.medium;
    }
    return LayoutClass.compact;
  }
}
