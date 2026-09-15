/// 概览页：统计卡、类别分布、时间窗切换。
///
/// 页面标题与说明由外壳渲染，这里只负责内容。
library;

import 'package:cupertino_ui/cupertino_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/models.dart';
import '../../i18n/model_translations.dart';
import '../../i18n/strings.g.dart';
import '../../state/providers.dart';
import '../../theme/palette.dart';
import '../../theme/tokens.dart';
import '../widgets.dart';

/// 统计窗口（小时）。
class StatsWindow extends Notifier<int> {
  @override
  int build() => 24;

  void select(int hours) => state = hours;
}

final statsWindowProvider = NotifierProvider<StatsWindow, int>(StatsWindow.new);

/// 可选的时间窗。集中定义，避免界面与请求各写一份。
const statsWindows = <int>[1, 24, 168, 720];

extension StatsWindowTranslations on int {
  String localizedStatsWindow(Translations text) => switch (this) {
    1 => text.overview.window.oneHour,
    24 => text.overview.window.day,
    168 => text.overview.window.week,
    720 => text.overview.window.month,
    _ => '$this h',
  };
}

class OverviewPage extends ConsumerWidget {
  const OverviewPage({required this.isDesktop, super.key});

  final bool isDesktop;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final hours = ref.watch(statsWindowProvider);
    final statistics = ref.watch(statisticsProvider(hours));

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        // 选择器独占一行：它需要确定宽度，放进 Row 的非弹性位置会拿到无界约束，
        // 组件库对无界宽度求整会抛异常（详见 test/segmented_layout_test.dart）。
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
          loading: () => LoadingState(label: context.t.overview.loading),
          error: (error, _) => MessageState(
            message: context.t.overview.loadFailed(error: error),
            icon: CupertinoIcons.exclamationmark_triangle,
            action: RetryButton(
              onRetry: () => ref.invalidate(statisticsProvider),
            ),
          ),
          data: (value) =>
              _OverviewBody(statistics: value, isDesktop: isDesktop),
        ),
      ],
    );
  }
}

class _OverviewBody extends StatelessWidget {
  const _OverviewBody({required this.statistics, required this.isDesktop});

  final Statistics statistics;
  final bool isDesktop;

  @override
  Widget build(BuildContext context) {
    final palette = ConsolePalette.of(context);
    // 抹去占比是这张页面上唯一需要「看一眼就紧张」的数字，因此只有它上语义色。
    final redactedRatio = statistics.redactedRatio;
    final ratioTint = redactedRatio >= 0.5 ? palette.caution : palette.positive;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        CardRow(
          isDesktop: isDesktop,
          cards: [
            MetricCard(
              title: context.t.overview.requests,
              value: statistics.requests,
              subtitle: context.t.overview.requestsSubtitle,
              emphasis: true,
            ),
            MetricCard(
              title: context.t.overview.fragments,
              value: statistics.fragments,
              subtitle: context.t.overview.fragmentsSubtitle,
            ),
            MetricCard(
              title: context.t.overview.detections,
              value: statistics.spans,
              subtitle: context.t.overview.detectionBreakdown(
                redacted: statistics.redactedSpans,
                released: statistics.releasedSpans,
              ),
            ),
            MetricCard(
              title: context.t.overview.redactedRatio,
              value: redactedRatio * 100,
              suffix: '%',
              fractionDigits: 1,
              subtitle: context.t.overview.redactedRatioSubtitle,
              tint: ratioTint,
            ),
          ],
        ),
        const SizedBox(height: ConsoleSpace.section),
        SectionHeader(
          title: context.t.overview.distribution,
          subtitle: context.t.overview.distributionSubtitle,
        ),
        GroupedList(children: _distribution(context, statistics)),
      ],
    );
  }

  List<Widget> _distribution(BuildContext context, Statistics statistics) {
    final entries = statistics.byEntityGroup;
    if (entries.isEmpty) {
      return [
        ListRow(
          title: context.t.overview.empty,
          subtitle: context.t.overview.emptySubtitle,
        ),
      ];
    }

    final largest = entries
        .map((entry) => entry.count)
        .reduce((a, b) => a > b ? a : b);

    return [
      for (final entry in entries)
        ListRow(
          // 用色点而不是大图标：这一列要能横向扫读，不该抢镜。
          icon: CupertinoIcons.circle_fill,
          iconTint: entry.entityGroup.tint.resolveFrom(context),
          title: entry.entityGroup.localized(context.t),
          subtitleWidget: _Bar(
            fraction: entry.count / largest,
            tint: entry.entityGroup.tint.resolveFrom(context),
          ),
          trailing: Text(
            '${entry.count}',
            style: ConsoleText.value.copyWith(
              color: ConsolePalette.of(context).label,
            ),
          ),
        ),
    ];
  }
}

/// 横向占比条。自绘而非引入图表依赖。
class _Bar extends StatelessWidget {
  const _Bar({required this.fraction, required this.tint});

  final double fraction;
  final Color tint;

  @override
  Widget build(BuildContext context) {
    final palette = ConsolePalette.of(context);

    return ClipRRect(
      borderRadius: BorderRadius.circular(3),
      child: SizedBox(
        height: 5,
        child: Stack(
          children: [
            ColoredBox(color: palette.fill),
            FractionallySizedBox(
              widthFactor: fraction.clamp(0.02, 1.0),
              child: ColoredBox(color: tint),
            ),
          ],
        ),
      ),
    );
  }
}
