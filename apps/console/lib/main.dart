/// 控制台入口。
///
/// 顶层只用 `cupertino_ui` 的 Cupertino 组件；glass 组件库的主题由 `ConsoleTheme` 装配，
/// 并通过一层桥接兼容它内部的 legacy Cupertino 引用。
///
/// 亮度在这里合流：**外观设置 + 系统亮度 → 一个亮度**，然后同时喂给 `CupertinoApp.theme`
/// 与 legacy 桥接。两者必须一致，否则会出现「语义色是浅色、玻璃是深色」的错配。
library;

import 'package:cupertino_ui/cupertino_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_localizations/flutter_localizations.dart'
    as flutter_l10n;
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';

import 'i18n/strings.g.dart';
import 'i18n/language.dart';
import 'state/providers.dart';
import 'demo/demo_banner.dart';
import 'theme/palette.dart';
import 'theme/console_theme.dart';
import 'theme/tokens.dart';
import 'ui/pages/login_page.dart';
import 'ui/pages/setup_page.dart';
import 'ui/shell.dart';

/// 全应用统一的滚动物理。
///
/// 默认的 `ClampingScrollPhysics` 是 Android 手感：滚到底就硬梓梓地停住。桌面浏览器
/// （鼠标滚轮）下这个手感尤其错——每次滑到底都像被膜截了一下。换成
/// `BouncingScrollPhysics`：越界时阻尼递减地再走一点再回弹，就是 macOS／iOS 的惯性阻尼。
///
/// 桌面鼠标滚轮与触控板都走这里，因为 `CupertinoApp` 没有全局的 scrollBehavior 入口，
/// 只能靠这层包裹。
class ConsoleScrollBehavior extends ScrollBehavior {
  const ConsoleScrollBehavior();

  @override
  ScrollPhysics getScrollPhysics(BuildContext context) =>
      const BouncingScrollPhysics(parent: AlwaysScrollableScrollPhysics());

  @override
  Widget buildOverscrollIndicator(
    BuildContext context,
    Widget child,
    ScrollableDetails details,
  ) => child;
}

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  // 预热着色器，避免首帧出现白色闪烁。
  await LiquidGlassWidgets.initialize();
  await ConsoleLanguage.restore();

  runApp(
    TranslationProvider(
      child: ProviderScope(
        child: LiquidGlassWidgets.wrap(
          theme: ConsoleGlass.theme(),
          child: const ConsoleApp(),
        ),
      ),
    ),
  );
}

class ConsoleApp extends ConsumerWidget {
  const ConsoleApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final session = ref.watch(sessionProvider);
    final demo = ref.watch(demoModeProvider);
    final appearance = ref.watch(appearanceProvider);
    final translations = TranslationProvider.of(context);

    // 外观设置优先，其次才是系统亮度：`system` 会跟着系统实时切换，因为这里依赖了
    // MediaQuery，系统亮度变化时整棵树会重建。
    final brightness = appearance.resolve(
      MediaQuery.maybePlatformBrightnessOf(context) ?? Brightness.light,
    );
    // 会话变化时切换根页面，从而让登录与主界面各自持有独立的导航栈。
    final health = ref.watch(healthProvider);
    final home = switch (session) {
      LoggedIn() => const ConsoleShell(),
      RestoringSession() => const _Launching(),
      // 首次安装：还没有管理员就自助创建，不需要命令行。
      _ when health.value?.adminConfigured == false => const SetupPage(),
      // 首次健康检查还没回来之前不表态：否则两个表单会闪一下再切换。已经失败过就不再
      // 停留在空白页——那时用户需要看到「连不上」而不是无限等待。
      _ when health.isLoading && !health.hasValue && !health.hasError =>
        const _Launching(),
      _ => const LoginPage(),
    };

    return CupertinoApp(
      title: context.t.app.title,
      locale: translations.flutterLocale,
      supportedLocales: AppLocaleUtils.supportedLocales,
      localizationsDelegates:
          flutter_l10n.GlobalCupertinoLocalizations.delegates,
      debugShowCheckedModeBanner: false,
      scrollBehavior: const ConsoleScrollBehavior(),
      // 亮度不在构造时定死：外观设置与系统亮度在上面的 builder 里合流。
      theme: ConsoleTheme.cupertino(brightness),
      builder: (context, child) => LegacyCupertinoBridge(
        brightness: brightness,
        child: demo
            ? Column(
                children: [
                  const DemoBanner(),
                  Expanded(
                    child: GlassNavigationShell(
                      child: child ?? const SizedBox.shrink(),
                    ),
                  ),
                ],
              )
            : GlassNavigationShell(child: child ?? const SizedBox.shrink()),
      ),
      home: home,
    );
  }
}

/// 启动时的空屏：健康检查结果回来之前，不抢先展示任何一个表单。
class _Launching extends StatelessWidget {
  const _Launching();

  @override
  Widget build(BuildContext context) {
    return GlassScaffold(
      background: const ConsoleCanvas(),
      backgroundColor: ConsolePalette.of(context).canvas,
      body: const Center(
        child: GlassProgressIndicator.circular(size: 26, strokeWidth: 3),
      ),
    );
  }
}
