import '../api/api_client.dart';
import '../api/models.dart';
import 'demo_config.dart';

/// A self-contained console dataset. There is no HTTP client or server fallback.
/// Changes live in this instance; reloading or resetting restores the samples.
class DemoApiClient implements ApiClient {
  DemoApiClient({DateTime Function()? now}) : _now = now ?? DateTime.now {
    final time = _now();
    _entries = [
      for (final (index, sample) in _samples.indexed)
        _DemoEntry(
          id: 'sample-$index',
          text: sample.$1,
          group: sample.$2,
          sentence: sample.$3,
          format: ApiFormat.values[index % ApiFormat.values.length],
          time: time.subtract(Duration(minutes: 7 + index * 19)),
          mixed: index.isEven,
        ),
    ];
    _rules = {
      'protect-secrets': const Rule(
        id: 'protect-secrets',
        name: 'Protect credentials',
        priority: 900,
        action: Decision.redact,
        enabled: true,
        source: RuleSource.builtin,
        condition: FilterRuleExpression(EntityRuleFilter(EntityGroup.secret)),
      ),
      'protect-email': const Rule(
        id: 'protect-email',
        name: 'Redact email addresses',
        priority: 700,
        action: Decision.redact,
        enabled: true,
        source: RuleSource.console,
        condition: FilterRuleExpression(
          EntityRuleFilter(EntityGroup.privateEmail),
        ),
      ),
    };
    _upstreams = {
      for (final format in ApiFormat.values)
        format.wire: Upstream(
          id: format.wire,
          name: switch (format) {
            ApiFormat.openaiChat => 'Chat workspace',
            ApiFormat.openaiResponses => 'Research assistant',
            ApiFormat.anthropicMessages => 'Code assistant',
          },
          baseUrl: 'https://${format.wire.replaceAll('_', '-')}.example.com/v1',
          apiFormat: format,
          enabled: true,
        ),
    };
    _register(_entries.last, Decision.release);
  }

  final DateTime Function() _now;
  late final List<_DemoEntry> _entries;
  late final Map<String, Rule> _rules;
  late final Map<String, Upstream> _upstreams;
  Decision _fallback = Decision.redact;
  int _nextId = 0;
  bool _signedOut = false;

  static const _samples = [
    (
      'alex.morgan@example.com',
      EntityGroup.privateEmail,
      'Send the project update to alex.morgan@example.com before Friday.',
    ),
    (
      'Jordan Lee',
      EntityGroup.privatePerson,
      'Jordan Lee will review the launch checklist with the team.',
    ),
    (
      '+1 202-555-0142',
      EntityGroup.privatePhone,
      'The test contact number is +1 202-555-0142.',
    ),
    (
      'DEMO_KEY_NOT_A_REAL_SECRET',
      EntityGroup.secret,
      'The example configuration contains DEMO_KEY_NOT_A_REAL_SECRET.',
    ),
    (
      '42 Example Lane',
      EntityGroup.privateAddress,
      'Deliver the sample package to 42 Example Lane.',
    ),
    (
      '4111 1111 1111 1111',
      EntityGroup.bankCard,
      'The payment sandbox uses test card 4111 1111 1111 1111.',
    ),
    (
      '198.51.100.42',
      EntityGroup.ipAddress,
      'The documentation host is 198.51.100.42.',
    ),
    (
      'support@example.com',
      EntityGroup.privateEmail,
      'Public enquiries go to support@example.com.',
    ),
  ];

  @override
  String get baseUrl => DemoConfig.storageScope;

  @override
  void close() {}

  void _authenticate(String token) {
    if (_signedOut || token != DemoConfig.sessionToken) {
      throw ApiException(401, 'unauthorized', 'Sign in with the demo account.');
    }
  }

  @override
  Future<IssuedSession> login(String username, String password) async {
    if (username != DemoConfig.username || password != DemoConfig.password) {
      throw ApiException(
        401,
        'unauthorized',
        'Use the demo credentials shown on the sign-in page.',
      );
    }
    _signedOut = false;
    return IssuedSession(
      token: DemoConfig.sessionToken,
      expiresAt: _now().add(const Duration(hours: 12)).millisecondsSinceEpoch,
    );
  }

  @override
  Future<IssuedSession> setup(String username, String password) async =>
      throw ApiException(
        409,
        'demo',
        'The demo account is already configured.',
      );

  @override
  Future<void> logout(String token) async {
    _authenticate(token);
    _signedOut = true;
  }

  @override
  Future<Health> health() async => Health(
    status: 'ok',
    rules: _rules.values.where((rule) => rule.enabled).length,
    providers: _upstreams.values.where((upstream) => upstream.enabled).length,
    adminConfigured: true,
  );

  @override
  Future<Statistics> statistics(String token, {int hours = 24}) async {
    _authenticate(token);
    final cutoff = _now().subtract(Duration(hours: hours));
    final occurrences = _entries
        .expand((entry) => entry.occurrences)
        .where((item) => !item.createdAt.isBefore(cutoff))
        .toList();
    final redacted = occurrences
        .where((item) => item.action == Decision.redact)
        .length;
    return Statistics(
      requests: occurrences.length,
      fragments: occurrences.length,
      spans: occurrences.length,
      redactedSpans: redacted,
      releasedSpans: occurrences.length - redacted,
      cachedFragments: occurrences.length ~/ 3,
      inferredFragments: occurrences.length - occurrences.length ~/ 3,
      latency: const LatencySummary(
        averageMs: 184,
        p50Ms: 161,
        p95Ms: 302,
        maxMs: 428,
        averageInferenceMs: 42,
        averageUpstreamMs: 142,
      ),
      model: ModelSummary(
        tokens: occurrences.length * 128,
        throughput: 3047,
        firstResultMs: 42,
        tokenizationMs: 2.4,
        validationMs: 0.6,
        forwardMs: 36,
        decodingMs: 3,
      ),
      byEntityGroup: [
        for (final group in EntityGroup.values)
          if (occurrences.any((item) => item.entityGroup == group))
            EntityGroupCount(
              entityGroup: group,
              count: occurrences
                  .where((item) => item.entityGroup == group)
                  .length,
            ),
      ],
    );
  }

  Rule? _registration(_DemoEntry entry) {
    final rule = _rules[entry.ruleId];
    if (rule == null || !rule.enabled) return null;
    return switch (rule.condition) {
      FilterRuleExpression(filter: KeywordRuleFilter(pattern: final pattern))
          when pattern.kind == RulePatternKind.exact &&
              pattern.text == entry.text =>
        rule,
      _ => null,
    };
  }

  @override
  Future<ContentPage> content({
    required String token,
    ContentFilter filter = ContentFilter.unreviewed,
    int limit = 50,
    int offset = 0,
  }) async {
    _authenticate(token);
    final items = _entries
        .where(
          (entry) => switch (filter) {
            ContentFilter.unreviewed => _registration(entry) == null,
            ContentFilter.reviewed => _registration(entry) != null,
            ContentFilter.all => true,
          },
        )
        .map((entry) => entry.summary(_registration(entry)?.action))
        .toList();
    return ContentPage(
      total: items.length,
      items: items
          .skip(offset.clamp(0, items.length))
          .take(limit.clamp(0, 100))
          .toList(),
    );
  }

  _DemoEntry _entry(String spanId) => _entries.firstWhere(
    (entry) => entry.occurrences.any((item) => item.spanId == spanId),
    orElse: () =>
        throw ApiException(404, 'not_found', 'Sample content not found.'),
  );

  @override
  Future<List<ContentOccurrence>> occurrences(
    String token,
    String spanId,
  ) async {
    _authenticate(token);
    return _entry(spanId).occurrences;
  }

  @override
  Future<FragmentDetail> fragment(String token, String spanId) async {
    _authenticate(token);
    return _entry(spanId).fragment(spanId);
  }

  SpanDecisionOutcome _register(_DemoEntry entry, Decision action) {
    final changed = _registration(entry)?.action != action;
    _rules[entry.ruleId] = Rule(
      id: entry.ruleId,
      name: '${action.name}: ${entry.text}',
      priority: 1000,
      action: action,
      enabled: true,
      source: RuleSource.operator,
      condition: FilterRuleExpression(
        KeywordRuleFilter(
          RulePattern(
            text: entry.text,
            kind: RulePatternKind.exact,
            caseSensitive: true,
          ),
        ),
      ),
    );
    return SpanDecisionOutcome(ruleId: entry.ruleId, changed: changed);
  }

  @override
  Future<SpanDecisionOutcome> releaseSpan(String token, String spanId) async {
    _authenticate(token);
    return _register(_entry(spanId), Decision.release);
  }

  @override
  Future<SpanDecisionOutcome> redactSpan(String token, String spanId) async {
    _authenticate(token);
    return _register(_entry(spanId), Decision.redact);
  }

  @override
  Future<RuleList> rules(String token) async {
    _authenticate(token);
    return RuleList(
      items: _rules.values.toList()
        ..sort((a, b) => b.priority.compareTo(a.priority)),
      fallbackAction: _fallback,
    );
  }

  @override
  Future<void> setFallback(String token, Decision action) async {
    _authenticate(token);
    _fallback = action;
  }

  @override
  Future<Rule> createRule(String token, Map<String, dynamic> draft) async {
    _authenticate(token);
    final rule = Rule.fromJson({
      ...draft,
      'id': 'rule-${_nextId++}',
      'source': RuleSource.console.wire,
    });
    _rules[rule.id] = rule;
    return rule;
  }

  @override
  Future<Rule> updateRule(
    String token,
    String id,
    Map<String, dynamic> draft,
  ) async {
    _authenticate(token);
    final existing = _rules[id];
    if (existing == null) {
      throw ApiException(404, 'not_found', 'Rule not found.');
    }
    final rule = Rule.fromJson({
      ...existing.toJson(),
      ...draft,
      'id': id,
      'source': existing.source.wire,
    });
    _rules[id] = rule;
    return rule;
  }

  @override
  Future<void> setRuleEnabled(String token, String id, bool enabled) async {
    await updateRule(token, id, {'enabled': enabled});
  }

  @override
  Future<void> deleteRule(String token, String id) async {
    _authenticate(token);
    _rules.remove(id);
  }

  @override
  Future<List<Upstream>> providers(String token) async {
    _authenticate(token);
    return _upstreams.values.toList();
  }

  Upstream _saveProvider(String id, Map<String, dynamic> draft) {
    final upstream = Upstream.fromJson({...draft, 'id': id});
    if (upstream.name.trim().isEmpty ||
        _upstreams.values.any(
          (item) => item.id != id && item.name == upstream.name,
        )) {
      throw ApiException(
        409,
        'invalid_provider',
        'Use a unique upstream name.',
      );
    }
    final uri = Uri.tryParse(upstream.baseUrl);
    if (uri == null ||
        !uri.hasAuthority ||
        !['http', 'https'].contains(uri.scheme)) {
      throw ApiException(
        400,
        'invalid_provider',
        'Enter an HTTP or HTTPS URL.',
      );
    }
    _upstreams[id] = upstream;
    return upstream;
  }

  @override
  Future<Upstream> createProvider(
    String token,
    Map<String, dynamic> draft,
  ) async {
    _authenticate(token);
    return _saveProvider('provider-${_nextId++}', draft);
  }

  @override
  Future<Upstream> updateProvider(
    String token,
    String id,
    Map<String, dynamic> draft,
  ) async {
    _authenticate(token);
    final existing = _upstreams[id];
    if (existing == null) {
      throw ApiException(404, 'not_found', 'Upstream not found.');
    }
    return _saveProvider(id, {...existing.toJson(), ...draft});
  }

  @override
  Future<void> deleteProvider(String token, String id) async {
    _authenticate(token);
    _upstreams.remove(id);
  }
}

/// Every view of a sample is derived from its historical occurrences.
class _DemoEntry {
  _DemoEntry({
    required this.id,
    required this.text,
    required this.group,
    required this.sentence,
    required this.format,
    required DateTime time,
    required bool mixed,
  }) : occurrences = [
         for (var i = 0; i < 4; i++)
           ContentOccurrence(
             spanId: '$id-$i',
             entityGroup: group,
             score: 0.98 - i * 0.04,
             action: mixed && i.isOdd ? Decision.release : Decision.redact,
             apiFormat: format,
             path: format.endpoint,
             createdAt: time.subtract(Duration(minutes: i * 27)),
           ),
       ];

  final String id, text, sentence;
  final EntityGroup group;
  final ApiFormat format;
  final List<ContentOccurrence> occurrences;
  String get ruleId => 'decision-$id';

  ContentSummary summary(Decision? registeredAction) {
    final redacted = occurrences
        .where((item) => item.action == Decision.redact)
        .length;
    final scores = occurrences.map((item) => item.score).toList()..sort();
    return ContentSummary(
      anchorSpanId: occurrences.first.spanId,
      originalText: text,
      occurrences: occurrences.length,
      turns: occurrences.length,
      released: occurrences.length - redacted,
      redacted: redacted,
      scoreMin: scores.first,
      scoreMax: scores.last,
      firstSeen: occurrences.last.createdAt,
      lastSeen: occurrences.first.createdAt,
      latestAction: occurrences.first.action,
      categories: [
        ContentCategory(entityGroup: group, count: occurrences.length),
      ],
      registeredAction: registeredAction,
    );
  }

  FragmentDetail fragment(String spanId) {
    final occurrence = occurrences.firstWhere((item) => item.spanId == spanId);
    return FragmentDetail(
      fragment: Fragment(
        id: 'fragment-$spanId',
        requestId: 'request-$spanId',
        apiFormat: format,
        path: format.endpoint,
        originalText: sentence,
        redactedText: occurrence.action == Decision.redact
            ? sentence.replaceAll(text, '[REDACTED]')
            : sentence,
        createdAt: occurrence.createdAt,
      ),
      spans: [
        DetectedSpan(
          id: spanId,
          entityGroup: group,
          score: occurrence.score,
          charLength: text.runes.length,
          originalText: text,
          action: occurrence.action,
        ),
      ],
    );
  }
}
