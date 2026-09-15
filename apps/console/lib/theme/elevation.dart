/// 悬浮层级：环境光遮蔽阴影 + 0.5pt 边缘高光。
///
/// ## Apple 的立体感从哪来
///
/// 不是「又黑又硬的投影」，而是两种极微弱的物理线索叠加：
///
/// 1. **环境光遮蔽（AO）**：扩散半径很大（20–48）、不透明度极低（4%–16%）的柔和阴影。
///    它只负责把物体从背景里「托起来」，不该被看清。
/// 2. **倒角反光（rim light）**：0.5pt 的边缘线。浅色模式下是极浅的黑，深色模式下是极浅
///    的白——模拟真实物体边缘受光后那一线反光。深色界面里它才是分层的主力：纯黑底上的
///    黑投影是不可见的，真正让卡片分开的是它比底板亮一点，以及这条细边。
///
/// 两者都不需要「更深的灰」这种手工调色，因此这里只有几何与不透明度，没有色值枚举。
///
/// ## 为什么阴影在深色模式下依然要给
///
/// 组件库的玻璃材质刻意跳过深色阴影（玻璃自带发光边缘，投影会被吞掉）。但**不透明**表面
/// （分组列表、卡片、面板）在深色模式下仍然需要 AO——否则纯黑底上的 #1C1C1E 卡片会显得
/// 像贴纸。本文件只服务这些不透明表面；玻璃面板的层次交给组件库的材质与这里的边缘高光。
library;

import 'package:cupertino_ui/cupertino_ui.dart';

import 'palette.dart';

/// 悬浮程度。数值越靠后，离底板越远。
enum ConsoleElevation {
  /// 贴底：整屏底板上没有材质的内容。
  flat,

  /// 抬起：内容区的卡片、分组列表。
  raised,

  /// 漂浮：导航面板、侧边栏、检查器。
  floating,

  /// 覆盖：模态面板、弹层。
  overlay,
}

/// 每一级悬浮的几何与光学取值。
extension ConsoleElevationStyle on ConsoleElevation {
  /// 边缘高光的宽度。0.5pt 是 Apple 的「发丝线」；再粗就从反光变成描边了。
  static const double rimWidth = 0.5;

  /// 材质：填充的不透明度。
  ///
  /// 卡片不应是刷在底板上的一块纯白，而应是**压在底板上的一层膜**：让底板透上来一点，
  /// 卡片才有「浮起」的重量。纯 `#FFFFFF` 压在浅灰底板上会显得像贴纸，因为它与底板之间
  /// 没有光学关系，只有明度差。
  ///
  /// 数值随悬浮层级提高而降低：离底板越远的东西，越接近它背后那层被模糊过的背景。
  double get fillAlpha => switch (this) {
    ConsoleElevation.flat => 1.0,
    ConsoleElevation.raised => 0.82,
    ConsoleElevation.floating => 0.72,
    ConsoleElevation.overlay => 0.92,
  };

  /// 材质：背景模糊半径。`0` 表示不铺模糊层。
  ///
  /// 只有真正**有东西从它下面滚过**的表面才需要模糊：侧边栏、检查器、模态面板。
  /// 内容区的卡片压在平坦的底板上，模糊一层纯色仍然是那层纯色——付费但不换货，所以
  /// 内容表面（[ConsoleElevation.raised]）保持 `0`，把预算留给真实需要它的地方。
  double get materialBlur => switch (this) {
    ConsoleElevation.flat => 0,
    ConsoleElevation.raised => 0,
    ConsoleElevation.floating => 36,
    ConsoleElevation.overlay => 48,
  };

  /// 投影。浅色模式用低不透明度多层叠加模拟柔和遮蔽；深色模式提高不透明度，
  /// 因为纯黑底板会把浅阴影完全吃掉。
  List<BoxShadow> shadows(BuildContext context) {
    final dark = ConsolePalette.of(context).isDark;
    return switch (this) {
      ConsoleElevation.flat => const [],
      ConsoleElevation.raised =>
        dark
            ? const [
                BoxShadow(
                  color: Color(0x7A000000),
                  blurRadius: 24,
                  offset: Offset(0, 8),
                ),
              ]
            : const [
                BoxShadow(
                  color: Color(0x14000000),
                  blurRadius: 24,
                  offset: Offset(0, 8),
                ),
                // 第二层贴近物体，勾出接触面；只有一层大模糊会显得「飘」。
                BoxShadow(
                  color: Color(0x0A000000),
                  blurRadius: 4,
                  offset: Offset(0, 1),
                ),
              ],
      ConsoleElevation.floating =>
        dark
            ? const [
                BoxShadow(
                  color: Color(0x99000000),
                  blurRadius: 36,
                  offset: Offset(0, 12),
                ),
              ]
            : const [
                BoxShadow(
                  color: Color(0x1A000000),
                  blurRadius: 36,
                  offset: Offset(0, 12),
                ),
                BoxShadow(
                  color: Color(0x0F000000),
                  blurRadius: 6,
                  offset: Offset(0, 2),
                ),
              ],
      ConsoleElevation.overlay =>
        dark
            ? const [
                BoxShadow(
                  color: Color(0xB3000000),
                  blurRadius: 56,
                  offset: Offset(0, 24),
                ),
              ]
            : const [
                BoxShadow(
                  color: Color(0x29000000),
                  blurRadius: 56,
                  offset: Offset(0, 24),
                ),
                BoxShadow(
                  color: Color(0x14000000),
                  blurRadius: 8,
                  offset: Offset(0, 2),
                ),
              ],
    };
  }

  /// 边缘高光（发丝描边）。浅色模式下是极浅的黑（物体边缘背光），深色模式下是极浅的白（受光）。
  ///
  /// 它是「物体边缘受光后那一线反光」，不是描边：宽度恒为 [rimWidth]（0.5pt），颜色只在
  /// 明度上偏离底板一点点。视网膜屏上有了它，卡片的轮廓才是「磨出来的」而不是「切出来的」。
  ///
  /// 浅色模式刻意只给 4%：更重就从「边界光」变成「描边」，而描边会让卡片看着像图片框。
  Color rimLight(BuildContext context) => ConsolePalette.of(context).isDark
      ? const Color(0x1AFFFFFF)
      : const Color(0x0A000000);
}
