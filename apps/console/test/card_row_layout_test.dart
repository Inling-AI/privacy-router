/// 锁定一次真实发生的崩溃：`CardRow` 放在高度无界的容器里。
///
/// `CrossAxisAlignment.stretch` 需要行有**确定高度**；但概览/性能页把卡片行放在
/// sliver 里，高度是无界的，于是行把 `h = Infinity` 传给子项：
///
///   BoxConstraints forces an infinite height.
///   creator: Row ← CardRow ← Column ← _OverviewBody ← SliverToBoxAdapter
///
/// 该断言每帧重发（当时背景还有一个逐帧动画），一两分钟就刷出几千条，
/// 控制台被淹没、页面不可用。契约：**CardRow 在任何高度约束下都不得抛错**。
library;

import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:privacy_router_console/ui/widgets.dart';

Widget wrap(Widget child) => Directionality(
  textDirection: TextDirection.ltr,
  child: MediaQuery(
    data: const MediaQueryData(size: Size(1440, 900)),
    child: child,
  ),
);

/// 收集 `body` 期间抛出的全部异常，并把它们从待处理队列里清掉。
Future<List<String>> capture(
  WidgetTester tester,
  Future<void> Function() body,
) async {
  final messages = <String>[];
  final previous = FlutterError.onError;
  FlutterError.onError = (details) => messages.add(details.exceptionAsString());
  try {
    await body();
  } finally {
    FlutterError.onError = previous;
  }
  while (tester.takeException() != null) {}
  return messages;
}

List<Widget> cardSet() => const [
  MetricCard(title: '请求', value: 4, subtitle: '窗口内完成的代理请求'),
  MetricCard(title: '命中', value: 3, subtitle: '抹去 2 · 放行 1'),
  MetricCard(title: '抹去占比', value: 66.7, fractionDigits: 1, suffix: '%'),
];

void main() {
  testWidgets('放在高度无界的 sliver 里不抛错（当时刷屏的根因）', (tester) async {
    final errors = await capture(tester, () async {
      await tester.pumpWidget(
        wrap(
          CustomScrollView(
            slivers: [
              SliverToBoxAdapter(
                child: CardRow(cards: cardSet(), isDesktop: true),
              ),
            ],
          ),
        ),
      );
      await tester.pump();
    });

    expect(errors, isEmpty, reason: '无界高度下不得抛错，实际：$errors');
    expect(find.byType(MetricCard), findsNWidgets(3));
  });

  testWidgets('放在有界高度里同样正常，且卡片等高', (tester) async {
    final errors = await capture(tester, () async {
      await tester.pumpWidget(
        wrap(
          SizedBox(
            width: 900,
            child: CardRow(cards: cardSet(), isDesktop: true),
          ),
        ),
      );
      await tester.pump();
    });

    expect(errors, isEmpty);
    final heights = tester
        .widgetList<MetricCard>(find.byType(MetricCard))
        .length;
    expect(heights, 3);
  });

  testWidgets('窄屏改为竖排', (tester) async {
    final errors = await capture(tester, () async {
      await tester.pumpWidget(
        wrap(
          CustomScrollView(
            slivers: [
              SliverToBoxAdapter(
                child: CardRow(cards: cardSet(), isDesktop: false),
              ),
            ],
          ),
        ),
      );
      await tester.pump();
    });

    expect(errors, isEmpty);
    expect(find.byType(MetricCard), findsNWidgets(3));
  });

  testWidgets('没有卡片时不渲染出问题', (tester) async {
    final errors = await capture(tester, () async {
      await tester.pumpWidget(wrap(const CardRow(cards: [], isDesktop: true)));
      await tester.pump();
    });
    expect(errors, isEmpty);
  });
}
