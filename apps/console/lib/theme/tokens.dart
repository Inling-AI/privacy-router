/// 排版刻度、间距刻度、圆角刻度，以及玻璃面板的唯一取值口。
///
/// ## 为什么排版只有一份
///
/// Apple 的排版靠**字号阶梯 + 字重**拉开秩序，而不是靠装饰线、色块和斜体。SF Pro 每个
/// 字号都对应一个语义角色（Large Title / Title / Headline / Body / Footnote…），界面里
/// 出现「34pt 粗体」时它永远只有一个意思。所以这里按角色命名，页面不写裸数字。
///
/// 字号取 Apple 的标准阶梯；中文界面的字距统一为 0：中文字面本身是满框的，负字距会挤，
/// 正字距会散。
///
/// ## 为什么圆角是超椭圆
///
/// 普通 `border-radius` 是「直线瞬间接一段圆弧」，连接处曲率突变，视觉上有拐角感。
/// Apple 全系统用超椭圆（Lamé curve），曲率从直线渐进过渡到圆弧，像打磨过的鹅卵石。
/// 组件库的 `LiquidRoundedSuperellipse` 就是这个形状，因此所有面板/卡片/控件都从这一处取。
library;

import 'package:cupertino_ui/cupertino_ui.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';

import 'palette.dart';

/// 排版角色。数值就是 Apple 的字号阶梯。
abstract final class ConsoleText {
  /// 字体回退栈。
  ///
  /// Flutter Web 在未声明字体时回退到引擎自带的默认无衬线体，中文会落到浏览器的兜底字形，
  /// 字重与字距都和 SF Pro / 苹方对不上，一眼就能看出「这是网页」。这里显式按 Apple 的
  /// 顺序声明系统字体：命中就拿到与原生一致的字体，一个都没命中时引擎仍用自己的默认，
  /// 不会更差。原生端本来就用系统字体，多这一层不影响结果。
  static const fontFallback = <String>[
    '-apple-system',
    'BlinkMacSystemFont',
    'SF Pro Text',
    'SF Pro Display',
    'PingFang SC',
    'Helvetica Neue',
  ];

  /// 页面大标题：34 / w700。
  static const largeTitle = TextStyle(
    fontSize: 34,
    fontWeight: FontWeight.w700,
    letterSpacing: 0,
    height: 1.18,
    fontFamilyFallback: fontFallback,
  );

  /// 区块标题：17 / w600（Headline）。标题层级比正文高，但不需要更大。
  static const headline = TextStyle(
    fontSize: 17,
    fontWeight: FontWeight.w600,
    height: 1.29,
    fontFamilyFallback: fontFallback,
  );

  /// 分组表头：13 / w600（Footnote）。iOS 用它做大写表头。
  static const groupHeader = TextStyle(
    fontSize: 13,
    fontWeight: FontWeight.w600,
    letterSpacing: 0,
    height: 1.38,
    fontFamilyFallback: fontFallback,
  );

  /// 把一套字体样式变成**等宽数字**。
  ///
  /// 比例数字里 `1` 比 `8` 窄，数字一变整行就左右挪动，指标、耗时、优先级都因此显得不安定。
  /// 需要纵向对齐数值的样式一律经它派生，不在各处重复写这一条字形特性。
  static TextStyle tabular(TextStyle base) =>
      base.copyWith(fontFeatures: const [FontFeature.tabularFigures()]);

  /// 卡片上的数值。
  static final metric = tabular(
    const TextStyle(
      fontSize: 30,
      fontWeight: FontWeight.w700,
      letterSpacing: 0,
      height: 1.1,
      fontFamilyFallback: fontFallback,
    ),
  );

  /// 主体正文：17（Body）。iOS 正文就是 17，不是 15。
  static const body = TextStyle(
    fontSize: 17,
    height: 1.29,
    fontFamilyFallback: fontFallback,
  );

  /// 次级正文：15（Subheadline）。
  static const subheadline = TextStyle(
    fontSize: 15,
    height: 1.33,
    fontFamilyFallback: fontFallback,
  );

  /// 说明文字：13（Footnote）。列表副标题、脚注都用它。
  static const footnote = TextStyle(
    fontSize: 13,
    height: 1.38,
    fontFamilyFallback: fontFallback,
  );

  /// 带数值的说明文字：[footnote] 加等宽数字。
  ///
  /// 列表副标题里的数字会随刷新变化（优先级、命中数），等宽后整行文字不会跟着数位跳动。
  static final numericFootnote = tabular(footnote);

  /// 行尾的数值：与文本同一档字号，但加粗且等宽。
  ///
  /// 列表右端一列数值（耗时、次数）用同一种字形，纵向扫读时小数点才能对上。
  static final value = tabular(
    subheadline.copyWith(fontWeight: FontWeight.w600),
  );

  /// 最小说明：12（Caption 1）。
  static const caption = TextStyle(
    fontSize: 12,
    height: 1.33,
    fontFamilyFallback: fontFallback,
  );

  /// 等宽文本：原文与脱敏结果并排比对时用。
  static const mono = TextStyle(
    fontFamily: 'Menlo',
    fontSize: 13,
    height: 1.5,
    fontFamilyFallback: fontFallback,
  );
}

/// 间距刻度。4 的倍数，只有这几档。
abstract final class ConsoleSpace {
  static const double xs = 4;
  static const double s = 8;
  static const double m = 12;
  static const double l = 16;
  static const double xl = 20;
  static const double xxl = 28;

  /// 区块之间的距离。
  static const double section = 36;
}

/// 圆角刻度（超椭圆半径）。
abstract final class ConsoleRadius {
  /// 行内小元素：徽标、指示条。
  static const double small = 8;

  /// 列表行左侧图标底片（28pt 方块）。
  ///
  /// 比 [small] 小一档，因为底片本身只有 28pt：再大的圆角就从「方块」变成「胶囊」，
  /// 与 iOS 设置面板里那种图标底片的观感对不上。
  static const double iconTile = 7;

  /// 控件：按钮、输入框、导航项。
  static const double control = 12;

  /// 卡片与分组列表。
  static const double card = 18;

  /// 面板：侧边栏、检查器。
  static const double panel = 24;

  /// 模态面板。
  static const double sheet = 28;
}

/// 尺寸。
abstract final class ConsoleMetrics {
  /// 悬浮侧栏与详情导航共享的窗口边距。
  static const double splitViewInset = 12;

  /// 本机可点区域的最小边长（44pt）。
  static const double touchTarget = 44;

  /// 顶栏高度。
  static const double barHeight = 44;

  /// 分组列表里一行的最小高度。
  static const double rowHeight = 48;

  /// 滚动到底部留出的空间；有底部栏时用 120。
  static const double bottomInset = 120;

  /// 页面左右边距。桌面留得比手机宽，内容才会有「版面」而不是「拉满」。
  static double pageInset(bool isDesktop) => isDesktop ? 32 : 20;

  /// 正文在宽屏上的最大版心宽度。
  ///
  /// 页面边距只能保证内容不贴边，不能阻止列表在超宽窗口上被拉成一条横幅。大标题、说明、
  /// 分组列表共用这一宽度，才能始终落在同一条竖向基准线上。
  static const double contentMaxWidth = 960;

  /// 桌面模态面板的最大宽度。表单不应随显示器宽度继续生长。
  static const double sheetMaxWidth = 760;

  /// 分组列表的左右内边距。
  static const double groupInset = 16;

  /// 分组表头与脚注的水平缩进。
  ///
  /// 比 [groupInset] 再退一档：它们是列表的注释，不该与行内容抢同一条左边界。
  /// 列表头的表头/脚注与页面级的补充说明（见 `ListNote`）共用它，否则同一页里
  /// 会出现两条相差几像素的注释基准线。
  static const double groupTextInset = groupInset + ConsoleSpace.xs;

  /// 侧边栏一行内容的水平内缩。
  ///
  /// 选中底片铺满面板宽度，行内容（图标、文字）再内缩这么多，才不会贴着底片的边；
  /// 词标用同一个值，才能与导航图标的左缘对齐。
  static const double sidebarRowInset = ConsoleSpace.m;

  /// 侧边栏一行的图标列宽度（图标槽）。
  ///
  /// 侧边栏里所有东西都占一个同样宽的槽：导航图标、词标的盾牌、收起按钮（正好 28）。
  /// 图标在槽里居中，槽再决定自己靠左还是居中，于是整栏只有**一条**竖向基准线——
  /// 而不是每个元素各算各的对齐。
  static const double sidebarIconColumn = 28;

  /// 独立按钮（表单、页面动作）的高度。就等于最小可点区域。
  static const double buttonHeight = touchTarget;

  /// 放进顶部导航栏的控件高度。
  ///
  /// 比 [buttonHeight] 矮一档：栏本身只有 [barHeight] 高，控件撑满整条栏就没了上下余地。
  /// 栏里所有控件（主操作、图标按钮）都用它，否则会出现「同一条工具栏上两个按钮不一样高」。
  static const double barControl = 32;
}

/// 玻璃材质的取值口。
abstract final class ConsoleGlass {
  /// 玻璃质量。
  ///
  /// Web 被组件库静态封顶在 `standard`（见其 `glass_quality_adapter`），所以显式使用
  /// `standard`，不去请求 `premium`——那只会得到一个静默降级。
  static const GlassQuality quality = GlassQuality.standard;

  /// 导航与内容操作共用同一材质，暗色控件保留可辨认的灰阶表面。
  static LiquidGlassSettings control(BuildContext context) {
    final palette = ConsolePalette.of(context);
    if (!palette.isDark) return const LiquidGlassSettings(shadowElevation: 2);
    return LiquidGlassSettings(
      backerColor: palette.surface.withValues(alpha: 0.95),
      glassColor: CupertinoColors.white.withValues(alpha: 0.06),
      thickness: 8,
      blur: 12,
      refractiveIndex: 0.08,
      lightIntensity: 0.28,
      ambientStrength: 0.10,
      fresnelStrength: 0.12,
      specularSharpness: GlassSpecularSharpness.soft,
    );
  }

  static Color controlForeground(BuildContext context) =>
      ConsolePalette.of(context).label;

  static LiquidGlassSettings sidebar(BuildContext context) {
    final base = sheet(context);
    if (!ConsolePalette.of(context).isDark) return base;
    return base.copyWith(
      glassColor: CupertinoColors.white.withValues(alpha: 0.015),
      thickness: 8,
      blur: 24,
      lightIntensity: 0.38,
      ambientStrength: 0.10,
      refractiveIndex: 0.15,
      fresnelStrength: 0.12,
      specularSharpness: GlassSpecularSharpness.soft,
    );
  }

  static LiquidGlassSettings inspector(BuildContext context) {
    final palette = ConsolePalette.of(context);
    if (palette.isDark) return sidebar(context);
    return LiquidGlassSettings(
      backerColor: palette.canvas.withValues(alpha: 0.94),
      glassColor: CupertinoColors.white.withValues(alpha: 0.12),
      blur: 24,
      thickness: 8,
      refractiveIndex: 0.08,
      lightIntensity: 0.15,
      ambientStrength: 0.03,
      fresnelStrength: 0.06,
      shadowElevation: 1,
      specularSharpness: GlassSpecularSharpness.soft,
    );
  }

  /// 大面积面板（侧边栏、检查器、模态面板）的玻璃设置。
  ///
  /// 唯一需要显式覆盖的角色，原因是**几何**而非配色：`refractiveIndex` 在 0.7 时会在大
  /// 面板四周留一圈明显亮边，面板越大越刺眼，上游为此把 sheet 预设降到 0.15。其余字段仍
  /// 从当前亮度的主题变体派生，因此这里没有引入任何硬编码色值。
  static LiquidGlassSettings sheet(BuildContext context) {
    final base = const LiquidGlassSettings();
    final theme = GlassThemeData.of(context).settingsFor(context);
    return (theme?.applyTo(base) ?? base).copyWith(refractiveIndex: 0.15);
  }

  /// 玻璃主题的明暗两套变体。只覆盖质量，其余沿用组件库按 iOS 26 调校的取值。
  static GlassThemeData theme() => GlassThemeData.simple(quality: quality);
}
