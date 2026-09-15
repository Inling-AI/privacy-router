/// 通用小部件：悬浮表面、玻璃面板、分组列表、状态提示、控件。
///
/// ## 分工
///
/// - **玻璃**（[GlassPanel]、[ActionButton]、导航栏、浮层）负责「悬浮在内容之上」的东西。
/// - **不透明表面**（[ElevatedSurface]、[GroupedList]、[MetricCard]）负责滚动的正文。
///   正文用不透明表面有三个理由：滚动时不再逐帧做模糊采样；层次由底色台阶 + 阴影表达，
///   正是 iOS 分组列表的样子；玻璃套玻璃会让内侧退化成非折射路径（上游的硬约束）。
///
/// ## 两条来自上游的约束
///
/// 1. **文字按钮必须用 `GlassButton.custom`**：`GlassButton` 的 `label` 参数只是给读屏
///    软件的语义标签，传了也不会渲染文字。
/// 2. **交互式玻璃控件不能放进 `GlassContainer` / `GlassCard`**：容器会给整棵子树设置
///    `avoidsRefraction`，内侧玻璃退化为非折射路径，弹性动画的过冲还会被容器形状裁剪。
library;

import 'dart:ui' show ImageFilter;

import 'package:animated_flip_counter/animated_flip_counter.dart';
import 'package:cupertino_ui/cupertino_ui.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';

import '../api/models.dart';
import '../i18n/model_translations.dart';
import '../i18n/strings.g.dart';
import '../theme/elevation.dart';
import '../theme/palette.dart';
import '../theme/tokens.dart';
import 'motion.dart';

// ---------------------------------------------------------------------------
// 表面
// ---------------------------------------------------------------------------

/// 不透明悬浮表面：超椭圆形状 + 半透明材质 + 环境光遮蔽阴影 + 0.5pt 边缘高光。
///
/// ## 三件事合在一起才像 Apple 的卡片
///
/// 1. **超椭圆**。`BorderRadius` 是「直线瞬间接一段圆弧」，连接处曲率突变，看起来有个拐角。
///    Apple 全系统用连续曲率（超椭圆），曲率从直线渐进过渡到圆弧。`LiquidRoundedSuperellipse`
///    就是这条曲线，因此它同时决定了填充、裁剪与边缘高光的形状——三者的轮廓是同一条曲线，
///    不会出现「裁剪是圆角矩形、描边是超椭圆」的错位。
/// 2. **半透明填充**。见 [ConsoleElevationStyle.fillAlpha]：卡片是压在底板上的一层膜，
///    不是一块纯色。
/// 3. **0.5pt 边缘高光**。见 [ConsoleElevationStyle.rimLight]。
class ElevatedSurface extends StatelessWidget {
  const ElevatedSurface({
    required this.child,
    this.elevation = ConsoleElevation.raised,
    this.color,
    this.radius = ConsoleRadius.card,
    this.padding = EdgeInsets.zero,
    this.margin = EdgeInsets.zero,
    super.key,
  });

  final Widget child;
  final ConsoleElevation elevation;

  /// 底色。默认取当前亮度的卡片底色；要换的场合一般只有一种——比底板更靠前。
  ///
  /// 层级透明度乘在语义色自身的透明度上，保留 systemFill 等颜色的叠加语义。
  final Color? color;

  final double radius;
  final EdgeInsetsGeometry padding;
  final EdgeInsetsGeometry margin;

  @override
  Widget build(BuildContext context) {
    final palette = ConsolePalette.of(context);
    final shape = LiquidRoundedSuperellipse(borderRadius: radius);
    final base = color ?? palette.surface;
    final fill = base.withValues(alpha: base.a * elevation.fillAlpha);

    Widget surface = DecoratedBox(
      // 形状只用来定阴影的轮廓；填充与描边由下面两层分别画，
      // 因为填充要压在模糊层上，而描边要压住内容（行内高亮会铺到边）。
      decoration: ShapeDecoration(
        shape: shape,
        shadows: elevation.shadows(context),
      ),
      child: ClipPath(
        clipper: LiquidShapeClipper(shape),
        child: _material(context, shape, fill),
      ),
    );

    // 边缘高光画在整个表面之上：它模拟的是物体边缘受光，必须压住底色与内容。
    // 内嵌块（flat）不加这一层：它嵌在卡片里，靠填充对比度区分，套一圈亮边会像另一个浮起物。
    if (elevation != ConsoleElevation.flat) {
      surface = CustomPaint(
        foregroundPainter: RimLightPainter(
          shape: shape,
          color: elevation.rimLight(context),
        ),
        child: surface,
      );
    }

    return Padding(padding: margin, child: surface);
  }

  /// 填充层。需要模糊的层级才铺 [BackdropFilter]，否则直接刷半透明底色。
  Widget _material(BuildContext context, LiquidShape shape, Color fill) {
    final content = DecoratedBox(
      decoration: ShapeDecoration(shape: shape, color: fill),
      child: Padding(padding: padding, child: child),
    );

    final blur = elevation.materialBlur;
    if (blur <= 0) return content;

    return BackdropFilter(
      filter: ImageFilter.blur(sigmaX: blur, sigmaY: blur),
      child: content,
    );
  }
}

/// 把一个 [LiquidShape] 当作裁剪路径。
///
/// 组件库只给形状（`getOuterPath`），没给现成的 clipper；而这个应用里所有表面的裁剪都必须
/// 与填充、描边用**同一条曲线**，所以按形状裁这件事只能有这一个实现。
class LiquidShapeClipper extends CustomClipper<Path> {
  const LiquidShapeClipper(this.shape);

  final LiquidShape shape;

  @override
  Path getClip(Size size) => shape.getOuterPath(Offset.zero & size);

  @override
  bool shouldReclip(LiquidShapeClipper oldClipper) => oldClipper.shape != shape;
}

/// 玻璃面板：组件库渲染的高斯模糊 + 饱和度提升材质，外面补上阴影与边缘高光。
class GlassPanel extends StatelessWidget {
  const GlassPanel({
    required this.child,
    this.padding = EdgeInsets.zero,
    this.radius = ConsoleRadius.panel,
    this.elevation = ConsoleElevation.floating,
    this.settings,
    super.key,
  });

  final Widget child;
  final EdgeInsetsGeometry padding;
  final double radius;
  final ConsoleElevation elevation;

  /// 玻璃设置。留空即继承主题（按当前亮度取组件库调好的那一支）。
  final LiquidGlassSettings? settings;

  @override
  Widget build(BuildContext context) {
    // 浅色模式下组件库自己会画玻璃的 AO 阴影（藏在玻璃外圈，不会把自己的影子糊掉），
    // 深色模式下它刻意跳过——纯黑底上玻璃自带发光边缘，投影会被吞掉。但我们的底板不总是
    // 纯黑内容：深色模式下不补这一层，面板就会像贴纸一样贴在内容上。
    final shadows = ConsolePalette.of(context).isDark
        ? elevation.shadows(context)
        : const <BoxShadow>[];

    return CustomPaint(
      foregroundPainter: RimLightPainter(
        shape: LiquidRoundedSuperellipse(borderRadius: radius),
        color: elevation.rimLight(context),
      ),
      child: DecoratedBox(
        decoration: ShapeDecoration(
          shape: LiquidRoundedSuperellipse(borderRadius: radius),
          shadows: shadows,
        ),
        child: GlassContainer(
          settings: settings,
          useOwnLayer: settings != null,
          padding: padding,
          shape: LiquidRoundedSuperellipse(borderRadius: radius),
          child: child,
        ),
      ),
    );
  }
}

/// 沿超椭圆描一圈发丝线，模拟物体边缘的倒角反光。
///
/// 用组件库的 `LiquidShape.getOuterPath`，因此这条线与玻璃的轮廓是**同一条曲线**：
/// 自己用 `BorderRadius` 画会得到曲率不同的两种圆角，叠在一起能看出错位。
class RimLightPainter extends CustomPainter {
  const RimLightPainter({required this.shape, required this.color});

  final LiquidShape shape;
  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = ConsoleElevationStyle.rimWidth
      ..isAntiAlias = true
      ..color = color;
    canvas.drawPath(shape.getOuterPath(Offset.zero & size), paint);
  }

  @override
  bool shouldRepaint(RimLightPainter oldDelegate) =>
      oldDelegate.color != color || oldDelegate.shape != shape;
}

// ---------------------------------------------------------------------------
// 分组列表
// ---------------------------------------------------------------------------

/// 苹果式插入式分组列表：一块圆角表面，行之间是发丝线。
///
/// 分隔线从**文字列**开始而不是贴到边缘——这是 iOS 列表最容易被忽略、也最容易被看出
/// 是「网页仿的」的一处细节。
class GroupedList extends StatelessWidget {
  const GroupedList({
    required this.children,
    this.header,
    this.footer,
    this.separatorInset = ListRow.iconColumn + ConsoleSpace.m,
    super.key,
  });

  final List<Widget> children;

  /// 列表上方的分组标题（小字、大写、弱化）。
  final String? header;

  /// 列表下方的脚注说明。
  final String? footer;

  /// 分隔线左端缩进（相对行内边距），默认对齐「带前导图标」的文字列。
  ///
  /// 默认值由 [ListRow.iconColumn] 推导，不写常量：图标列的宽度属于行，不属于列表。
  /// 无图标的列表传 `0`，分隔线才会落在文字的左边缘上。
  final double separatorInset;

  @override
  Widget build(BuildContext context) {
    final palette = ConsolePalette.of(context);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        if (header case final header?)
          Padding(
            padding: const EdgeInsets.fromLTRB(
              ConsoleMetrics.groupInset + 4,
              0,
              ConsoleMetrics.groupInset + 4,
              ConsoleSpace.s,
            ),
            child: Text(
              header.toUpperCase(),
              style: ConsoleText.groupHeader.copyWith(
                color: palette.secondaryLabel,
              ),
            ),
          ),
        ElevatedSurface(
          elevation: ConsoleElevation.flat,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              for (var index = 0; index < children.length; index++) ...[
                children[index],
                if (index != children.length - 1)
                  Hairline(
                    color: palette.separator,
                    indent: separatorInset + ConsoleMetrics.groupInset,
                  ),
              ],
            ],
          ),
        ),
        if (footer case final footer?)
          Padding(
            padding: const EdgeInsets.fromLTRB(
              ConsoleMetrics.groupInset + 4,
              ConsoleSpace.s,
              ConsoleMetrics.groupInset + 4,
              0,
            ),
            child: Text(
              footer,
              style: ConsoleText.footnote.copyWith(
                height: 16 / 13,
                color: palette.secondaryLabel,
              ),
            ),
          ),
      ],
    );
  }
}

/// 1 物理像素的分隔线。
///
/// 不用 `1.0` 逻辑像素：在 2x/3x 屏上那是 2–3 个像素，看起来是一条灰带而不是一道线。
class Hairline extends StatelessWidget {
  const Hairline({required this.color, this.indent = 0, super.key});

  final Color color;
  final double indent;

  @override
  Widget build(BuildContext context) {
    final ratio = MediaQuery.devicePixelRatioOf(context);
    return Padding(
      padding: EdgeInsets.only(left: indent),
      child: SizedBox(
        height: ratio > 0 ? 1 / ratio : 0.5,
        child: ColoredBox(color: color),
      ),
    );
  }
}

/// 分组列表里的一行。
///
/// 桌面端补上悬停高亮与选中底色：鼠标平台上「这一行可点」不能只靠指针形状。
class ListRow extends StatefulWidget {
  const ListRow({
    required this.title,
    this.subtitle,
    this.subtitleStyle,
    this.subtitleWidget,
    this.icon,
    this.iconTint,
    this.trailing,
    this.onTap,
    this.selected = false,
    this.chevron = false,
    this.titleStyle,
    super.key,
  });

  final String title;
  final String? subtitle;

  /// [subtitle] 的样式覆盖。默认是 [ConsoleText.footnote] 加次级文字色；
  /// 副标题里带数值时传 [ConsoleText.numericFootnote] 的同色版本，让数字等宽。
  final TextStyle? subtitleStyle;

  /// 代替 [subtitle] 的自定义内容（例如一条占比横杠）。两者同时给出时以本字段为准。
  final Widget? subtitleWidget;

  /// 前导图标。给了它，行会按 iOS 的图标列宽对齐。
  final IconData? icon;
  final Color? iconTint;

  final Widget? trailing;
  final VoidCallback? onTap;
  final bool selected;

  /// 尾部箭头。只用于「进入下一层」，不要用来装饰可点行。
  final bool chevron;

  final TextStyle? titleStyle;

  /// 前导图标列的宽度。
  ///
  /// 行的分隔线缩进、后续其它行型都从它推导——「图标列多宽」这个事实只在这里定义一次。
  static const double iconColumn = 28;

  @override
  State<ListRow> createState() => _ListRowState();
}

class _ListRowState extends State<ListRow> {
  bool _hovering = false;

  @override
  Widget build(BuildContext context) {
    final palette = ConsolePalette.of(context);
    final hoverable = widget.onTap != null || widget.trailing != null;
    final background = widget.selected
        ? palette.fill
        : (_hovering && hoverable)
        ? palette.secondaryFill
        : null;

    return MouseRegion(
      onEnter: (_) => setState(() => _hovering = true),
      onExit: (_) => setState(() => _hovering = false),
      cursor: widget.onTap == null
          ? MouseCursor.defer
          : SystemMouseCursors.click,
      child: GestureDetector(
        onTap: widget.onTap,
        behavior: HitTestBehavior.opaque,
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 150),
          curve: Curves.easeOut,
          color: background ?? const Color(0x00000000),
          child: ConstrainedBox(
            constraints: const BoxConstraints(
              minHeight: ConsoleMetrics.rowHeight,
            ),
            child: Padding(
              padding: const EdgeInsets.symmetric(
                horizontal: ConsoleMetrics.groupInset,
                vertical: ConsoleSpace.m,
              ),
              child: Row(
                children: [
                  if (widget.icon case final icon?) ...[
                    SizedBox(
                      width: ListRow.iconColumn,
                      child: _IconTile(
                        icon: icon,
                        tint: widget.iconTint ?? palette.tint,
                      ),
                    ),
                    const SizedBox(width: ConsoleSpace.m),
                  ],
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Text(
                          widget.title,
                          maxLines: 2,
                          overflow: TextOverflow.ellipsis,
                          style:
                              widget.titleStyle ??
                              ConsoleText.body.copyWith(color: palette.label),
                        ),
                        if (widget.subtitleWidget
                            case final subtitleWidget?) ...[
                          const SizedBox(height: ConsoleSpace.s),
                          subtitleWidget,
                        ] else if (widget.subtitle case final subtitle?) ...[
                          const SizedBox(height: 2),
                          Text(
                            subtitle,
                            maxLines: 2,
                            overflow: TextOverflow.ellipsis,
                            style:
                                widget.subtitleStyle ??
                                ConsoleText.footnote.copyWith(
                                  color: palette.secondaryLabel,
                                ),
                          ),
                        ],
                      ],
                    ),
                  ),
                  if (widget.trailing case final trailing?) ...[
                    const SizedBox(width: ConsoleSpace.m),
                    trailing,
                  ],
                  if (widget.chevron) ...[
                    const SizedBox(width: ConsoleSpace.s),
                    Icon(
                      CupertinoIcons.chevron_forward,
                      size: 14,
                      color: palette.tertiaryLabel,
                    ),
                  ],
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// 卡片
// ---------------------------------------------------------------------------

/// 指标卡：小标题、大号数值、可选副标题与强调色。
///
/// 三级文字的呼吸间距刻意拉开：数值上下各留一档，它才是这张卡上要被人读到的东西。
/// 三者都贴在一起时（旧版就是），眼睛得先在三个字号里找哪个是重点。
class MetricCard extends StatelessWidget {
  const MetricCard({
    required this.title,
    required this.value,
    this.secondaryValue,
    this.suffix = '',
    this.fractionDigits = 0,
    this.subtitle,
    this.tint,
    this.emphasis = false,
    super.key,
  });

  final String title;
  final num? value;
  final num? secondaryValue;
  final String suffix;
  final int fractionDigits;
  final String? subtitle;
  final Color? tint;

  /// 强调数值本身（用于最重要的那个指标）。
  final bool emphasis;

  @override
  Widget build(BuildContext context) {
    final palette = ConsolePalette.of(context);

    return ElevatedSurface(
      // 内边距刻意**不等于**圆角半径（`ConsoleRadius.card` = 18）：两者相等时，
      // 文字的左边缘正好落在圆角与直边的切点上，短标题看起来像掉进圆角里。
      padding: const EdgeInsets.symmetric(
        horizontal: ConsoleSpace.l,
        vertical: ConsoleSpace.l,
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: [
          Text(
            title,
            style: ConsoleText.footnote.copyWith(
              fontWeight: FontWeight.w500,
              color: palette.secondaryLabel,
            ),
          ),
          const SizedBox(height: ConsoleSpace.m),
          SizedBox(
            width: double.infinity,
            height: MediaQuery.textScalerOf(context).scale(emphasis ? 44 : 38),
            child: FittedBox(
              fit: BoxFit.scaleDown,
              alignment: Alignment.centerLeft,
              child: value == null
                  ? Text(
                      '--',
                      style: ConsoleText.metric.copyWith(
                        color: palette.secondaryLabel,
                      ),
                    )
                  : MetricValue(
                      value: value!,
                      secondaryValue: secondaryValue,
                      suffix: suffix,
                      fractionDigits: fractionDigits,
                      style: ConsoleText.metric.copyWith(
                        fontSize: emphasis ? 34 : 28,
                        color: tint ?? palette.label,
                      ),
                    ),
            ),
          ),
          if (subtitle case final subtitle?) ...[
            const SizedBox(height: ConsoleSpace.m),
            Text(
              subtitle,
              style: ConsoleText.caption.copyWith(
                color: palette.secondaryLabel,
              ),
            ),
          ],
        ],
      ),
    );
  }
}

/// Numeric values keep their identity across updates; units never animate.
class MetricValue extends StatelessWidget {
  const MetricValue({
    required this.value,
    this.secondaryValue,
    this.suffix = '',
    this.fractionDigits = 0,
    this.style,
    super.key,
  });

  final num value;
  final num? secondaryValue;
  final String suffix;
  final int fractionDigits;
  final TextStyle? style;

  @override
  Widget build(BuildContext context) {
    final duration = MediaQuery.disableAnimationsOf(context)
        ? Duration.zero
        : const Duration(milliseconds: 350);
    final textStyle =
        style ??
        ConsoleText.value.copyWith(color: ConsolePalette.of(context).label);
    return Semantics(
      label:
          '${value.toStringAsFixed(fractionDigits)}'
          '${secondaryValue == null ? '' : ' / ${secondaryValue!.toStringAsFixed(fractionDigits)}'}$suffix',
      child: ExcludeSemantics(
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            for (final (index, number) in [value, ?secondaryValue].indexed) ...[
              if (index > 0) Text(' / ', style: textStyle),
              AnimatedFlipCounter(
                value: number,
                fractionDigits: fractionDigits,
                duration: duration,
                negativeSignDuration: duration,
                curve: Curves.easeOutCubic,
                textStyle: textStyle,
              ),
            ],
            if (suffix.isNotEmpty) Text(suffix, style: textStyle),
          ],
        ),
      ),
    );
  }
}

/// 指标卡的排布：桌面横排，窄屏竖排。
class CardRow extends StatelessWidget {
  const CardRow({required this.cards, required this.isDesktop, super.key});

  final List<Widget> cards;
  final bool isDesktop;

  @override
  Widget build(BuildContext context) {
    if (!isDesktop) {
      return Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          for (final card in cards) ...[
            card,
            const SizedBox(height: ConsoleSpace.m),
          ],
        ],
      );
    }
    // `CrossAxisAlignment.stretch` 需要行有确定高度；而本控件通常放在 sliver 里，
    // 高度是**无界**的，直接 stretch 会把 h=Infinity 传给子项并触发断言。
    // 用 IntrinsicHeight 先算出内容高度，再 stretch 让卡片等高。
    return IntrinsicHeight(
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          for (var index = 0; index < cards.length; index++) ...[
            Expanded(child: cards[index]),
            if (index != cards.length - 1)
              const SizedBox(width: ConsoleSpace.m),
          ],
        ],
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// 标题与提示
// ---------------------------------------------------------------------------

/// 列表行前导图标的底片：超椭圆微彩色方块 + 同色图标。
///
/// 就是 iOS「设置」里那种图标：光秃蔾的线性图标在密集列表里会变成一堆同样粗细的线条，
/// 扫读时无法靠颜色和形状快速定位；包上一层 12% 同色底的方块后，颜色先被眼睛看到，
/// 形状其次。底片尺寸固定，因此所有行的图标列天然对齐。
class _IconTile extends StatelessWidget {
  const _IconTile({required this.icon, required this.tint});

  final IconData icon;
  final Color tint;

  @override
  Widget build(BuildContext context) {
    final shape = LiquidRoundedSuperellipse(
      borderRadius: ConsoleRadius.iconTile,
    );

    return Align(
      alignment: Alignment.centerLeft,
      child: ClipPath(
        clipper: LiquidShapeClipper(shape),
        child: ColoredBox(
          color: tint.withValues(alpha: 0.12),
          child: SizedBox.square(
            dimension: ListRow.iconColumn,
            child: Icon(icon, size: 16, color: tint),
          ),
        ),
      ),
    );
  }
}

/// 区块标题。大标题由外壳渲染，这里是页面内部的分段。
///
/// 只有标题与小标题，没有右侧插槽：一级标题右侧原本可以挂一个主操作，但主操作已经
/// 统一归顶部工具栏（见外壳的 `_appBarActions`）。同一个「主操作在哪」的事实只留一处，
/// 否则页面会重新长出第二套按钮位置。
class SectionHeader extends StatelessWidget {
  const SectionHeader({required this.title, this.subtitle, super.key});

  final String title;
  final String? subtitle;

  @override
  Widget build(BuildContext context) {
    final palette = ConsolePalette.of(context);

    return Padding(
      padding: const EdgeInsets.only(bottom: ConsoleSpace.m),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  title,
                  style: ConsoleText.headline.copyWith(color: palette.label),
                ),
                if (subtitle case final subtitle?) ...[
                  const SizedBox(height: 2),
                  Text(
                    subtitle,
                    style: ConsoleText.footnote.copyWith(
                      color: palette.secondaryLabel,
                    ),
                  ),
                ],
              ],
            ),
          ),
        ],
      ),
    );
  }
}

/// 提示的语气。语义色只用来定调，不用来铺满大面积。
///
/// 只剩两档：中性提示与告警提示。列表的常规说明已经归 `ListNote`（纯文字，无语气），
/// 不需要给每段说明配一种颜色。
enum NoticeTone { neutral, negative }

/// 说明性提示：兜底行为、路由规则、环境未就绪等。
///
/// 刻意**不加背景块**：它跟在列表之后，是那张列表的脚注，而不是一块新卡片。
/// 旧版用一个带底色的圆角大药丸包住它，结果一条辅助说明在页面底部比正文列表还重，
/// 眼睛先被它捕获。现在只保留小图标上的语义色——语气该由颜色表达，而不是由面积表达。
class InfoNotice extends StatelessWidget {
  const InfoNotice({
    required this.text,
    this.icon = CupertinoIcons.info_circle,
    this.tone = NoticeTone.neutral,
    super.key,
  });

  final String text;
  final IconData icon;
  final NoticeTone tone;

  @override
  Widget build(BuildContext context) {
    final palette = ConsolePalette.of(context);
    final accent = switch (tone) {
      NoticeTone.neutral => palette.secondaryLabel,
      NoticeTone.negative => palette.negative,
    };

    // 与分组列表的脚注同一缩进（左侧多一格对齐图标列），两端留白一致。
    return Padding(
      padding: const EdgeInsets.fromLTRB(
        ConsoleSpace.xs,
        0,
        ConsoleSpace.xs,
        0,
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Padding(
            padding: const EdgeInsets.only(top: 2),
            child: Icon(icon, size: 13, color: accent),
          ),
          const SizedBox(width: ConsoleSpace.s),
          Expanded(
            child: Text(
              text,
              style: ConsoleText.caption.copyWith(
                color: palette.secondaryLabel,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

/// 类别徽标：用颜色区分实体类别，便于在密集列表里快速扫读。
class EntityChip extends StatelessWidget {
  const EntityChip({required this.group, super.key});

  final EntityGroup group;

  @override
  Widget build(BuildContext context) {
    final tint = group.tint.resolveFrom(context);

    return DecoratedBox(
      decoration: ShapeDecoration(
        color: tint.withValues(alpha: 0.16),
        shape: LiquidRoundedSuperellipse(borderRadius: ConsoleRadius.small),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(
          horizontal: ConsoleSpace.s,
          vertical: 3,
        ),
        child: Text(
          group.localized(context.t),
          style: ConsoleText.caption.copyWith(
            fontWeight: FontWeight.w600,
            color: tint,
          ),
        ),
      ),
    );
  }
}

/// 弱化的小字说明。
class Muted extends StatelessWidget {
  const Muted({required this.text, super.key});

  final String text;

  @override
  Widget build(BuildContext context) {
    return Text(
      text,
      style: ConsoleText.footnote.copyWith(
        color: ConsolePalette.of(context).secondaryLabel,
      ),
    );
  }
}

/// 列表下方的补充说明。
///
/// 与分组列表自带的分组脚注（`GroupedList.footer`）是同一个角色：都是「这张列表之外
/// 还有一件事要说」。区别只在挂靠对象——脚注属于**某一组**，本控件属于**整页**，
/// 因此它跟在最后一张卡片之后，而不是被塞进某一张卡里。
///
/// 两者共用 [ConsoleMetrics.groupTextInset]，同一页里的注释文字才会落在同一条左边界上。
class ListNote extends StatelessWidget {
  const ListNote({required this.text, super.key});

  final String text;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(
        horizontal: ConsoleMetrics.groupTextInset,
      ),
      child: Text(
        text,
        style: ConsoleText.caption.copyWith(
          color: ConsolePalette.of(context).secondaryLabel,
        ),
      ),
    );
  }
}

/// 空状态与错误提示。
class MessageState extends StatelessWidget {
  const MessageState({
    required this.message,
    this.icon = CupertinoIcons.info_circle,
    this.action,
    super.key,
  });

  final String message;
  final IconData icon;
  final Widget? action;

  @override
  Widget build(BuildContext context) {
    final palette = ConsolePalette.of(context);

    return Padding(
      padding: const EdgeInsets.symmetric(
        vertical: ConsoleSpace.section,
        horizontal: ConsoleSpace.xl,
      ),
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(icon, size: 34, color: palette.tertiaryLabel),
          const SizedBox(height: ConsoleSpace.m),
          Text(
            message,
            textAlign: TextAlign.center,
            style: ConsoleText.subheadline.copyWith(
              color: palette.secondaryLabel,
            ),
          ),
          if (action != null) ...[
            const SizedBox(height: ConsoleSpace.xl),
            action!,
          ],
        ],
      ),
    );
  }
}

/// 加载指示器。
class LoadingState extends StatelessWidget {
  const LoadingState({this.label, super.key});

  final String? label;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: ConsoleSpace.section),
      child: Column(
        children: [
          const GlassProgressIndicator.circular(size: 22, strokeWidth: 3),
          if (label != null) ...[
            const SizedBox(height: ConsoleSpace.l),
            Text(
              label!,
              style: ConsoleText.subheadline.copyWith(
                color: ConsolePalette.of(context).secondaryLabel,
              ),
            ),
          ],
        ],
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// 操作
// ---------------------------------------------------------------------------

/// 操作按钮。
///
/// - 默认是**弱按钮**：玻璃胶囊 + tint 文字。对应 iOS 的 `.glass` 按钮，用在标题行右侧
///   （「新建规则」「重试」「放行」）。
/// - [prominent] 是**主按钮**：tint 实色胶囊 + 白字，一屏只该出现一个，对应 iOS 的
///   `borderedProminent`（「登录」「保存」）。
class ActionButton extends StatelessWidget {
  const ActionButton({
    required this.text,
    required this.onPressed,
    this.icon,
    this.prominent = false,
    this.expand = false,
    this.height,
    this.tint,
    super.key,
  });

  final String text;

  /// 为 `null` 时按钮呈禁用态。上游约定：保持 `onTap` 非空并传 `enabled: false`。
  final VoidCallback? onPressed;

  final IconData? icon;
  final bool prominent;

  /// 撑满父级宽度（表单里的主操作）。
  final bool expand;

  final double? height;

  /// 前景/填充色，用于区分危险操作等。
  final Color? tint;

  @override
  Widget build(BuildContext context) {
    final palette = ConsolePalette.of(context);
    final enabled = onPressed != null;
    final color = tint ?? palette.tint;
    final buttonHeight = height ?? ConsoleMetrics.buttonHeight;
    // GlassButton applies disabled opacity; do not dim its label a second time.
    final foreground = prominent
        ? palette.onTint
        : tint ?? ConsoleGlass.controlForeground(context);

    final label = Padding(
      padding: const EdgeInsets.symmetric(horizontal: ConsoleSpace.l),
      child: Row(
        mainAxisSize: expand ? MainAxisSize.max : MainAxisSize.min,
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          if (icon != null) ...[
            Icon(icon, size: 16, color: foreground),
            const SizedBox(width: ConsoleSpace.s),
          ],
          Text(
            text,
            style: ConsoleText.subheadline.copyWith(
              fontWeight: FontWeight.w600,
              color: foreground,
            ),
          ),
        ],
      ),
    );

    if (!prominent) {
      return GlassButton.custom(
        settings: ConsoleGlass.control(context),
        useOwnLayer: true,
        onTap: onPressed ?? () {},
        enabled: enabled,
        width: expand ? double.infinity : null,
        height: buttonHeight,
        shape: LiquidRoundedRectangle(borderRadius: ConsoleRadius.control),
        child: label,
      );
    }

    return _ProminentButton(
      onPressed: onPressed,
      expand: expand,
      height: buttonHeight,
      color: color,
      child: label,
    );
  }
}

/// 实色主按钮。按下时用弹簧缩一下——iOS 的按钮在手感上是「实体」，不是平面。
class _ProminentButton extends StatefulWidget {
  const _ProminentButton({
    required this.onPressed,
    required this.expand,
    required this.height,
    required this.color,
    required this.child,
  });

  final VoidCallback? onPressed;
  final bool expand;
  final double height;
  final Color color;
  final Widget child;

  @override
  State<_ProminentButton> createState() => _ProminentButtonState();
}

class _ProminentButtonState extends State<_ProminentButton> {
  bool _pressed = false;

  void _setPressed(bool value) {
    if (widget.onPressed == null || _pressed == value) return;
    setState(() => _pressed = value);
  }

  @override
  Widget build(BuildContext context) {
    final enabled = widget.onPressed != null;

    return SpringValue(
      value: _pressed ? 0.97 : 1,
      builder: (context, scale) => Transform.scale(
        scale: scale,
        child: MouseRegion(
          cursor: enabled ? SystemMouseCursors.click : MouseCursor.defer,
          child: GestureDetector(
            onTapDown: (_) => _setPressed(true),
            onTapUp: (_) => _setPressed(false),
            onTapCancel: () => _setPressed(false),
            onTap: widget.onPressed,
            behavior: HitTestBehavior.opaque,
            child: DecoratedBox(
              decoration: BoxDecoration(
                color: enabled
                    ? widget.color
                    : widget.color.withValues(alpha: 0.35),
                borderRadius: BorderRadius.circular(widget.height / 2),
              ),
              child: SizedBox(
                width: widget.expand ? double.infinity : null,
                height: widget.height,
                child: Center(child: widget.child),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

/// 失败后的重试入口。
class RetryButton extends StatelessWidget {
  const RetryButton({required this.onRetry, super.key});

  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    return ActionButton(
      text: context.t.common.retry,
      icon: CupertinoIcons.arrow_clockwise,
      onPressed: onRetry,
    );
  }
}

// ---------------------------------------------------------------------------
// 文本
// ---------------------------------------------------------------------------

/// 可选中、可复制的只读文本。
///
/// framework 里能选中文本的 `SelectableText` 与 `SelectionArea` 都属于 material，本项目只
/// 依赖 Cupertino，因此用只读的 `CupertinoTextField` 实现同样效果。
class SelectableValue extends StatelessWidget {
  const SelectableValue({
    required this.text,
    this.fontSize = 13,
    this.color,
    this.height,
    super.key,
  });

  final String text;
  final double fontSize;
  final Color? color;
  final double? height;

  @override
  Widget build(BuildContext context) {
    return CupertinoTextField.borderless(
      controller: TextEditingController(text: text),
      readOnly: true,
      maxLines: null,
      style: TextStyle(
        fontFamily: 'Menlo',
        fontSize: fontSize,
        height: height,
        color: color ?? ConsolePalette.of(context).label,
      ),
      padding: EdgeInsets.zero,
    );
  }
}

/// 键值对一行。键在左，值可复制。
class DetailRow extends StatelessWidget {
  const DetailRow({required this.label, required this.value, super.key});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: ConsoleSpace.s),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 96,
            child: Text(
              label,
              style: ConsoleText.footnote.copyWith(
                color: ConsolePalette.of(context).secondaryLabel,
              ),
            ),
          ),
          Expanded(child: SelectableValue(text: value)),
        ],
      ),
    );
  }
}

/// 把组件库基于下标的 `GlassSegmentedControl` 适配成按值选择。
///
/// 组件库的接口是 `segments` + `selectedIndex` + `onSegmentSelected`，直接用会让每个调用点
/// 各写一遍「值 ↔ 下标」的换算；配色也会在每个调用点各写一遍。这里换算与外观都只写一次。
///
/// ## 两个硬约束（都由组件库的行为决定，不是风格偏好）
///
/// 1. **必须有确定宽度**。组件库在 paint 阶段对自身宽度调用 `ceil()`，而 `Row` 会给非
///    `Expanded` 的子项**无界宽度**；对 `double.infinity` 求整会抛
///    `Unsupported operation: Infinity or NaN toInt`，并且**每帧重复抛出**。因此本控件内部
///    用 [LayoutBuilder] 检测到无界约束时给出可读的断言，而不是让异常在渲染层刷屏。
///    用法：放在 `Column` 里独占一行，或包在 `SizedBox`/`Expanded` 中。
///
/// 2. **不能放进 `GlassContainer` / `GlassCard`**。容器会给子树设置 `avoidsRefraction`，
///    内侧玻璃退化为非折射路径，弹性动画的过冲还会被容器裁剪。本控件自带表面，直接放页面。
class SegmentedPicker<T> extends StatelessWidget {
  const SegmentedPicker({
    required this.values,
    required this.labels,
    required this.selected,
    required this.onChanged,
    super.key,
  });

  final List<T> values;
  final List<String> labels;
  final T selected;
  final ValueChanged<T> onChanged;

  @override
  Widget build(BuildContext context) {
    assert(values.length == labels.length, 'Each value must have a label.');

    return LayoutBuilder(
      builder: (context, constraints) {
        // 无界宽度会在渲染层引发每帧异常，这里提前给出指明原因的错误。
        assert(
          constraints.hasBoundedWidth,
          'SegmentedPicker requires a finite width. Place it on its own row '
          'or wrap it in Expanded or SizedBox.',
        );
        if (!constraints.hasBoundedWidth) {
          // release 构建下断言被剥离；此时直接放弃渲染，避免每帧刷屏。
          return const SizedBox.shrink();
        }

        final index = values.indexOf(selected);
        final palette = ConsolePalette.of(context);

        return GlassSegmentedControl(
          segments: [for (final label in labels) GlassSegment(label: label)],
          selectedIndex: index < 0 ? 0 : index,
          onSegmentSelected: (value) {
            final target = value.clamp(0, values.length - 1);
            onChanged(values[target]);
          },
          // 轨道：极浅的半透明灰。它是一块凹下去的槽，不是一块实心灰。
          backgroundColor: palette.field,
          // 滑块：抬起来的白片，不靠玻璃折射表现自己。
          // 用 `indicatorSettings` 把玻璃压成薄薄一层，再交给 `indicatorShadow` 投影——
          // 这才是 iOS 分段控件的样子：一块有一点环境光的白片，不是一块发光的玻璃。
          indicatorColor: palette.surface,
          indicatorSettings: const LiquidGlassSettings(
            thickness: 0,
            blur: 0,
            refractiveIndex: 0,
            lightIntensity: 0,
            ambientStrength: 0,
            chromaticAberration: 0,
            saturation: 1,
          ),
          // 环境微阴影：贴近、向下 1pt、很浅。白色滑块与外框凹槽之间只有 2pt 的缝，
          // 没有这层影子，滑块就只是贴在框里的一块白，而不是「浮」在凹槽上。
          indicatorShadow: [
            BoxShadow(
              color: const Color(0x14000000),
              blurRadius: 4,
              offset: const Offset(0, 1),
            ),
          ],
          // 形状跟卡片走同一条超椭圆，控件与容器才是同一套语言。
          borderRadius: ConsoleRadius.control,
          // 不传 `indicatorBorderRadius`：组件库会自己算成 `外框半径 - 内边距`（10），
          // 两者同心；写死一个值就等于把这个关系拄到了调用点。
          selectedTextStyle: ConsoleText.footnote.copyWith(
            fontWeight: FontWeight.w600,
            color: palette.label,
          ),
          unselectedTextStyle: ConsoleText.footnote.copyWith(
            fontWeight: FontWeight.w500,
            color: palette.secondaryLabel,
          ),
        );
      },
    );
  }
}
