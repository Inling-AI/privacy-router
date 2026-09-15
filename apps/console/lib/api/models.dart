/// 控制台 API 的数据类型。
///
/// 这些类型是服务端 JSON 在前端的唯一映射处：界面只消费这里的类型，不直接触碰 `Map`。
library;

/// 命中实体的类别。取值与服务端 `entity_group` 的序列化名称一致。
enum EntityGroup {
  accountNumber('account_number'),
  bankCard('bank_card'),
  creditCode('credit_code'),
  idCard('id_card'),
  ipAddress('ip_address'),
  macAddress('mac_address'),
  privateAddress('private_address'),
  privateDate('private_date'),
  privateEmail('private_email'),
  privatePerson('private_person'),
  privatePhone('private_phone'),
  privateUrl('private_url'),
  secret('secret'),
  unknown('unknown');

  const EntityGroup(this.wire);

  /// 服务端使用的名称。
  final String wire;

  static EntityGroup parse(Object? value) => EntityGroup.values.firstWhere(
    (group) => group.wire == value,
    orElse: () => EntityGroup.unknown,
  );
}

/// 判定动作。
///
/// 命名避开 `Action`：framework 的 widgets 库里有同名类型，`cupertino_ui` 会把它带进来。
enum Decision {
  release('release'),
  redact('redact');

  const Decision(this.wire);

  final String wire;

  static Decision parse(Object? value) =>
      value == 'release' ? Decision.release : Decision.redact;
}

/// 规则来源。只描述规则最初如何产生。
enum RuleSource {
  builtin('builtin'),
  console('console'),

  /// 管理员在内容池逐条登记的决定；方向由规则动作给出。
  ///
  /// 线上取值保持 `approval`：它先于「禁止」出现，是已落库的历史取值。
  operator('approval');

  const RuleSource(this.wire);

  final String wire;

  static RuleSource parse(Object? value) => RuleSource.values.firstWhere(
    (source) => source.wire == value,
    orElse: () => RuleSource.console,
  );
}

/// 文本条件的匹配方式。取值与服务端 `PatternKind` 的序列化名称一致。
enum RulePatternKind {
  exact('exact'),
  substring('substring'),
  glob('glob');

  const RulePatternKind(this.wire);

  final String wire;

  static RulePatternKind parse(Object? value) =>
      RulePatternKind.values.firstWhere(
        (kind) => kind.wire == value,
        orElse: () => RulePatternKind.exact,
      );
}

enum RuleComparison {
  greater('greater'),
  equal('equal'),
  less('less');

  const RuleComparison(this.wire);
  final String wire;

  static RuleComparison parse(Object? value) =>
      RuleComparison.values.firstWhere(
        (operator) => operator.wire == value,
        orElse: () => RuleComparison.equal,
      );
}

enum RuleLogic {
  all('all'),
  any('any');

  const RuleLogic(this.wire);
  final String wire;
}

/// 关键词模式。服务端默认完全匹配且不区分大小写，前端显式保留这两个维度以便编辑。
class RulePattern {
  const RulePattern({
    required this.text,
    required this.kind,
    required this.caseSensitive,
  });

  final String text;
  final RulePatternKind kind;
  final bool caseSensitive;

  factory RulePattern.fromJson(Map<String, dynamic> json) => RulePattern(
    text: json['text'] as String? ?? '',
    kind: RulePatternKind.parse(json['kind']),
    caseSensitive: json['case_sensitive'] as bool? ?? false,
  );

  Map<String, dynamic> toJson() => {
    'text': text,
    'kind': kind.wire,
    'case_sensitive': caseSensitive,
  };
}

sealed class RuleFilter {
  const RuleFilter();

  factory RuleFilter.fromJson(Map<String, dynamic> json) =>
      switch (json['kind']) {
        'entity' => EntityRuleFilter(EntityGroup.parse(json['group'])),
        'keyword' => KeywordRuleFilter(
          RulePattern.fromJson(json['pattern'] as Map<String, dynamic>),
        ),
        'character_length' => CharacterLengthRuleFilter(
          RuleComparison.parse(json['operator']),
          (json['value'] as num).toInt(),
        ),
        'confidence' => ConfidenceRuleFilter(
          RuleComparison.parse(json['operator']),
          (json['value'] as num).toDouble(),
        ),
        _ => throw FormatException('unknown rule filter: ${json['kind']}'),
      };

  Map<String, dynamic> toJson();
}

class EntityRuleFilter extends RuleFilter {
  const EntityRuleFilter(this.group);
  final EntityGroup group;

  @override
  Map<String, dynamic> toJson() => {'kind': 'entity', 'group': group.wire};
}

class KeywordRuleFilter extends RuleFilter {
  const KeywordRuleFilter(this.pattern);
  final RulePattern pattern;

  @override
  Map<String, dynamic> toJson() => {
    'kind': 'keyword',
    'pattern': pattern.toJson(),
  };
}

class CharacterLengthRuleFilter extends RuleFilter {
  const CharacterLengthRuleFilter(this.operator, this.value);
  final RuleComparison operator;
  final int value;

  @override
  Map<String, dynamic> toJson() => {
    'kind': 'character_length',
    'operator': operator.wire,
    'value': value,
  };
}

class ConfidenceRuleFilter extends RuleFilter {
  const ConfidenceRuleFilter(this.operator, this.value);
  final RuleComparison operator;
  final double value;

  @override
  Map<String, dynamic> toJson() => {
    'kind': 'confidence',
    'operator': operator.wire,
    'value': value,
  };
}

sealed class RuleExpression {
  const RuleExpression();

  factory RuleExpression.fromJson(Map<String, dynamic> json) {
    return switch (json['kind']) {
      'filter' => FilterRuleExpression(
        RuleFilter.fromJson(json['filter'] as Map<String, dynamic>),
      ),
      'all' || 'any' => RuleGroupExpression(
        json['kind'] == 'any' ? RuleLogic.any : RuleLogic.all,
        (json['conditions'] as List<dynamic>)
            .map(
              (item) => RuleExpression.fromJson(item as Map<String, dynamic>),
            )
            .toList(),
      ),
      final kind => throw FormatException('unknown rule expression: $kind'),
    };
  }

  Map<String, dynamic> toJson();
}

class RuleGroupExpression extends RuleExpression {
  const RuleGroupExpression(this.logic, this.conditions);
  final RuleLogic logic;
  final List<RuleExpression> conditions;

  @override
  Map<String, dynamic> toJson() => {
    'kind': logic.wire,
    'conditions': [for (final condition in conditions) condition.toJson()],
  };
}

class FilterRuleExpression extends RuleExpression {
  const FilterRuleExpression(this.filter);
  final RuleFilter filter;

  @override
  Map<String, dynamic> toJson() => {
    'kind': 'filter',
    'filter': filter.toJson(),
  };
}

/// 上游协议格式。
enum ApiFormat {
  openaiResponses('openai_responses'),
  openaiChat('openai_chat'),
  anthropicMessages('anthropic_messages');

  const ApiFormat(this.wire);

  final String wire;

  static ApiFormat parse(Object? value) => ApiFormat.values.firstWhere(
    (format) => format.wire == value,
    orElse: () => ApiFormat.openaiChat,
  );

  /// 该协议对应的入站端点，用于界面提示。
  String get endpoint => switch (this) {
    ApiFormat.openaiResponses => '/v1/responses',
    ApiFormat.openaiChat => '/v1/chat/completions',
    ApiFormat.anthropicMessages => '/v1/messages',
  };
}

/// 内容池的复核过滤：聚合之后哪些行值得人看。
///
/// 复核的对象是**还没被人决定过**的内容：一行内容有没有被登记过，就是它有没有被审过。
/// 判定稳不稳只影响排序——摇摆得越厉害越该先看——它不是「审没审过」的定义。
enum ContentFilter {
  /// 还没有登记过决定的内容。默认视图。
  unreviewed('unreviewed'),

  /// 已经登记过决定的内容。
  reviewed('reviewed'),

  /// 全部有命中的内容。
  all('all');

  const ContentFilter(this.wire);

  final String wire;
}

/// 内容池的一行：同一段内容在池子里的判定统计。
///
/// 聚合身份是原文本身——列表里的一行与一次登记必须是同一个东西。类别不进身份，它只是
/// 这一行为什么摇摆的证据。
class ContentSummary {
  const ContentSummary({
    required this.anchorSpanId,
    required this.originalText,
    required this.occurrences,
    required this.turns,
    required this.released,
    required this.redacted,
    required this.scoreMin,
    required this.scoreMax,
    required this.firstSeen,
    required this.lastSeen,
    required this.latestAction,
    required this.categories,
    required this.registeredAction,
  });

  /// 最近一次出现的命中。取出现列表与登记都用它做锚点，URL 里不放被审查的文本本身。
  final String anchorSpanId;
  final String originalText;

  /// 出现次数，以及它分布在多少个 turn 上。
  final int occurrences;
  final int turns;
  final int released;
  final int redacted;

  /// 置信度区间：它直接解释这段内容为什么摇摆。
  final double scoreMin;
  final double scoreMax;
  final DateTime firstSeen;
  final DateTime lastSeen;
  final Decision latestAction;

  /// 这段内容被识别成过哪些类别、各多少次；次数最多的排在最前。
  final List<ContentCategory> categories;

  /// 管理员已经为这段内容登记的方向；`null` 表示尚未登记。
  final Decision? registeredAction;

  /// 判定在放行与抹去之间摇摆过——这就是需要人来定夺的东西。
  bool get isUnstable => released > 0 && redacted > 0;

  factory ContentSummary.fromJson(Map<String, dynamic> json) => ContentSummary(
    anchorSpanId: json['anchor_span_id'] as String,
    originalText: json['original_text'] as String,
    occurrences: _int(json['occurrences']),
    turns: _int(json['turns']),
    released: _int(json['released']),
    redacted: _int(json['redacted']),
    scoreMin: (json['score_min'] as num?)?.toDouble() ?? 0,
    scoreMax: (json['score_max'] as num?)?.toDouble() ?? 0,
    firstSeen: _timestamp(json['first_seen']),
    lastSeen: _timestamp(json['last_seen']),
    latestAction: Decision.parse(json['latest_action']),
    categories: (json['categories'] as List<dynamic>? ?? const [])
        .map((item) => ContentCategory.fromJson(item as Map<String, dynamic>))
        .toList(),
    registeredAction: switch (json['registered_action']) {
      final String wire => Decision.parse(wire),
      _ => null,
    },
  );
}

/// 一段内容在一个类别上的出现次数。
class ContentCategory {
  const ContentCategory({required this.entityGroup, required this.count});

  final EntityGroup entityGroup;
  final int count;

  factory ContentCategory.fromJson(Map<String, dynamic> json) =>
      ContentCategory(
        entityGroup: EntityGroup.parse(json['entity_group']),
        count: _int(json['count']),
      );
}

/// 内容池分页。
class ContentPage {
  const ContentPage({required this.total, required this.items});

  final int total;
  final List<ContentSummary> items;

  factory ContentPage.fromJson(Map<String, dynamic> json) => ContentPage(
    total: _int(json['total']),
    items: (json['items'] as List<dynamic>)
        .map((item) => ContentSummary.fromJson(item as Map<String, dynamic>))
        .toList(),
  );
}

/// 同一段内容的某一次出现。带上来源与判定，但不带整段 turn 原文。
class ContentOccurrence {
  const ContentOccurrence({
    required this.spanId,
    required this.entityGroup,
    required this.score,
    required this.action,
    required this.apiFormat,
    required this.path,
    required this.createdAt,
    this.matchedRuleId,
  });

  final String spanId;
  final EntityGroup entityGroup;
  final double score;
  final Decision action;
  final ApiFormat apiFormat;
  final String path;
  final DateTime createdAt;
  final String? matchedRuleId;

  factory ContentOccurrence.fromJson(Map<String, dynamic> json) =>
      ContentOccurrence(
        spanId: json['span_id'] as String,
        entityGroup: EntityGroup.parse(json['entity_group']),
        score: (json['score'] as num?)?.toDouble() ?? 0,
        action: Decision.parse(json['action']),
        apiFormat: ApiFormat.parse(json['api_format']),
        path: json['path'] as String? ?? '',
        createdAt: _timestamp(json['created_at']),
        matchedRuleId: json['matched_rule_id'] as String?,
      );
}

/// 一次 turn 的原文与转发结果。
///
/// 被隐藏的内容就是两者之差：原文里不再出现在转发文本中的部分。查看一次判定只需要这一份
/// 读取，界面不再自己拼片段与命中。
class Fragment {
  const Fragment({
    required this.id,
    required this.requestId,
    required this.apiFormat,
    required this.path,
    required this.originalText,
    required this.redactedText,
    required this.createdAt,
  });

  final String id;
  final String requestId;
  final ApiFormat apiFormat;
  final String path;
  final String originalText;
  final String redactedText;
  final DateTime createdAt;

  factory Fragment.fromJson(Map<String, dynamic> json) => Fragment(
    id: json['id'] as String,
    requestId: json['request_id'] as String? ?? '',
    apiFormat: ApiFormat.parse(json['api_format']),
    path: json['path'] as String? ?? '',
    originalText: json['original_text'] as String? ?? '',
    redactedText: json['redacted_text'] as String? ?? '',
    createdAt: _timestamp(json['created_at']),
  );
}

/// 一次 turn 里的一条命中。
class DetectedSpan {
  const DetectedSpan({
    required this.id,
    required this.entityGroup,
    required this.score,
    required this.charLength,
    required this.originalText,
    required this.action,
    this.matchedRuleId,
  });

  final String id;
  final EntityGroup entityGroup;
  final double score;
  final int charLength;
  final String originalText;
  final Decision action;
  final String? matchedRuleId;

  factory DetectedSpan.fromJson(Map<String, dynamic> json) => DetectedSpan(
    id: json['id'] as String,
    entityGroup: EntityGroup.parse(json['entity_group']),
    score: (json['score'] as num?)?.toDouble() ?? 0,
    charLength: _int(json['char_len']),
    originalText: json['original_text'] as String? ?? '',
    action: Decision.parse(json['action']),
    matchedRuleId: json['matched_rule_id'] as String?,
  );
}

/// 一次命中所在的 turn：原文、转发出去的内容，以及这次的全部命中。
class FragmentDetail {
  const FragmentDetail({required this.fragment, required this.spans});

  final Fragment fragment;
  final List<DetectedSpan> spans;

  factory FragmentDetail.fromJson(Map<String, dynamic> json) => FragmentDetail(
    fragment: Fragment.fromJson(json['fragment'] as Map<String, dynamic>),
    spans: (json['spans'] as List<dynamic>? ?? const [])
        .map((item) => DetectedSpan.fromJson(item as Map<String, dynamic>))
        .toList(),
  );
}

/// 一条判定规则。
class Rule {
  const Rule({
    required this.id,
    required this.name,
    required this.priority,
    required this.action,
    required this.enabled,
    required this.source,
    required this.condition,
  });

  final String id;
  final String name;
  final int priority;
  final Decision action;
  final bool enabled;
  final RuleSource source;
  final RuleExpression condition;

  Map<String, dynamic> toJson() => {
    'id': id,
    'name': name,
    'priority': priority,
    'action': action.wire,
    'enabled': enabled,
    'source': source.wire,
    'condition': condition.toJson(),
  };

  factory Rule.fromJson(Map<String, dynamic> json) {
    return Rule(
      id: json['id'] as String,
      name: json['name'] as String,
      priority: (json['priority'] as num).toInt(),
      action: Decision.parse(json['action']),
      enabled: json['enabled'] as bool? ?? true,
      source: RuleSource.parse(json['source']),
      condition: RuleExpression.fromJson(
        json['condition'] as Map<String, dynamic>,
      ),
    );
  }
}

/// 规则列表与兜底动作。
class RuleList {
  const RuleList({required this.items, required this.fallbackAction});

  final List<Rule> items;
  final Decision fallbackAction;

  factory RuleList.fromJson(Map<String, dynamic> json) => RuleList(
    items: (json['items'] as List<dynamic>)
        .map((item) => Rule.fromJson(item as Map<String, dynamic>))
        .toList(),
    fallbackAction: Decision.parse(json['fallback_action']),
  );
}

/// 上游 provider。
///
/// 命名避开 `Provider`：riverpod 的 `Provider` 会被各页面导入，同名会造成歧义。
class Upstream {
  const Upstream({
    required this.id,
    required this.name,
    required this.baseUrl,
    required this.apiFormat,
    required this.enabled,
  });

  final String id;
  final String name;
  final String baseUrl;
  final ApiFormat apiFormat;
  final bool enabled;

  factory Upstream.fromJson(Map<String, dynamic> json) => Upstream(
    id: json['id'] as String,
    name: json['name'] as String,
    baseUrl: json['base_url'] as String,
    apiFormat: ApiFormat.parse(json['api_format']),
    enabled: json['enabled'] as bool? ?? true,
  );

  Map<String, dynamic> toJson() => {
    'name': name,
    'base_url': baseUrl,
    'api_format': apiFormat.wire,
    'enabled': enabled,
  };
}

/// 某一类别命中的条数。
class EntityGroupCount {
  const EntityGroupCount({required this.entityGroup, required this.count});

  final EntityGroup entityGroup;
  final int count;

  factory EntityGroupCount.fromJson(Map<String, dynamic> json) =>
      EntityGroupCount(
        entityGroup: EntityGroup.parse(json['entity_group']),
        count: (json['count'] as num).toInt(),
      );
}

/// 请求耗时分位，单位毫秒。
class LatencySummary {
  const LatencySummary({
    required this.averageMs,
    required this.p50Ms,
    required this.p95Ms,
    required this.maxMs,
    required this.averageInferenceMs,
    required this.averageUpstreamMs,
  });

  final int averageMs;
  final int p50Ms;
  final int p95Ms;
  final int maxMs;
  final int averageInferenceMs;
  final int averageUpstreamMs;

  factory LatencySummary.fromJson(Map<String, dynamic> json) => LatencySummary(
    averageMs: _int(json['average_ms']),
    p50Ms: _int(json['p50_ms']),
    p95Ms: _int(json['p95_ms']),
    maxMs: _int(json['max_ms']),
    averageInferenceMs: _int(json['average_inference_ms']),
    averageUpstreamMs: _int(json['average_upstream_ms']),
  );
}

/// 概览统计。所有比率都由这里的计数导出，界面不再各算一遍。
class Statistics {
  const Statistics({
    required this.requests,
    required this.fragments,
    required this.spans,
    required this.releasedSpans,
    required this.redactedSpans,
    required this.cachedFragments,
    required this.inferredFragments,
    required this.latency,
    required this.byEntityGroup,
    this.model = const ModelSummary(),
  });

  final int requests;
  final int fragments;
  final int spans;
  final int releasedSpans;
  final int redactedSpans;
  final int cachedFragments;
  final int inferredFragments;
  final LatencySummary latency;
  final ModelSummary model;
  final List<EntityGroupCount> byEntityGroup;

  /// 被抹去的占比；没有命中时返回 0 而不是 NaN。
  double get redactedRatio => spans == 0 ? 0 : redactedSpans / spans;

  /// 缓存命中率：命中片段占全部片段的比例。
  double get cacheHitRatio {
    final total = cachedFragments + inferredFragments;
    return total == 0 ? 0 : cachedFragments / total;
  }

  factory Statistics.fromJson(Map<String, dynamic> json) => Statistics(
    model: ModelSummary.fromJson(
      json['model'] as Map<String, dynamic>? ?? const {},
    ),
    requests: _int(json['requests']),
    fragments: _int(json['fragments']),
    spans: _int(json['spans']),
    releasedSpans: _int(json['released_spans']),
    redactedSpans: _int(json['redacted_spans']),
    cachedFragments: _int(json['cached_fragments']),
    inferredFragments: _int(json['inferred_fragments']),
    latency: LatencySummary.fromJson(
      json['latency'] as Map<String, dynamic>? ?? const {},
    ),
    byEntityGroup: (json['by_entity_group'] as List<dynamic>? ?? const [])
        .map((item) => EntityGroupCount.fromJson(item as Map<String, dynamic>))
        .toList(),
  );
}

/// 放行结果。
class ModelSummary {
  const ModelSummary({
    this.tokens = 0,
    this.throughput,
    this.firstResultMs,
    this.tokenizationMs,
    this.validationMs,
    this.forwardMs,
    this.decodingMs,
  });
  final int tokens;
  final double? throughput,
      firstResultMs,
      tokenizationMs,
      validationMs,
      forwardMs,
      decodingMs;
  factory ModelSummary.fromJson(Map<String, dynamic> json) => ModelSummary(
    tokens: _int(json['tokens']),
    throughput: (json['tokens_per_second'] as num?)?.toDouble(),
    firstResultMs: (json['average_first_result_ms'] as num?)?.toDouble(),
    tokenizationMs: (json['average_tokenization_ms'] as num?)?.toDouble(),
    validationMs: (json['average_validation_ms'] as num?)?.toDouble(),
    forwardMs: (json['average_forward_ms'] as num?)?.toDouble(),
    decodingMs: (json['average_decoding_ms'] as num?)?.toDouble(),
  );
}

class SpanDecisionOutcome {
  const SpanDecisionOutcome({required this.ruleId, required this.changed});

  final String ruleId;

  /// 为 false 表示这段内容此前已登记过同一方向，这一次没有改动任何东西。
  final bool changed;

  factory SpanDecisionOutcome.fromJson(Map<String, dynamic> json) =>
      SpanDecisionOutcome(
        ruleId: json['rule_id'] as String,
        changed: json['changed'] as bool? ?? false,
      );
}

/// 服务端签发的一次控制台会话。明文令牌只在这一刻返回，之后无法再取回。
class IssuedSession {
  const IssuedSession({required this.token, required this.expiresAt});

  final String token;
  final int expiresAt;

  factory IssuedSession.fromJson(Map<String, dynamic> json) => IssuedSession(
    token: json['token'] as String,
    expiresAt: _int(json['expires_at']),
  );
}

/// 健康检查。
class Health {
  const Health({
    required this.status,
    required this.rules,
    required this.providers,
    required this.adminConfigured,
  });

  final String status;
  final int rules;
  final int providers;
  final bool adminConfigured;

  factory Health.fromJson(Map<String, dynamic> json) => Health(
    status: json['status'] as String? ?? 'unknown',
    rules: _int(json['rules']),
    providers: _int(json['providers']),
    adminConfigured: json['admin_configured'] as bool? ?? false,
  );
}

/// 服务端以 unix 毫秒整数存储时间，避免驱动层的类型集成。
DateTime _timestamp(Object? value) =>
    DateTime.fromMillisecondsSinceEpoch(value is num ? value.toInt() : 0);

int _int(Object? value) => value is num ? value.toInt() : 0;
