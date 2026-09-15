/// 内容池逐行登记的界面契约。
///
/// 复核的对象是一段内容，不是某一次请求，因此：
/// - 一行给出**两个方向**，已经登记过的那个方向不再是可选项；
/// - 登记发到这条内容自己的锚点（命中标识）上，被审查的文本不进 URL；
/// - 出现记录按同一个锚点拉取，列表里只给判定与来源。
library;

import 'dart:convert';

import 'package:cupertino_ui/cupertino_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';
import 'package:privacy_router_console/api/api_client.dart';
import 'package:privacy_router_console/api/models.dart';
import 'package:privacy_router_console/i18n/strings.g.dart';
import 'package:privacy_router_console/state/providers.dart';
import 'package:privacy_router_console/theme/console_theme.dart';
import 'package:privacy_router_console/ui/pages/content_detail_page.dart';
import 'package:privacy_router_console/ui/pages/pool_page.dart';
import 'package:privacy_router_console/ui/widgets.dart';

class _LoggedIn extends SessionNotifier {
  @override
  SessionState build() =>
      const LoggedIn(Session(token: 'test-token', username: 'admin', expiresAt: 4102444800000));
}

/// 一段内容的一行：既被放行过、也被抹去过，因此判定是摇摆的。
Map<String, dynamic> rowJson({String? registeredAction}) => {
  'anchor_span_id': 's-anchor',
  'original_text': 'ACME-secret',
  'occurrences': 3,
  'turns': 2,
  'released': 1,
  'redacted': 2,
  'score_min': 0.62,
  'score_max': 0.98,
  'first_seen': 1700000000000,
  'last_seen': 1700000006000,
  'latest_action': 'redact',
  'categories': [
    {'entity_group': 'private_email', 'count': 2},
    {'entity_group': 'private_person', 'count': 1},
  ],
  'registered_action': registeredAction,
};

/// 记录收到的请求，供「点哪个按钮发到哪个端点」这类断言使用。
http.Client _client(List<String> requests, {String? registeredAction}) =>
    MockClient((request) async {
      requests.add('${request.method} ${request.url.path}');
      final body = switch (request.url.path) {
        '/api/content' => {
          'items': [rowJson(registeredAction: registeredAction)],
          'total': 1,
        },
        '/api/spans/s-anchor/occurrences' => {
          'items': [
            {
              'span_id': 's-1',
              'entity_group': 'private_email',
              'score': 0.98,
              'action': 'redact',
              'matched_rule_id': 'builtin.redact.private_email',
              'api_format': 'openai_chat',
              'path': '/v1/chat/completions',
              'created_at': 1700000006000,
            },
            {
              'span_id': 's-2',
              'entity_group': 'private_email',
              'score': 0.62,
              'action': 'release',
              'matched_rule_id': null,
              'api_format': 'openai_chat',
              'path': '/v1/chat/completions',
              'created_at': 1700000000000,
            },
          ],
        },
        // 登记端点只回答「登记到哪条规则、这次有没有改动它」。
        _ when request.url.path.startsWith('/api/spans/') => {
          'rule_id': 'operator.1',
          'changed': true,
        },
        _ => const <String, dynamic>{},
      };
      return http.Response(
        jsonEncode(body),
        200,
        headers: const {'content-type': 'application/json; charset=utf-8'},
      );
    });

Widget _app(Widget child, http.Client client, {bool scrollable = true}) =>
    TranslationProvider(
      child: ProviderScope(
        overrides: [
          sessionProvider.overrideWith(_LoggedIn.new),
          apiClientProvider.overrideWithValue(
            ApiClient(baseUrl: 'http://test.invalid', client: client),
          ),
        ],
        child: CupertinoApp(
          theme: ConsoleTheme.cupertino(Brightness.light),
          builder: (context, inner) =>
              LegacyCupertinoBridge(brightness: Brightness.light, child: inner!),
          // 整页详情自带滚动与导航栏，不能再套一层不定高的滚动容器。
          home: scrollable
              ? CupertinoPageScaffold(child: SingleChildScrollView(child: child))
              : child,
        ),
      ),
    );

Future<void> _pump(
  WidgetTester tester,
  Widget child,
  http.Client client, {
  bool scrollable = true,
}) async {
  tester.view.physicalSize = const Size(1440, 900);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(_app(child, client, scrollable: scrollable));
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 400));
}

/// 这一行里的按钮。分页与刷新也是同样的控件，因此按那段内容本身把范围收窄。
Finder _rowButtons() => find.descendant(
  of: find.byKey(const ValueKey('ACME-secret')),
  matching: find.byType(ActionButton),
);

List<String> _buttonLabels(WidgetTester tester) => tester
    .widgetList<ActionButton>(_rowButtons())
    .map((button) => button.text)
    .toList();

void main() {
  setUpAll(LiquidGlassWidgets.initialize);

  testWidgets('未登记的一行给出两个方向，登记过的方向不再是可选项', (tester) async {
    final pool = LocaleSettings.currentLocale.translations.pool;

    await _pump(tester, const PoolPage(), _client([]));
    expect(
      _buttonLabels(tester),
      [pool.release, pool.deny, pool.occurrenceCount],
      reason: '两个方向都要给，出现记录是第三个入口',
    );

    await _pump(
      tester,
      const PoolPage(),
      _client([], registeredAction: 'release'),
    );
    final buttons = tester.widgetList<ActionButton>(_rowButtons());
    expect(
      buttons.map((button) => button.text),
      [pool.registeredRelease, pool.deny, pool.occurrenceCount],
    );
    expect(
      buttons.first.onPressed,
      isNull,
      reason: '这段内容已经按放行处理，放行不再是可选项',
    );
    expect(buttons.elementAt(1).onPressed, isNotNull, reason: '禁止仍然可以改判');
    expect(tester.takeException(), isNull);
  });

  testWidgets('禁止把决定发到这条内容的锚点上，并把结果告诉用户', (tester) async {
    final requests = <String>[];
    await _pump(
      tester,
      const PoolPage(),
      _client(requests),
    );
    final pool = LocaleSettings.currentLocale.translations.pool;

    await tester.tap(find.widgetWithText(ActionButton, pool.deny));
    await tester.pumpAndSettle();

    expect(requests, contains('POST /api/spans/s-anchor/redact'));
    expect(find.text(pool.deniedCreated), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('出现记录按锚点拉取，并逐条给出判定与来源', (tester) async {
    final requests = <String>[];
    final summary = ContentSummary.fromJson(rowJson());
    await _pump(
      tester,
      ContentDetailPage(summary: summary),
      _client(requests),
      scrollable: false,
    );
    await tester.pumpAndSettle();
    final pool = LocaleSettings.currentLocale.translations.pool;
    final decision = LocaleSettings.currentLocale.translations.model.decision;

    expect(requests, contains('GET /api/spans/s-anchor/occurrences'));
    // 两次出现各自给出方向：抹去与放行都在，这才叫「摇摆」。
    expect(find.text(decision.redact), findsWidgets);
    expect(find.text(decision.release), findsWidgets);
    expect(
      find.text(pool.matchedRule(id: 'builtin.redact.private_email')),
      findsOneWidget,
      reason: '命中规则要带着说明出现，而不是只剩下标识',
    );
    expect(
      find.textContaining('/v1/chat/completions'),
      findsNWidgets(2),
      reason: '每一次出现都带上来源',
    );
    expect(tester.takeException(), isNull);
  });
}
