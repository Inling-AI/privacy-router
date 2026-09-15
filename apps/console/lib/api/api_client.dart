/// 控制台 API 客户端。所有 HTTP 细节集中在这里，界面只调用方法。
library;

import 'dart:async';
import 'dart:convert';

import 'package:http/http.dart' as http;

import 'models.dart';

/// 一次失败的 API 调用。
class ApiException implements Exception {
  ApiException(this.statusCode, this.kind, this.message);

  final int statusCode;
  final String kind;
  final String message;

  /// 会话失效。界面据此跳回登录。
  bool get isUnauthorized => statusCode == 401;

  @override
  String toString() => message;
}

/// 控制台 API 的访问入口。
///
/// 认证令牌由调用方持有并在每次请求时传入，客户端本身不保存状态——会话的生命周期属于
/// Riverpod 的 provider，不属于传输层。
abstract interface class ApiClient {
  factory ApiClient({required String baseUrl, http.Client? client}) =
      _HttpApiClient;

  static const String defaultBaseUrl = String.fromEnvironment(
    'PRIVACY_ROUTER_API',
  );

  String get baseUrl;
  void close();
  Future<IssuedSession> login(String username, String password);
  Future<IssuedSession> setup(String username, String password);
  Future<void> logout(String token);
  Future<Health> health();
  Future<Statistics> statistics(String token, {int hours = 24});
  Future<ContentPage> content({
    required String token,
    ContentFilter filter = ContentFilter.unreviewed,
    int limit = 50,
    int offset = 0,
  });
  Future<List<ContentOccurrence>> occurrences(String token, String spanId);
  Future<FragmentDetail> fragment(String token, String spanId);
  Future<SpanDecisionOutcome> releaseSpan(String token, String spanId);
  Future<SpanDecisionOutcome> redactSpan(String token, String spanId);
  Future<RuleList> rules(String token);
  Future<void> setFallback(String token, Decision action);
  Future<Rule> createRule(String token, Map<String, dynamic> draft);
  Future<Rule> updateRule(String token, String id, Map<String, dynamic> draft);
  Future<void> setRuleEnabled(String token, String id, bool enabled);
  Future<void> deleteRule(String token, String id);
  Future<List<Upstream>> providers(String token);
  Future<Upstream> createProvider(String token, Map<String, dynamic> draft);
  Future<Upstream> updateProvider(
    String token,
    String id,
    Map<String, dynamic> draft,
  );
  Future<void> deleteProvider(String token, String id);
}

class _HttpApiClient implements ApiClient {
  _HttpApiClient({required this.baseUrl, http.Client? client})
    : _client = client ?? http.Client();

  @override
  final String baseUrl;
  final http.Client _client;

  /// 释放底层连接。由持有它的 provider 在销毁时调用。
  @override
  void close() => _client.close();

  Uri _uri(String path) => Uri.parse('$baseUrl$path');

  Future<Map<String, dynamic>> _send(
    String method,
    String path, {
    String? token,
    Object? body,
  }) async {
    final headers = <String, String>{
      if (token != null) 'authorization': 'Bearer $token',
      if (body != null) 'content-type': 'application/json',
    };
    final request = http.Request(method, _uri(path))..headers.addAll(headers);
    if (body != null) {
      request.body = jsonEncode(body);
    }

    final http.Response response;
    try {
      response = await http.Response.fromStream(await _client.send(request));
    } on Exception catch (error) {
      // 传输失败与服务端拒绝是两类问题，界面需要区分对待。
      throw ApiException(0, 'network', '$error');
    }

    if (response.statusCode == 204 || response.body.isEmpty) {
      return const {};
    }

    final decoded = jsonDecode(utf8.decode(response.bodyBytes));
    if (response.statusCode >= 400) {
      final error = (decoded as Map<String, dynamic>)['error'];
      final kind = error is Map ? error['kind'] as String? ?? 'error' : 'error';
      throw ApiException(
        response.statusCode,
        kind,
        error is Map ? error['message'] as String? ?? kind : kind,
      );
    }
    return decoded as Map<String, dynamic>;
  }

  // -- 会话 -----------------------------------------------------------------

  /// 提交一组凭据换取会话。登录与首次初始化是同一个请求/响应契约，只是端点不同：
  /// 服务端只在「还没有管理员」时才接受 `/api/setup`。
  Future<IssuedSession> _submitCredentials(
    String path,
    String username,
    String password,
  ) async => IssuedSession.fromJson(
    await _send(
      'POST',
      path,
      body: {'username': username, 'password': password},
    ),
  );

  @override
  Future<IssuedSession> login(String username, String password) =>
      _submitCredentials('/api/session', username, password);

  /// 首次安装的自助初始化：创建唯一管理员，成功即拿到可直接使用的会话。
  @override
  Future<IssuedSession> setup(String username, String password) =>
      _submitCredentials('/api/setup', username, password);

  @override
  Future<void> logout(String token) =>
      _send('DELETE', '/api/session', token: token);

  // -- 健康检查 -------------------------------------------------------------

  @override
  Future<Health> health() async =>
      Health.fromJson(await _send('GET', '/api/health'));

  // -- 统计 -----------------------------------------------------------------

  @override
  Future<Statistics> statistics(String token, {int hours = 24}) async =>
      Statistics.fromJson(
        await _send('GET', '/api/stats?hours=$hours', token: token),
      );

  // -- 内容池 ---------------------------------------------------------------

  /// 按内容聚合的一页。默认只给还没登记过决定的内容。
  @override
  Future<ContentPage> content({
    required String token,
    ContentFilter filter = ContentFilter.unreviewed,
    int limit = 50,
    int offset = 0,
  }) async => ContentPage.fromJson(
    await _send(
      'GET',
      '/api/content?filter=${filter.wire}&limit=$limit&offset=$offset',
      token: token,
    ),
  );

  /// 一段内容的全部出现，最近的在前。锚点是命中标识，不是被审查的文本。
  @override
  Future<List<ContentOccurrence>> occurrences(
    String token,
    String spanId,
  ) async {
    final json = await _send(
      'GET',
      '/api/spans/$spanId/occurrences',
      token: token,
    );
    return (json['items'] as List<dynamic>)
        .map((item) => ContentOccurrence.fromJson(item as Map<String, dynamic>))
        .toList();
  }

  /// 一次出现所属的 turn：原文、转发出去的内容，以及这次的全部命中。
  @override
  Future<FragmentDetail> fragment(String token, String spanId) async =>
      FragmentDetail.fromJson(
        await _send('GET', '/api/spans/$spanId/fragment', token: token),
      );

  /// 放行一条命中：服务端登记放行规则，后续相同内容直接放行。
  @override
  Future<SpanDecisionOutcome> releaseSpan(String token, String spanId) async =>
      SpanDecisionOutcome.fromJson(
        await _send('POST', '/api/spans/$spanId/release', token: token),
      );

  /// 禁止一条命中：服务端登记强制抹去规则，后续相同内容不再上行。
  @override
  Future<SpanDecisionOutcome> redactSpan(String token, String spanId) async =>
      SpanDecisionOutcome.fromJson(
        await _send('POST', '/api/spans/$spanId/redact', token: token),
      );

  // -- 规则 -----------------------------------------------------------------

  @override
  Future<RuleList> rules(String token) async =>
      RuleList.fromJson(await _send('GET', '/api/rules', token: token));

  @override
  Future<void> setFallback(String token, Decision action) => _send(
    'PATCH',
    '/api/rule-policy',
    token: token,
    body: {'fallback_action': action.wire},
  );

  @override
  Future<Rule> createRule(String token, Map<String, dynamic> draft) async =>
      Rule.fromJson(
        await _send('POST', '/api/rules', token: token, body: draft),
      );

  @override
  Future<Rule> updateRule(
    String token,
    String id,
    Map<String, dynamic> draft,
  ) async => Rule.fromJson(
    await _send('PATCH', '/api/rules/$id', token: token, body: draft),
  );

  @override
  Future<void> setRuleEnabled(String token, String id, bool enabled) => _send(
    'POST',
    '/api/rules/$id/enabled',
    token: token,
    body: {'enabled': enabled},
  );

  @override
  Future<void> deleteRule(String token, String id) =>
      _send('DELETE', '/api/rules/$id', token: token);

  // -- 上游 -----------------------------------------------------------------

  @override
  Future<List<Upstream>> providers(String token) async {
    final json = await _send('GET', '/api/providers', token: token);
    return (json['items'] as List<dynamic>)
        .map((item) => Upstream.fromJson(item as Map<String, dynamic>))
        .toList();
  }

  @override
  Future<Upstream> createProvider(
    String token,
    Map<String, dynamic> draft,
  ) async => Upstream.fromJson(
    await _send('POST', '/api/providers', token: token, body: draft),
  );

  @override
  Future<Upstream> updateProvider(
    String token,
    String id,
    Map<String, dynamic> draft,
  ) async => Upstream.fromJson(
    await _send('PATCH', '/api/providers/$id', token: token, body: draft),
  );

  @override
  Future<void> deleteProvider(String token, String id) =>
      _send('DELETE', '/api/providers/$id', token: token);
}
