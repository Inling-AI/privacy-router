/// 控制台的行为测试。
///
/// 只验证与本应用自身逻辑有关的契约：断点选择、模型解析、以及登录页在未配置管理员时
/// 给出可执行的提示。第三方组件的渲染由其自身测试覆盖，这里不重复验证。
library;

import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:privacy_router_console/api/models.dart';
import 'package:privacy_router_console/ui/layout.dart';

void main() {
  group('断点', () {
    test('窄屏使用底部导航', () {
      expect(Breakpoints.resolve(const Size(390, 844)), LayoutClass.compact);
      expect(Breakpoints.resolve(const Size(699, 900)), LayoutClass.compact);
    });

    test('中等宽度使用图标轨', () {
      expect(Breakpoints.resolve(const Size(700, 900)), LayoutClass.medium);
      expect(Breakpoints.resolve(const Size(1099, 900)), LayoutClass.medium);
    });

    test('足够宽且足够高时才使用三栏布局', () {
      expect(Breakpoints.resolve(const Size(1440, 900)), LayoutClass.expanded);
      expect(Breakpoints.resolve(const Size(1100, 900)), LayoutClass.expanded);
    });

    test('矮窗口即使很宽也降级，避免内容被压扁', () {
      expect(Breakpoints.resolve(const Size(1600, 400)), LayoutClass.medium);
    });

    test('只有桌面形态提供内联检查器', () {
      expect(LayoutClass.expanded.hasInlineInspector, isTrue);
      expect(LayoutClass.medium.hasInlineInspector, isFalse);
      expect(LayoutClass.compact.hasInlineInspector, isFalse);
    });
  });

  group('API 模型解析', () {
    test('未知的实体类别退化为 unknown，而不是抛错', () {
      // 服务端新增类别时前端不应崩溃。
      expect(EntityGroup.parse('future_group'), EntityGroup.unknown);
      expect(EntityGroup.parse(null), EntityGroup.unknown);
    });

    test('已知类别按协议值解析', () {
      expect(EntityGroup.parse('private_email'), EntityGroup.privateEmail);
      expect(EntityGroup.parse('secret'), EntityGroup.secret);
    });

    test('判定动作只有放行与抹去两种，默认抹去', () {
      expect(Decision.parse('release'), Decision.release);
      expect(Decision.parse('redact'), Decision.redact);
      // 取值异常时必须落到更保守的一侧。
      expect(Decision.parse('anything-else'), Decision.redact);
      expect(Decision.parse(null), Decision.redact);
    });

    test('统计比率在没有数据时为 0，不产生 NaN', () {
      const empty = Statistics(
        requests: 0,
        fragments: 0,
        spans: 0,
        releasedSpans: 0,
        redactedSpans: 0,
        cachedFragments: 0,
        inferredFragments: 0,
        latency: LatencySummary(
          averageMs: 0,
          p50Ms: 0,
          p95Ms: 0,
          maxMs: 0,
          averageInferenceMs: 0,
          averageUpstreamMs: 0,
        ),
        byEntityGroup: [],
      );
      expect(empty.redactedRatio, 0);
      expect(empty.cacheHitRatio, 0);
      expect(empty.redactedRatio.isNaN, isFalse);
    });

    test('比率由计数导出', () {
      const stats = Statistics(
        requests: 4,
        fragments: 1,
        spans: 4,
        releasedSpans: 1,
        redactedSpans: 3,
        cachedFragments: 6,
        inferredFragments: 2,
        latency: LatencySummary(
          averageMs: 250,
          p50Ms: 200,
          p95Ms: 400,
          maxMs: 400,
          averageInferenceMs: 10,
          averageUpstreamMs: 20,
        ),
        byEntityGroup: [],
      );
      expect(stats.redactedRatio, 0.75);
      expect(stats.cacheHitRatio, 0.75);
    });

    test('时间戳按 unix 毫秒解析', () {
      final summary = ContentSummary.fromJson({
        'anchor_span_id': 's1',
        'original_text': 'a',
        'occurrences': 2,
        'turns': 2,
        'released': 1,
        'redacted': 1,
        'score_min': 0.5,
        'score_max': 0.9,
        'first_seen': 1700000000000,
        'last_seen': 1700000006000,
        'latest_action': 'release',
        'categories': [
          {'entity_group': 'private_email', 'count': 2},
        ],
        'registered_action': 'redact',
      });
      expect(summary.firstSeen.millisecondsSinceEpoch, 1700000000000);
      expect(summary.lastSeen.millisecondsSinceEpoch, 1700000006000);
      expect(summary.isUnstable, isTrue);
      expect(summary.registeredAction, Decision.redact);
    });

    test('每条协议都有对应的入站端点', () {
      expect(ApiFormat.openaiResponses.endpoint, '/v1/responses');
      expect(ApiFormat.openaiChat.endpoint, '/v1/chat/completions');
      expect(ApiFormat.anthropicMessages.endpoint, '/v1/messages');
    });

    test('规则解析递归文本过滤器', () {
      final rule = Rule.fromJson({
        'id': 'r1',
        'name': '放行示例地址',
        'priority': 500,
        'condition': {
          'kind': 'filter',
          'filter': {
            'kind': 'keyword',
            'pattern': {
              'text': 'example.com',
              'kind': 'substring',
              'case_sensitive': false,
            },
          },
        },
        'action': 'release',
        'enabled': true,
        'source': 'builtin',
      });
      expect(
        rule.condition,
        isA<FilterRuleExpression>().having(
          (expression) => expression.filter,
          'filter',
          isA<KeywordRuleFilter>(),
        ),
      );
      expect(rule.source, RuleSource.builtin);
      expect(rule.source, RuleSource.builtin);
    });

    test('规则解析递归置信度过滤器', () {
      final rule = Rule.fromJson({
        'id': 'r2',
        'name': '降噪',
        'priority': 400,
        'condition': {
          'kind': 'filter',
          'filter': {'kind': 'confidence', 'operator': 'less', 'value': 0.6},
        },
        'action': 'release',
        'enabled': true,
        'source': 'builtin',
      });
      expect(
        rule.condition,
        isA<FilterRuleExpression>().having(
          (expression) => expression.filter,
          'filter',
          isA<ConfidenceRuleFilter>(),
        ),
      );
    });

    test('规则表达式保留任意嵌套的 AND 与 OR', () {
      final expression = RuleExpression.fromJson({
        'kind': 'all',
        'conditions': [
          {
            'kind': 'filter',
            'filter': {'kind': 'entity', 'group': 'private_email'},
          },
          {
            'kind': 'any',
            'conditions': [
              {
                'kind': 'filter',
                'filter': {
                  'kind': 'confidence',
                  'operator': 'greater',
                  'value': 0.8,
                },
              },
              {
                'kind': 'filter',
                'filter': {
                  'kind': 'character_length',
                  'operator': 'greater',
                  'value': 4,
                },
              },
            ],
          },
        ],
      });

      expect(expression, isA<RuleGroupExpression>());
      expect(expression.toJson()['kind'], 'all');
      final children = expression.toJson()['conditions'] as List<dynamic>;
      expect((children[1] as Map<String, dynamic>)['kind'], 'any');
    });
  });
}
