/// 内容池的两级详情：一行内容的全部出现，以及一次出现所属的 turn。
///
/// 详情一律用和规则页同一条导航推入独立页面，而不是占用外壳的侧栏：窄屏与桌面因此在
/// 同一处展开内容，跳转动作也只有一个来源。
library;

import 'package:cupertino_ui/cupertino_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';

import '../../api/models.dart';
import '../../i18n/model_translations.dart';
import '../../i18n/strings.g.dart';
import '../../state/providers.dart';
import '../../theme/elevation.dart';
import '../../theme/palette.dart';
import '../../theme/tokens.dart';
import '../console_toast.dart';
import '../detail_page.dart';
import '../pool_format.dart';
import '../widgets.dart';

/// 打开一行内容的详情页。
Future<void> presentContentDetail(
  BuildContext context,
  ContentSummary summary,
) => Navigator.of(context).push<void>(
  CupertinoPageRoute(builder: (_) => ContentDetailPage(summary: summary)),
);

/// 打开一次出现所属的 turn。
Future<void> presentTurnDetail(BuildContext context, String spanId) =>
    Navigator.of(context).push<void>(
      CupertinoPageRoute(builder: (_) => TurnDetailPage(spanId: spanId)),
    );

/// 一行内容的详情：聚合统计、登记入口与全部出现。
class ContentDetailPage extends ConsumerStatefulWidget {
  const ContentDetailPage({required this.summary, super.key});

  final ContentSummary summary;

  @override
  ConsumerState<ContentDetailPage> createState() => _ContentDetailPageState();
}

class _ContentDetailPageState extends ConsumerState<ContentDetailPage> {
  /// 这次打开页面之后登记的方向。
  ///
  /// 页面拿的是进入那一刻的行快照，列表会在登记后刷新，但这一页不会；本地记下方向，
  /// 页面上的「已登记」就不会与刚点下的按钮对不上。
  Decision? _registered;

  Decision? get _decision =>
      _registered ?? widget.summary.registeredAction;

  @override
  Widget build(BuildContext context) {
    final summary = widget.summary;
    final pool = context.t.pool;
    final palette = ConsolePalette.of(context);
    final occurrences = ref.watch(occurrencesProvider(summary.anchorSpanId));

    return DetailPage(
      title: pool.contentTitle,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        mainAxisSize: MainAxisSize.min,
        children: [
          SelectableValue(text: summary.originalText, color: palette.label),
          const SizedBox(height: ConsoleSpace.m),
          Wrap(
            spacing: ConsoleSpace.s,
            runSpacing: ConsoleSpace.s,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              Muted(text: pool.categories),
              for (final category in summary.categories)
                _CategoryChip(category: category),
            ],
          ),
          const SizedBox(height: ConsoleSpace.m),
          DetailRow(
            label: pool.occurrencesLabel,
            value: pool.occurrences(
              count: summary.occurrences,
              turns: summary.turns,
            ),
          ),
          DetailRow(
            label: pool.decisionsLabel,
            value: pool.decisions(
              redacted: summary.redacted,
              released: summary.released,
            ),
          ),
          DetailRow(
            label: pool.confidenceLabel,
            value: confidenceLabel(context.t, summary),
          ),
          DetailRow(
            label: pool.firstSeenLabel,
            value: formatTime(summary.firstSeen),
          ),
          DetailRow(
            label: pool.lastSeenLabel,
            value: formatTime(summary.lastSeen),
          ),
          DetailRow(
            label: pool.registered,
            value: switch (_decision) {
              Decision.release => pool.release,
              Decision.redact => pool.deny,
              null => pool.registeredNone,
            },
          ),
          const SizedBox(height: ConsoleSpace.m),
          ContentDecisionButtons(
            spanId: summary.anchorSpanId,
            registered: _decision,
            onDecided: (decision) => setState(() => _registered = decision),
          ),
          const SizedBox(height: ConsoleSpace.m),
          InfoNotice(text: pool.detailHint, icon: CupertinoIcons.info_circle),
          const SizedBox(height: ConsoleSpace.l),
          SectionHeader(title: pool.occurrenceCount),
          occurrences.when(
            loading: () => const LoadingState(),
            error: (error, _) => MessageState(
              message: pool.occurrenceFailed(error: error),
              action: RetryButton(
                onRetry: () => ref.invalidate(
                  occurrencesProvider(summary.anchorSpanId),
                ),
              ),
            ),
            data: (items) => items.isEmpty
                ? InfoNotice(text: pool.noOccurrences, icon: CupertinoIcons.tray)
                : Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      for (final item in items) _OccurrenceCard(occurrence: item),
                    ],
                  ),
          ),
        ],
      ),
    );
  }
}

/// 一段内容的两个方向：放行与禁止。已登记的方向不再是可选项。
///
/// 列表行与内容详情页登记的是同一件事，因此按钮、忙碌状态与提示语只有这一份实现——
/// 两处的文案与「点下去会发生什么」不会漂移。
class ContentDecisionButtons extends ConsumerStatefulWidget {
  const ContentDecisionButtons({
    required this.spanId,
    required this.registered,
    this.onDecided,
    super.key,
  });

  final String spanId;

  /// 已经登记的方向；为 null 表示还没登记过。
  final Decision? registered;

  /// 登记成功后回调，页面据此更新自己的「已登记」。
  final ValueChanged<Decision>? onDecided;

  @override
  ConsumerState<ContentDecisionButtons> createState() =>
      _ContentDecisionButtonsState();
}

class _ContentDecisionButtonsState
    extends ConsumerState<ContentDecisionButtons> {
  bool _busy = false;

  Future<void> _decide(Decision decision) async {
    final pool = context.t.pool;
    setState(() => _busy = true);
    try {
      final outcome = await ref.read(spanDecisionProvider)(
        widget.spanId,
        decision,
      );
      if (!mounted) return;
      final (message, icon) = switch (decision) {
        Decision.release => (
          outcome.changed ? pool.releasedCreated : pool.releasedExisting,
          CupertinoIcons.check_mark_circled,
        ),
        Decision.redact => (
          outcome.changed ? pool.deniedCreated : pool.deniedExisting,
          CupertinoIcons.xmark_circle,
        ),
      };
      widget.onDecided?.call(decision);
      ConsoleToast.show(
        context,
        message: message,
        icon: Icon(icon),
        position: GlassToastPosition.top,
      );
    } on Exception catch (error) {
      if (!mounted) return;
      ConsoleToast.show(
        context,
        message: switch (decision) {
          Decision.release => pool.releaseFailed(error: error),
          Decision.redact => pool.denyFailed(error: error),
        },
        type: GlassToastType.error,
        position: GlassToastPosition.top,
      );
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final pool = context.t.pool;
    final registered = widget.registered;

    return Wrap(
      spacing: ConsoleSpace.s,
      runSpacing: ConsoleSpace.s,
      crossAxisAlignment: WrapCrossAlignment.center,
      children: [
        for (final decision in Decision.values)
          ActionButton(
            text: decision == registered
                ? switch (decision) {
                    Decision.release => pool.registeredRelease,
                    Decision.redact => pool.registeredRedact,
                  }
                : switch (decision) {
                    Decision.release => pool.release,
                    Decision.redact => pool.deny,
                  },
            icon: switch (decision) {
              Decision.release => CupertinoIcons.checkmark,
              Decision.redact => CupertinoIcons.xmark_circle,
            },
            // 已经登记过的方向不再是可选项：这段内容现在就按它处理。
            onPressed: _busy || decision == registered
                ? null
                : () => _decide(decision),
          ),
        if (_busy) const GlassProgressIndicator.circular(size: 18),
      ],
    );
  }
}

/// 一次出现所属的 turn：进模型前的原文、转发出去的文本，以及这次的全部命中。
class TurnDetailPage extends ConsumerWidget {
  const TurnDetailPage({required this.spanId, super.key});

  final String spanId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final pool = context.t.pool;
    final detail = ref.watch(fragmentProvider(spanId));

    return DetailPage(
      title: pool.turnTitle,
      child: detail.when(
        loading: () => const LoadingState(),
        error: (error, _) => MessageState(
          message: pool.detailFailed(error: error),
          action: RetryButton(
            onRetry: () => ref.invalidate(fragmentProvider(spanId)),
          ),
        ),
        data: (value) => Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          mainAxisSize: MainAxisSize.min,
          children: [
            _TextBlock(label: pool.original, text: value.fragment.originalText),
            const SizedBox(height: ConsoleSpace.m),
            _TextBlock(
              label: pool.forwarded,
              text: value.fragment.redactedText,
              highlight:
                  value.fragment.redactedText != value.fragment.originalText,
            ),
            const SizedBox(height: ConsoleSpace.l),
            DetailRow(
              label: pool.protocol,
              value: value.fragment.apiFormat.localized(context.t),
            ),
            DetailRow(
              label: pool.time,
              value: formatTime(value.fragment.createdAt),
            ),
            const SizedBox(height: ConsoleSpace.l),
            SectionHeader(title: pool.detectedSpans),
            if (value.spans.isEmpty)
              InfoNotice(
                text: pool.noDetections,
                icon: CupertinoIcons.checkmark_shield,
              )
            else
              for (final span in value.spans) _DetectedSpanCard(span: span),
          ],
        ),
      ),
    );
  }
}

/// 一段内容的一次出现。点进去看这一次的原文、转发内容与全部命中。
class _OccurrenceCard extends StatelessWidget {
  const _OccurrenceCard({required this.occurrence});

  final ContentOccurrence occurrence;

  @override
  Widget build(BuildContext context) {
    final pool = context.t.pool;
    final palette = ConsolePalette.of(context);
    final released = occurrence.action == Decision.release;

    return Padding(
      padding: const EdgeInsets.only(bottom: ConsoleSpace.s),
      child: ElevatedSurface(
        elevation: ConsoleElevation.flat,
        color: palette.surface,
        radius: ConsoleRadius.control,
        padding: const EdgeInsets.all(ConsoleSpace.m),
        child: MouseRegion(
          cursor: SystemMouseCursors.click,
          child: GestureDetector(
            behavior: HitTestBehavior.opaque,
            onTap: () => presentTurnDetail(context, occurrence.spanId),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Wrap(
                  spacing: ConsoleSpace.s,
                  runSpacing: ConsoleSpace.s,
                  crossAxisAlignment: WrapCrossAlignment.center,
                  children: [
                    Text(
                      occurrence.action.localized(context.t),
                      style: ConsoleText.caption.copyWith(
                        fontWeight: FontWeight.w600,
                        color: released ? palette.positive : palette.caution,
                      ),
                    ),
                    EntityChip(group: occurrence.entityGroup),
                    Muted(text: percent(occurrence.score)),
                    Muted(text: formatTime(occurrence.createdAt)),
                    Icon(
                      CupertinoIcons.chevron_forward,
                      size: 13,
                      color: palette.secondaryLabel,
                    ),
                  ],
                ),
                const SizedBox(height: ConsoleSpace.s),
                Muted(
                  text: pool.occurrenceSource(
                    protocol: occurrence.apiFormat.localized(context.t),
                    path: occurrence.path,
                  ),
                ),
                if (occurrence.matchedRuleId case final ruleId?) ...[
                  const SizedBox(height: ConsoleSpace.xs),
                  Muted(text: pool.matchedRule(id: ruleId)),
                ],
              ],
            ),
          ),
        ),
      ),
    );
  }
}

/// turn 里的一条命中。复核看的是这条记录，因此这里给出识别到的原文本身。
class _DetectedSpanCard extends StatelessWidget {
  const _DetectedSpanCard({required this.span});

  final DetectedSpan span;

  @override
  Widget build(BuildContext context) {
    final pool = context.t.pool;
    final palette = ConsolePalette.of(context);

    return Padding(
      padding: const EdgeInsets.only(bottom: ConsoleSpace.m),
      child: ElevatedSurface(
        elevation: ConsoleElevation.flat,
        color: palette.surface,
        radius: ConsoleRadius.control,
        padding: const EdgeInsets.all(ConsoleSpace.m + 2),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Wrap(
              spacing: ConsoleSpace.s,
              runSpacing: ConsoleSpace.s,
              crossAxisAlignment: WrapCrossAlignment.center,
              children: [
                EntityChip(group: span.entityGroup),
                Muted(text: percent(span.score)),
                Muted(text: pool.characters(count: span.charLength)),
                Text(
                  span.action.localized(context.t),
                  style: ConsoleText.caption.copyWith(
                    fontWeight: FontWeight.w600,
                    color: span.action == Decision.release
                        ? palette.positive
                        : palette.caution,
                  ),
                ),
              ],
            ),
            const SizedBox(height: ConsoleSpace.m),
            SelectableValue(text: span.originalText, color: palette.label),
            if (span.matchedRuleId case final ruleId?) ...[
              const SizedBox(height: ConsoleSpace.s),
              Muted(text: pool.matchedRule(id: ruleId)),
            ],
          ],
        ),
      ),
    );
  }
}

/// 一段文本：原文与转发出去的内容共用同一种块。
class _TextBlock extends StatelessWidget {
  const _TextBlock({
    required this.label,
    required this.text,
    this.highlight = false,
  });

  final String label;
  final String text;
  final bool highlight;

  @override
  Widget build(BuildContext context) {
    final palette = ConsolePalette.of(context);

    return ElevatedSurface(
      elevation: ConsoleElevation.flat,
      color: palette.surface,
      radius: ConsoleRadius.control,
      padding: const EdgeInsets.all(ConsoleSpace.m + 2),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              if (highlight) ...[
                Icon(
                  CupertinoIcons.shield_lefthalf_fill,
                  size: 15,
                  color: palette.caution,
                ),
                const SizedBox(width: ConsoleSpace.s),
              ],
              Expanded(
                child: Text(
                  label,
                  style: ConsoleText.caption.copyWith(
                    fontWeight: FontWeight.w600,
                    color: palette.secondaryLabel,
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: ConsoleSpace.s),
          SelectableValue(text: text, height: 1.5, color: palette.label),
        ],
      ),
    );
  }
}

/// 一个类别在这段内容上出现过多少次。类别不进聚合身份，所以次数是它唯一的权重。
class _CategoryChip extends StatelessWidget {
  const _CategoryChip({required this.category});

  final ContentCategory category;

  @override
  Widget build(BuildContext context) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        EntityChip(group: category.entityGroup),
        const SizedBox(width: ConsoleSpace.xs),
        Muted(text: '×${category.count}'),
      ],
    );
  }
}
