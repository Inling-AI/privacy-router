/// 内容池：复核还没被人决定过的内容。
///
/// 复核的对象不是某一次请求，而是同一段内容本身的判定史。判定带概率，同一段文本在不同
/// 上下文里会一次被放行、一次被抹去；按原文聚合成一行后，摇摆得越厉害的行排得越前，但它
/// 不是「该不该看」的定义——登记过决定的行才离开待审列表。登记一次即对这段内容的每一次
/// 出现生效，因此这里没有「逐条审核某个 turn」的入口。
///
/// 出现记录与具体到某一次 turn 的原文都推入独立页面，和规则页共用同一条导航。
///
/// 逐行用普通容器而非玻璃：上游明确写着玻璃属于导航与浮层，滚动内容里的重复行用玻璃
/// 既拖慢滚动，也会让放行按钮「玻璃套玻璃」而退化。
library;

import 'package:cupertino_ui/cupertino_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/models.dart';
import '../../i18n/strings.g.dart';
import '../../state/providers.dart';
import '../../theme/elevation.dart';
import '../../theme/palette.dart';
import '../../theme/tokens.dart';
import '../motion.dart';
import '../pool_format.dart';
import '../widgets.dart';
import 'content_detail_page.dart';

class PoolPage extends ConsumerStatefulWidget {
  const PoolPage({super.key});

  @override
  ConsumerState<PoolPage> createState() => _PoolPageState();
}

class _PoolPageState extends ConsumerState<PoolPage> {
  ContentQuery _query = const ContentQuery();

  void _move(int offset) => setState(() => _query = _query.at(offset));

  void _filter(ContentFilter filter) =>
      setState(() => _query = _query.filtered(filter));

  @override
  Widget build(BuildContext context) {
    final pool = context.t.pool;
    final page = ref.watch(contentProvider(_query));
    final value = page.asData?.value;
    // 首屏数据到达之前不显示「空」：那会把「还没拉到」说成「本来就没有」。
    final settled = !page.isLoading || value != null;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Wrap(
          spacing: ConsoleSpace.s,
          runSpacing: ConsoleSpace.s,
          crossAxisAlignment: WrapCrossAlignment.center,
          children: [
            ActionButton(
              text: pool.previous,
              onPressed: !page.isLoading && _query.offset > 0
                  ? () => _move(
                      (_query.offset - _query.limit).clamp(0, _query.offset),
                    )
                  : null,
            ),
            ActionButton(
              text: pool.next,
              onPressed:
                  !page.isLoading &&
                      value != null &&
                      _query.offset + _query.limit < value.total
                  ? () => _move(_query.offset + _query.limit)
                  : null,
            ),
            ActionButton(
              text: pool.refresh,
              icon: CupertinoIcons.refresh,
              onPressed: page.isLoading
                  ? null
                  : () => ref.invalidate(contentProvider(_query)),
            ),
            SegmentedPicker<ContentFilter>(
              values: ContentFilter.values,
              labels: [
                for (final filter in ContentFilter.values)
                  switch (filter) {
                    ContentFilter.unreviewed => pool.filterUnreviewed,
                    ContentFilter.reviewed => pool.filterReviewed,
                    ContentFilter.all => pool.filterAll,
                  },
              ],
              selected: _query.filter,
              onChanged: _filter,
            ),
            if (value != null)
              Muted(
                text: pool.range(
                  start: value.items.isEmpty ? 0 : _query.offset + 1,
                  end: _query.offset + value.items.length,
                  total: value.total,
                ),
              ),
          ],
        ),
        const SizedBox(height: ConsoleSpace.l),
        page.when(
          skipError: settled,
          loading: () => LoadingState(label: pool.loading),
          error: (error, _) => MessageState(
            message: pool.loadFailed(error: error),
            icon: CupertinoIcons.exclamationmark_triangle,
            action: RetryButton(
              onRetry: () => ref.invalidate(contentProvider(_query)),
            ),
          ),
          data: (value) => value.items.isEmpty
              ? MessageState(
                  message: _query.offset > 0
                      ? pool.emptyPage
                      : switch (_query.filter) {
                          ContentFilter.unreviewed => pool.emptyUnreviewed,
                          ContentFilter.reviewed => pool.emptyReviewed,
                          ContentFilter.all => pool.empty,
                        },
                  icon: CupertinoIcons.tray,
                )
              : Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    _ContentList(page: value),
                    const SizedBox(height: ConsoleSpace.l),
                    ListNote(text: pool.footer(count: value.total)),
                  ],
                ),
        ),
      ],
    );
  }
}

class _ContentList extends ConsumerWidget {
  const _ContentList({required this.page});

  final ContentPage page;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        for (final summary in page.items)
          // 键是这段内容本身：自动刷新时只有新出现的内容会播放入场，已在列表里的行
          // 沿用原来的 Element，不会因为多了一次出现就重播一遍。
          EntranceTransition(
            key: ValueKey(summary.originalText),
            child: _ContentRow(
              summary: summary,
              onInspect: () => presentContentDetail(context, summary),
            ),
          ),
      ],
    );
  }
}

/// 内容池的一行：一段内容 + 它的判定统计 + 两个方向的登记入口。
class _ContentRow extends StatelessWidget {
  const _ContentRow({required this.summary, required this.onInspect});

  final ContentSummary summary;
  final VoidCallback onInspect;

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
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Icon(
                  summary.isUnstable
                      ? CupertinoIcons.shield_lefthalf_fill
                      : CupertinoIcons.checkmark_shield,
                  size: 16,
                  color: summary.isUnstable
                      ? palette.caution
                      : palette.positive,
                ),
                const SizedBox(width: ConsoleSpace.s),
                Expanded(
                  child: Text(
                    preview(summary.originalText),
                    style: ConsoleText.mono.copyWith(color: palette.label),
                  ),
                ),
              ],
            ),
            const SizedBox(height: ConsoleSpace.s),
            Wrap(
              spacing: ConsoleSpace.s,
              runSpacing: ConsoleSpace.s,
              crossAxisAlignment: WrapCrossAlignment.center,
              children: [
                Muted(text: pool.categories),
                for (final category in summary.categories)
                  _CategoryChip(category: category),
                Muted(
                  text: pool.occurrences(
                    count: summary.occurrences,
                    turns: summary.turns,
                  ),
                ),
              ],
            ),
            const SizedBox(height: ConsoleSpace.s),
            Wrap(
              spacing: ConsoleSpace.m,
              runSpacing: ConsoleSpace.s,
              children: [
                Muted(
                  text: pool.decisions(
                    redacted: summary.redacted,
                    released: summary.released,
                  ),
                ),
                Muted(text: confidenceLabel(context.t, summary)),
                Muted(text: pool.lastSeen(time: formatTime(summary.lastSeen))),
              ],
            ),
            const SizedBox(height: ConsoleSpace.m),
            Wrap(
              spacing: ConsoleSpace.s,
              runSpacing: ConsoleSpace.s,
              crossAxisAlignment: WrapCrossAlignment.center,
              children: [
                ContentDecisionButtons(
                  spanId: summary.anchorSpanId,
                  registered: summary.registeredAction,
                ),
                ActionButton(
                  text: pool.occurrenceCount,
                  icon: CupertinoIcons.clock,
                  onPressed: onInspect,
                ),
              ],
            ),
          ],
        ),
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
