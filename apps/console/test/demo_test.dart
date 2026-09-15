import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';
import 'package:privacy_router_console/api/api_client.dart';
import 'package:privacy_router_console/api/models.dart';
import 'package:privacy_router_console/demo/demo_api_client.dart';
import 'package:privacy_router_console/demo/demo_config.dart';
import 'package:privacy_router_console/i18n/strings.g.dart';
import 'package:privacy_router_console/main.dart';
import 'package:privacy_router_console/state/providers.dart';
import 'package:privacy_router_console/ui/pages/login_page.dart';
import 'package:privacy_router_console/ui/shell.dart';
import 'package:privacy_router_console/ui/widgets.dart';
import 'package:shared_preferences/shared_preferences.dart';

void main() {
  final now = DateTime.utc(2026, 9, 15, 12);
  const token = DemoConfig.sessionToken;
  test('demo login and logout enforce local session access', () async {
    final api = DemoApiClient(now: () => now);
    await expectLater(
      api.login('admin', 'private'),
      throwsA(isA<ApiException>()),
    );
    final session = await api.login(DemoConfig.username, DemoConfig.password);
    expect(await api.providers(session.token), isNotEmpty);
    await api.logout(session.token);
    await expectLater(
      api.providers(session.token),
      throwsA(isA<ApiException>()),
    );
  });
  test('review changes rules and filters while preserving history', () async {
    final api = DemoApiClient(now: () => now);
    final before = await api.content(token: token);
    final item = before.items.first;
    final stats = await api.statistics(token);
    final outcome = await api.releaseSpan(token, item.anchorSpanId);
    expect(outcome.changed, isTrue);
    expect((await api.releaseSpan(token, item.anchorSpanId)).changed, isFalse);
    expect((await api.content(token: token)).total, before.total - 1);
    final reviewed = await api.content(
      token: token,
      filter: ContentFilter.reviewed,
    );
    expect(
      reviewed.items
          .firstWhere((row) => row.anchorSpanId == item.anchorSpanId)
          .registeredAction,
      Decision.release,
    );
    expect(
      (await api.rules(token)).items
          .firstWhere((rule) => rule.id == outcome.ruleId)
          .action,
      Decision.release,
    );
    expect((await api.statistics(token)).redactedSpans, stats.redactedSpans);
    await api.setRuleEnabled(token, outcome.ruleId, false);
    expect((await api.content(token: token)).total, before.total);
  });
  test('upstream edits reset when a fresh demo is loaded', () async {
    final api = DemoApiClient(now: () => now);
    final original = (await api.providers(token)).first;
    await api.updateProvider(token, original.id, {
      'name': 'My edit',
      'enabled': false,
    });
    final edited = (await api.providers(token)).first;
    expect(edited.name, 'My edit');
    expect(edited.enabled, isFalse);
    expect(
      (await DemoApiClient(now: () => now).providers(token)).first.name,
      original.name,
    );
    await api.deleteProvider(token, original.id);
    expect(
      (await api.providers(token)).any((item) => item.id == original.id),
      isFalse,
    );
  });
  setUpAll(LiquidGlassWidgets.initialize);
  for (final width in [390.0, 1440.0]) {
    testWidgets('demo login reset and reload at $width', (tester) async {
      SharedPreferences.setMockInitialValues({});
      tester.view.physicalSize = Size(width, 1000);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      Widget app() => TranslationProvider(
        child: ProviderScope(
          overrides: [demoModeProvider.overrideWithValue(true)],
          child: const ConsoleApp(),
        ),
      );
      await tester.pumpWidget(app());
      await tester.pumpAndSettle();
      expect(find.byType(LoginPage), findsOneWidget);
      await tester.tap(find.byType(ActionButton));
      await tester.pumpAndSettle();
      expect(find.byType(ConsoleShell), findsOneWidget);
      expect(tester.takeException(), isNull);
      await tester.tap(find.text(t.demo.reset));
      await tester.pumpAndSettle();
      expect(find.byType(ConsoleShell), findsOneWidget);
      await tester.pumpWidget(const SizedBox());
      await tester.pumpWidget(app());
      await tester.pumpAndSettle();
      expect(find.byType(ConsoleShell), findsOneWidget);
      expect(find.byType(LoginPage), findsNothing);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox());
    });
  }
}
