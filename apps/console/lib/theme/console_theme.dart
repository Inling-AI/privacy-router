/// 外观设置与 Cupertino 主题装配。
///
/// 主题里只有三样东西：**亮度**、**强调色**、**字体**。其余语义取值都在 `palette.dart`，
/// 玻璃取值在 `tokens.dart` —— 一个事实只在一处。
library;

import 'package:cupertino_ui/cupertino_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'palette.dart';

/// 界面外观。与 iOS「设置 → 显示与亮度」一致：跟随系统、浅色、深色。
enum ConsoleAppearance {
  system(CupertinoIcons.circle_lefthalf_fill),
  light(CupertinoIcons.sun_max),
  dark(CupertinoIcons.moon);

  const ConsoleAppearance(this.icon);

  final IconData icon;

  /// 结合系统亮度求出生效的亮度。
  Brightness resolve(Brightness platform) => switch (this) {
    ConsoleAppearance.system => platform,
    ConsoleAppearance.light => Brightness.light,
    ConsoleAppearance.dark => Brightness.dark,
  };
}

/// 当前外观。仅存在于本机，不进入服务端设置。
///
/// 默认「跟随系统」：把亮度决定权交给用户的操作系统，而不是替用户钉死一种。之前的默认值是
/// 深色，于是白天打开控制台第一眼就是一块黑屏——这不叫暗色适配，叫没适配。
class AppearanceNotifier extends Notifier<ConsoleAppearance> {
  @override
  ConsoleAppearance build() => ConsoleAppearance.system;

  void select(ConsoleAppearance appearance) => state = appearance;
}

final appearanceProvider =
    NotifierProvider<AppearanceNotifier, ConsoleAppearance>(
      AppearanceNotifier.new,
    );

/// 控制台主题的装配入口。
abstract final class ConsoleTheme {
  /// 按生效亮度构造 Cupertino 主题。
  ///
  /// 这里**显式**写进 `brightness`：组件库的亮度级联第二级读的就是
  /// `CupertinoThemeData.brightness`，钉住它，玻璃材质与语义色才会跟着「外观」设置走，
  /// 而不是各自去猜系统亮度。系统亮度和外观设置不一致时（系统深色 + 应用浅色），
  /// 这正是两边不打架的原因。
  static CupertinoThemeData cupertino(Brightness brightness) =>
      CupertinoThemeData(
        brightness: brightness,
        primaryColor: ConsolePalette.accent,
      );
}

/// 把当前 `CupertinoThemeData` 桥接给仍走 legacy Cupertino 的第三方组件。
///
/// `liquid_glass_widgets` 内部仍 `import 'package:flutter/cupertino.dart'`，并在亮度级联里
/// 读取 legacy 的 `CupertinoTheme.of(context)`；本应用使用拆分后的 `package:cupertino_ui`，
/// 两者在类型层面互不相通。不桥接的话玻璃组件读不到应用主题，明暗与配色会错乱。
///
/// 上游为此提供了 `CupertinoUiCompatibilityBridge`，并标注为临时迁移工具。因此桥接只在此处
/// 出现一次：将来上游完成迁移或更换组件库时，改动范围就是这一个类。
class LegacyCupertinoBridge extends StatelessWidget {
  const LegacyCupertinoBridge({
    required this.brightness,
    required this.child,
    super.key,
  });

  /// 生效亮度。玻璃库的亮度级联会读取它（见其 `resolveGlassBrightness` 第二级）。
  final Brightness brightness;

  final Widget child;

  @override
  Widget build(BuildContext context) {
    return CupertinoTheme(
      data: ConsoleTheme.cupertino(brightness),
      // 上游把该桥接标注为临时迁移工具；这里是有意使用的，且只在这一处。
      // ignore: deprecated_member_use
      child: CupertinoUiCompatibilityBridge(child: child),
    );
  }
}
