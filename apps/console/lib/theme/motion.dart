/// 动效：真实世界的弹簧，而不是固定时长的曲线。
///
/// iOS 的交互动效是**物理弹簧**（质量 / 刚度 / 阻尼），不是 `ease-in-out 300ms`。差别不在
/// 观感而在可打断性：弹簧动画在任何时刻都能被新的目标重定向，并继承当前速度——手指随时
/// 能接管正在运动的元素，滑到边界会橡皮筋回弹。定时曲线做不到这一点，中途改目标只会
/// 从头再跑一遍。
///
/// 这里只声明**用哪一个弹簧**，不重复实现弹簧本身：取值来自组件库的 `GlassSpring`，
/// 它与 Cupertino 的 `Motion` 预设一一对应，改动只需改这一处。
library;

import 'package:flutter/physics.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';

/// 控制台使用的弹簧与时长。
abstract final class ConsoleMotion {
  /// 视图切换、版面变化。轻微过冲，是 iOS 最常见的那个手感。
  static SpringDescription get snappy => GlassSpring.snappy();

  /// 高亮、指示器跟随。回弹更小，因为它一直在被追着跑。
  static SpringDescription get interactive => GlassSpring.interactive();

  /// 纯淡入淡出、不需要位移时用它。
  static SpringDescription get smooth => GlassSpring.smooth();

  /// 交叉淡出的兜底时长；弹簧自己会收敛，这个值只给 `AnimatedSwitcher` 这类定时驱动者。
  static const Duration crossFade = Duration(milliseconds: 220);
}
