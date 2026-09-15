/// 外壳的渲染契约：三种断点 × 两种亮度都必须能构建，且不抛任何异常。
///
/// 这不是「渲染出什么颜色」的测试——配色由语义色与组件库负责，断言色值只会变成同义反复。
/// 这里锁的是**我们自己的布局决策**在极端条件下是否成立：宽屏侧边栏 + 检查器的行约束、
/// 弹簧容器里的 `Stack`、大标题 sliver 的滚动控制器，任何一处拿到无界约束或重复挂载
/// 滚动控制器，都会在真实使用中表现为整页报错。黑盒断言「不抛错」即可覆盖它们。
library;

import 'dart:convert';

import 'package:cupertino_ui/cupertino_ui.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';
import 'package:privacy_router_console/api/api_client.dart';
import 'package:privacy_router_console/i18n/strings.g.dart';
import 'package:privacy_router_console/state/providers.dart';
import 'package:privacy_router_console/theme/console_theme.dart';
import 'package:privacy_router_console/theme/palette.dart';
import 'package:privacy_router_console/ui/shell.dart';
import 'package:privacy_router_console/ui/console_toast.dart';
import 'package:privacy_router_console/ui/sections.dart';
import 'package:privacy_router_console/ui/pages/rules_page.dart';

/// 全部接口都返回结构完整的空数据：外壳在每个断点都要能渲染，而不是靠错误态蒙混过关。
http.Client _client() => MockClient((request) async {
  final path = request.url.path;
  final body = switch (path) {
    '/api/health' => {
      'status': 'ok',
      'rules': 13,
      'providers': 1,
      'admin_configured': true,
    },
    '/api/stats' => {
      'requests': 4,
      'fragments': 3,
      'spans': 3,
      'released_spans': 1,
      'redacted_spans': 2,
      'cached_fragments': 1,
      'inferred_fragments': 2,
      'latency': {
        'average_ms': 120,
        'p50_ms': 100,
        'p95_ms': 200,
        'max_ms': 210,
        'average_inference_ms': 20,
        'average_upstream_ms': 90,
      },
      'by_entity_group': const [
        {'entity_group': 'private_email', 'count': 2},
        {'entity_group': 'secret', 'count': 1},
      ],
    },
    '/api/content' => {
      'items': [
        {
          'anchor_span_id': 's1',
          'original_text': 'mail me at <redacted_private_email>',
          'occurrences': 2,
          'turns': 2,
          'released': 1,
          'redacted': 1,
          'score_min': 0.62,
          'score_max': 0.98,
          'first_seen': 1700000000000,
          'last_seen': 1700000006000,
          'latest_action': 'redact',
          'categories': [
            {'entity_group': 'private_email', 'count': 2},
          ],
          'registered_action': null,
        },
      ],
      'total': 1,
    },
    '/api/rules' => {
      'items': [
        {
          'id': 'builtin-secret',
          'name': '密钥一律抹去',
          'priority': 1000,
          'condition': {
            'kind': 'filter',
            'filter': {'kind': 'entity', 'group': 'secret'},
          },
          'action': 'redact',
          'enabled': true,
          'source': 'builtin',
        },
      ],
      'fallback_action': 'redact',
    },
    '/api/providers' => {
      'items': [
        {
          'id': 'p1',
          'name': 'openai',
          'base_url': 'https://api.openai.com',
          'api_format': 'openai_chat',
          'enabled': true,
        },
      ],
    },
    _ => const <String, dynamic>{},
  };
  return http.Response(
    jsonEncode(body),
    200,
    headers: {'content-type': 'application/json; charset=utf-8'},
  );
});

/// 已登录的会话：内容页需要令牌才会发请求。
class _LoggedIn extends SessionNotifier {
  @override
  SessionState build() =>
      const LoggedIn(Session(token: 'test-token', username: 'admin', expiresAt: 4102444800000));
}

Widget _app({
  required Size size,
  required Brightness brightness,
  http.Client? client,
}) => TranslationProvider(
  child: ProviderScope(
    overrides: [
      sessionProvider.overrideWith(_LoggedIn.new),
      apiClientProvider.overrideWithValue(
        ApiClient(baseUrl: 'http://test.invalid', client: client ?? _client()),
      ),
      appearanceProvider.overrideWith(() => _PinnedAppearance(brightness)),
    ],
    child: CupertinoApp(
      theme: ConsoleTheme.cupertino(brightness),
      builder: (context, child) =>
          LegacyCupertinoBridge(brightness: brightness, child: child!),
      home: const ConsoleShell(),
    ),
  ),
);

class _PinnedAppearance extends AppearanceNotifier {
  _PinnedAppearance(this._brightness);

  final Brightness _brightness;

  @override
  ConsoleAppearance build() => _brightness == Brightness.dark
      ? ConsoleAppearance.dark
      : ConsoleAppearance.light;
}

/// 收集 `body` 期间抛出的全部异常，并把它们从待处理队列里清掉。
Future<List<String>> _capture(
  WidgetTester tester,
  Future<void> Function() body,
) async {
  final messages = <String>[];
  final previous = FlutterError.onError;
  FlutterError.onError = (details) => messages.add(details.exceptionAsString());
  try {
    await body();
  } finally {
    FlutterError.onError = previous;
  }
  while (tester.takeException() != null) {}
  return messages;
}

Future<void> _pump(
  WidgetTester tester, {
  required Size size,
  required Brightness brightness,
  http.Client? client,
}) async {
  tester.view.physicalSize = size;
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    _app(size: size, brightness: brightness, client: client),
  );
  // 让 provider 的 Future 落地，再让弹簧与分段控件完成一次布局。
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 400));
}

void main() {
  setUpAll(LiquidGlassWidgets.initialize);

  testWidgets('统计自动更新、暂时失败保留数值、离开页面停止订阅', (tester) async {
    final baseline = _client();
    var statsRequests = 0;
    final client = MockClient((request) async {
      if (request.url.path == '/api/stats') {
        statsRequests++;
        if (statsRequests == 2) {
          return http.Response('temporarily unavailable', 503);
        }
        final response = await baseline.get(
          request.url,
          headers: request.headers,
        );
        final body = jsonDecode(response.body) as Map<String, dynamic>;
        (body['latency'] as Map<String, dynamic>)['average_ms'] =
            statsRequests * 1000;
        return http.Response(jsonEncode(body), 200, headers: response.headers);
      }
      return baseline.get(request.url, headers: request.headers);
    });
    await _pump(
      tester,
      size: const Size(1440, 900),
      brightness: Brightness.dark,
      client: client,
    );
    final container = ProviderScope.containerOf(
      tester.element(find.byType(ConsoleShell)),
    );
    container
        .read(selectedSectionProvider.notifier)
        .select(ConsoleSection.performance);
    await tester.pumpAndSettle();
    expect(find.bySemanticsLabel('1000 ms'), findsOneWidget);
    await tester.pump(const Duration(seconds: 2));
    await tester.pump();
    expect(
      find.bySemanticsLabel('1000 ms'),
      findsOneWidget,
      reason: '暂时失败不清空已有数据',
    );
    await tester.pump(const Duration(seconds: 2));
    await tester.pumpAndSettle();
    expect(find.bySemanticsLabel('3000 ms'), findsOneWidget);
    container
        .read(selectedSectionProvider.notifier)
        .select(ConsoleSection.rules);
    await tester.pumpAndSettle();
    final requestsAfterLeaving = statsRequests;
    await tester.pump(const Duration(seconds: 6));
    expect(statsRequests, requestsAfterLeaving);
    expect(tester.takeException(), isNull);
  });

  for (final width in [390.0, 1440.0]) {
    testWidgets('提示保持紧凑居中、替换后自动关闭 $width', (tester) async {
      await _pump(tester, size: Size(width, 900), brightness: Brightness.light);
      final context = tester.element(find.byType(CustomScrollView).first);
      ConsoleToast.show(
        context,
        message: 'Released and registered as a release rule',
      );
      await tester.pump();
      final bounds = tester.getRect(find.byType(GlassToast));
      expect(bounds.width, lessThanOrEqualTo(400));
      expect(bounds.left, greaterThanOrEqualTo(16));
      expect(bounds.center.dx, closeTo(width / 2, 1));
      expect(bounds.top, greaterThanOrEqualTo(56));
      ConsoleToast.show(context, message: 'Updated');
      await tester.pump();
      expect(find.byType(GlassToast), findsOneWidget);
      expect(find.text('Updated'), findsOneWidget);
      await tester.pump(const Duration(seconds: 5));
      expect(find.byType(GlassToast), findsNothing);
      expect(tester.takeException(), isNull);
    });
  }

  for (final size in [const Size(820, 1180), const Size(1440, 900)]) {
    testWidgets('规则整行进入详情，列表编辑模式选择而不跳转 $size', (tester) async {
      await _pump(tester, size: size, brightness: Brightness.light);
      final container = ProviderScope.containerOf(
        tester.element(find.byType(ConsoleShell)),
      );
      container
          .read(selectedSectionProvider.notifier)
          .select(ConsoleSection.rules);
      await tester.pumpAndSettle();
      expect(find.byType(GlassSwitch), findsNothing);
      await tester.tap(
        find
            .text(LocaleSettings.currentLocale.translations.rules.editList)
            .hitTestable()
            .last,
      );
      await tester.pumpAndSettle();
      await tester.tap(find.text('密钥一律抹去'));
      await tester.pumpAndSettle();
      expect(container.read(ruleSelectionProvider), contains('builtin-secret'));
      await tester.tap(
        find
            .bySemanticsLabel(
              LocaleSettings.currentLocale.translations.common.cancel,
            )
            .last,
      );
      await tester.pumpAndSettle();
      expect(find.byType(RulesPage), findsOneWidget);
      final nameField = find.byWidgetPredicate(
        (widget) =>
            widget is GlassTextField &&
            widget.placeholder ==
                LocaleSettings.currentLocale.translations.rules.name,
      );
      expect(nameField, findsNothing);
      await tester.tap(find.text('密钥一律抹去'));
      await tester.pumpAndSettle();
      expect(nameField, findsOneWidget);
      expect(find.byType(GlassSearchBar), findsOneWidget, reason: '编辑时侧栏仍保留');
      await tester.tap(find.bySemanticsLabel('Back').last);
      await tester.pumpAndSettle();
      expect(find.byType(RulesPage), findsOneWidget);
      expect(nameField, findsNothing);
      expect(tester.takeException(), isNull);
    });
  }

  testWidgets('类别下拉完整显示长类别名，菜单整体留在窗口内', (tester) async {
    await _pump(
      tester,
      size: const Size(1440, 900),
      brightness: Brightness.light,
    );
    final container = ProviderScope.containerOf(
      tester.element(find.byType(ConsoleShell)),
    );
    container
        .read(selectedSectionProvider.notifier)
        .select(ConsoleSection.rules);
    await tester.pumpAndSettle();

    final translations = LocaleSettings.currentLocale.translations;
    await tester.tap(find.text('密钥一律抹去'));
    await tester.pumpAndSettle();

    // 缩到装不下十三个类别的窗口高度：菜单必须自己收进来，否则下半个菜单既看不到也滚不到。
    const height = 600.0;
    tester.view.physicalSize = const Size(1440, height);
    await tester.pumpAndSettle();

    // 编辑器的类别下拉：当前值就是这条规则的类别。
    final entity = translations.model.entity;
    await tester.tap(find.text(entity.secret).hitTestable().last);
    await tester.pumpAndSettle();

    // 最长的类别名一个字都不能省：省掉之后「是哪个类别」就无从判断。
    final longest = find.text(entity.creditCode);
    expect(longest, findsOneWidget);
    expect(
      tester.renderObject<RenderParagraph>(longest).didExceedMaxLines,
      isFalse,
      reason: '长类别名被省略号截断了',
    );

    final surface = find
        .ancestor(of: longest, matching: find.byType(GlassContainer))
        .first;
    final menu = tester.getRect(surface);
    // 弹簧停在整数像素附近，留 1px 容差；没裁到屏幕时菜单会高出窗口二十多像素。
    expect(menu.top, greaterThanOrEqualTo(-1));
    expect(menu.bottom, lessThanOrEqualTo(height + 1));
    expect(
      tester
          .state<ScrollableState>(
            find.descendant(of: surface, matching: find.byType(Scrollable)).first,
          )
          .position
          .maxScrollExtent,
      greaterThan(0),
      reason: '这一屏装不下全部类别，剩下的必须能滚出来',
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('已打开的详情路由背景跟随深浅色切换', (tester) async {
    await _pump(
      tester,
      size: const Size(1440, 900),
      brightness: Brightness.dark,
    );
    Color detailBackground() => tester
        .widget<ColoredBox>(
          find
              .ancestor(
                of: find.byType(CustomScrollView).first,
                matching: find.byType(ColoredBox),
              )
              .first,
        )
        .color;
    final dark = detailBackground();
    await tester.pumpWidget(
      _app(size: const Size(1440, 900), brightness: Brightness.light),
    );
    await tester.pumpAndSettle();
    expect(detailBackground(), isNot(dark));
    expect(tester.takeException(), isNull);
  });

  testWidgets('上游新增和编辑使用右栏详情页并保留侧栏', (tester) async {
    await _pump(
      tester,
      size: const Size(1440, 900),
      brightness: Brightness.light,
    );
    final container = ProviderScope.containerOf(
      tester.element(find.byType(ConsoleShell)),
    );
    container
        .read(selectedSectionProvider.notifier)
        .select(ConsoleSection.providers);
    await tester.pumpAndSettle();
    final text = LocaleSettings.currentLocale.translations;
    final nameField = find.byWidgetPredicate(
      (widget) =>
          widget is GlassTextField && widget.placeholder == text.providers.name,
    );
    await tester.tap(find.bySemanticsLabel(text.providers.add).last);
    await tester.pumpAndSettle();
    expect(tester.widget<GlassTextField>(nameField).controller!.text, isEmpty);
    expect(find.byType(GlassSearchBar), findsOneWidget);
    await tester.tap(find.bySemanticsLabel('Back').last);
    await tester.pumpAndSettle();
    expect(nameField, findsNothing);
    await tester.tap(find.text('openai'));
    await tester.pumpAndSettle();
    expect(tester.widget<GlassTextField>(nameField).controller!.text, 'openai');
    expect(find.byType(GlassSearchBar), findsOneWidget);
    expect(find.bySemanticsLabel(text.common.save), findsWidgets);
    await tester.tap(find.bySemanticsLabel('Back').last);
    await tester.pumpAndSettle();
    expect(find.text('openai'), findsOneWidget);
    expect(nameField, findsNothing);
    expect(tester.takeException(), isNull);
  });

  // 三种断点各是一条真实的代码路径：底部胶囊、图标轨、侧边栏 + 检查器。
  const breakpoints = <String, Size>{
    '手机': Size(390, 844),
    '平板': Size(820, 1180),
    '桌面': Size(1440, 900),
  };

  for (final entry in breakpoints.entries) {
    for (final brightness in Brightness.values) {
      final name = brightness == Brightness.dark ? '深色' : '浅色';
      testWidgets('${entry.key}布局在$name下渲染且不抛错', (tester) async {
        final errors = await _capture(
          tester,
          () => _pump(tester, size: entry.value, brightness: brightness),
        );
        expect(errors, isEmpty, reason: '实际抛出：$errors');
      });
    }
  }

  testWidgets('浅色与深色取到不同的底板，语义色确实跟随亮度', (tester) async {
    Future<Color> canvas(Brightness brightness) async {
      await _pump(tester, size: const Size(1440, 900), brightness: brightness);
      final context = tester.element(find.byType(ConsoleShell));
      return ConsolePalette.of(context).canvas;
    }

    final light = await canvas(Brightness.light);
    final dark = await canvas(Brightness.dark);

    expect(light, isNot(dark), reason: '深浅两个亮度必须落到底色阶梯的不同一级');
  });
}
