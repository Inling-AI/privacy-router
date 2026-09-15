/// 语义配色：界面只说角色，不说色值。
///
/// ## 为什么这里没有调色板
///
/// Apple 的界面不靠人手枚举颜色，靠两套系统：
///
/// 1. **透明度阶梯**。前景强弱由语义角色决定，与具体色值无关：
///    `label` 100%、`secondaryLabel` 60%、`tertiaryLabel` 30%、`quaternaryLabel` 18%。
/// 2. **底色阶梯**。前后距离由三级台阶表达。浅色模式下是白色 → 浅灰 → 更浅的灰；
///    深色模式下是纯黑 → `#1C1C1E` → `#2C2C2E`——**逐级变亮**。屏幕越黑，浮起来的东西
///    越亮，空间感由此而来，而不是靠投影。
///
/// 这些取值本来就是 `CupertinoColors` 里的语义色（明暗各一份），所以本文件不复制任何
/// 数字：它把「角色 → Apple 语义色」的映射收敛到这一处，让页面里不再散落
/// `CupertinoColors.x.resolveFrom(context)`。
///
/// ## 等感知亮度
///
/// `systemBlue` 这类系统色在深浅模式下不是同一色值取反：Apple 会在深色模式下降低灰度、
/// 提高亮度与饱和度，使文字在纯黑与浅灰底板上的对比度始终一致。用语义色就自动获得这个
/// 性质；手写十六进制会同时丢掉它和后续系统版本的调校。
library;

import 'package:cupertino_ui/cupertino_ui.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';

import '../api/models.dart';

/// 当前亮度下的一套语义取值。
///
/// 用 [ConsolePalette.of] 取；不要在页面里直接读 `CupertinoColors`，否则同一个语义角色会
/// 在多个文件里各写一遍解析。
@immutable
class ConsolePalette {
  const ConsolePalette._({
    required this.brightness,
    required this.label,
    required this.secondaryLabel,
    required this.tertiaryLabel,
    required this.quaternaryLabel,
    required this.canvas,
    required this.surface,
    required this.raised,
    required this.sidebarFill,
    required this.fill,
    required this.secondaryFill,
    required this.field,
    required this.separator,
    required this.opaqueSeparator,
    required this.tint,
    required this.positive,
    required this.caution,
    required this.negative,
    required this.onTint,
  });

  /// 控制台的强调色（tint color）。
  ///
  /// iOS 的默认 tint 就是 systemBlue，我们没有品牌色要另造一套。界面里所有「可点、
  /// 选中、进行中」都由它表达；强调色是**唯一**的彩色信号，用得越省越有秩序。
  static const CupertinoDynamicColor accent = CupertinoColors.systemBlue;

  /// 当前上下文生效的取值。
  ///
  /// 亮度取 `GlassTheme.brightnessOf`——它是玻璃组件库的**唯一**亮度权威，级联顺序为
  /// 「外观设置 → CupertinoTheme → 系统」。用它做分辨率可以保证玻璃材质与我们的语义色
  /// 永远落在同一个亮度上；两者不一致时会出现「深色底板 + 浅色文字」这类错配。
  factory ConsolePalette.of(BuildContext context) {
    final brightness = GlassTheme.brightnessOf(context);
    Color ink(CupertinoDynamicColor color) => color.resolveFrom(context);

    return ConsolePalette._(
      brightness: brightness,
      // 前景：透明度阶梯。
      label: ink(CupertinoColors.label),
      secondaryLabel: ink(CupertinoColors.secondaryLabel),
      tertiaryLabel: ink(CupertinoColors.tertiaryLabel),
      quaternaryLabel: ink(CupertinoColors.quaternaryLabel),
      // 底色：三级台阶。canvas 是最底层，surface 是它上面的卡片，
      // raised 是卡片里的内嵌分组。
      canvas: ink(CupertinoColors.systemGroupedBackground),
      surface: ink(CupertinoColors.secondarySystemGroupedBackground),
      raised: ink(CupertinoColors.tertiarySystemGroupedBackground),
      // 侧边栏窗格的底色。窗格要盖住从它下面滚过的内容，所以比卡片更透一些，
      // 剩下的交给窗格自己的背景模糊（见 `_SidebarPane`）。
      sidebarFill: brightness == Brightness.dark
          ? Color.lerp(
              ink(CupertinoColors.systemGroupedBackground),
              ink(CupertinoColors.secondarySystemGroupedBackground),
              0.5,
            )!
          : ink(CupertinoColors.secondarySystemGroupedBackground),
      // 中性填充：选中态、内嵌文本框等非玻璃表面。
      fill: ink(CupertinoColors.systemFill),
      secondaryFill: ink(CupertinoColors.secondarySystemFill),
      field: ink(CupertinoColors.tertiarySystemFill),
      separator: ink(CupertinoColors.separator),
      opaqueSeparator: ink(CupertinoColors.opaqueSeparator),
      // 语义色。状态一律用系统色，不自己调深浅。
      tint: ConsolePalette.accent.resolveFrom(context),
      positive: ink(CupertinoColors.systemGreen),
      caution: ink(CupertinoColors.systemOrange),
      negative: ink(CupertinoColors.systemRed),
      onTint: CupertinoColors.white,
    );
  }

  final Brightness brightness;

  /// 主文字：标题、正文、数值。100%。
  final Color label;

  /// 次要文字：说明、副标题、行内注解。60%。
  final Color secondaryLabel;

  /// 三级文字：占位符、极弱的提示。30%。
  final Color tertiaryLabel;

  /// 四级：分割线、禁用态。18%。
  final Color quaternaryLabel;

  /// 最底层。整屏铺底。
  final Color canvas;

  /// 底色之上的一层：分组列表、卡片。
  final Color surface;

  /// 卡片里的内嵌分组：比 [surface] 再进一层。
  final Color raised;

  /// 侧边栏窗格的底色（半透明，压在背景模糊之上）。
  final Color sidebarFill;

  /// 中性填充（大色块）。
  final Color fill;

  /// 中性填充（中色块）。
  final Color secondaryFill;

  /// 表单控件的空闲底色。
  final Color field;

  /// 分割线。半透明，压在什么材质上都成立。
  final Color separator;

  /// 不透明分割线。用于密集表格，避免半透明叠加后深浅不一。
  final Color opaqueSeparator;

  /// 强调色。
  final Color tint;

  /// 成功 / 通过 / 已启用。
  final Color positive;

  /// 需要留意 / 已抹去。
  final Color caution;

  /// 失败 / 危险操作（退出登录、删除）。
  final Color negative;

  /// 压在 [tint] 实色上的前景色。
  final Color onTint;

  bool get isDark => brightness == Brightness.dark;
}

/// 识别类别 → 颜色。
///
/// 这是类别色的**唯一**映射：内容池、概览、检查器都从这里取。之前它长在概览页里，
/// 由内容池跨页 import——同一事实放在两个页面之间传递，改一次要翻两个文件。
extension EntityGroupTint on EntityGroup {
  CupertinoDynamicColor get tint => switch (this) {
    EntityGroup.secret => CupertinoColors.systemRed,
    EntityGroup.idCard => CupertinoColors.systemRed,
    EntityGroup.bankCard => CupertinoColors.systemMint,
    EntityGroup.creditCode => CupertinoColors.systemIndigo,
    EntityGroup.ipAddress => CupertinoColors.systemCyan,
    EntityGroup.macAddress => CupertinoColors.systemBrown,
    EntityGroup.accountNumber => CupertinoColors.systemOrange,
    EntityGroup.privatePerson => CupertinoColors.systemPink,
    EntityGroup.privateEmail => CupertinoColors.systemBlue,
    EntityGroup.privatePhone => CupertinoColors.systemTeal,
    EntityGroup.privateAddress => CupertinoColors.systemGreen,
    EntityGroup.privateDate => CupertinoColors.systemYellow,
    EntityGroup.privateUrl => CupertinoColors.systemPurple,
    EntityGroup.unknown => CupertinoColors.systemGrey,
  };
}
