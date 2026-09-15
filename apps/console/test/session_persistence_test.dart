import 'dart:async';
import 'dart:convert';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:privacy_router_console/api/api_client.dart';
import 'package:privacy_router_console/state/providers.dart';
import 'package:privacy_router_console/state/session.dart';
import 'package:shared_preferences/shared_preferences.dart';

class SessionHarness {
  DateTime now = DateTime.utc(2026, 1, 1);
  bool rejectLogin = false;
  final logoutResponse = Completer<http.Response>();

  SessionStore store([String origin = 'http://console.test']) =>
      SessionStore(apiBaseUrl: origin, now: () => now);

  ProviderContainer open() {
    final container = ProviderContainer(
      overrides: [
        sessionStoreProvider.overrideWithValue(store()),
        apiClientProvider.overrideWithValue(
          ApiClient(
            baseUrl: 'http://console.test',
            client: MockClient((request) async {
              if (request.method == 'DELETE') return logoutResponse.future;
              if (rejectLogin) {
                return http.Response(
                  jsonEncode({
                    'error': {
                      'kind': 'unauthorized',
                      'message': 'Invalid credentials',
                    },
                  }),
                  401,
                );
              }
              return http.Response(
                jsonEncode({
                  'token': 'issued-token',
                  'expires_at': 1767229200000,
                }),
                200,
              );
            }),
          ),
        ),
      ],
    );
    addTearDown(container.dispose);
    return container;
  }

  Future<SessionState> ready(ProviderContainer container) async {
    final completed = Completer<SessionState>();
    final subscription = container.listen(sessionProvider, (_, state) {
      if (state is! RestoringSession && !completed.isCompleted) {
        completed.complete(state);
      }
    }, fireImmediately: true);
    try {
      return await completed.future;
    } finally {
      subscription.close();
    }
  }
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  setUp(() => SharedPreferences.setMockInitialValues({}));

  for (final setup in [false, true]) {
    test(
      '${setup ? 'setup' : 'login'} survives a fresh application container',
      () async {
        final harness = SessionHarness();
        final first = harness.open();
        expect(await harness.ready(first), isA<LoggedOut>());
        final notifier = first.read(sessionProvider.notifier);
        await (setup
            ? notifier.setup('admin', 'password')
            : notifier.login('admin', 'password'));
        final restored = await harness.ready(harness.open()) as LoggedIn;
        expect(restored.session.token, 'issued-token');
        expect(restored.session.username, 'admin');
        expect(restored.session.expiresAt, 1767229200000);
      },
    );
  }

  test('logout clears persisted login before the server responds', () async {
    final harness = SessionHarness();
    final container = harness.open();
    await harness.ready(container);
    final notifier = container.read(sessionProvider.notifier);
    await notifier.login('admin', 'password');
    final loggedOut = Completer<void>();
    final subscription = container.listen(sessionProvider, (_, state) {
      if (state is LoggedOut) loggedOut.complete();
    });
    final logout = notifier.logout();
    await loggedOut.future;
    subscription.close();
    expect(await harness.ready(harness.open()), isA<LoggedOut>());
    harness.logoutResponse.complete(
      http.Response(
        jsonEncode({
          'error': {'kind': 'network', 'message': 'Unavailable'},
        }),
        503,
      ),
    );
    await logout;
    expect(container.read(sessionProvider), isA<LoggedOut>());
  });

  test('expired sessions are discarded at the expiry boundary', () async {
    final harness = SessionHarness();
    final container = harness.open();
    await harness.ready(container);
    await container.read(sessionProvider.notifier).login('admin', 'password');
    harness.now = DateTime.fromMillisecondsSinceEpoch(
      1767229200000,
      isUtc: true,
    );
    expect(await harness.ready(harness.open()), isA<LoggedOut>());
    harness.now = DateTime.utc(2026, 1, 1);
    expect(await harness.store().restore(), isNull);
  });

  test('rejected login does not create a restorable session', () async {
    final harness = SessionHarness()..rejectLogin = true;
    final container = harness.open();
    await harness.ready(container);
    await container.read(sessionProvider.notifier).login('admin', 'wrong');
    expect(container.read(sessionProvider), isA<AuthenticationFailed>());
    expect(await harness.ready(harness.open()), isA<LoggedOut>());
  });

  test('credentials are isolated by API origin', () async {
    final harness = SessionHarness();
    final container = harness.open();
    await harness.ready(container);
    await container.read(sessionProvider.notifier).login('admin', 'password');
    expect(await harness.store('http://another.test').restore(), isNull);
  });
}
