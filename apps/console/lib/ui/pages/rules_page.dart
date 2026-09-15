/// 规则页：查看、启用停用、新建、编辑与删除判定规则。
///
/// 普通状态整行进入详情；编辑状态勾选删除。兜底策略独立成组并持久化。
///
/// 页面不再自渲染标题与「新建规则」按钮：标题与说明属于分区定义（见 `sections.dart`），
/// 主操作属于顶部工具栏（见外壳里的 `_appBarActions`）。页面只负责列表本身。
library;

import 'dart:math' as math;

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

class RulesPage extends ConsumerWidget {
  const RulesPage({required this.isDesktop, super.key});

  final bool isDesktop;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final rules = ref.watch(rulesProvider);
    final selection = ref.watch(ruleSelectionProvider);

    return rules.when(
      loading: () => LoadingState(label: context.t.rules.loading),
      error: (error, _) => MessageState(
        message: context.t.rules.loadFailed(error: error),
        action: RetryButton(onRetry: () => ref.invalidate(rulesProvider)),
      ),
      data: (value) {
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            GroupedList(
              header: context.t.rules.listTitle,
              children: [
                for (final rule in value.items)
                  ListRow(
                    icon: selection == null
                        ? null
                        : selection.contains(rule.id)
                        ? CupertinoIcons.check_mark_circled_solid
                        : CupertinoIcons.circle,
                    title: rule.localizedName(context.t),
                    subtitle: rule.condition.localizedSummary(context.t),
                    chevron: selection == null,
                    trailing: selection == null
                        ? Muted(
                            text: rule.enabled
                                ? context.t.common.enabled
                                : context.t.common.disabled,
                          )
                        : null,
                    onTap: () => selection == null
                        ? presentRuleEditor(context, rule)
                        : ref
                              .read(ruleSelectionProvider.notifier)
                              .toggle(rule.id),
                  ),
                if (value.items.isEmpty) ListRow(title: context.t.rules.empty),
              ],
            ),
            const SizedBox(height: ConsoleSpace.xl),
            if (selection == null) ...[
              GroupedList(
                children: [
                  ListRow(
                    title: context.t.rules.kNew,
                    icon: CupertinoIcons.add,
                    onTap: () => presentRuleEditor(context, null),
                  ),
                ],
              ),
              const SizedBox(height: ConsoleSpace.xl),
              _FallbackOptions(action: value.fallbackAction),
            ],
          ],
        );
      },
    );
  }
}

class RuleSelection extends Notifier<Set<String>?> {
  @override
  Set<String>? build() => null;
  void edit() => state = <String>{};
  void close() => state = null;
  void remove(String id) =>
      state = state == null ? null : ({...state!}..remove(id));
  void toggle(String id) {
    final next = {...?state};
    if (!next.remove(id)) next.add(id);
    state = next;
  }
}

final ruleSelectionProvider = NotifierProvider<RuleSelection, Set<String>?>(
  RuleSelection.new,
);

class RuleListToolbar extends ConsumerStatefulWidget {
  const RuleListToolbar({
    this.leadingOnly = false,
    this.trailingOnly = false,
    super.key,
  });
  final bool leadingOnly;
  final bool trailingOnly;
  static DetailNavigationAction _close(BuildContext context, WidgetRef ref) =>
      DetailNavigationAction(
        id: 'rules-close',
        label: context.t.common.cancel,
        icon: CupertinoIcons.xmark,
        onPressed: ref.read(ruleSelectionProvider.notifier).close,
      );

  static List<GlassBarItem> items(BuildContext context, WidgetRef ref) => [
    if (ref.watch(ruleSelectionProvider) != null)
      _close(context, ref).barItem
    else
      GlassBarItem.custom(
        id: 'rules-edit',
        child: const RuleListToolbar(trailingOnly: true),
        background: GlassBarItemBackground.own,
      ),
  ];
  static List<GlassBarItem> leadingItems(WidgetRef ref) =>
      ref.watch(ruleSelectionProvider) == null
      ? []
      : [
          GlassBarItem.custom(
            id: 'rules-delete',
            child: const RuleListToolbar(leadingOnly: true),
            background: GlassBarItemBackground.own,
          ),
        ];
  @override
  ConsumerState<RuleListToolbar> createState() => _RuleListToolbarState();
}

class _RuleListToolbarState extends ConsumerState<RuleListToolbar> {
  bool _busy = false;
  Future<void> _delete(Set<String> ids) async {
    if (!await RuleDeletion.confirm(context)) return;
    setState(() => _busy = true);
    try {
      for (final id in ids.toList()) {
        await ref.read(ruleActionsProvider).delete(id);
        ref.read(ruleSelectionProvider.notifier).remove(id);
      }
      ref.read(ruleSelectionProvider.notifier).close();
    } catch (error) {
      if (mounted) {
        ConsoleToast.show(
          context,
          message: context.t.common.deleteFailed(error: error),
          type: GlassToastType.error,
        );
      }
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final selected = ref.watch(ruleSelectionProvider);
    final controller = ref.read(ruleSelectionProvider.notifier);
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        if (selected != null && !widget.trailingOnly)
          ActionButton(
            text: context.t.common.delete,
            tint: ConsolePalette.of(context).negative,
            height: ConsoleMetrics.barControl,
            onPressed: _busy || selected.isEmpty
                ? null
                : () => _delete(selected),
          ),
        if (selected != null && !widget.leadingOnly && !widget.trailingOnly)
          const SizedBox(width: ConsoleSpace.s),
        if (!widget.leadingOnly)
          selected == null
              ? ActionButton(
                  text: context.t.rules.editList,
                  height: ConsoleMetrics.barControl,
                  onPressed: controller.edit,
                )
              : IgnorePointer(
                  ignoring: _busy,
                  child: RuleListToolbar._close(context, ref),
                ),
      ],
    );
  }
}

class _FallbackOptions extends ConsumerStatefulWidget {
  const _FallbackOptions({required this.action});
  final Decision action;
  @override
  ConsumerState<_FallbackOptions> createState() => _FallbackOptionsState();
}

class _FallbackOptionsState extends ConsumerState<_FallbackOptions> {
  bool _busy = false;
  Future<void> _select(Decision action) async {
    if (_busy || action == widget.action) return;
    setState(() => _busy = true);
    try {
      await ref.read(ruleActionsProvider).setFallback(action);
    } catch (error) {
      if (mounted) {
        ConsoleToast.show(
          context,
          message: context.t.common.saveFailed(error: error),
          type: GlassToastType.error,
        );
      }
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) => GroupedList(
    header: context.t.rules.unmatched,
    children: [
      for (final action in Decision.values)
        ListRow(
          title: action.localized(context.t),
          trailing: widget.action == action
              ? Icon(
                  CupertinoIcons.check_mark,
                  color: ConsolePalette.of(context).tint,
                )
              : null,
          onTap: _busy ? null : () => _select(action),
        ),
    ],
  );
}

Future<void> presentRuleEditor(BuildContext context, Rule? existing) async {
  await Navigator.of(context).push<void>(
    CupertinoPageRoute(
      builder: (context) => _RuleEditor(
        existing: existing,
        initialName: existing?.localizedName(context.t),
      ),
    ),
  );
}

abstract final class RuleDeletion {
  static Future<bool> confirm(BuildContext context) async =>
      await showCupertinoDialog<bool>(
        context: context,
        useRootNavigator: false,
        builder: (context) => GlassDialog(
          title: context.t.common.delete,
          message: context.t.common.deleteConfirmation,
          actions: [
            GlassDialogAction(
              onPressed: () => Navigator.of(context).pop(false),
              label: context.t.common.cancel,
            ),
            GlassDialogAction(
              isDestructive: true,
              onPressed: () => Navigator.of(context).pop(true),
              label: context.t.common.delete,
            ),
          ],
        ),
      ) ??
      false;
}

class _RuleEditor extends ConsumerStatefulWidget {
  const _RuleEditor({this.existing, this.initialName});

  final Rule? existing;
  final String? initialName;

  @override
  ConsumerState<_RuleEditor> createState() => _RuleEditorState();
}

class _RuleEditorState extends ConsumerState<_RuleEditor> {
  late final _name = TextEditingController(text: widget.initialName ?? '');
  late final _priority = TextEditingController(
    text: '${widget.existing?.priority ?? 600}',
  );
  late final _expression = widget.existing == null
      ? _GroupDraft(RuleLogic.all, [_FilterDraft.entity()])
      : _ExpressionDraft.fromDomain(widget.existing!.condition).asRootGroup();
  late Decision _action = widget.existing?.action ?? Decision.release;
  late bool _enabled = widget.existing?.enabled ?? true;
  bool _busy = false;

  @override
  void dispose() {
    _name.dispose();
    _priority.dispose();
    _expression.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    final name = _name.text.trim();
    if (name.isEmpty) {
      ConsoleToast.show(
        context,
        message: context.t.rules.nameRequired,
        type: GlassToastType.warning,
        position: GlassToastPosition.top,
      );
      return;
    }

    late final RuleExpression condition;
    try {
      condition = _expression.toDomain(context.t);
    } on FormatException catch (error) {
      ConsoleToast.show(
        context,
        message: error.message.toString(),
        type: GlassToastType.warning,
        position: GlassToastPosition.top,
      );
      return;
    }

    final existing = widget.existing;
    final persistedName =
        existing != null &&
            existing.usesLocalizedDefaultName &&
            name == widget.initialName
        ? existing.name
        : name;
    final draft = <String, dynamic>{
      'name': persistedName,
      'priority': int.tryParse(_priority.text.trim()) ?? 600,
      'condition': condition.toJson(),
      'action': _action.wire,
      'enabled': _enabled,
    };

    setState(() => _busy = true);
    try {
      final actions = ref.read(ruleActionsProvider);
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
          ? context.t.rules.kNew
          : context.t.rules.edit,
      save: (id: 'rule-primary', onSave: _save),
      busy: _busy,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        mainAxisSize: MainAxisSize.min,
        children: [
          SectionHeader(
            title: widget.existing == null
                ? context.t.rules.kNew
                : context.t.rules.edit,
          ),
          GlassTextField(controller: _name, placeholder: context.t.rules.name),
          const SizedBox(height: ConsoleSpace.m),
          Row(
            children: [
              Expanded(child: Text(context.t.common.enabled)),
              GlassSwitch(
                value: _enabled,
                onChanged: (value) => setState(() => _enabled = value),
                useOwnLayer: true,
              ),
            ],
          ),
          const SizedBox(height: ConsoleSpace.xl),
          SectionHeader(
            title: context.t.rules.matchingLogic,
            subtitle: context.t.rules.matchingLogicSubtitle,
          ),
          _GroupEditor(
            group: _expression,
            root: true,
            onChanged: () => setState(() {}),
          ),
          const SizedBox(height: ConsoleSpace.xl),
          SectionHeader(title: context.t.rules.result),
          GlassTextField(
            controller: _priority,
            placeholder: context.t.rules.priority,
            keyboardType: TextInputType.number,
          ),
          const SizedBox(height: ConsoleSpace.m),
          // SegmentedPicker 自带表面，不能包在 GlassContainer 里。
          SegmentedPicker<Decision>(
            values: Decision.values,
            labels: [
              for (final action in Decision.values) action.localized(context.t),
            ],
            selected: _action,
            onChanged: (value) => setState(() => _action = value),
          ),
          const SizedBox(height: ConsoleSpace.xxl),
          if (widget.existing != null)
            ActionButton(
              onPressed: _delete,
              text: context.t.common.delete,
              icon: CupertinoIcons.trash,
              tint: ConsolePalette.of(context).negative,
            ),
        ],
      ),
    );
  }

  Future<void> _delete() async {
    if (!await RuleDeletion.confirm(context) || !mounted) return;
    setState(() => _busy = true);
    try {
      await ref.read(ruleActionsProvider).delete(widget.existing!.id);
      if (mounted) Navigator.of(context).pop();
    } on Exception catch (error) {
      if (mounted) {
        ConsoleToast.show(
          context,
          message: context.t.common.deleteFailed(error: error),
          type: GlassToastType.error,
        );
      }
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }
}

enum _FilterKind { entity, keyword, characterLength, confidence }

extension on _FilterKind {
  String localized(Translations text) => switch (this) {
    _FilterKind.entity => text.rules.filterKind.entity,
    _FilterKind.keyword => text.rules.filterKind.keyword,
    _FilterKind.characterLength => text.rules.filterKind.characterLength,
    _FilterKind.confidence => text.rules.filterKind.confidence,
  };
}

sealed class _ExpressionDraft {
  const _ExpressionDraft();

  factory _ExpressionDraft.fromDomain(RuleExpression expression) =>
      switch (expression) {
        RuleGroupExpression(:final logic, :final conditions) => _GroupDraft(
          logic,
          [
            for (final condition in conditions)
              _ExpressionDraft.fromDomain(condition),
          ],
        ),
        FilterRuleExpression(:final filter) => _FilterDraft.fromDomain(filter),
      };

  RuleExpression toDomain(Translations text);
  void dispose();

  _GroupDraft asRootGroup() => switch (this) {
    final _GroupDraft group => group,
    final _FilterDraft filter => _GroupDraft(RuleLogic.all, [filter]),
  };
}

class _GroupDraft extends _ExpressionDraft {
  _GroupDraft(this.logic, this.conditions);

  RuleLogic logic;
  final List<_ExpressionDraft> conditions;

  @override
  RuleExpression toDomain(Translations text) {
    if (conditions.isEmpty) throw FormatException(text.rules.groupEmpty);
    return RuleGroupExpression(logic, [
      for (final item in conditions) item.toDomain(text),
    ]);
  }

  @override
  void dispose() {
    for (final condition in conditions) {
      condition.dispose();
    }
  }
}

class _FilterDraft extends _ExpressionDraft {
  _FilterDraft({
    required this.kind,
    required String value,
    this.group = EntityGroup.privateEmail,
    this.comparison = RuleComparison.greater,
    this.patternKind = RulePatternKind.exact,
    this.caseSensitive = false,
  }) : controller = TextEditingController(text: value);

  factory _FilterDraft.entity() =>
      _FilterDraft(kind: _FilterKind.entity, value: '');

  factory _FilterDraft.fromDomain(RuleFilter filter) => switch (filter) {
    EntityRuleFilter(:final group) => _FilterDraft(
      kind: _FilterKind.entity,
      value: '',
      group: group,
    ),
    KeywordRuleFilter(:final pattern) => _FilterDraft(
      kind: _FilterKind.keyword,
      value: pattern.text,
      patternKind: pattern.kind,
      caseSensitive: pattern.caseSensitive,
    ),
    CharacterLengthRuleFilter(:final operator, :final value) => _FilterDraft(
      kind: _FilterKind.characterLength,
      value: '$value',
      comparison: operator,
    ),
    ConfidenceRuleFilter(:final operator, :final value) => _FilterDraft(
      kind: _FilterKind.confidence,
      value: '$value',
      comparison: operator,
    ),
  };

  _FilterKind kind;
  EntityGroup group;
  RuleComparison comparison;
  RulePatternKind patternKind;
  bool caseSensitive;
  final TextEditingController controller;

  void selectKind(_FilterKind target) {
    kind = target;
    controller.text = switch (target) {
      _FilterKind.entity => '',
      _FilterKind.keyword => '',
      _FilterKind.characterLength => '1',
      _FilterKind.confidence => '0.8',
    };
  }

  @override
  RuleExpression toDomain(Translations text) {
    final filter = switch (kind) {
      _FilterKind.entity => EntityRuleFilter(group),
      _FilterKind.keyword => KeywordRuleFilter(
        RulePattern(
          text: controller.text.trim(),
          kind: patternKind,
          caseSensitive: caseSensitive,
        ),
      ),
      _FilterKind.characterLength => CharacterLengthRuleFilter(
        comparison,
        int.tryParse(controller.text.trim()) ?? -1,
      ),
      _FilterKind.confidence => ConfidenceRuleFilter(
        comparison,
        double.tryParse(controller.text.trim()) ?? double.nan,
      ),
    };
    if (filter case KeywordRuleFilter(:final pattern)
        when pattern.text.isEmpty) {
      throw FormatException(text.rules.keywordRequired);
    }
    if (filter case CharacterLengthRuleFilter(:final value) when value < 0) {
      throw FormatException(text.rules.lengthInvalid);
    }
    if (filter case ConfidenceRuleFilter(:final value)
        when !value.isFinite || value < 0 || value > 1) {
      throw FormatException(text.rules.confidenceInvalid);
    }
    return FilterRuleExpression(filter);
  }

  @override
  void dispose() => controller.dispose();
}

class _GroupEditor extends StatelessWidget {
  const _GroupEditor({
    required this.group,
    required this.onChanged,
    this.root = false,
  });

  final _GroupDraft group;
  final VoidCallback onChanged;
  final bool root;

  @override
  Widget build(BuildContext context) {
    final palette = ConsolePalette.of(context);
    return DecoratedBox(
      decoration: BoxDecoration(
        border: root
            ? null
            : Border(left: BorderSide(color: palette.separator, width: 2)),
      ),
      child: Padding(
        padding: EdgeInsetsDirectional.only(start: root ? 0 : ConsoleSpace.m),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            SegmentedPicker<RuleLogic>(
              values: RuleLogic.values,
              labels: [
                for (final logic in RuleLogic.values)
                  logic.localized(context.t),
              ],
              selected: group.logic,
              onChanged: (value) {
                group.logic = value;
                onChanged();
              },
            ),
            const SizedBox(height: ConsoleSpace.m),
            for (final (index, condition) in group.conditions.indexed) ...[
              Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Expanded(
                    child: switch (condition) {
                      final _GroupDraft nested => _GroupEditor(
                        group: nested,
                        onChanged: onChanged,
                      ),
                      final _FilterDraft filter => _FilterEditor(
                        filter: filter,
                        onChanged: onChanged,
                      ),
                    },
                  ),
                  CupertinoButton(
                    padding: EdgeInsets.zero,
                    minimumSize: const Size.square(ConsoleMetrics.touchTarget),
                    onPressed: () {
                      condition.dispose();
                      group.conditions.removeAt(index);
                      onChanged();
                    },
                    child: Icon(
                      CupertinoIcons.trash,
                      size: 17,
                      color: palette.negative,
                    ),
                  ),
                ],
              ),
              if (index != group.conditions.length - 1)
                const SizedBox(height: ConsoleSpace.m),
            ],
            const SizedBox(height: ConsoleSpace.m),
            Row(
              children: [
                Expanded(
                  child: ActionButton(
                    text: context.t.rules.addFilter,
                    icon: CupertinoIcons.add,
                    onPressed: () {
                      group.conditions.add(_FilterDraft.entity());
                      onChanged();
                    },
                  ),
                ),
                const SizedBox(width: ConsoleSpace.s),
                Expanded(
                  child: ActionButton(
                    text: context.t.rules.addGroup,
                    icon: CupertinoIcons.folder_badge_plus,
                    onPressed: () {
                      group.conditions.add(
                        _GroupDraft(RuleLogic.all, [_FilterDraft.entity()]),
                      );
                      onChanged();
                    },
                  ),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

class _FilterEditor extends StatelessWidget {
  const _FilterEditor({required this.filter, required this.onChanged});

  final _FilterDraft filter;
  final VoidCallback onChanged;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _ChoiceMenu<_FilterKind>(
          value: filter.kind,
          values: _FilterKind.values,
          label: (kind) => kind.localized(context.t),
          onChanged: (value) {
            filter.selectKind(value);
            onChanged();
          },
        ),
        const SizedBox(height: ConsoleSpace.s),
        switch (filter.kind) {
          _FilterKind.entity => _ChoiceMenu<EntityGroup>(
            value: filter.group,
            values: EntityGroup.values
                .where((group) => group != EntityGroup.unknown)
                .toList(),
            label: (group) => group.localized(context.t),
            onChanged: (value) {
              filter.group = value;
              onChanged();
            },
          ),
          _FilterKind.keyword => Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              GlassTextField(
                controller: filter.controller,
                placeholder: context.t.rules.keywordPlaceholder,
              ),
              const SizedBox(height: ConsoleSpace.s),
              SegmentedPicker<RulePatternKind>(
                values: RulePatternKind.values,
                labels: [
                  for (final kind in RulePatternKind.values)
                    kind.localized(context.t),
                ],
                selected: filter.patternKind,
                onChanged: (value) {
                  filter.patternKind = value;
                  onChanged();
                },
              ),
              Row(
                children: [
                  Transform.scale(
                    scale: 0.84,
                    child: CupertinoSwitch(
                      value: filter.caseSensitive,
                      onChanged: (value) {
                        filter.caseSensitive = value;
                        onChanged();
                      },
                    ),
                  ),
                  const SizedBox(width: ConsoleSpace.s),
                  Muted(text: context.t.rules.caseSensitive),
                ],
              ),
            ],
          ),
          _FilterKind.characterLength || _FilterKind.confidence => Row(
            children: [
              Expanded(
                flex: 2,
                child: SegmentedPicker<RuleComparison>(
                  values: RuleComparison.values,
                  labels: [
                    for (final item in RuleComparison.values)
                      item.localized(context.t),
                  ],
                  selected: filter.comparison,
                  onChanged: (value) {
                    filter.comparison = value;
                    onChanged();
                  },
                ),
              ),
              const SizedBox(width: ConsoleSpace.s),
              Expanded(
                child: GlassTextField(
                  controller: filter.controller,
                  placeholder: filter.kind == _FilterKind.confidence
                      ? '0-1'
                      : context.t.rules.lengthPlaceholder,
                  keyboardType: TextInputType.numberWithOptions(
                    decimal: filter.kind == _FilterKind.confidence,
                  ),
                ),
              ),
            ],
          ),
        },
      ],
    );
  }
}

class _ChoiceMenu<T> extends StatelessWidget {
  const _ChoiceMenu({
    required this.value,
    required this.values,
    required this.label,
    required this.onChanged,
  });

  final T value;
  final List<T> values;
  final String Function(T value) label;
  final ValueChanged<T> onChanged;

  @override
  Widget build(BuildContext context) {
    final palette = ConsolePalette.of(context);
    return GlassMenu(
      menuWidth: _menuWidth(context),
      // 条目多时默认高度不裁到屏幕，菜单会溢出窗口并且滚不动。
      autoAdjustToScreen: true,
      triggerBuilder: (context, toggle) => MouseRegion(
        cursor: SystemMouseCursors.click,
        child: GestureDetector(
          onTap: toggle,
          behavior: HitTestBehavior.opaque,
          child: DecoratedBox(
            decoration: ShapeDecoration(
              color: palette.field,
              shape: const LiquidRoundedSuperellipse(
                borderRadius: ConsoleRadius.control,
              ),
            ),
            child: SizedBox(
              height: ConsoleMetrics.touchTarget,
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: ConsoleSpace.m),
                child: Row(
                  children: [
                    Expanded(
                      child: Text(
                        label(value),
                        style: ConsoleText.body.copyWith(color: palette.label),
                      ),
                    ),
                    Icon(
                      CupertinoIcons.chevron_up_chevron_down,
                      size: 14,
                      color: palette.secondaryLabel,
                    ),
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
      items: [
        for (final option in values)
          GlassMenuItem(
            title: label(option),
            isSelected: option == value,
            onTap: () => onChanged(option),
          ),
      ],
    );
  }

  /// 装得下最宽选项的菜单宽度。
  ///
  /// 类别名里有「确定性规则·中国大陆统一社会信用代码」这种长名字，菜单默认的 200 会把它们截成
  /// 「确定性规则·中国…」——被截掉的恰好是「哪个类别」。所以宽度按内容量出来，不写死。
  double _menuWidth(BuildContext context) {
    final textScaler = MediaQuery.textScalerOf(context);
    final widest = values.fold<double>(0, (width, option) {
      final painter = TextPainter(
        text: TextSpan(
          text: label(option),
          // 条目标题的字号与字重：菜单自己的默认值。
          style: const TextStyle(fontSize: 17, fontWeight: FontWeight.w400),
        ),
        textScaler: textScaler,
        textDirection: TextDirection.ltr,
      )..layout();
      return math.max(width, painter.width);
    });

    // 内容之外的部分：菜单左右内边距（12 × 2）加上条目的左右内边距（16 × 2）。
    const chrome = 12.0 * 2 + 16.0 * 2;
    final available = MediaQuery.sizeOf(context).width - ConsoleSpace.l * 2;
    return (widest + chrome).clamp(240.0, math.max(240.0, available));
  }
}
