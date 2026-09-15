import 'package:cupertino_ui/cupertino_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../i18n/strings.g.dart';
import '../state/providers.dart';
import '../theme/palette.dart';
import '../theme/tokens.dart';

/// Visible on every route so sample data cannot be mistaken for a live proxy.
class DemoBanner extends ConsumerWidget {
  const DemoBanner({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final palette = ConsolePalette.of(context);
    return ColoredBox(
      color: palette.surface,
      child: SafeArea(
        bottom: false,
        child: Padding(
          padding: const EdgeInsets.symmetric(
            horizontal: ConsoleSpace.l,
            vertical: ConsoleSpace.s,
          ),
          child: Row(
            children: [
              Icon(CupertinoIcons.play_circle, size: 20, color: palette.tint),
              const SizedBox(width: ConsoleSpace.s),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Text(
                      context.t.demo.banner,
                      style: ConsoleText.subheadline.copyWith(
                        color: palette.label,
                      ),
                    ),
                    Text(
                      context.t.demo.detail,
                      style: ConsoleText.caption.copyWith(
                        color: palette.secondaryLabel,
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(width: ConsoleSpace.s),
              CupertinoButton(
                padding: const EdgeInsets.symmetric(horizontal: ConsoleSpace.s),
                onPressed: () => ref.invalidate(apiClientProvider),
                child: Text(context.t.demo.reset),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
