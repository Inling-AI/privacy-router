/// 首次安装的自助初始化契约。
///
/// 断言用户看得见的结果：还没有管理员时直接进入初始化表单，创建成功后立刻进入控制台；
/// 代理连不上时不展示这个表单——它此时什么也做不了。表单本身的错误（两次口令不一致）
/// 必须在本地拦住，不能把用户的输入当成一次失败的创建请求发出去。
library;

import 'dart:convert';

import 'package:shared_preferences/shared_preferences.dart';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';
import 'package:privacy_router_console/api/api_client.dart';
import 'package:privacy_router_console/i18n/strings.g.dart';
import 'package:privacy_router_console/main.dart';
import 'package:privacy_router_console/state/providers.dart';
import 'package:privacy_router_console/state/session.dart';
import 'package:privacy_router_console/ui/pages/login_page.dart';
import 'package:privacy_router_console/ui/pages/setup_page.dart';
import 'package:privacy_router_console/ui/shell.dart';
import 'package:privacy_router_console/ui/widgets.dart';

/// 按要求应答的假服务端，同时记录收到的调用。
class _FakeServer {
  _FakeServer({required this.adminConfigured, this.reachable = true});

  final bool adminConfigured;

  /// 为 false 时模拟代理没有运行：健康检查失败。
  final bool reachable;

  final List<String> calls = [];

  http.Client client() => MockClient((request) async {
    final path = request.url.path;
    calls.add('${request.method} $path');
    if (path == '/api/health') {
      if (!reachable) {
        return _json(
          {'error': {'kind': 'network', 'message': 'unreachable'}},
          status: 503,
        );
      }
      return _json({
        'status': 'ok',
        'rules': 0,
        'providers': 0,
        'admin_configured': adminConfigured,
      });
    }
    return _json(switch (path) {
      '/api/setup' => {'token': 'issued-token', 'expires_at': 0},
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
      '/api/rules' => {'items': const <dynamic>[], 'fallback_action': 'redact'},
      '/api/providers' => {'items': const <dynamic>[]},
      '/api/content' => {'items': const <dynamic>[], 'total': 0},
      _ => const <String, dynamic>{},
    });
  });

  static http.Response _json(Object body, {int status = 200}) => http.Response(
    jsonEncode(body),
    status,
    headers: const {'content-type': 'application/json; charset=utf-8'},
  );
}

/// 装配真正会显示的根页面——由 `ConsoleApp` 自己决定给哪一张表单。
Future<ProviderContainer> _pump(WidgetTester tester, _FakeServer server) async {
  await tester.pumpWidget(
    TranslationProvider(
      child: ProviderScope(
        overrides: [
          apiClientProvider.overrideWithValue(
            ApiClient(baseUrl: 'http://console.test', client: server.client()),
          ),
        ],
        child: const ConsoleApp(),
      ),
    ),
  );
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 400));
  return ProviderScope.containerOf(tester.element(find.byType(ConsoleApp)));
}

Future<void> _fillPasswords(WidgetTester tester, String password, String confirmation) async {
  await tester.enterText(find.byType(GlassTextField).at(1), password);
  await tester.enterText(find.byType(GlassTextField).at(2), confirmation);
  await tester.tap(find.byType(ActionButton));
  await tester.pump();
}

void main() {
  setUpAll(LiquidGlassWidgets.initialize);
  setUp(() => SharedPreferences.setMockInitialValues({}));

  testWidgets('restored session opens the console without a login form', (tester) async {
    await SessionStore(apiBaseUrl: 'http://console.test').save(
      const Session(token: 'saved-token', username: 'admin', expiresAt: 4102444800000),
    );
    await _pump(tester, _FakeServer(adminConfigured: true));

    expect(find.byType(ConsoleShell), findsOneWidget);
    expect(find.byType(LoginPage), findsNothing);
  });

  testWidgets('还没有管理员时直接给出初始化表单', (tester) async {
    await _pump(tester, _FakeServer(adminConfigured: false));

    expect(find.byType(SetupPage), findsOneWidget);
    expect(find.byType(LoginPage), findsNothing);
  });

  testWidgets('已有管理员时给出的是登录表单', (tester) async {
    await _pump(tester, _FakeServer(adminConfigured: true));

    expect(find.byType(LoginPage), findsOneWidget);
    expect(find.byType(SetupPage), findsNothing);
  });

  testWidgets('代理连不上时不展示初始化表单', (tester) async {
    await _pump(tester, _FakeServer(adminConfigured: false, reachable: false));

    expect(find.byType(SetupPage), findsNothing);
    expect(find.byType(LoginPage), findsOneWidget);
  });

  testWidgets('两次口令不一致时只在本地拦下，不发出创建请求', (tester) async {
    final server = _FakeServer(adminConfigured: false);
    final container = await _pump(tester, server);

    await _fillPasswords(tester, 'correct horse', 'correct hors');

    expect(server.calls, isNot(contains('POST /api/setup')));
    expect(container.read(sessionProvider), isA<LoggedOut>());
  });

  testWidgets('创建成功后立即进入控制台', (tester) async {
    final server = _FakeServer(adminConfigured: false);
    final container = await _pump(tester, server);

    await _fillPasswords(tester, 'correct horse', 'correct horse');
    await tester.pump(const Duration(milliseconds: 400));

    expect(server.calls, contains('POST /api/setup'));
    final session = container.read(sessionProvider);
    expect(session, isA<LoggedIn>());
    expect((session as LoggedIn).session.token, 'issued-token');
    expect(find.byType(SetupPage), findsNothing);
    expect(find.byType(ConsoleShell), findsOneWidget);
  });
}
