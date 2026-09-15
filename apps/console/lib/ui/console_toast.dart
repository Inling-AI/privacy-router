import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';

import '../theme/tokens.dart';
import '../theme/palette.dart';

/// Keep the library toast surface, but release the overlay's tight width.
abstract final class ConsoleToast {
  static VoidCallback? _dismissCurrent;

  static VoidCallback show(
    BuildContext context, {
    required String message,
    Widget? icon,
    GlassToastType type = GlassToastType.success,
    GlassToastPosition position = GlassToastPosition.top,
    Duration duration = const Duration(seconds: 4),
  }) {
    _dismissCurrent?.call();
    final overlay = Overlay.of(context, rootOverlay: true);
    late OverlayEntry entry;
    bool removed = false;
    void dismiss() {
      if (removed) return;
      removed = true;
      entry.remove();
      entry.dispose();
      if (_dismissCurrent == dismiss) _dismissCurrent = null;
    }

    entry = OverlayEntry(
      builder: (context) => _ToastOverlay(
        message: message,
        icon: icon,
        type: type,
        position: position,
        duration: duration,
        onDismiss: dismiss,
      ),
    );
    _dismissCurrent = dismiss;
    overlay.insert(entry);
    return dismiss;
  }
}

class _ToastOverlay extends StatefulWidget {
  const _ToastOverlay({
    required this.message,
    required this.icon,
    required this.type,
    required this.position,
    required this.duration,
    required this.onDismiss,
  });
  final String message;
  final Widget? icon;
  final GlassToastType type;
  final GlassToastPosition position;
  final Duration duration;
  final VoidCallback onDismiss;

  @override
  State<_ToastOverlay> createState() => _ToastOverlayState();
}

class _ToastOverlayState extends State<_ToastOverlay> {
  Timer? _timer;
  @override
  void initState() {
    super.initState();
    if (widget.duration > Duration.zero) {
      _timer = Timer(widget.duration, widget.onDismiss);
    }
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => Positioned.fill(
    child: SafeArea(
      minimum: const EdgeInsets.all(16),
      child: Padding(
        padding: EdgeInsets.only(
          top: widget.position == GlassToastPosition.top ? 56 : 0,
          bottom: MediaQuery.viewInsetsOf(context).bottom,
        ),
        child: Align(
          alignment: switch (widget.position) {
            GlassToastPosition.top => Alignment.topCenter,
            GlassToastPosition.center => Alignment.center,
            GlassToastPosition.bottom => Alignment.bottomCenter,
          },
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 400),
            child: Dismissible(
              key: const ValueKey('console-toast'),
              direction: DismissDirection.horizontal,
              onDismissed: (_) => widget.onDismiss(),
              // The library's 70% fill lets underlying navigation text compete
              // with the message. Back the surface before rendering its glass.
              child: DecoratedBox(
                decoration: BoxDecoration(
                  color: ConsolePalette.of(context).surface
                      .withValues(alpha: 0.98),
                  borderRadius: BorderRadius.circular(24),
                ),
                child: GlassToast(
                  message: widget.message,
                  icon: widget.icon,
                  type: widget.type,
                  position: widget.position,
                  settings: ConsoleGlass.inspector(context),
                  quality: ConsoleGlass.quality,
                ),
              ),
            ),
          ),
        ),
      ),
    ),
  );
}
