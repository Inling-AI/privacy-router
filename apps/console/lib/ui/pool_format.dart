import '../../api/models.dart';
import '../../i18n/strings.g.dart';

/// 内容池各视图共用的展示格式。
///
/// 列表、内容详情与 turn 详情必须写出同一种百分比、同一种截断、同一种时间；任何一处
/// 想要「稍微不一样」，都应该先改这里，而不是在页面里再写一份。

/// 概率的展示格式。
String percent(double score) => '${(score * 100).toStringAsFixed(1)}%';

/// 列表里的一行预览：超过长度就截断，截断规则只在这里定义一次。
String preview(String text) =>
    text.length <= 80 ? text : '${text.substring(0, 80)}…';

/// 服务端给出的时间戳是 unix 毫秒。
String formatTime(DateTime time) {
  String pad(int value) => value.toString().padLeft(2, '0');
  return '${pad(time.month)}-${pad(time.day)} '
      '${pad(time.hour)}:${pad(time.minute)}:${pad(time.second)}';
}

/// 置信度区间的文案：只有真的跨过一段区间时才写成区间。
String confidenceLabel(Translations text, ContentSummary summary) =>
    summary.scoreMin == summary.scoreMax
    ? text.pool.confidence(value: percent(summary.scoreMin))
    : text.pool.confidenceRange(
        min: percent(summary.scoreMin),
        max: percent(summary.scoreMax),
      );
