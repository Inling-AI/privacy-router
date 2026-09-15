/// 上游页：增删改 provider，设置 base url、协议与认证方式。
///
/// 上游只配置转发目标和协议，客户端凭证原样透传。
///
/// 标题与说明由外壳渲染（见 `sections.dart`），主操作由外壳摆进工具栏（见外壳的
/// `_appBarActions`）；本页只负责列表。
library;

import 'package:cupertino_ui/cupertino_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';

import '../../api/models.dart';
import '../../i18n/model_translations.dart';
import '../../i18n/strings.g.dart';
import '../../state/providers.dart';
import '../../theme/palette.dart';
import '../../theme/tokens.dart';
import '../widgets.dart';
import '../console_toast.dart';
import '../detail_page.dart';
import '../detail_navigation_bar.dart';

class ProvidersPage extends ConsumerWidget {
  const ProvidersPage({required this.isDesktop, super.key});

  final bool isDesktop;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final upstreams = ref.watch(upstreamsProvider);

    return upstreams.when(
      loading: () => LoadingState(label: context.t.providers.loading),
      error: (error, _) => MessageState(
        message: context.t.providers.loadFailed(error: error),
        action: RetryButton(onRetry: () => ref.invalidate(upstreamsProvider)),
      ),
      data: (items) => Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (items.isEmpty)
            MessageState(
              message: context.t.providers.empty,
              icon: CupertinoIcons.antenna_radiowaves_left_right,
            )
          else ...[
            GroupedList(
              children: [
                for (final upstream in items) _UpstreamRow(upstream: upstream),
              ],
            ),
            // 注释跟在列表之后，且只在真有列表时出现：空状态已经说明了下步该做什么，
            // 再堆一段选路规则只会把那句话淹掉。
            const SizedBox(height: ConsoleSpace.l),
            ListNote(text: context.t.providers.routingNote),
          ],
        ],
      ),
    );
  }
}

/// 工具栏上的主操作：添加上游。
///
/// 与 `NewRuleButton` 同形：外壳负责摆放，本文件负责定义它打开的面板。
class NewUpstreamButton extends StatelessWidget {
  const NewUpstreamButton({super.key});

  static GlassBarItem barItem(BuildContext context) => DetailNavigationAction(
    id: 'upstream-primary',
    label: context.t.providers.add,
    icon: CupertinoIcons.add,
    onPressed: () => presentUpstreamEditor(context, null),
  ).barItem;

  @override
  Widget build(BuildContext context) {
    return ActionButton(
      text: context.t.providers.add,
      icon: CupertinoIcons.add,
      prominent: false,
      height: ConsoleMetrics.barControl,
      onPressed: () => presentUpstreamEditor(context, null),
    );
  }
}

class _UpstreamRow extends ConsumerStatefulWidget {
  const _UpstreamRow({required this.upstream});

  final Upstream upstream;

  @override
  ConsumerState<_UpstreamRow> createState() => _UpstreamRowState();
}

class _UpstreamRowState extends ConsumerState<_UpstreamRow> {
  bool _busy = false;

  Future<void> _delete() async {
    setState(() => _busy = true);
    try {
      await ref.read(upstreamActionsProvider).delete(widget.upstream.id);
    } on Exception catch (error) {
      if (!mounted) return;
      ConsoleToast.show(
        context,
        message: context.t.common.deleteFailed(error: error),
        type: GlassToastType.error,
        position: GlassToastPosition.top,
      );
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final palette = ConsolePalette.of(context);
    final upstream = widget.upstream;

    return ListRow(
      icon: upstream.enabled
          ? CupertinoIcons.checkmark_circle_fill
          : CupertinoIcons.pause_circle,
      iconTint: upstream.enabled ? palette.positive : palette.tertiaryLabel,
      title: upstream.name,
      onTap: () => presentUpstreamEditor(context, upstream),
      subtitle:
          '${upstream.apiFormat.localized(context.t)} · ${upstream.baseUrl}',
      trailing: _busy
          ? const GlassProgressIndicator.circular(size: 18)
          : Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                CupertinoButton(
                  padding: EdgeInsets.zero,
                  minimumSize: const Size(32, 32),
                  onPressed: () => presentUpstreamEditor(context, upstream),
                  child: Icon(
                    CupertinoIcons.pencil,
                    size: 18,
                    color: palette.tint,
                  ),
                ),
                CupertinoButton(
                  padding: EdgeInsets.zero,
                  minimumSize: const Size(32, 32),
                  onPressed: _delete,
                  child: Icon(
                    CupertinoIcons.trash,
                    size: 18,
                    color: palette.negative,
                  ),
                ),
              ],
            ),
    );
  }
}

/// 打开上游编辑器。
Future<void> presentUpstreamEditor(
  BuildContext context,
  Upstream? existing,
) async {
  await Navigator.of(context).push<void>(
    CupertinoPageRoute(builder: (_) => _UpstreamEditor(existing: existing)),
  );
}

class _UpstreamEditor extends ConsumerStatefulWidget {
  const _UpstreamEditor({this.existing});

  final Upstream? existing;

  @override
  ConsumerState<_UpstreamEditor> createState() => _UpstreamEditorState();
}

class _UpstreamEditorState extends ConsumerState<_UpstreamEditor> {
  late final _name = TextEditingController(text: widget.existing?.name ?? '');
  late final _baseUrl = TextEditingController(
    text: widget.existing?.baseUrl ?? '',
  );
  late ApiFormat _format = widget.existing?.apiFormat ?? ApiFormat.openaiChat;
  late bool _enabled = widget.existing?.enabled ?? true;
  bool _busy = false;

  @override
  void dispose() {
    _name.dispose();
    _baseUrl.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    final name = _name.text.trim();
    final baseUrl = _baseUrl.text.trim();
    if (name.isEmpty || baseUrl.isEmpty) {
      ConsoleToast.show(
        context,
        message: context.t.providers.required,
        type: GlassToastType.warning,
        position: GlassToastPosition.top,
      );
      return;
    }

    final draft = <String, dynamic>{
      'name': name,
      'base_url': baseUrl,
      'api_format': _format.wire,
      'enabled': _enabled,
    };

    setState(() => _busy = true);
    try {
      final actions = ref.read(upstreamActionsProvider);
      final existing = widget.existing;
      if (existing == null) {
        await actions.create(draft);
      } else {
        await actions.update(existing.id, draft);
      }
      if (mounted) Navigator.of(context).pop();
    } on Exception catch (error) {
      if (!mounted) return;
      ConsoleToast.show(
        context,
        message: context.t.common.saveFailed(error: error),
        type: GlassToastType.error,
        position: GlassToastPosition.top,
      );
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return DetailPage(
      title: widget.existing == null
          ? context.t.providers.add
          : context.t.providers.edit,
      save: (id: 'upstream-primary', onSave: _save),
      busy: _busy,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        mainAxisSize: MainAxisSize.min,
        children: [
          SectionHeader(
            title: widget.existing == null
                ? context.t.providers.add
                : context.t.providers.edit,
          ),
          GlassTextField(
            controller: _name,
            placeholder: context.t.providers.name,
          ),
          const SizedBox(height: ConsoleSpace.m),
          GlassTextField(
            controller: _baseUrl,
            placeholder: context.t.providers.baseUrl,
          ),
          const SizedBox(height: ConsoleSpace.xl),
          SectionHeader(title: context.t.providers.format),
          // SegmentedPicker 自带表面，不能包进 GlassContainer。
          SegmentedPicker<ApiFormat>(
            values: ApiFormat.values,
            labels: [
              for (final format in ApiFormat.values)
                format.localized(context.t),
            ],
            selected: _format,
            onChanged: (value) => setState(() => _format = value),
          ),
          const SizedBox(height: ConsoleSpace.s),
          Muted(text: context.t.providers.endpoint(endpoint: _format.endpoint)),
          const SizedBox(height: ConsoleSpace.xl),
          Muted(text: context.t.providers.credentialNote),
          const SizedBox(height: ConsoleSpace.l),
          Row(
            children: [
              GlassSwitch(
                useOwnLayer: true,
                quality: ConsoleGlass.quality,
                value: _enabled,
                onChanged: (value) => setState(() => _enabled = value),
              ),
              const SizedBox(width: ConsoleSpace.m),
              Muted(
                text: _enabled
                    ? context.t.common.enabled
                    : context.t.common.disabled,
              ),
            ],
          ),
        ],
      ),
    );
  }
}
