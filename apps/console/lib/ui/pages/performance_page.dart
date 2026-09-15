/// 性能页：推理与转发的耗时分解、结果缓存命中率。
library;

import 'package:cupertino_ui/cupertino_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/models.dart';
import '../../i18n/strings.g.dart';
import '../../state/providers.dart';
import '../../theme/palette.dart';
import '../../theme/tokens.dart';
import '../widgets.dart';
import 'overview_page.dart'
    show StatsWindowTranslations, statsWindowProvider, statsWindows;

class PerformancePage extends ConsumerWidget {
  const PerformancePage({required this.isDesktop, super.key});

  final bool isDesktop;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final hours = ref.watch(statsWindowProvider);
    final statistics = ref.watch(statisticsProvider(hours));

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        // 选择器和概览页共用同一个时间窗：它们是同一段时间的两种看法，各选各的会让人对不上号。
        SegmentedPicker<int>(
          values: statsWindows,
          labels: [
            for (final value in statsWindows)
              value.localizedStatsWindow(context.t),
          ],
          selected: hours,
          onChanged: (value) =>
              ref.read(statsWindowProvider.notifier).select(value),
        ),
        const SizedBox(height: ConsoleSpace.xl),
        statistics.when(
          skipError: true,
          loading: () => LoadingState(label: context.t.performance.loading),
          error: (error, _) => MessageState(
            message: context.t.performance.loadFailed(error: error),
            action: RetryButton(
              onRetry: () => ref.invalidate(statisticsProvider),
            ),
          ),
          data: (value) =>
              _PerformanceBody(statistics: value, isDesktop: isDesktop),
        ),
      ],
    );
  }
}

class _PerformanceBody extends StatelessWidget {
  const _PerformanceBody({required this.statistics, required this.isDesktop});

  final Statistics statistics;
  final bool isDesktop;

  @override
  Widget build(BuildContext context) {
    final latency = statistics.latency;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        CardRow(
          isDesktop: isDesktop,
          cards: [
            MetricCard(
              title: context.t.performance.firstResult,
              value: statistics.model.firstResultMs,
              suffix: ' ms',
              fractionDigits: 1,
              subtitle: context.t.performance.firstResultSubtitle,
            ),
            MetricCard(
              title: context.t.performance.throughput,
              value: statistics.model.throughput,
              suffix: ' tokens/s',
              fractionDigits: 1,
              subtitle: context.t.performance.throughputSubtitle,
              emphasis: true,
            ),
            MetricCard(
              title: context.t.performance.modelTokens,
              value: statistics.model.tokens,
              subtitle: context.t.performance.modelTokensSubtitle,
            ),
          ],
        ),
        const SizedBox(height: ConsoleSpace.section),
        GroupedList(
          children: [
            for (final item in [
              (
                context.t.performance.tokenization,
                statistics.model.tokenizationMs,
              ),
              (context.t.performance.validation, statistics.model.validationMs),
              (context.t.performance.forward, statistics.model.forwardMs),
              (context.t.performance.decoding, statistics.model.decodingMs),
            ])
              ListRow(
                icon: CupertinoIcons.timer,
                title: item.$1,
                trailing: item.$2 == null
                    ? const Text('--')
                    : MetricValue(
                        value: item.$2!,
                        fractionDigits: 1,
                        suffix: ' ms',
                      ),
              ),
          ],
        ),
        const SizedBox(height: ConsoleSpace.section),
        CardRow(
          isDesktop: isDesktop,
          cards: [
            MetricCard(
              title: context.t.performance.average,
              value: latency.averageMs,
              suffix: ' ms',
              subtitle: context.t.performance.averageSubtitle,
              emphasis: true,
            ),
            MetricCard(
              title: 'P50 / P95',
              value: latency.p50Ms,
              secondaryValue: latency.p95Ms,
              suffix: ' ms',
              subtitle: context.t.performance.percentileSubtitle,
            ),
            MetricCard(
              title: context.t.performance.maximum,
              value: latency.maxMs,
              suffix: ' ms',
              subtitle: context.t.performance.maximumSubtitle,
            ),
          ],
        ),
        const SizedBox(height: ConsoleSpace.section),
        SectionHeader(title: context.t.performance.breakdown),
        GroupedList(
          children: [
            ListRow(
              icon: CupertinoIcons.timer,
              title: context.t.performance.inference,
              subtitle: context.t.performance.inferenceSubtitle,
              trailing: MetricValue(
                value: latency.averageInferenceMs,
                suffix: ' ms',
                style: ConsoleText.value.copyWith(
                  color: ConsolePalette.of(context).label,
                ),
              ),
            ),
            ListRow(
              icon: CupertinoIcons.arrow_up_circle,
              title: context.t.performance.upstream,
              subtitle: context.t.performance.upstreamSubtitle,
              trailing: MetricValue(
                value: latency.averageUpstreamMs,
                suffix: ' ms',
                style: ConsoleText.value.copyWith(
                  color: ConsolePalette.of(context).label,
                ),
              ),
            ),
            ListRow(
              icon: CupertinoIcons.bolt_fill,
              title: context.t.performance.cacheHit,
              subtitle: context.t.performance.cacheBreakdown(
                cached: statistics.cachedFragments,
                inferred: statistics.inferredFragments,
              ),
              trailing: MetricValue(
                value: statistics.cacheHitRatio * 100,
                fractionDigits: 1,
                suffix: '%',
                style: ConsoleText.value.copyWith(
                  color: ConsolePalette.of(context).label,
                ),
              ),
            ),
          ],
        ),
        const SizedBox(height: ConsoleSpace.l),
        InfoNotice(text: context.t.performance.note),
      ],
    );
  }
}
