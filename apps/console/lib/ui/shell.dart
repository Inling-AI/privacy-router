/// 主界面外壳：按断点在三套布局之间切换。
///
/// - **compact**（< 700）：底部玻璃胶囊导航，大标题随内容滚动收起。
/// - **medium**（700–1099）：可折叠侧栏 + 独立详情导航栈。
/// - **expanded**（≥ 1100）：可折叠侧栏 + 独立详情导航栈 + 内容检查器。
///
/// 组件库只提供手机形态的导航原语（`GlassTabBar` 的各构造、`GlassAppBar`、`GlassToolbar`），
/// 没有侧边栏或分栏原语，因此宽屏布局由本项目用容器原语自行组合。
///
/// ## 三个决定
///
/// 1. **玻璃只用在导航与浮层上**（侧边栏、检查器、按钮、顶部栏）。滚动正文用不透明表面：
///    滚动时不必逐帧做模糊采样，层次由底色台阶 + 阴影表达——正是 iOS 分组列表的样子。
/// 2. **大标题由组件库的 `GlassLargeTitle` 驱动**：它按滚动进度收起，并与顶部栏的居中标题
///    交叉淡出。自己写一个 34pt 的 Text 不会有这个行为。
/// 3. **分区切换是弹簧驱动的交叉淡出**（见 `SectionSwitcher`），不是瞬间替换。
library;

import 'package:cupertino_ui/cupertino_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';

import '../state/providers.dart';
import 'language_menu.dart';
import '../i18n/model_translations.dart';
import '../i18n/strings.g.dart';
import '../theme/motion.dart';
import '../theme/brand.dart';
import '../theme/console_theme.dart';
import '../theme/palette.dart';
import '../theme/elevation.dart';
import '../theme/tokens.dart';
import 'layout.dart';
import 'detail_navigation_bar.dart';
import 'motion.dart';
import 'pages/overview_page.dart';
import 'pages/performance_page.dart';
import 'pages/pool_page.dart';
import 'pages/providers_page.dart';
import 'pages/rules_page.dart';
import 'sections.dart';
import 'widgets.dart';

/// 当前选中的分区。
class SelectedSection extends Notifier<ConsoleSection> {
  @override
  ConsoleSection build() => ConsoleSection.overview;

  void select(ConsoleSection section) => state = section;
}

final selectedSectionProvider =
    NotifierProvider<SelectedSection, ConsoleSection>(SelectedSection.new);

/// 侧边栏是否收起成图标轨。
///
/// 平板和桌面共用此偏好，可在完整侧栏与图标轨之间切换。
/// 仅存在本机，不进服务端设置——与外观一样，属于这台机器上的视图状态。
class SidebarCollapsed extends Notifier<bool> {
  @override
  bool build() => false;

  void toggle(bool collapsed) => state = collapsed;
}

final sidebarCollapsedProvider = NotifierProvider<SidebarCollapsed, bool>(
  SidebarCollapsed.new,
);

/// 整屏底板。
///
/// 刻意**不用渐变**：层级由三级底色台阶表达，靠前后距离而不是装饰。渐变会让玻璃折射出
/// 廉价的高光带，也会让浅色主题失去参照——白底上的白玻璃本该看不见。
class ConsoleCanvas extends StatelessWidget {
  const ConsoleCanvas({super.key});

  @override
  Widget build(BuildContext context) => ColoredBox(
    color: ConsolePalette.of(context).canvas,
    child: const SizedBox.expand(),
  );
}

/// 分区 → 页面。
///
/// 私有自由函数：这是一个对单一枚举的**完全映射**，写成自由函数是为了不让 `sections.dart`
/// 反过来依赖所有页面（那会形成枚举 ↔ 页面的循环）。除它之外本文件不再有自由函数。
Widget _pageFor(ConsoleSection section, {required bool isDesktop}) =>
    switch (section) {
      ConsoleSection.overview => OverviewPage(isDesktop: isDesktop),
      ConsoleSection.pool => const PoolPage(),
      ConsoleSection.rules => RulesPage(isDesktop: isDesktop),
      ConsoleSection.providers => ProvidersPage(isDesktop: isDesktop),
      ConsoleSection.performance => PerformancePage(isDesktop: isDesktop),
    };

/// 分区 → 顶部工具栏里的主操作。
///
/// 与 [_pageFor] 并列且同形：工具栏（[GlassAppBar]）归外壳所有，因此「哪个分区有什么
/// 主操作」也由外壳统一定位；按钮本身与它的面板仍留在各页面文件里。旧版把主操作挂在
/// 页面内的一级标题右侧，于是页面里多出一层「标题 → 说明 → 标题 → 按钮」，
/// 而它本该在顶栏上。
///
/// 一个分区最多一个主操作：多个实色按钮会同时争夺注意力，次要动作属于页面内容。
List<Widget> _appBarActions(ConsoleSection section) => switch (section) {
  ConsoleSection.rules => const [RuleListToolbar()],
  ConsoleSection.providers => const [NewUpstreamButton()],
  ConsoleSection.overview ||
  ConsoleSection.pool ||
  ConsoleSection.performance => const [],
};

/// 主界面。
class ConsoleShell extends ConsumerStatefulWidget {
  const ConsoleShell({super.key});

  @override
  ConsumerState<ConsoleShell> createState() => _ConsoleShellState();
}

class _ConsoleShellState extends ConsumerState<ConsoleShell> {
  /// 每个分区各持一个大标题控制器，它同时是那块内容的 `ScrollController`。
  ///
  /// 共用一个控制器会让「滚动位置」与「标题收起进度」在分区之间互相污染：从滚到一半的
  /// 列表切到短页面，标题会维持收起状态，看上去就是坏的。每分区一个之后两者各自独立。
  final _titles = {
    for (final section in ConsoleSection.values)
      section: GlassLargeTitleController(),
  };

  @override
  void dispose() {
    for (final controller in _titles.values) {
      controller.dispose();
    }
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final section = ref.watch(selectedSectionProvider);

    return LayoutBuilder(
      builder: (context, constraints) =>
          switch (Breakpoints.resolve(constraints.biggest)) {
            LayoutClass.compact => _compact(section),
            LayoutClass.medium => _framed(section, compactRail: false),
            LayoutClass.expanded => _framed(section, compactRail: false),
          },
    );
  }

  /// 手机：底部玻璃胶囊。顶部栏由 `GlassScaffold` 摆放，正文从它下面开始。
  Widget _compact(ConsoleSection section) {
    final palette = ConsolePalette.of(context);

    return GlassScaffold(
      background: const ConsoleCanvas(),
      // 边缘渐隐的目标色。不给的话深色模式下会退到近黑，把浅色底板洗出一道灰。
      backgroundColor: palette.canvas,
      appBar: GlassAppBar(
        title: Text(section.label(context.t)),
        largeTitleController: _titles[section],
        // 手机端没有侧边栏，外观入口只能放在顶栏。
        // 这里只能用**纯图标**触发器：带文字的那条路径会把文本放进不受约束的 `Row`，
        // 容器一窄就溢出（详见 [_AccountMenu]）。图标路径没有 `Row`，不可能溢出。
        actions: [
          ..._appBarActions(section),
          const LanguageMenu(),
          const _CompactAppearanceMenu(),
        ],
      ),
      bottomBar: GlassTabBar.bottom(
        verticalPadding: 0,
        horizontalPadding: ConsoleSpace.m,
        indicatorColor: palette.fill,
        // 选中项用强调色：iOS 的标签栏就是这样，颜色只标「你在哪」。
        selectedIconColor: palette.tint,
        selectedLabelColor: palette.tint,
        unselectedIconColor: palette.secondaryLabel,
        unselectedLabelColor: palette.secondaryLabel,
        tabs: [
          for (final item in ConsoleSection.values)
            GlassTab(
              label: item.label(context.t),
              icon: Icon(item.icon),
              activeIcon: Icon(item.activeIcon),
            ),
        ],
        selectedIndex: section.index,
        onTabSelected: (index) => _select(ConsoleSection.values[index]),
      ),
      body: _sections(
        section,
        isDesktop: false,
        // 顶部留出状态栏 + 导航栏：大标题在栏下，内容滚动时才从栏后穿过并淡出。
        topInset: MediaQuery.paddingOf(context).top + ConsoleMetrics.barHeight,
      ),
    );
  }

  /// 平板 / 桌面：左侧悬浮玻璃面板 + 拥有独立导航栈的详情区。
  Widget _framed(ConsoleSection section, {required bool compactRail}) {
    final palette = ConsolePalette.of(context);
    final collapsed = ref.watch(sidebarCollapsedProvider);
    // 断点要求轨道、或使用者主动收起，两种原因产生同一个结果。
    final rail = compactRail || collapsed;
    // 收起/展开的中点：宽度跨过这里时切到另一套内容布局。
    // 用**当前动画宽度**而不是目标状态做判断，内容才是跟着窗格一起变的，
    // 而不是在动画开头就瞬间就位。
    const threshold = Breakpoints.sidebarCollapseMidpoint;

    return GlassScaffold(
      background: const ConsoleCanvas(),
      backgroundColor: palette.canvas,
      body: SpringValue(
        value: rail ? Breakpoints.railWidth : Breakpoints.sidebarWidth,
        spring: ConsoleMotion.snappy,
        builder: (context, width) => MediaQuery(
          data: MediaQuery.of(context).copyWith(
            padding: MediaQuery.paddingOf(context).copyWith(
              top:
                  MediaQuery.paddingOf(context).top +
                  ConsoleMetrics.splitViewInset,
            ),
          ),
          child: GlassNavigationShell(
            chromeInsets: EdgeInsets.only(
              left: width + ConsoleMetrics.splitViewInset * 2,
            ),
            child: Stack(
              children: [
                Row(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    SizedBox(width: width + ConsoleMetrics.splitViewInset * 2),
                    Expanded(
                      child: Navigator(
                        key: ValueKey(section),
                        onGenerateRoute: (_) => CupertinoPageRoute<void>(
                          builder: (context) => ColoredBox(
                            color: ConsolePalette.of(context).canvas,
                            child: Stack(
                              children: [
                                Positioned.fill(
                                  child: _SectionScroll(
                                    section: section,
                                    controller: _titles[section]!,
                                    isDesktop: true,
                                    topInset:
                                        DetailNavigationBar.height +
                                        MediaQuery.paddingOf(context).top,
                                  ),
                                ),
                                Positioned(
                                  top: 0,
                                  left: 0,
                                  right: 0,
                                  child: DetailNavigationBar(
                                    title: section.label(context.t),
                                    controller: _titles[section],
                                    leading: section == ConsoleSection.rules
                                        ? RuleListToolbar.leadingItems(ref)
                                        : const [],
                                    actions: section == ConsoleSection.rules
                                        ? RuleListToolbar.items(context, ref)
                                        : section == ConsoleSection.providers
                                        ? [NewUpstreamButton.barItem(context)]
                                        : const [],
                                  ),
                                ),
                              ],
                            ),
                          ),
                        ),
                      ),
                    ),
                  ],
                ),
                // Pane surfaces paint above route backgrounds; the shell's
                // window-sized chrome overlay paints above both panes.
                Positioned(
                  top: 0,
                  bottom: 0,
                  left: 0,
                  child: MediaQuery(
                    data: MediaQuery.of(context),
                    child: _SidebarPane(
                      width: width,
                      child: _Sidebar(
                        compact: width < threshold,
                        canToggle: !compactRail,
                      ),
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }

  void _select(ConsoleSection section) =>
      ref.read(selectedSectionProvider.notifier).select(section);

  /// 分区内容的交叉淡出。新旧两份内容在切换期间同时存在，键由分区序号给出。
  Widget _sections(
    ConsoleSection section, {
    required bool isDesktop,
    required double topInset,
  }) {
    return SectionSwitcher(
      index: section.index,
      builder: (context, index) {
        final target = ConsoleSection.values[index];
        return _SectionScroll(
          section: target,
          controller: _titles[target]!,
          isDesktop: isDesktop,
          topInset: topInset,
        );
      },
    );
  }
}

/// 一个分区的滚动内容：大标题 + 说明 + 页面。
class _SectionScroll extends StatelessWidget {
  const _SectionScroll({
    required this.section,
    required this.controller,
    required this.isDesktop,
    required this.topInset,
  });

  final ConsoleSection section;
  final GlassLargeTitleController controller;
  final bool isDesktop;
  final double topInset;

  @override
  Widget build(BuildContext context) {
    final baseInset = ConsoleMetrics.pageInset(isDesktop);

    return LayoutBuilder(
      builder: (context, constraints) {
        final available = constraints.maxWidth - baseInset * 2;
        final extra = available > ConsoleMetrics.contentMaxWidth
            ? (available - ConsoleMetrics.contentMaxWidth) / 2
            : 0.0;
        final inset = baseInset + extra;

        return CustomScrollView(
          // 分区各存一份滚动位置：切回来时还在原处，与 iOS 的分页行为一致。
          key: PageStorageKey(section),
          controller: controller.scrollController,
          slivers: [
            SliverPadding(
              padding: EdgeInsets.fromLTRB(
                inset,
                topInset + ConsoleSpace.l,
                inset,
                ConsoleMetrics.bottomInset,
              ),
              sliver: SliverToBoxAdapter(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    _SectionIntro(section: section),
                    const SizedBox(height: ConsoleSpace.xxl),
                    _pageFor(section, isDesktop: isDesktop),
                  ],
                ),
              ),
            ),
          ],
        );
      },
    );
  }
}

/// 每个分区正文顶部的设置风格介绍卡：图标、标题和用途说明属于同一份分区定义。
class _SectionIntro extends StatelessWidget {
  const _SectionIntro({required this.section});
  final ConsoleSection section;

  @override
  Widget build(BuildContext context) {
    final palette = ConsolePalette.of(context);
    return ElevatedSurface(
      elevation: ConsoleElevation.flat,
      color: palette.surface,
      radius: ConsoleRadius.panel,
      padding: const EdgeInsets.all(ConsoleSpace.xl),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Container(
            width: 56,
            height: 56,
            decoration: ShapeDecoration(
              color: section.iconBackground.resolveFrom(context),
              shape: LiquidRoundedSuperellipse(
                borderRadius: ConsoleRadius.iconTile,
              ),
            ),
            child: Icon(section.activeIcon, color: palette.onTint, size: 30),
          ),
          const SizedBox(height: ConsoleSpace.l),
          Text(section.label(context.t), style: ConsoleText.headline),
          const SizedBox(height: ConsoleSpace.s),
          Text(
            section.description(context.t),
            style: ConsoleText.subheadline.copyWith(
              color: palette.secondaryLabel,
            ),
          ),
        ],
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// 导航
// ---------------------------------------------------------------------------

/// 独立悬浮的侧栏面板：四周留白、连续圆角和柔和阴影。
/// 玻璃背景与交互内容互为兄弟，避免搜索控件嵌入玻璃容器而失去折射。
class _SidebarPane extends StatelessWidget {
  const _SidebarPane({required this.width, required this.child});

  final double width;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    const margin = ConsoleMetrics.splitViewInset;
    const radius = ConsoleRadius.panel;
    const shape = LiquidRoundedSuperellipse(borderRadius: radius);
    final reduceMotion = MediaQuery.disableAnimationsOf(context);

    return SizedBox(
      width: width + margin * 2,
      child: Padding(
        padding: const EdgeInsets.all(margin),
        child: SafeArea(
          child: DecoratedBox(
            decoration: ShapeDecoration(
              shape: shape,
              shadows: ConsoleElevation.floating.shadows(context),
            ),
            // LiquidStretch 是库内部实现；通过公开的透明交互壳复用相同的弹簧物理。
            child: GlassButton.custom(
              onTap: () {},
              width: null,
              height: null,
              shape: shape,
              style: GlassButtonStyle.transparent,
              canRequestFocus: false,
              excludeFromSemantics: true,
              glowOpacity: 0,
              ambientBaseLight: 0,
              interactionScale: reduceMotion ? 1 : 1.003,
              stretch: reduceMotion ? 0 : 0.06,
              resistance: 0.12,
              child: DecoratedBox(
                decoration: ShapeDecoration(
                  shape: shape,
                  // 遮住投影在面板内部的填充，避免玻璃把自身阴影采样成整块灰色。
                  color: ConsolePalette.of(context).sidebarFill,
                ),
                child: Stack(
                  fit: StackFit.expand,
                  children: [
                    ClipPath(
                      clipper: const LiquidShapeClipper(shape),
                      child: GlassPanel(
                        radius: radius,
                        elevation: ConsoleElevation.flat,
                        settings: ConsoleGlass.sidebar(context),
                        child: const SizedBox.expand(),
                      ),
                    ),
                    GlassGlow(
                      enabled: ConsolePalette.of(context).isDark,
                      clipper: const LiquidShapeClipper(shape),
                      glowColor: CupertinoColors.white.withValues(alpha: 0.10),
                      glowRadius: 0.8,
                      glowBlurRadius: 16,
                      glowOnTapOnly: true,
                      child: ClipPath(
                        clipper: const LiquidShapeClipper(shape),
                        child: child,
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

/// 左侧导航：窗格 + 弹簧跟随的选中底片。
///
/// 两种形态：展开（含词标与完整文字）与图标轨（`compact`）。桌面端点表头右侧的按钮可随时
/// 在两者之间切换，对应 iPadOS / macOS 侧边栏那个收起折叠的动作。
class _Sidebar extends ConsumerStatefulWidget {
  const _Sidebar({required this.compact, required this.canToggle});

  /// 图标轨形态。
  final bool compact;

  /// 是否提供展开/收起。中等宽度下窗格本来就是轨道，没得选。
  final bool canToggle;

  @override
  ConsumerState<_Sidebar> createState() => _SidebarState();
}

class _SidebarState extends ConsumerState<_Sidebar> {
  String _query = '';
  bool get compact => widget.compact;
  bool get canToggle => widget.canToggle;

  @override
  Widget build(BuildContext context) {
    final section = ref.watch(selectedSectionProvider);
    final notifier = ref.read(selectedSectionProvider.notifier);
    // 水平内缩只在这里给一次。行（`_NavItem`）、词标、底部操作都不再各自内缩：
    // 叠加两次内缩会让图标轨形态只剩 20pt，连图标加间距都放不下。
    const inset = EdgeInsets.fromLTRB(
      ConsoleMetrics.sidebarRowInset,
      ConsoleSpace.l,
      ConsoleMetrics.sidebarRowInset,
      ConsoleSpace.l,
    );
    // 宽度取**实际约束**而不是目标形态：窗格宽度正被弹簧驱动，用目标值会让控件
    // 先跳到终态宽度再等窗格追上来。
    return LayoutBuilder(
      builder: (context, constraints) {
        // 展开程度：0 = 图标轨，1 = 完整窗格。它是**当前宽度的函数**，
        // 因此所有跟随它的位置都以同一个弹簧同步变化，不会各跳各的。
        final progress =
            ((constraints.maxWidth - Breakpoints.railWidth) /
                    (Breakpoints.sidebarWidth - Breakpoints.railWidth))
                .clamp(0.0, 1.0);

        return Padding(
          padding: inset,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              // 侧栏开关固定在窗口最左上角；词标从它之后开始。它不再挂在产品名右侧，
              // 因而在展开与收起过程中都保持标准窗口工具的位置。
              SizedBox(
                height: _SidebarToggle.extent,
                child: Stack(
                  children: [
                    Padding(
                      padding: const EdgeInsetsDirectional.only(
                        start:
                            ConsoleMetrics.sidebarIconColumn + ConsoleSpace.s,
                      ),
                      child: Align(
                        alignment: Alignment.centerLeft,
                        child: _Collapsed(
                          factor: progress,
                          child: const _Wordmark(),
                        ),
                      ),
                    ),
                    if (canToggle)
                      Align(
                        alignment: Alignment.centerLeft,
                        child: _SidebarToggle(
                          collapsed: compact,
                          onTap: () => ref
                              .read(sidebarCollapsedProvider.notifier)
                              .toggle(!compact),
                        ),
                      ),
                  ],
                ),
              ),
              const SizedBox(height: ConsoleSpace.xl),
              if (!compact) ...[
                GlassSearchBar(
                  useOwnLayer: true,
                  quality: ConsoleGlass.quality,
                  settings: ConsolePalette.of(context).isDark
                      ? ConsoleGlass.control(context)
                      : null,
                  placeholder: context.t.common.search,
                  onChanged: (value) =>
                      setState(() => _query = value.trim().toLowerCase()),
                ),
                const SizedBox(height: ConsoleSpace.xl),
              ],
              Expanded(
                child: SingleChildScrollView(
                  child: _NavList(
                    progress: progress,
                    selected: section,
                    onSelect: notifier.select,
                    sections: ConsoleSection.values
                        .where(
                          (item) =>
                              compact ||
                              item
                                  .label(context.t)
                                  .toLowerCase()
                                  .contains(_query) ||
                              item
                                  .description(context.t)
                                  .toLowerCase()
                                  .contains(_query),
                        )
                        .toList(),
                  ),
                ),
              ),
              // 账户、外观与退出属于同一组应用级动作，收进一个标准底部菜单。
              _AccountMenu(progress: progress),
            ],
          ),
        );
      },
    );
  }
}

/// 随窗格收起而淡出、并滑出边界的内容。
///
/// 三层各司其职，缺一不可：
///
/// - `OverflowBox` 让子项按**固有宽度**布局。窄窗格里文字比可用宽度还宽，
///   若直接放进 `Row` 会报 `RenderFlex overflow`。
/// - `ClipRect` 把它裁在窗格内：于是「消失」是滑出去，而不是瞬间不见。
/// - `Opacity` 负责淡出。
///
/// 调用点必须给它**确定宽度**（`Expanded`、或占满宽度的 `Align`）：
/// 无界宽度下 `OverflowBox` 没有可对齐的参照。
class _Collapsed extends StatelessWidget {
  const _Collapsed({required this.factor, required this.child});

  /// 窗格展开程度（0 = 图标轨，1 = 完整）。
  final double factor;

  final Widget child;

  @override
  Widget build(BuildContext context) {
    final t = factor.clamp(0.0, 1.0);

    return ClipRect(
      child: Opacity(
        opacity: t,
        child: OverflowBox(
          alignment: Alignment.centerLeft,
          maxWidth: double.infinity,
          maxHeight: double.infinity,
          child: child,
        ),
      ),
    );
  }
}

/// 侧边栏里所有会随收起/展开变形的行，共用的几何。
///
/// ## 为什么是两层 `Stack`，而不是一个算好宽度的 `Row`
///
/// 「收起时图标居中、展开时图标贴左」是一个**定位**问题，不是一个**尺寸**问题。
/// 用 `Row` + 手算的前导留白去做，就是拿尺寸去模拟位置：留白、图标、间距三项之和
/// 总会比可用宽度差那么零点几像素（可用宽度是弹簧驱动的小数），于是每帧都报
/// `RenderFlex overflow`，然后再去调那个魔数——这是个死循环。
///
/// 交给 [Align] 就不会有这个问题：它是从**实际盒子尺寸**反推偏移的，
/// 偏移量与被定位物体的尺寸无关，因此无论窗格多宽都不会越界。
///
/// 两层各管一件事：
///
/// - **图标层**：`Align` 在「居中」与「贴左」两个对齐之间插值，偏移由框架算出。
/// - **文字层**：左端缩进是**常量**，不随动画变——它本来就只在原地淡出，不该横移。
///   靠 [ClipRect] 在行的右端被裁掉，于是收起时看到的是文字滑出窗格。
class _SidebarRow extends StatelessWidget {
  const _SidebarRow({
    required this.progress,
    required this.icon,
    required this.iconColor,
    required this.label,
    required this.labelStyle,
    this.iconBackground,
  });

  /// 窗格展开程度（0 = 图标轨，1 = 完整）。
  final double progress;

  final IconData icon;
  final Color iconColor;
  final Color? iconBackground;
  final String label;
  final TextStyle labelStyle;

  /// 文字相对行左端的缩进：一个图标列加一档间距。
  ///
  /// 常量，且与展开程度无关：图标在两个位置之间移动，文字的**位置**始终不变。
  static const double _labelInset =
      ConsoleMetrics.sidebarIconColumn + ConsoleSpace.s;

  @override
  Widget build(BuildContext context) {
    final t = progress.clamp(0.0, 1.0);

    return Stack(
      children: [
        Padding(
          padding: const EdgeInsetsDirectional.only(start: _labelInset),
          child: _Collapsed(
            factor: t,
            child: Text(label, maxLines: 1, softWrap: false, style: labelStyle),
          ),
        ),
        // 图标槽固定宽度，整列图标共用一条竖向基准线；
        // 它在行内的位置完全交给 `Align`，这里不写任何数字。
        Align(
          alignment: Alignment.lerp(Alignment.center, Alignment.centerLeft, t)!,
          child: SizedBox(
            width: ConsoleMetrics.sidebarIconColumn,
            child: iconBackground == null
                ? Icon(icon, size: 24, color: iconColor)
                : Container(
                    width: 28,
                    height: 28,
                    decoration: ShapeDecoration(
                      color: iconBackground,
                      shape: LiquidRoundedSuperellipse(
                        borderRadius: ConsoleRadius.iconTile,
                      ),
                    ),
                    child: Icon(icon, size: 18, color: CupertinoColors.white),
                  ),
          ),
        ),
      ],
    );
  }
}

/// 侧边栏收起/展开按钮。
///
/// 用一个不抢镜的普通控件而不是玻璃按钮：它就在窗格底部与图标列同一列上，
/// 玻璃质感在这么大的面积上只会变成一块发光的方块。
class _SidebarToggle extends StatefulWidget {
  const _SidebarToggle({required this.collapsed, required this.onTap});

  /// 控件本身与它所在行的高度。表头高度取它，两种形态下才不会跳。
  static const double extent = 28;

  /// 当前是否处于收起态（图标轨）。
  final bool collapsed;

  final VoidCallback onTap;

  @override
  State<_SidebarToggle> createState() => _SidebarToggleState();
}

class _SidebarToggleState extends State<_SidebarToggle> {
  bool _hovering = false;

  @override
  Widget build(BuildContext context) {
    final palette = ConsolePalette.of(context);

    return Semantics(
      label: widget.collapsed
          ? context.t.navigation.expandSidebar
          : context.t.navigation.collapseSidebar,
      button: true,
      child: MouseRegion(
        onEnter: (_) => setState(() => _hovering = true),
        onExit: (_) => setState(() => _hovering = false),
        cursor: SystemMouseCursors.click,
        child: GestureDetector(
          onTap: widget.onTap,
          behavior: HitTestBehavior.opaque,
          child: DecoratedBox(
            decoration: ShapeDecoration(
              color: _hovering ? palette.fill : null,
              shape: LiquidRoundedSuperellipse(
                borderRadius: ConsoleRadius.control,
              ),
            ),
            child: SizedBox.square(
              dimension: _SidebarToggle.extent,
              child: Icon(
                CupertinoIcons.sidebar_left,
                size: 17,
                color: palette.secondaryLabel,
              ),
            ),
          ),
        ),
      ),
    );
  }
}

/// 导航列表。
///
/// 选中底片是一个**弹簧跟随**的独立图层，而不是每行各自变色的静态背景：位置由选中项序号
/// 乘固定行高算出，因此不需要测量，也不会有「先跳到新位置再淡出」的两段感。
class _NavList extends StatelessWidget {
  const _NavList({
    required this.progress,
    required this.selected,
    required this.onSelect,
    required this.sections,
  });

  /// 窗格展开程度（0 = 图标轨，1 = 完整）。
  final double progress;

  final ConsoleSection selected;
  final ValueChanged<ConsoleSection> onSelect;
  final List<ConsoleSection> sections;

  /// 行距。固定值，底片位置才能纯靠算数得出。
  static const double extent = 44;
  static const double pillHeight = 40;

  @override
  Widget build(BuildContext context) {
    final palette = ConsolePalette.of(context);
    final index = sections.indexOf(selected).toDouble();

    return SizedBox(
      height: extent * sections.length,
      child: Stack(
        children: [
          if (index >= 0)
            SpringValue(
              value: index,
              builder: (context, value) => Positioned(
                top: value * extent + (extent - pillHeight) / 2,
                left: 0,
                right: 0,
                height: pillHeight,
                child: DecoratedBox(
                  // 选中底片用强调色的淡色版，不是中性灰：iOS 侧边栏里「你在哪」这个信息
                  // 由颜色表达。中性灰的底片与悬停高亮长得一样，两者得靠猜。
                  decoration: ShapeDecoration(
                    color: palette.secondaryFill,
                    shape: LiquidRoundedSuperellipse(
                      borderRadius: ConsoleRadius.control,
                    ),
                  ),
                ),
              ),
            ),
          Column(
            children: [
              for (final section in sections)
                SizedBox(
                  height: extent,
                  child: _NavItem(
                    section: section,
                    progress: progress,
                    selected: section == selected,
                    onTap: () => onSelect(section),
                  ),
                ),
            ],
          ),
        ],
      ),
    );
  }
}

class _NavItem extends StatefulWidget {
  const _NavItem({
    required this.section,
    required this.progress,
    required this.selected,
    required this.onTap,
  });

  final ConsoleSection section;
  final double progress;
  final bool selected;
  final VoidCallback onTap;

  @override
  State<_NavItem> createState() => _NavItemState();
}

class _NavItemState extends State<_NavItem> {
  bool _hovering = false;

  @override
  Widget build(BuildContext context) {
    final palette = ConsolePalette.of(context);
    final selected = widget.selected;
    // 选中的图标用强调色、文字用主文字色：iOS 侧边栏的取法——颜色只标一处。
    // 选中行：图标与文字都用强调色，文字加粗。颜色只标一处——「你在哪」——而这一处就是整行。
    final tint = palette.tint;
    final iconColor = selected ? tint : palette.secondaryLabel;
    final labelColor = selected
        ? tint
        : _hovering
        ? palette.label
        : palette.secondaryLabel;

    return Semantics(
      label: widget.section.label(context.t),
      button: true,
      selected: selected,
      child: MouseRegion(
        onEnter: (_) => setState(() => _hovering = true),
        onExit: (_) => setState(() => _hovering = false),
        cursor: SystemMouseCursors.click,
        child: GestureDetector(
          onTap: widget.onTap,
          behavior: HitTestBehavior.opaque,
          // 行的几何由 `_SidebarRow` 统一给出（与底部操作共用同一套）。
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 8),
            child: _SidebarRow(
              progress: widget.progress,
              icon: widget.section.icon,
              iconColor: iconColor,
              iconBackground: widget.section.iconBackground.resolveFrom(
                context,
              ),
              label: widget.section.label(context.t),
              labelStyle: ConsoleText.subheadline.copyWith(
                fontWeight: selected ? FontWeight.w600 : FontWeight.w400,
                color: labelColor,
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _Wordmark extends StatelessWidget {
  const _Wordmark();

  @override
  Widget build(BuildContext context) {
    final palette = ConsolePalette.of(context);

    // 标识占侧边栏统一的图标槽，与导航图标共用同一条竖向基准线；
    // 槽宽固定，收起按钮（也是 28）才能与它对齐。
    // 亮暗两版走 `BrandMark`：它是不透明方形图标，不做 tint——着色会把品牌色抹掉。
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        SizedBox(
          width: ConsoleMetrics.sidebarIconColumn,
          child: Center(
            child: Image.asset(
              BrandMark.of(palette.brightness),
              width: 22,
              height: 22,
              // 64px 源图缩到 22pt，用高质量缩放；默认 low 在圆角边缘会看出来。
              filterQuality: FilterQuality.medium,
            ),
          ),
        ),
        const SizedBox(width: ConsoleSpace.s),
        Text(
          'Privacy Router',
          maxLines: 1,
          softWrap: false,
          style: ConsoleText.headline.copyWith(
            fontSize: 16,
            color: palette.label,
          ),
        ),
      ],
    );
  }
}

/// 侧边栏底部的账户菜单：账户身份、外观与退出只占一个常驻入口。
class _AccountMenu extends ConsumerStatefulWidget {
  const _AccountMenu({required this.progress});

  /// 窗格展开程度（0 = 图标轨，1 = 完整）。
  final double progress;

  @override
  ConsumerState<_AccountMenu> createState() => _AccountMenuState();
}

class _AccountMenuState extends ConsumerState<_AccountMenu> {
  bool _hovering = false;

  @override
  Widget build(BuildContext context) {
    final appearance = ref.watch(appearanceProvider);
    final session = ref.watch(sessionProvider);
    final palette = ConsolePalette.of(context);
    final username = switch (session) {
      LoggedIn(:final session) => session.username,
      _ => context.t.common.account,
    };

    return GlassMenu(
      // 触发器由我们自己画，不用库的 `GlassPullDownButton`。
      //
      // 原因是一个真实的溢出：库的触发器把 label 交给不受约束的 `Text` 再放进一个
      // `Row`，当容器宽度比文字窄时（收起动画中段就是这样）它每帧都报
      // `RenderFlex overflow`。而它的 label 是 `String`，没法让文字跟着宽度变性。
      //
      // 换成自定义触发器后，行结构与导航行**完全同一套**（见 [_SidebarRow]）：
      // 图标位置交给 `Align`，文字恒定缩进、超宽就裁。库里那一条路径不再被走到。
      triggerBuilder: (context, toggle) => Semantics(
        label: context.t.common.accountMenu(username: username),
        button: true,
        child: MouseRegion(
          onEnter: (_) => setState(() => _hovering = true),
          onExit: (_) => setState(() => _hovering = false),
          cursor: SystemMouseCursors.click,
          child: GestureDetector(
            onTap: toggle,
            behavior: HitTestBehavior.opaque,
            child: DecoratedBox(
              decoration: ShapeDecoration(
                color: _hovering ? palette.fill : palette.surface,
                shape: LiquidRoundedSuperellipse(
                  borderRadius: ConsoleRadius.control,
                ),
              ),
              child: Padding(
                padding: const EdgeInsets.all(8),
                child: SizedBox(
                  height: 44,
                  child: _SidebarRow(
                    progress: widget.progress,
                    icon: CupertinoIcons.person_crop_circle,
                    iconColor: palette.secondaryLabel,
                    label: username,
                    labelStyle: ConsoleText.subheadline.copyWith(
                      fontWeight: FontWeight.w500,
                      color: palette.secondaryLabel,
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
      items: [
        GlassMenuLabel(
          child: Text(
            username,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: ConsoleText.footnote.copyWith(
              fontWeight: FontWeight.w600,
              color: palette.secondaryLabel,
            ),
          ),
        ),
        for (final option in ConsoleAppearance.values)
          GlassMenuItem(
            title: option.localized(context.t),
            icon: Icon(option.icon, size: 18),
            isSelected: option == appearance,
            onTap: () => ref.read(appearanceProvider.notifier).select(option),
          ),
        const GlassMenuDivider(),
        GlassMenuItem(
          title: context.t.common.signOut,
          icon: const Icon(CupertinoIcons.square_arrow_right, size: 18),
          isDestructive: true,
          onTap: () => ref.read(sessionProvider.notifier).logout(),
        ),
        const GlassMenuDivider(),
        ...LanguageMenu.items(context),
      ],
    );
  }
}

/// 顶栏上的外观入口（仅手机端）：纯图标按钮。
///
/// 与侧边栏账户菜单里的外观选项是同一件事的两种形态。分成两个控件而不是加一个
/// `compact` 开关，是因为它们的**结构不同**：这里是图标按钮，那里是整行控件。
class _CompactAppearanceMenu extends ConsumerWidget {
  const _CompactAppearanceMenu();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final appearance = ref.watch(appearanceProvider);

    return GlassPullDownButton(
      icon: Icon(appearance.icon),
      semanticLabel: context.t.common.appearance(
        value: appearance.localized(context.t),
      ),
      buttonWidth: ConsoleMetrics.barControl,
      buttonHeight: ConsoleMetrics.barControl,
      buttonShape: const LiquidRoundedRectangle(
        borderRadius: ConsoleRadius.control,
      ),
      items: [
        for (final option in ConsoleAppearance.values)
          GlassMenuItem(
            title: option.localized(context.t),
            icon: Icon(option.icon, size: 18),
            isSelected: option == appearance,
            onTap: () => ref.read(appearanceProvider.notifier).select(option),
          ),
      ],
    );
  }
}
