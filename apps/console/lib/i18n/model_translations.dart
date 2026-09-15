/// Localized presentation for protocol/domain enums.
///
/// Wire values remain in the API model. Human-readable names live here so a
/// domain type never stores a second, locale-specific representation.
library;

import '../api/models.dart';
import '../theme/console_theme.dart';
import '../ui/sections.dart';
import 'strings.g.dart';

extension EntityGroupTranslations on EntityGroup {
  String localized(Translations text) => switch (this) {
    EntityGroup.accountNumber => text.model.entity.accountNumber,
    EntityGroup.bankCard => text.model.entity.bankCard,
    EntityGroup.creditCode => text.model.entity.creditCode,
    EntityGroup.idCard => text.model.entity.idCard,
    EntityGroup.ipAddress => text.model.entity.ipAddress,
    EntityGroup.macAddress => text.model.entity.macAddress,
    EntityGroup.privateAddress => text.model.entity.privateAddress,
    EntityGroup.privateDate => text.model.entity.privateDate,
    EntityGroup.privateEmail => text.model.entity.privateEmail,
    EntityGroup.privatePerson => text.model.entity.privatePerson,
    EntityGroup.privatePhone => text.model.entity.privatePhone,
    EntityGroup.privateUrl => text.model.entity.privateUrl,
    EntityGroup.secret => text.model.entity.secret,
    EntityGroup.unknown => text.model.entity.unknown,
  };
}

extension DecisionTranslations on Decision {
  String localized(Translations text) => switch (this) {
    Decision.release => text.model.decision.release,
    Decision.redact => text.model.decision.redact,
  };
}

extension RuleSourceTranslations on RuleSource {
  String localized(Translations text) => switch (this) {
    RuleSource.builtin => text.model.source.builtin,
    RuleSource.console => text.model.source.console,
    RuleSource.operator => text.model.source.operator,
  };

  String groupHeader(Translations text) => switch (this) {
    RuleSource.builtin => text.rules.group.builtin,
    RuleSource.console => text.rules.group.console,
    RuleSource.operator => text.rules.group.operator,
  };

  String? groupFooter(Translations text) => switch (this) {
    RuleSource.builtin => text.rules.footer.builtin,
    RuleSource.console => null,
    RuleSource.operator => text.rules.footer.operator,
  };
}

extension RulePatternKindTranslations on RulePatternKind {
  String localized(Translations text) => switch (this) {
    RulePatternKind.exact => text.model.pattern.exact,
    RulePatternKind.substring => text.model.pattern.substring,
    RulePatternKind.glob => text.model.pattern.glob,
  };
}

extension RuleComparisonTranslations on RuleComparison {
  String localized(Translations text) => switch (this) {
    RuleComparison.greater => text.model.comparison.greater,
    RuleComparison.equal => text.model.comparison.equal,
    RuleComparison.less => text.model.comparison.less,
  };
}

extension RuleLogicTranslations on RuleLogic {
  String localized(Translations text) => switch (this) {
    RuleLogic.all => text.model.logic.all,
    RuleLogic.any => text.model.logic.any,
  };
}

extension ApiFormatTranslations on ApiFormat {
  String localized(Translations text) => switch (this) {
    ApiFormat.openaiResponses => text.model.apiFormat.openaiResponses,
    ApiFormat.openaiChat => text.model.apiFormat.openaiChat,
    ApiFormat.anthropicMessages => text.model.apiFormat.anthropicMessages,
  };
}

extension AppearanceTranslations on ConsoleAppearance {
  String localized(Translations text) => switch (this) {
    ConsoleAppearance.system => text.appearance.system,
    ConsoleAppearance.light => text.appearance.light,
    ConsoleAppearance.dark => text.appearance.dark,
  };
}

extension ConsoleSectionTranslations on ConsoleSection {
  String label(Translations text) => switch (this) {
    ConsoleSection.overview => text.navigation.overview.label,
    ConsoleSection.pool => text.navigation.pool.label,
    ConsoleSection.rules => text.navigation.rules.label,
    ConsoleSection.providers => text.navigation.providers.label,
    ConsoleSection.performance => text.navigation.performance.label,
  };

  String description(Translations text) => switch (this) {
    ConsoleSection.overview => text.navigation.overview.description,
    ConsoleSection.pool => text.navigation.pool.description,
    ConsoleSection.rules => text.navigation.rules.description,
    ConsoleSection.providers => text.navigation.providers.description,
    ConsoleSection.performance => text.navigation.performance.description,
  };
}

extension RuleExpressionTranslations on RuleExpression {
  EntityGroup? get _firstEntityGroup => switch (this) {
    RuleGroupExpression(:final conditions) =>
      conditions
          .map((condition) => condition._firstEntityGroup)
          .whereType<EntityGroup>()
          .firstOrNull,
    FilterRuleExpression(filter: EntityRuleFilter(:final group)) => group,
    FilterRuleExpression() => null,
  };

  String localizedSummary(Translations text) => switch (this) {
    RuleGroupExpression(:final logic, :final conditions) =>
      conditions
          .map((condition) => condition.localizedSummary(text))
          .join(
            logic == RuleLogic.all
                ? text.model.filterSummary.allJoiner
                : text.model.filterSummary.anyJoiner,
          ),
    FilterRuleExpression(:final filter) => filter.localizedSummary(text),
  };
}

/// 管理员逐条登记时写入的稳定标识：服务端只按方向写入这两个名字，展示时按方向翻译。
///
/// `approval.` 是与线上取值配套的历史前缀；`approval.release` 是禁用之前的旧名字，
/// 已经落库的规则仍然要用同一个默认名展示。
const _operatorDefaultNames = {'approval.release', 'approval.redact'};

extension RuleTranslations on Rule {
  bool get usesLocalizedDefaultName =>
      (source == RuleSource.builtin && name == id) ||
      (source == RuleSource.operator && _operatorDefaultNames.contains(name));

  String localizedName(Translations text) {
    if (source == RuleSource.builtin && name == id) {
      final group = condition._firstEntityGroup;
      if (group != null) {
        return text.rules.defaultName(group: group.localized(text));
      }
    }
    if (source == RuleSource.operator && _operatorDefaultNames.contains(name)) {
      return switch (action) {
        Decision.release => text.rules.operatorReleaseName,
        Decision.redact => text.rules.operatorRedactName,
      };
    }
    return name;
  }
}

extension RuleFilterTranslations on RuleFilter {
  String localizedSummary(Translations text) => switch (this) {
    EntityRuleFilter(:final group) => text.model.filterSummary.entity(
      group: group.localized(text),
    ),
    KeywordRuleFilter(:final pattern) => text.model.filterSummary.keyword(
      operator: pattern.kind.localized(text),
      text: pattern.text,
    ),
    CharacterLengthRuleFilter(:final operator, :final value) =>
      text.model.filterSummary.characterLength(
        operator: operator.localized(text),
        value: value,
      ),
    ConfidenceRuleFilter(:final operator, :final value) =>
      text.model.filterSummary.confidence(
        operator: operator.localized(text),
        value: value,
      ),
  };
}
