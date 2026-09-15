/// Riverpod 状态层。
///
/// 会话令牌是唯一的认证事实来源：`sessionProvider` 持有它，其余 provider 通过 `ref.watch`
/// 取得。API 客户端本身不保存令牌，因此不存在两处状态不同步的可能。
///
/// 采用手写 provider 而非代码生成：provider 的依赖关系已经由 `ref.watch` 在编译期约束，
/// 代码生成在这里只会额外引入一次构建步骤与一份需要防漂移的产物。
library;

import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../api/api_client.dart';
import '../api/models.dart';
import '../demo/demo_api_client.dart';
import '../demo/demo_config.dart';
import 'session.dart';

export 'session.dart' show Session;

/// API 客户端。默认同源；`--dart-define=PRIVACY_ROUTER_API=...` 可指向别处的代理，
/// 供 `flutter run` 热重载开发使用。
final demoModeProvider = Provider<bool>((ref) => DemoConfig.enabled);

final apiClientProvider = Provider<ApiClient>((ref) {
  final ApiClient client = ref.watch(demoModeProvider)
      ? DemoApiClient()
      : ApiClient(baseUrl: ApiClient.defaultBaseUrl);
  ref.onDispose(client.close);
  return client;
});

final sessionStoreProvider = Provider<SessionStore>((ref) {
  return SessionStore(apiBaseUrl: ref.watch(apiClientProvider).baseUrl);
});

/// 会话状态：恢复中、未登录、登录中、已登录、失败。
sealed class SessionState {
  const SessionState();
}

class RestoringSession extends SessionState {
  const RestoringSession();
}

class LoggedOut extends SessionState {
  const LoggedOut();
}

/// 正在提交凭据：登录与首次初始化共用这一个中间态。
class InFlight extends SessionState {
  const InFlight();
}

class LoggedIn extends SessionState {
  const LoggedIn(this.session);

  final Session session;
}

/// 凭据提交被拒。表单把消息直接显示给用户，因此成功与失败各自只有一条路径。
class AuthenticationFailed extends SessionState {
  const AuthenticationFailed(this.message);

  final String message;
}

/// 会话的生命周期。
class SessionNotifier extends Notifier<SessionState> {
  @override
  SessionState build() {
    final store = ref.watch(sessionStoreProvider);
    unawaited(_restore(store));
    return const RestoringSession();
  }

  Future<void> _restore(SessionStore store) async {
    Session? session;
    try {
      session = await store.restore();
    } on Exception {
      // Storage may be unavailable; the sign-in form must remain accessible.
    }
    if (ref.mounted && state is RestoringSession) {
      state = session == null ? const LoggedOut() : LoggedIn(session);
    }
  }

  /// 提交凭据换取会话。登录与首次初始化走同一条状态机：提交中、成功即已登录、
  /// 失败则带着消息回到表单。
  Future<void> _submit(
    String username,
    Future<IssuedSession> Function() issue,
  ) async {
    state = const InFlight();
    try {
      final issued = await issue();
      final session = Session(
        token: issued.token,
        username: username,
        expiresAt: issued.expiresAt,
      );
      await ref.read(sessionStoreProvider).save(session);
      if (ref.mounted) state = LoggedIn(session);
    } on ApiException catch (error) {
      if (ref.mounted) state = AuthenticationFailed(error.message);
    } catch (_) {
      if (ref.mounted) {
        state = const AuthenticationFailed(
          'Could not save the session. Please try again.',
        );
      }
    }
  }

  Future<void> login(String username, String password) => _submit(
    username,
    () => ref.read(apiClientProvider).login(username, password),
  );

  /// 首次安装：创建唯一管理员并直接进入控制台。
  ///
  /// 无论成功与否都重新确认一次服务端状态：成功后「还没有管理员」这个判断已经过期，
  /// 而失败也可能正是因为别的标签页/别人刚刚完成了初始化。
  Future<void> setup(String username, String password) async {
    await _submit(
      username,
      () => ref.read(apiClientProvider).setup(username, password),
    );
    ref.invalidate(healthProvider);
  }

  Future<void> logout() async {
    final current = state;
    await ref.read(sessionStoreProvider).clear();
    if (!ref.mounted) return;
    state = const LoggedOut();
    if (current is LoggedIn) {
      // 注销失败不应把用户困在已登录界面；本地状态先清掉。
      try {
        await ref.read(apiClientProvider).logout(current.session.token);
      } on ApiException {
        // 忽略：令牌可能已过期，服务端已无可注销的会话。
      }
    }
  }
}

final sessionProvider = NotifierProvider<SessionNotifier, SessionState>(
  SessionNotifier.new,
);

/// 从会话状态里取令牌；未登录时抛出，使需要认证的 provider 不必各自处理空值。
String _requireToken(SessionState state) => switch (state) {
  LoggedIn(:final session) => session.token,
  _ => throw const _NotAuthenticated(),
};

/// 在 provider 体内取令牌。`watch` 让会话变化时自动重新拉取。
String _watchToken(Ref ref) => _requireToken(ref.watch(sessionProvider));

/// 在写操作回调里取令牌。回调不在构建期，因此必须用 `read`。
String _readToken(Ref ref) => _requireToken(ref.read(sessionProvider));

class _NotAuthenticated implements Exception {
  const _NotAuthenticated();

  @override
  String toString() => 'unauthenticated';
}

/// 健康状态。无需认证，用于在登录页提示环境是否就绪。
final healthProvider = FutureProvider<Health>((ref) async {
  return ref.watch(apiClientProvider).health();
});

/// 「停在页面上就自己更新」的节拍。统计与内容池共用同一个间隔：在用户眼里这是同一件事
/// （数据会自己动），各定各的只会让人以为是两套机制。
const _liveInterval = Duration(seconds: 2);

/// 把一次请求变成持续订阅：订阅建立时立刻拉一次，之后每 [_liveInterval] 拉一次；没人订阅
/// 就停表，不再打服务器。
///
/// 首个数据到达前失败会把错误交给订阅者，界面据此显示失败态；此后的失败静默忽略——上一次的
/// 数据还在，把整块内容换成错误页只会更差。
///
/// 私有自由函数：它是与业务实体完全解耦的纯组合子（订阅生命周期 + 一次请求 → 数据流），
/// 统计与内容池两条实时数据源都由它产生。它是一个完整的能力，不属于任何单个数据类型，
/// 塞进哪个类型里都只是替那个类型多背一份它并不理解的生命周期。
Stream<T> _liveSubscription<T>(Ref ref, Future<T> Function() request) {
  final stream = StreamController<T>();
  Timer? timer;
  var disposed = false;
  var fetching = false;
  var hasValue = false;

  Future<void> fetch() async {
    if (disposed || fetching) return;
    fetching = true;
    try {
      final value = await request();
      if (!disposed) {
        hasValue = true;
        stream.add(value);
      }
    } catch (error, stack) {
      if (!disposed && !hasValue) stream.addError(error, stack);
    } finally {
      fetching = false;
    }
  }

  void resume() {
    timer?.cancel();
    unawaited(fetch());
    timer = Timer.periodic(_liveInterval, (_) => unawaited(fetch()));
  }

  ref.onCancel(() => timer?.cancel());
  ref.onResume(resume);
  ref.onDispose(() {
    disposed = true;
    timer?.cancel();
    unawaited(stream.close());
  });
  resume();
  return stream.stream;
}

/// 两个统计视图共用同一条按时间窗的实时订阅。
final statisticsProvider = StreamProvider.autoDispose.family<Statistics, int>((
  ref,
  hours,
) {
  final client = ref.watch(apiClientProvider);
  final token = _watchToken(ref);
  return _liveSubscription(ref, () => client.statistics(token, hours: hours));
});

/// 内容池的一页：看哪种过滤、第几页。
class ContentQuery {
  const ContentQuery({
    this.filter = ContentFilter.unreviewed,
    this.limit = 50,
    this.offset = 0,
  });

  final ContentFilter filter;
  final int limit;
  final int offset;

  ContentQuery at(int offset) =>
      ContentQuery(filter: filter, limit: limit, offset: offset);

  /// 换一种过滤条件时回到第一页：新条件下的第 N 页和旧条件的第 N 页不是一回事。
  ContentQuery filtered(ContentFilter filter) =>
      ContentQuery(filter: filter, limit: limit);

  @override
  bool operator ==(Object other) =>
      other is ContentQuery &&
      other.filter == filter &&
      other.limit == limit &&
      other.offset == offset;

  @override
  int get hashCode => Object.hash(filter, limit, offset);
}

/// 内容池一页。和统计一样是实时订阅：离开分区即停止，切回来时重新拉一次——列表拿着上一次
/// 的页当现状，用户看到的就永远是他离开那一刻的内容。
final contentProvider = StreamProvider.autoDispose
    .family<ContentPage, ContentQuery>((ref, query) {
      final client = ref.watch(apiClientProvider);
      final token = _watchToken(ref);
      return _liveSubscription(
        ref,
        () => client.content(
          token: token,
          filter: query.filter,
          limit: query.limit,
          offset: query.offset,
        ),
      );
    });

/// 一段内容的全部出现。锚点是命中标识：被审查的文本不进 URL。
final occurrencesProvider =
    FutureProvider.family<List<ContentOccurrence>, String>((ref, spanId) async {
      return ref.watch(apiClientProvider).occurrences(_watchToken(ref), spanId);
    });

/// 一次出现所属的 turn：原文、转发出去的内容与这次的全部命中。
final fragmentProvider = FutureProvider.family<FragmentDetail, String>((
  ref,
  spanId,
) async {
  return ref.watch(apiClientProvider).fragment(_watchToken(ref), spanId);
});

/// 规则列表。
final rulesProvider = FutureProvider<RuleList>((ref) async {
  return ref.watch(apiClientProvider).rules(_watchToken(ref));
});

/// 上游列表。
final upstreamsProvider = FutureProvider<List<Upstream>>((ref) async {
  return ref.watch(apiClientProvider).providers(_watchToken(ref));
});

/// 规则与上游的写操作。写成功后使相关读取 provider 失效，界面自动刷新。
class RuleActions {
  const RuleActions(this._ref);

  final Ref _ref;

  ApiClient get _api => _ref.read(apiClientProvider);

  Future<void> setFallback(Decision action) async {
    await _api.setFallback(_readToken(_ref), action);
    _ref.invalidate(rulesProvider);
  }

  Future<void> setEnabled(String id, bool enabled) async {
    await _api.setRuleEnabled(_readToken(_ref), id, enabled);
    _ref.invalidate(rulesProvider);
  }

  Future<void> delete(String id) async {
    await _api.deleteRule(_readToken(_ref), id);
    _ref.invalidate(rulesProvider);
  }

  Future<void> create(Map<String, dynamic> draft) async {
    await _api.createRule(_readToken(_ref), draft);
    _ref.invalidate(rulesProvider);
  }

  Future<void> update(String id, Map<String, dynamic> draft) async {
    await _api.updateRule(_readToken(_ref), id, draft);
    _ref.invalidate(rulesProvider);
  }
}

final ruleActionsProvider = Provider<RuleActions>(RuleActions.new);

class UpstreamActions {
  const UpstreamActions(this._ref);

  final Ref _ref;

  ApiClient get _api => _ref.read(apiClientProvider);

  Future<void> create(Map<String, dynamic> draft) async {
    await _api.createProvider(_readToken(_ref), draft);
    _ref.invalidate(upstreamsProvider);
  }

  Future<void> update(String id, Map<String, dynamic> draft) async {
    await _api.updateProvider(_readToken(_ref), id, draft);
    _ref.invalidate(upstreamsProvider);
  }

  Future<void> delete(String id) async {
    await _api.deleteProvider(_readToken(_ref), id);
    _ref.invalidate(upstreamsProvider);
  }
}

final upstreamActionsProvider = Provider<UpstreamActions>(UpstreamActions.new);

/// 登记管理员对一段内容的决定（放行或禁止），并刷新内容池与规则列表。
///
/// 两个方向改的都是同一条登记，因此刷新的范围完全相同：决定对后续请求即时生效，
/// 列表里那一行的「已登记」也随之变成新的方向。
final spanDecisionProvider =
    Provider<
      Future<SpanDecisionOutcome> Function(String spanId, Decision decision)
    >((ref) {
      return (String spanId, Decision decision) async {
        final client = ref.read(apiClientProvider);
        final token = _readToken(ref);
        final outcome = switch (decision) {
          Decision.release => await client.releaseSpan(token, spanId),
          Decision.redact => await client.redactSpan(token, spanId),
        };
        ref.invalidate(rulesProvider);
        ref.invalidate(contentProvider);
        ref.invalidate(occurrencesProvider);
        ref.invalidate(fragmentProvider);
        ref.invalidate(statisticsProvider);
        return outcome;
      };
    });
