/// 锁定一次真实发生的崩溃，以及本项目为它加的护栏。
///
/// `Row` 给非 `Expanded` 子项**无界宽度**。组件库会对自身宽度做像素对齐（`ceil()` /
/// `roundToDouble()`），对 `double.infinity` 求整会抛
/// `Unsupported operation: Infinity or NaN toInt`；该异常**每帧重复抛出**，
/// 表现为控制台刷屏（当时一两分钟两千条）且页面无法使用。
///
/// 因此契约是：**这个控件必须有确定宽度**。
library;

import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';
import 'package:privacy_router_console/ui/widgets.dart';

Widget wrap(Widget child) => Directionality(
  textDirection: TextDirection.ltr,
  child: MediaQuery(
    data: const MediaQueryData(size: Size(800, 600)),
    child: Center(child: child),
  ),
);

Widget control() => GlassSegmentedControl(
  segments: const [
    GlassSegment(label: '一天'),
    GlassSegment(label: '一周'),
  ],
  selectedIndex: 0,
  onSegmentSelected: (_) {},
);

/// 收集 `body` 期间抛出的全部异常文本，并把它们从测试框架的待处理队列里清掉。
Future<List<String>> captureErrors(
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
  // 清空挂起异常，避免框架把「已捕获」再次判为失败。
  while (tester.takeException() != null) {}
  return messages;
}

void main() {
  testWidgets('组件库在无界宽度下会抛错（当时刷屏的根因）', (tester) async {
    final errors = await captureErrors(tester, () async {
      await tester.pumpWidget(
        wrap(
          Row(
            children: [
              const Expanded(child: SizedBox()),
              control(),
            ],
          ),
        ),
      );
      await tester.pump();
    });

    expect(errors, isNotEmpty, reason: '无界宽度必须表现为渲染异常');
    // 约束里出现 Infinity 是该故障的特征。
    expect(
      errors.any((m) => m.contains('Infinity')),
      isTrue,
      reason: '异常应当来自无界约束，实际：$errors',
    );
  });

  testWidgets('给出确定宽度后组件库正常布局', (tester) async {
    final errors = await captureErrors(tester, () async {
      await tester.pumpWidget(wrap(SizedBox(width: 320, child: control())));
      await tester.pump(const Duration(milliseconds: 50));
    });

    expect(errors, isEmpty);
    expect(find.byType(GlassSegmentedControl), findsOneWidget);
  });

  testWidgets('护栏：SegmentedPicker 在无界位置给出指明原因的信息', (tester) async {
    final errors = await captureErrors(tester, () async {
      await tester.pumpWidget(
        wrap(
          Row(
            children: [
              const Expanded(child: SizedBox()),
              SegmentedPicker<int>(
                values: const [1, 2],
                labels: const ['一天', '一周'],
                selected: 1,
                onChanged: (_) {},
              ),
            ],
          ),
        ),
      );
      await tester.pump();
    });

    expect(
      errors.any((m) => m.contains('SegmentedPicker requires a finite width')),
      isTrue,
      reason: '应当给出指明原因的信息，实际：$errors',
    );
    // 护栏生效时不应再渲染出组件库控件，因此不会每帧刷屏。
    expect(find.byType(GlassSegmentedControl), findsNothing);
  });

  testWidgets('护栏：SegmentedPicker 在有界宽度下正常绘制', (tester) async {
    final errors = await captureErrors(tester, () async {
      await tester.pumpWidget(
        wrap(
          SizedBox(
            width: 360,
            child: SegmentedPicker<int>(
              values: const [1, 2],
              labels: const ['一天', '一周'],
              selected: 1,
              onChanged: (_) {},
            ),
          ),
        ),
      );
      await tester.pump(const Duration(milliseconds: 50));
    });

    expect(errors, isEmpty);
  });
}
