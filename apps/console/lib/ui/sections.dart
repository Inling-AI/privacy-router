/// 控制台的分区定义。
///
/// 导航（底部胶囊 / 图标轨 / 侧边栏）与页面映射都从这一份定义生成，不存在第二处列举分区的地方。
/// 标题与副标题也在此，由外壳统一渲染。
library;

import 'package:cupertino_ui/cupertino_ui.dart';

/// 一个控制台分区。
enum ConsoleSection {
  overview(
    icon: CupertinoIcons.chart_bar,
    activeIcon: CupertinoIcons.chart_bar_fill,
  ),
  pool(icon: CupertinoIcons.tray, activeIcon: CupertinoIcons.tray_fill),
  rules(
    icon: CupertinoIcons.slider_horizontal_3,
    activeIcon: CupertinoIcons.slider_horizontal_3,
  ),
  providers(
    icon: CupertinoIcons.antenna_radiowaves_left_right,
    activeIcon: CupertinoIcons.antenna_radiowaves_left_right,
  ),
  performance(
    icon: CupertinoIcons.speedometer,
    activeIcon: CupertinoIcons.speedometer,
  );

  const ConsoleSection({required this.icon, required this.activeIcon});

  final IconData icon;

  /// 选中态图标。底部胶囊用它，其余布局只用普通图标。
  final IconData activeIcon;

  CupertinoDynamicColor get iconBackground => switch (this) {
    overview => CupertinoColors.systemBlue,
    pool => CupertinoColors.systemGreen,
    rules => CupertinoColors.systemOrange,
    providers => CupertinoColors.systemTeal,
    performance => CupertinoColors.systemPurple,
  };
}
