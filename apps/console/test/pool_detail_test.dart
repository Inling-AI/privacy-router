/// 内容池详情的导航契约。
///
/// 详情走和规则页同一条路：点「出现记录」推入独立页面，点一次出现再推入它所属的 turn。
/// turn 页要能看见进模型前的原文、真正转发出去的内容，以及这次识别到的具体内容。
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

const _original = 'im AlexExample and my email is alex@example.com';
const _forwarded =
    'im <redacted_private_person> and my email is <redacted_private_email>';

Map<String, dynamic> rowJson() => {
  'anchor_span_id': 's-anchor',
  'original_text': 'AlexExample',
  'occurrences': 1,
  'turns': 1,
  'released': 0,
  'redacted': 1,
  'score_min': 0.99,
  'score_max': 0.99,
  'first_seen': 1700000000000,
  'last_seen': 1700000006000,
  'latest_action': 'redact',
  'categories': [
    {'entity_group': 'private_person', 'count': 1},
  ],
  'registered_action': null,
};

Map<String, dynamic> occurrenceJson() => {
  'span_id': 's-1',
  'entity_group': 'private_person',
  'score': 0.99,
  'action': 'redact',
  'matched_rule_id': 'builtin.redact.private_person',
  'api_format': 'openai_chat',
  'path': '/v1/chat/completions',
  'created_at': 1700000006000,
};

Map<String, dynamic> fragmentJson() => {
  'fragment': {
    'id': 'f-1',
    'request_id': 'req-1',
    'api_format': 'openai_chat',
    'path': '/v1/chat/completions',
    'original_text': _original,
    'redacted_text': _forwarded,
    'created_at': 1700000006000,
  },
  'spans': [
    {
      'id': 's-1',
      'entity_group': 'private_person',
      'score': 0.99,
      'char_len': 11,
      'byte_start': 3,
      'byte_end': 14,
      'original_text': 'AlexExample',
      'action': 'redact',
      'matched_rule_id': 'builtin.redact.private_person',
    },
  ],
};

http.Client _client(List<String> requests) => MockClient((request) async {
  requests.add('${request.method} ${request.url.path}');
  final body = switch (request.url.path) {
    '/api/content' => {
      'items': [rowJson()],
      'total': 1,
    },
    '/api/spans/s-anchor/occurrences' => {
      'items': [occurrenceJson()],
    },
    '/api/spans/s-1/fragment' => fragmentJson(),
    // 登记端点只回答「登记到哪条规则、这次有没有改动它」。
    _ when request.url.path.startsWith('/api/spans/') =>
      {'rule_id': 'operator.1', 'changed': true},
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

void main() {
  setUpAll(LiquidGlassWidgets.initialize);

  testWidgets('内容详情页同样能登记，已登记的方向不再是可选项', (tester) async {
    final requests = <String>[];
    final pool = LocaleSettings.currentLocale.translations.pool;
    final summary = ContentSummary.fromJson(
      rowJson()..['registered_action'] = 'release',
    );

    await _pump(
      tester,
      ContentDetailPage(summary: summary),
      _client(requests),
      scrollable: false,
    );
    await tester.pumpAndSettle();

    final buttons = tester
        .widgetList<ActionButton>(find.byType(ActionButton))
        .toList();
    expect(
      buttons.map((button) => button.text),
      [pool.registeredRelease, pool.deny],
      reason: '放行已经登记，只剩改判的入口',
    );
    expect(buttons.first.onPressed, isNull);

    await tester.tap(find.widgetWithText(ActionButton, pool.deny));
    await tester.pumpAndSettle();
    expect(requests, contains('POST /api/spans/s-anchor/redact'));
    expect(tester.takeException(), isNull);
  });

  testWidgets('点「出现记录」推入独立的内容详情页', (tester) async {
    final requests = <String>[];
    final pool = LocaleSettings.currentLocale.translations.pool;

    await _pump(tester, const PoolPage(), _client(requests));
    await tester.tap(find.widgetWithText(ActionButton, pool.occurrenceCount));
    await tester.pumpAndSettle();

    expect(find.text(pool.contentTitle), findsOneWidget);
    expect(
      requests,
      contains('GET /api/spans/s-anchor/occurrences'),
      reason: '详情页的出现记录仍按这条内容的锚点拉取',
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('点一次出现推入它所属的 turn：原文、转发内容与识别到的内容', (tester) async {
    final requests = <String>[];
    final pool = LocaleSettings.currentLocale.translations.pool;

    await _pump(
      tester,
      ContentDetailPage(summary: ContentSummary.fromJson(rowJson())),
      _client(requests),
      scrollable: false,
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byIcon(CupertinoIcons.chevron_forward));
    await tester.pumpAndSettle();

    expect(requests, contains('GET /api/spans/s-1/fragment'));
    expect(find.text(pool.turnTitle), findsOneWidget);
    expect(find.text(pool.original), findsOneWidget);
    expect(find.text(_original), findsOneWidget);
    expect(find.text(pool.forwarded), findsOneWidget);
    expect(find.text(_forwarded), findsOneWidget);
    expect(find.text(pool.detectedSpans), findsOneWidget);
    expect(find.text('AlexExample'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });
}
