import 'package:cupertino_ui/cupertino_ui.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';

import '../theme/tokens.dart';

/// One navigation action, rendered by the library both in bars and compact layouts.
class DetailNavigationAction extends StatelessWidget {
  const DetailNavigationAction({
    required this.id,
    required this.label,
    required this.icon,
    required this.onPressed,
    super.key,
  });

  final String id;
  final String label;
  final IconData icon;
  final VoidCallback? onPressed;

  GlassBarItem get barItem => GlassBarItem.icon(
    id: id,
    label: label,
    icon: Icon(icon),
    enabled: onPressed != null,
    onTap: onPressed ?? () {},
  );

  @override
  Widget build(BuildContext context) => GlassButtonGroup.icons(
    settings: ConsoleGlass.control(context),
    items: [
      GlassButtonGroupItem(
        icon: Icon(icon),
        label: label,
        enabled: onPressed != null,
        onTap: onPressed ?? () {},
      ),
    ],
  );
}

/// Shared navigation chrome for the detail pane and its pushed pages.
class DetailNavigationBar extends StatelessWidget {
  const DetailNavigationBar({
    required this.title,
    this.controller,
    this.actions = const [],
    this.leading = const [],
    this.canPop = true,
    super.key,
  });

  static const double height = 52;
  final String title;
  final GlassLargeTitleController? controller;
  final List<GlassBarItem> actions;
  final List<GlassBarItem> leading;
  final bool canPop;

  @override
  Widget build(BuildContext context) => SizedBox(
    height: height + MediaQuery.paddingOf(context).top,
    // 仅按钮自身具有玻璃材质；不再铺一块矩形磨砂底板截断共同背景。
    child: GlassAppBar.pinned(
      toolbarHeight: GlassNavPinnedMetrics.toolbarHeight,
      // 使用库的外侧阴影通道，阴影随 pinned chrome 一起过渡。
      buttonSettings: ConsoleGlass.control(context),
      title: Text(title),
      largeTitleController: controller,
      backButton: canPop,
      actions: actions,
      leading: leading,
    ),
  );
}
