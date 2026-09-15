/// 内容池的实时契约。
///
/// 与统计页同一套机制：停在分区上会自己拉取，离开分区即停止订阅，切回来时重新拉一次。
/// 自动刷新带来的**新内容**必须带着入场动画进入列表——只把版面重排一遍，用户看到的就是
/// 内容突然多了一行，分不清是新到的还是自己看漏了。
///
/// 反过来，已经在那里的内容又多出现了一次时，行本身不该重播入场：列表的身份是那段内容，
/// 不是这一次出现。键取原文就是为了让这条区分成立。
library;

import 'dart:convert';

import 'package:cupertino_ui/cupertino_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';
import 'package:privacy_router_console/api/api_client.dart';
import 'package:privacy_router_console/i18n/strings.g.dart';
import 'package:privacy_router_console/state/providers.dart';
import 'package:privacy_router_console/theme/console_theme.dart';
import 'package:privacy_router_console/ui/sections.dart';
import 'package:privacy_router_console/ui/shell.dart';

/// 内容可以随时增删的假服务端，同时记录内容池被拉取了几次。
class _PoolServer {
  _PoolServer() {
    rows.add(row('first fragment', occurrences: 1, released: 0, redacted: 1));
  }

  final List<Map<String, dynamic>> rows = [];

  int contentRequests = 0;

  /// 内容池的一行：同一段内容在池子里的判定统计。
  static Map<String, dynamic> row(
    String text, {
    required int occurrences,
    required int released,
    required int redacted,
  }) => {
    'anchor_span_id': 'span-$text',
    'original_text': text,
    'occurrences': occurrences,
    'turns': occurrences,
    'released': released,
    'redacted': redacted,
    'score_min': 0.71,
    'score_max': 0.94,
    'first_seen': 1700000000000,
    'last_seen': 1700000006000,
    'latest_action': released > 0 ? 'release' : 'redact',
    'categories': const [
      {'entity_group': 'private_email', 'count': 1},
    ],
    'registered_action': null,
  };

  http.Client client() => MockClient((request) async {
    final path = request.url.path;
    if (path == '/api/content') contentRequests++;
    final body = switch (path) {
      '/api/health' => {
        'status': 'ok',
        'rules': 7,
        'providers': 1,
        'admin_configured': true,
      },
      '/api/stats' => {
        'requests': 0,
        'fragments': 0,
        'spans': 0,
        'released_spans': 0,
        'redacted_spans': 0,
        'cached_fragments': 0,
        'inferred_fragments': 0,
        'latency': const <String, dynamic>{},
        'by_entity_group': const <dynamic>[],
      },
      '/api/content' => {'items': [...rows], 'total': rows.length},
      '/api/rules' => {'items': const <dynamic>[], 'fallback_action': 'redact'},
      '/api/providers' => {'items': const <dynamic>[]},
      _ => const <String, dynamic>{},
    };
    return http.Response(
      jsonEncode(body),
      200,
      headers: const {'content-type': 'application/json; charset=utf-8'},
    );
  });
}

/// 已登录的会话：内容页需要令牌才会发请求。
class _LoggedIn extends SessionNotifier {
  @override
  SessionState build() =>
      const LoggedIn(Session(token: 'test-token', username: 'admin', expiresAt: 4102444800000));
}

class _PinnedAppearance extends AppearanceNotifier {
  @override
  ConsoleAppearance build() => ConsoleAppearance.light;
}

Future<void> _pump(WidgetTester tester, _PoolServer server) async {
  tester.view.physicalSize = const Size(1440, 900);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    TranslationProvider(
      child: ProviderScope(
        overrides: [
          sessionProvider.overrideWith(_LoggedIn.new),
          appearanceProvider.overrideWith(_PinnedAppearance.new),
          apiClientProvider.overrideWithValue(
            ApiClient(baseUrl: 'http://test.invalid', client: server.client()),
          ),
        ],
        child: CupertinoApp(
          theme: ConsoleTheme.cupertino(Brightness.light),
          builder: (context, child) => LegacyCupertinoBridge(
            brightness: Brightness.light,
            child: child!,
          ),
          home: const ConsoleShell(),
        ),
      ),
    ),
  );
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 400));
}

ProviderContainer _container(WidgetTester tester) =>
    ProviderScope.containerOf(tester.element(find.byType(ConsoleShell)));

/// 某个条目当前的入场进度：就是它那层淡入的透明度。键是那段内容本身。
double _entrance(WidgetTester tester, String text) => tester
    .widget<FadeTransition>(
      find
          .descendant(
            of: find.byKey(ValueKey(text)),
            matching: find.byType(FadeTransition),
          )
          .first,
    )
    .opacity
    .value;

/// 弹簧在容差内收敛即可视为入场结束；没播完的入场离 1 很远，这个下界足够把两者分开。
final _settled = greaterThan(0.99);

void main() {
  setUpAll(LiquidGlassWidgets.initialize);

  testWidgets('切回内容池分区会重新拉取，停在其他分区则停止订阅', (tester) async {
    final server = _PoolServer();
    await _pump(tester, server);
    final sections = _container(tester).read(selectedSectionProvider.notifier);

    sections.select(ConsoleSection.pool);
    await tester.pumpAndSettle();
    expect(server.contentRequests, greaterThan(0), reason: '进入分区即拉取');

    sections.select(ConsoleSection.rules);
    await tester.pumpAndSettle();
    final whileAway = server.contentRequests;
    await tester.pump(const Duration(seconds: 6));
    expect(server.contentRequests, whileAway, reason: '离开分区后停止订阅');

    sections.select(ConsoleSection.pool);
    await tester.pumpAndSettle();
    expect(
      server.contentRequests,
      greaterThan(whileAway),
      reason: '切回来立刻重新拉取，而不是继续显示离开那一刻的页',
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('停在内容池会自动刷新，新内容带入场动画而已有行不重播', (tester) async {
    final server = _PoolServer();
    await _pump(tester, server);
    _container(tester)
        .read(selectedSectionProvider.notifier)
        .select(ConsoleSection.pool);
    await tester.pumpAndSettle();

    expect(find.text('first fragment'), findsOneWidget);
    expect(_entrance(tester, 'first fragment'), _settled, reason: '首屏的入场已经收敛');

    // 既有内容又多出现了一次，同时冒出一段新内容。
    server.rows[0] = _PoolServer.row(
      'first fragment',
      occurrences: 2,
      released: 1,
      redacted: 1,
    );
    server.rows.insert(
      0,
      _PoolServer.row('second fragment', occurrences: 1, released: 0, redacted: 1),
    );
    await tester.pump(const Duration(seconds: 2));
    await tester.pump();
    await tester.pump();

    expect(find.text('second fragment'), findsOneWidget, reason: '新内容已进入列表');
    expect(_entrance(tester, 'second fragment'), lessThan(1), reason: '新内容正在入场');
    expect(
      _entrance(tester, 'first fragment'),
      _settled,
      reason: '同一段内容只是又出现一次，行沿用原来的元素，不重播入场',
    );

    await tester.pumpAndSettle();
    expect(_entrance(tester, 'second fragment'), _settled, reason: '入场是有限动画，会收敛');
    expect(tester.takeException(), isNull);
  });
}
