import 'package:cupertino_ui/cupertino_ui.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';

import '../i18n/strings.g.dart';
import '../theme/palette.dart';
import '../theme/tokens.dart';
import 'detail_navigation_bar.dart';

/// 详情页的主操作：`id` 是稳定的动作标识，`onSave` 是动作本身。
///
/// 只读的详情页不给这个动作，导航栏因此不出现任何按钮——「有没有可保存的东西」由页面
/// 自己回答，布局只有这一份。
typedef DetailSave = ({String id, VoidCallback onSave});

/// 规则、上游与内容池共用的详情布局。
class DetailPage extends StatelessWidget {
  const DetailPage({
    required this.title,
    required this.child,
    this.save,
    this.busy = false,
    super.key,
  });

  final String title;
  final Widget child;
  final DetailSave? save;
  final bool busy;

  @override
  Widget build(BuildContext context) {
    final palette = ConsolePalette.of(context);
    return GlassScaffold(
      backgroundColor: palette.canvas,
      background: ColoredBox(color: palette.canvas),
      body: SafeArea(
        child: Stack(
          children: [
            Positioned.fill(
              child: SingleChildScrollView(
                padding: const EdgeInsets.fromLTRB(
                  32,
                  DetailNavigationBar.height + 24,
                  32,
                  40,
                ),
                child: Align(
                  alignment: Alignment.topCenter,
                  child: ConstrainedBox(
                    constraints: const BoxConstraints(
                      maxWidth: ConsoleMetrics.sheetMaxWidth,
                    ),
                    child: AbsorbPointer(absorbing: busy, child: child),
                  ),
                ),
              ),
            ),
            Positioned(
              top: 0,
              left: 0,
              right: 0,
              child: DetailNavigationBar(
                title: title,
                canPop: !busy,
                actions: [
                  if (save case final save?)
                    DetailNavigationAction(
                      id: save.id,
                      onPressed: busy ? null : save.onSave,
                      label: context.t.common.save,
                      icon: CupertinoIcons.checkmark,
                    ).barItem,
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}
