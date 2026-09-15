/// 弹簧驱动的动画原语。
///
/// 两者都基于 `dart:ui` 的弹簧模拟（`SpringSimulation`），因此**可随时打断**：目标值在
/// 动画途中改变时，新的模拟会继承当前位移与速度，而不是从头再跑一遍。这是 iOS 动效与
/// 「固定时长曲线」的本质区别，也是列表、指示器、视图切换能跟手的原因。
library;

import 'package:flutter/physics.dart';
import 'package:flutter/widgets.dart';

import '../theme/motion.dart';

/// 把一个目标值交给弹簧去追，并在每帧把当前值交给 [builder]。
///
/// 用于「跟随选中项」的一类动画：指示器位置、高亮强度、展开进度。
class SpringValue extends StatefulWidget {
  const SpringValue({
    required this.value,
    required this.builder,
    this.spring,
    super.key,
  });

  /// 目标值。改变它即重定向弹簧。
  final double value;

  /// 弹簧手感；默认取跟随类动画的 [ConsoleMotion.interactive]。
  final SpringDescription? spring;

  /// 当前值。可能略微超出目标（过冲），由调用点决定是否夹取。
  final Widget Function(BuildContext context, double value) builder;

  @override
  State<SpringValue> createState() => _SpringValueState();
}

class _SpringValueState extends State<SpringValue>
    with SingleTickerProviderStateMixin {
  // `unbounded` 是必需的：有界控制器会把过冲裁掉，弹簧的回弹就消失了。
  late final AnimationController _controller = AnimationController.unbounded(
    vsync: this,
    value: widget.value,
  );

  @override
  void didUpdateWidget(SpringValue oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.value != widget.value) {
      _animateTo(widget.value);
    }
  }

  void _animateTo(double target) {
    _controller.animateWith(
      SpringSimulation(
        widget.spring ?? ConsoleMotion.interactive,
        _controller.value,
        target,
        _controller.velocity,
      ),
    );
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: _controller,
      builder: (context, _) => widget.builder(context, _controller.value),
    );
  }
}

/// 分区内容之间的切换：方向感知的交叉淡出 + 位移，由弹簧驱动。
///
/// 切换过程中新旧两份内容同时存在，因此**构造内容时必须带键**（这里用分区序号），
/// 否则框架会按位置把旧内容的 Element 复用给新内容，页面状态会串台。
class SectionSwitcher extends StatefulWidget {
  const SectionSwitcher({
    required this.index,
    required this.builder,
    super.key,
  });

  /// 当前分区。变化即触发一次切换。
  final int index;

  /// 用分区序号构造内容。
  final Widget Function(BuildContext context, int index) builder;

  @override
  State<SectionSwitcher> createState() => _SectionSwitcherState();
}

class _SectionSwitcherState extends State<SectionSwitcher>
    with SingleTickerProviderStateMixin {
  late final AnimationController _progress = AnimationController(
    vsync: this,
    value: 1,
  );

  late int _current = widget.index;
  int? _leaving;

  /// 新内容进入的方向：向左右滑动，取决于分区是往前还是往后翻。
  double _direction = 1;

  @override
  void initState() {
    super.initState();
    // 离场内容在弹簧收敛后从树上摘掉；放在状态回调里，而不是 build 里顺手改状态。
    _progress.addStatusListener((status) {
      if (status == AnimationStatus.completed && _leaving != null) {
        setState(() => _leaving = null);
      }
    });
  }

  @override
  void didUpdateWidget(SectionSwitcher oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.index == _current) return;
    _leaving = _current;
    _direction = widget.index > _current ? 1 : -1;
    _current = widget.index;
    _progress.value = 0;
    _progress.animateWith(SpringSimulation(ConsoleMotion.snappy, 0, 1, 0));
  }

  @override
  void dispose() {
    _progress.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: _progress,
      builder: (context, _) {
        // 弹簧会过冲，透明度必须夹在 [0, 1]，否则会抛断言。
        final t = _progress.value.clamp(0.0, 1.0);
        final leaving = _leaving;

        return ClipRect(
          child: Stack(
            children: [
              if (leaving != null)
                Positioned.fill(
                  child: IgnorePointer(
                    child: ExcludeSemantics(
                      child: _Layer(
                        opacity: 1 - t,
                        offset: -_direction * 12 * t,
                        child: KeyedSubtree(
                          key: ValueKey(leaving),
                          child: widget.builder(context, leaving),
                        ),
                      ),
                    ),
                  ),
                ),
              _Layer(
                opacity: t,
                offset: _direction * 16 * (1 - t),
                child: KeyedSubtree(
                  key: ValueKey(_current),
                  child: widget.builder(context, _current),
                ),
              ),
            ],
          ),
        );
      },
    );
  }
}

/// 一层内容：透明度 + 水平位移。
class _Layer extends StatelessWidget {
  const _Layer({
    required this.opacity,
    required this.offset,
    required this.child,
  });

  final double opacity;
  final double offset;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return Opacity(
      opacity: opacity.clamp(0.0, 1.0),
      child: Transform.translate(offset: Offset(offset, 0), child: child),
    );
  }
}

/// 元素首次挂载时的入场：高度从零展开 + 淡入，由弹簧驱动。
///
/// 只对新出现的元素生效。调用点用它包住带稳定 key 的条目——键相同的元素沿用同一个
/// Element，于是刷新时只有新条目播放入场，已经在列表里的行既不动、也不重播。
///
/// 展开的是**高度**而不是位移：位移不占版面，下面的内容会在同一帧被顶下去，看起来仍然是
/// 一次「刷新了一下布局」。
class EntranceTransition extends StatefulWidget {
  const EntranceTransition({required this.child, super.key});

  final Widget child;

  @override
  State<EntranceTransition> createState() => _EntranceTransitionState();
}

class _EntranceTransitionState extends State<EntranceTransition>
    with SingleTickerProviderStateMixin {
  late final AnimationController _progress = AnimationController.unbounded(
    vsync: this,
    value: 0,
  );

  @override
  void initState() {
    super.initState();
    _progress.animateWith(SpringSimulation(ConsoleMotion.snappy, 0, 1, 0));
  }

  @override
  void dispose() {
    _progress.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: _progress,
      // 弹簧会过冲，而高度不能超过自身：夹取到 [0, 1]，位移与透明度因此始终同步。
      builder: (context, child) {
        final progress = _progress.value.clamp(0.0, 1.0);
        return SizeTransition(
          sizeFactor: AlwaysStoppedAnimation(progress),
          // 从顶边展开：新条目把下面的内容推开，而不是从自己的中心往外撑。
          alignment: Alignment.topCenter,
          child: FadeTransition(
            opacity: AlwaysStoppedAnimation(progress),
            child: child,
          ),
        );
      },
      child: widget.child,
    );
  }
}
