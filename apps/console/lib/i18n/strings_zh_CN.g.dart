///
/// Generated file. Do not edit.
///
// coverage:ignore-file
// ignore_for_file: type=lint, unused_import
// dart format off

import 'package:flutter/widgets.dart';
import 'package:intl/intl.dart';
import 'package:slang/generated.dart';
import 'strings.g.dart';

// Path: <root>
class TranslationsZhCn with BaseTranslations<AppLocale, Translations> implements Translations {
	/// You can call this constructor and build your own translation instance of this locale.
	/// Constructing via the enum [AppLocale.build] is preferred.
	TranslationsZhCn({Map<String, Node>? overrides, PluralResolver? cardinalResolver, PluralResolver? ordinalResolver, TranslationMetadata<AppLocale, Translations>? meta})
		: assert(overrides == null, 'Set "translation_overrides: true" in order to enable this feature.'),
		  _meta = meta ?? TranslationMetadata(
		    locale: AppLocale.zhCn,
		    overrides: overrides ?? {},
		    cardinalResolver: cardinalResolver,
		    ordinalResolver: ordinalResolver,
		  ) {
		_meta.setFlatMapFunction(_flatMapFunction);
	}

	/// Metadata for the translations of <zh-CN>.
	final TranslationMetadata<AppLocale, Translations> _meta;
	@override TranslationMetadata<AppLocale, Translations> get $meta => _meta;

	/// Access flat map
	@override dynamic operator[](String key) => _meta.getTranslation(key);

	late final TranslationsZhCn _root = this; // ignore: unused_field

	@override 
	TranslationsZhCn $copyWith({TranslationMetadata<AppLocale, Translations>? meta}) => TranslationsZhCn(meta: meta ?? this.$meta);

	// Translations
	@override late final _Translations$demo$zh_CN demo = _Translations$demo$zh_CN._(_root);
	@override String get language => '语言';
	@override late final _Translations$app$zh_CN app = _Translations$app$zh_CN._(_root);
	@override late final _Translations$common$zh_CN common = _Translations$common$zh_CN._(_root);
	@override late final _Translations$appearance$zh_CN appearance = _Translations$appearance$zh_CN._(_root);
	@override late final _Translations$navigation$zh_CN navigation = _Translations$navigation$zh_CN._(_root);
	@override late final _Translations$model$zh_CN model = _Translations$model$zh_CN._(_root);
	@override late final _Translations$login$zh_CN login = _Translations$login$zh_CN._(_root);
	@override late final _Translations$setup$zh_CN setup = _Translations$setup$zh_CN._(_root);
	@override late final _Translations$overview$zh_CN overview = _Translations$overview$zh_CN._(_root);
	@override late final _Translations$performance$zh_CN performance = _Translations$performance$zh_CN._(_root);
	@override late final _Translations$pool$zh_CN pool = _Translations$pool$zh_CN._(_root);
	@override late final _Translations$rules$zh_CN rules = _Translations$rules$zh_CN._(_root);
	@override late final _Translations$providers$zh_CN providers = _Translations$providers$zh_CN._(_root);
}

// Path: demo
class _Translations$demo$zh_CN implements Translations$demo$en {
	_Translations$demo$zh_CN._(this._root);

	final TranslationsZhCn _root; // ignore: unused_field

	// Translations
	@override String get banner => '交互演示 · 示例数据';
	@override String get detail => '刷新后重置修改。不会向上游发送请求。';
	@override String credentials({required Object username, required Object password}) => '演示账号：${username} / ${password}，无需真实凭据。';
	@override String get reset => '重置演示';
	@override String get enter => '进入演示';
	@override String get loginTitle => '体验 Privacy Router';
	@override String get tagline => '在示例工作区中体验隐私管理功能';
}

// Path: app
class _Translations$app$zh_CN implements Translations$app$en {
	_Translations$app$zh_CN._(this._root);

	final TranslationsZhCn _root; // ignore: unused_field

	// Translations
	@override String get title => 'Privacy Router';
	@override String get tagline => '本地隐私识别与脱敏转发';
}

// Path: common
class _Translations$common$zh_CN implements Translations$common$en {
	_Translations$common$zh_CN._(this._root);

	final TranslationsZhCn _root; // ignore: unused_field

	// Translations
	@override String get search => '搜索';
	@override String get cancel => '取消';
	@override String get delete => '删除';
	@override String get deleteConfirmation => '删除这条规则？此操作无法撤销。';
	@override String get retry => '重试';
	@override String get save => '保存';
	@override String deleteFailed({required Object error}) => '删除失败：${error}';
	@override String saveFailed({required Object error}) => '保存失败：${error}';
	@override String get enabled => '已启用';
	@override String get disabled => '已停用';
	@override String get account => '账户';
	@override String accountMenu({required Object username}) => '${username}，账户菜单';
	@override String appearance({required Object value}) => '外观：${value}';
	@override String get signOut => '退出登录';
}

// Path: appearance
class _Translations$appearance$zh_CN implements Translations$appearance$en {
	_Translations$appearance$zh_CN._(this._root);

	final TranslationsZhCn _root; // ignore: unused_field

	// Translations
	@override String get system => '跟随系统';
	@override String get light => '浅色';
	@override String get dark => '深色';
}

// Path: navigation
class _Translations$navigation$zh_CN implements Translations$navigation$en {
	_Translations$navigation$zh_CN._(this._root);

	final TranslationsZhCn _root; // ignore: unused_field

	// Translations
	@override late final _Translations$navigation$overview$zh_CN overview = _Translations$navigation$overview$zh_CN._(_root);
	@override late final _Translations$navigation$pool$zh_CN pool = _Translations$navigation$pool$zh_CN._(_root);
	@override late final _Translations$navigation$rules$zh_CN rules = _Translations$navigation$rules$zh_CN._(_root);
	@override late final _Translations$navigation$providers$zh_CN providers = _Translations$navigation$providers$zh_CN._(_root);
	@override late final _Translations$navigation$performance$zh_CN performance = _Translations$navigation$performance$zh_CN._(_root);
	@override String get expandSidebar => '展开侧边栏';
	@override String get collapseSidebar => '收起侧边栏';
}

// Path: model
class _Translations$model$zh_CN implements Translations$model$en {
	_Translations$model$zh_CN._(this._root);

	final TranslationsZhCn _root; // ignore: unused_field

	// Translations
	@override late final _Translations$model$entity$zh_CN entity = _Translations$model$entity$zh_CN._(_root);
	@override late final _Translations$model$decision$zh_CN decision = _Translations$model$decision$zh_CN._(_root);
	@override late final _Translations$model$source$zh_CN source = _Translations$model$source$zh_CN._(_root);
	@override late final _Translations$model$pattern$zh_CN pattern = _Translations$model$pattern$zh_CN._(_root);
	@override late final _Translations$model$comparison$zh_CN comparison = _Translations$model$comparison$zh_CN._(_root);
	@override late final _Translations$model$logic$zh_CN logic = _Translations$model$logic$zh_CN._(_root);
	@override late final _Translations$model$apiFormat$zh_CN apiFormat = _Translations$model$apiFormat$zh_CN._(_root);
	@override late final _Translations$model$filterSummary$zh_CN filterSummary = _Translations$model$filterSummary$zh_CN._(_root);
	@override String unknownFilter({required Object kind}) => '未知规则过滤器：${kind}';
}

// Path: login
class _Translations$login$zh_CN implements Translations$login$en {
	_Translations$login$zh_CN._(this._root);

	final TranslationsZhCn _root; // ignore: unused_field

	// Translations
	@override String get username => '管理员账号';
	@override String get password => '口令';
	@override String get submit => '登录';
	@override String get unreachable => '无法连接到代理。请确认它已在运行。';
	@override String get providerMissing => '尚未配置上游 provider，登录后在「上游」页添加。';
}

// Path: setup
class _Translations$setup$zh_CN implements Translations$setup$en {
	_Translations$setup$zh_CN._(this._root);

	final TranslationsZhCn _root; // ignore: unused_field

	// Translations
	@override String get title => '创建管理员';
	@override String get subtitle => '这个代理还没有管理员账号，在这里设置一个即可开始使用。';
	@override String get username => '管理员账号';
	@override String get password => '口令';
	@override String get confirmation => '重复口令';
	@override String get submit => '创建并登录';
	@override String get usernameRequired => '管理员账号不能为空';
	@override String get passwordRequired => '口令不能为空';
	@override String get passwordMismatch => '两次输入的口令不一致';
	@override String get localOnly => '初始化只能在本机完成：如果你不在运行代理的这台电脑上，请回到那台机器上打开这个页面。';
}

// Path: overview
class _Translations$overview$zh_CN implements Translations$overview$en {
	_Translations$overview$zh_CN._(this._root);

	final TranslationsZhCn _root; // ignore: unused_field

	// Translations
	@override late final _Translations$overview$window$zh_CN window = _Translations$overview$window$zh_CN._(_root);
	@override String get loading => '正在加载统计';
	@override String loadFailed({required Object error}) => '无法加载统计：${error}';
	@override String get requests => '请求';
	@override String get requestsSubtitle => '窗口内完成的代理请求';
	@override String get fragments => '判定片段';
	@override String get fragmentsSubtitle => '进入内容池的文本段';
	@override String get detections => '命中';
	@override String detectionBreakdown({required Object redacted, required Object released}) => '抹去 ${redacted} · 放行 ${released}';
	@override String get redactedRatio => '抹去占比';
	@override String get redactedRatioSubtitle => '命中中未放行的比例';
	@override String get distribution => '类别分布';
	@override String get distributionSubtitle => '窗口内每一类被识别出来的次数';
	@override String get empty => '窗口内还没有命中';
	@override String get emptySubtitle => '流量经过代理后会在这里出现';
}

// Path: performance
class _Translations$performance$zh_CN implements Translations$performance$en {
	_Translations$performance$zh_CN._(this._root);

	final TranslationsZhCn _root; // ignore: unused_field

	// Translations
	@override String get firstResult => '首结果耗时';
	@override String get firstResultSubtitle => '本地推理开始至首个识别结果的平均耗时';
	@override String get throughput => '模型吞吐';
	@override String get throughputSubtitle => '处理 token 数 / 模型前向执行时间';
	@override String get modelTokens => '已处理 Token';
	@override String get modelTokensSubtitle => '实际模型工作量，不计缓存命中';
	@override String get tokenization => '分词';
	@override String get validation => '校验与批次规划';
	@override String get forward => '模型前向计算';
	@override String get decoding => '实体解码';
	@override String get loading => '正在加载性能数据';
	@override String loadFailed({required Object error}) => '无法加载性能数据：${error}';
	@override String get average => '平均端到端';
	@override String get averageSubtitle => '排队 + 识别 + 转发';
	@override String get percentileSubtitle => '最近秩法分位';
	@override String get maximum => '最长';
	@override String get maximumSubtitle => '窗口内最慢的一次';
	@override String get breakdown => '耗时分解';
	@override String get inference => '识别（推理）';
	@override String get inferenceSubtitle => '本地模型前向 + 结果缓存命中';
	@override String get upstream => '转发（上游）';
	@override String get upstreamSubtitle => '脱敏后的请求送达上游并取得响应';
	@override String get cacheHit => '缓存命中';
	@override String cacheBreakdown({required Object cached, required Object inferred}) => '命中 ${cached} · 推理 ${inferred} 个片段';
	@override String get note => '模型指标仅统计实际推理，缓存命中不计入吞吐。转发耗时记录到上游响应头，响应体保持流式透传。';
}

// Path: pool
class _Translations$pool$zh_CN implements Translations$pool$en {
	_Translations$pool$zh_CN._(this._root);

	final TranslationsZhCn _root; // ignore: unused_field

	// Translations
	@override String get filterUnreviewed => '未审核';
	@override String get filterReviewed => '已审核';
	@override String get filterAll => '全部内容';
	@override String get previous => '上一页';
	@override String get next => '下一页';
	@override String get refresh => '刷新';
	@override String range({required Object start, required Object end, required Object total}) => '第 ${start}–${end} 条，共 ${total} 条';
	@override String get emptyPage => '本页暂无记录，请返回上一页或刷新。';
	@override String get emptyUnreviewed => '没有待审核的内容。\n判定过的内容都已经登记过决定。';
	@override String get emptyReviewed => '还没有已审核的内容。\n登记过放行或禁止的内容会出现在这里。';
	@override String get empty => '内容池是空的。\n流量经过代理后，被判定过的内容会出现在这里。';
	@override String get loading => '正在加载内容池';
	@override String loadFailed({required Object error}) => '无法加载内容池：${error}';
	@override String footer({required Object count}) => '共 ${count} 行。每一行是同一段内容的全部判定：先看摇摆的，登记一次即对这段内容的每一次出现生效。';
	@override String occurrenceFailed({required Object error}) => '无法加载出现记录：${error}';
	@override String get occurrencesLabel => '出现';
	@override String occurrences({required Object count, required Object turns}) => '${count} 次出现 · ${turns} 个 turn';
	@override String get decisionsLabel => '判定';
	@override String decisions({required Object redacted, required Object released}) => '抹去 ${redacted} · 放行 ${released}';
	@override String get confidenceLabel => '置信度';
	@override String confidence({required Object value}) => '置信度 ${value}';
	@override String confidenceRange({required Object min, required Object max}) => '置信度 ${min}–${max}';
	@override String get lastSeenLabel => '最近出现';
	@override String lastSeen({required Object time}) => '最近 ${time}';
	@override String get categories => '识别为';
	@override String get firstSeenLabel => '首次出现';
	@override String get registered => '已登记';
	@override String get registeredNone => '未登记';
	@override String get registeredRelease => '已登记：放行';
	@override String get registeredRedact => '已登记：禁止';
	@override String get release => '放行';
	@override String get deny => '禁止';
	@override String get releasedCreated => '已放行，并登记为放行规则';
	@override String get releasedExisting => '该内容此前已放行，无需重复登记';
	@override String releaseFailed({required Object error}) => '放行失败：${error}';
	@override String get deniedCreated => '已禁止，并登记为强制抹去规则';
	@override String get deniedExisting => '该内容此前已禁止，无需重复登记';
	@override String denyFailed({required Object error}) => '禁止失败：${error}';
	@override String get occurrenceCount => '出现记录';
	@override String get noOccurrences => '这段内容没有留下出现记录。';
	@override String occurrenceSource({required Object protocol, required Object path}) => '${protocol} · ${path}';
	@override String get detailHint => '登记一次，这段内容的每一次出现都按同一方向处理。';
	@override String get contentTitle => '内容详情';
	@override String get turnTitle => '命中详情';
	@override String get original => '原文';
	@override String get forwarded => '转发内容';
	@override String get detectedSpans => '识别到的内容';
	@override String get noDetections => '这一次没有命中任何类别。';
	@override String characters({required Object count}) => '${count} 字符';
	@override String matchedRule({required Object id}) => '命中规则：${id}';
	@override String get time => '时间';
	@override String get protocol => '协议';
	@override String detailFailed({required Object error}) => '无法加载详情：${error}';
}

// Path: rules
class _Translations$rules$zh_CN implements Translations$rules$en {
	_Translations$rules$zh_CN._(this._root);

	final TranslationsZhCn _root; // ignore: unused_field

	// Translations
	@override String get listTitle => '规则';
	@override String get empty => '暂无规则';
	@override String get editList => '编辑';
	@override String get unmatched => '未匹配任何规则时';
	@override String get loading => '正在加载规则';
	@override String loadFailed({required Object error}) => '无法加载规则：${error}';
	@override String get kNew => '新建规则';
	@override String get edit => '编辑规则';
	@override String prioritySummary({required Object priority, required Object condition}) => '优先级 ${priority} · ${condition}';
	@override String toggleFailed({required Object error}) => '切换失败：${error}';
	@override late final _Translations$rules$group$zh_CN group = _Translations$rules$group$zh_CN._(_root);
	@override late final _Translations$rules$footer$zh_CN footer = _Translations$rules$footer$zh_CN._(_root);
	@override String defaultName({required Object group}) => '拦截：${group}';
	@override String get operatorReleaseName => '逐条放行';
	@override String get operatorRedactName => '逐条禁止';
	@override String get evaluationRedact => '判定按优先级从大到小进行；没有规则命中时默认抹去，这是兜底行为，不出现在上面的列表里。';
	@override String get evaluationRelease => '判定按优先级从大到小进行；没有规则命中时默认放行。';
	@override String get nameRequired => '规则名称不能为空';
	@override String get name => '规则名称';
	@override String get matchingLogic => '匹配逻辑';
	@override String get matchingLogicSubtitle => '条件组可以继续嵌套；同一组按“全部满足”或“任一满足”计算。';
	@override String get result => '规则结果';
	@override String get priority => '优先级（数值越大越先判定）';
	@override late final _Translations$rules$filterKind$zh_CN filterKind = _Translations$rules$filterKind$zh_CN._(_root);
	@override String get groupEmpty => '每个条件组至少需要一个条件';
	@override String get keywordRequired => '文本过滤器的匹配内容不能为空';
	@override String get lengthInvalid => '字符数必须是大于等于 0 的整数';
	@override String get confidenceInvalid => '置信度必须是 0 到 1 之间的数字';
	@override String get addFilter => '添加过滤器';
	@override String get addGroup => '添加条件组';
	@override String get keywordPlaceholder => '匹配文本或 Glob 模式';
	@override String get caseSensitive => '区分大小写';
	@override String get lengthPlaceholder => '字符数';
}

// Path: providers
class _Translations$providers$zh_CN implements Translations$providers$en {
	_Translations$providers$zh_CN._(this._root);

	final TranslationsZhCn _root; // ignore: unused_field

	// Translations
	@override String get loading => '正在加载上游';
	@override String loadFailed({required Object error}) => '无法加载上游：${error}';
	@override String get empty => '还没有配置上游。\n至少添加一个，代理才能转发请求。';
	@override String get routingNote => '脱敏接口按协议选择第一个启用的上游；其他 /v1/* 接口直接透传到第一个启用的上游。可用 x-privacy-router-provider 请求头指定上游。';
	@override String get add => '添加上游';
	@override String get edit => '编辑上游';
	@override String get required => '名称与 base url 都不能为空';
	@override String get name => '名称（唯一）';
	@override String get baseUrl => 'Base URL，例如 https://api.openai.com/v1';
	@override String get format => '协议格式';
	@override String endpoint({required Object endpoint}) => '入站端点：${endpoint}';
	@override String get credentialNote => '客户端凭证原样透传。代理不保存或注入 API key。';
}

// Path: navigation.overview
class _Translations$navigation$overview$zh_CN implements Translations$navigation$overview$en {
	_Translations$navigation$overview$zh_CN._(this._root);

	final TranslationsZhCn _root; // ignore: unused_field

	// Translations
	@override String get label => '概览';
	@override String get description => '流量规模、判定结果与类别分布';
}

// Path: navigation.pool
class _Translations$navigation$pool$zh_CN implements Translations$navigation$pool$en {
	_Translations$navigation$pool$zh_CN._(this._root);

	final TranslationsZhCn _root; // ignore: unused_field

	// Translations
	@override String get label => '内容池';
	@override String get description => '复核还没登记过决定的内容，一次登记对每一次出现生效';
}

// Path: navigation.rules
class _Translations$navigation$rules$zh_CN implements Translations$navigation$rules$en {
	_Translations$navigation$rules$zh_CN._(this._root);

	final TranslationsZhCn _root; // ignore: unused_field

	// Translations
	@override String get label => '规则';
	@override String get description => '决定什么被抹去、什么被放行；没有命中时按兜底策略';
}

// Path: navigation.providers
class _Translations$navigation$providers$zh_CN implements Translations$navigation$providers$en {
	_Translations$navigation$providers$zh_CN._(this._root);

	final TranslationsZhCn _root; // ignore: unused_field

	// Translations
	@override String get label => '上游';
	@override String get description => '转发目标与协议；客户端凭证原样透传';
}

// Path: navigation.performance
class _Translations$navigation$performance$zh_CN implements Translations$navigation$performance$en {
	_Translations$navigation$performance$zh_CN._(this._root);

	final TranslationsZhCn _root; // ignore: unused_field

	// Translations
	@override String get label => '性能';
	@override String get description => '推理与转发耗时、结果缓存命中率';
}

// Path: model.entity
class _Translations$model$entity$zh_CN implements Translations$model$entity$en {
	_Translations$model$entity$zh_CN._(this._root);

	final TranslationsZhCn _root; // ignore: unused_field

	// Translations
	@override String get accountNumber => '智能识别·账号';
	@override String get bankCard => '确定性规则·银行卡';
	@override String get creditCode => '确定性规则·中国大陆统一社会信用代码';
	@override String get idCard => '确定性规则·中国大陆身份证';
	@override String get ipAddress => '确定性规则·IP 地址';
	@override String get macAddress => '确定性规则·MAC 地址';
	@override String get privateAddress => '智能识别·地址';
	@override String get privateDate => '智能识别·日期';
	@override String get privateEmail => '确定性规则·邮箱';
	@override String get privatePerson => '智能识别·姓名';
	@override String get privatePhone => '智能识别·电话';
	@override String get privateUrl => '智能识别·网址';
	@override String get secret => '智能识别·密钥';
	@override String get unknown => '未知';
}

// Path: model.decision
class _Translations$model$decision$zh_CN implements Translations$model$decision$en {
	_Translations$model$decision$zh_CN._(this._root);

	final TranslationsZhCn _root; // ignore: unused_field

	// Translations
	@override String get release => '放行';
	@override String get redact => '抹去';
}

// Path: model.source
class _Translations$model$source$zh_CN implements Translations$model$source$en {
	_Translations$model$source$zh_CN._(this._root);

	final TranslationsZhCn _root; // ignore: unused_field

	// Translations
	@override String get builtin => '内置';
	@override String get console => '自定义';
	@override String get operator => '管理员登记';
}

// Path: model.pattern
class _Translations$model$pattern$zh_CN implements Translations$model$pattern$en {
	_Translations$model$pattern$zh_CN._(this._root);

	final TranslationsZhCn _root; // ignore: unused_field

	// Translations
	@override String get exact => '完全相同';
	@override String get substring => '包含文本';
	@override String get glob => '通配模式';
}

// Path: model.comparison
class _Translations$model$comparison$zh_CN implements Translations$model$comparison$en {
	_Translations$model$comparison$zh_CN._(this._root);

	final TranslationsZhCn _root; // ignore: unused_field

	// Translations
	@override String get greater => '大于';
	@override String get equal => '等于';
	@override String get less => '小于';
}

// Path: model.logic
class _Translations$model$logic$zh_CN implements Translations$model$logic$en {
	_Translations$model$logic$zh_CN._(this._root);

	final TranslationsZhCn _root; // ignore: unused_field

	// Translations
	@override String get all => '全部满足';
	@override String get any => '任一满足';
}

// Path: model.apiFormat
class _Translations$model$apiFormat$zh_CN implements Translations$model$apiFormat$en {
	_Translations$model$apiFormat$zh_CN._(this._root);

	final TranslationsZhCn _root; // ignore: unused_field

	// Translations
	@override String get openaiResponses => 'OpenAI Responses';
	@override String get openaiChat => 'OpenAI Chat';
	@override String get anthropicMessages => 'Anthropic Messages';
}

// Path: model.filterSummary
class _Translations$model$filterSummary$zh_CN implements Translations$model$filterSummary$en {
	_Translations$model$filterSummary$zh_CN._(this._root);

	final TranslationsZhCn _root; // ignore: unused_field

	// Translations
	@override String entity({required Object group}) => '类别是${group}';
	@override String keyword({required Object operator, required Object text}) => '文本${operator}「${text}」';
	@override String characterLength({required Object operator, required Object value}) => '字符数${operator}${value}';
	@override String confidence({required Object operator, required Object value}) => '置信度${operator}${value}';
	@override String get allJoiner => ' 且 ';
	@override String get anyJoiner => ' 或 ';
}

// Path: overview.window
class _Translations$overview$window$zh_CN implements Translations$overview$window$en {
	_Translations$overview$window$zh_CN._(this._root);

	final TranslationsZhCn _root; // ignore: unused_field

	// Translations
	@override String get oneHour => '1 小时';
	@override String get day => '24 小时';
	@override String get week => '7 天';
	@override String get month => '30 天';
}

// Path: rules.group
class _Translations$rules$group$zh_CN implements Translations$rules$group$en {
	_Translations$rules$group$zh_CN._(this._root);

	final TranslationsZhCn _root; // ignore: unused_field

	// Translations
	@override String get builtin => '内置规则';
	@override String get console => '自定义规则';
	@override String get operator => '管理员登记';
}

// Path: rules.footer
class _Translations$rules$footer$zh_CN implements Translations$rules$footer$en {
	_Translations$rules$footer$zh_CN._(this._root);

	final TranslationsZhCn _root; // ignore: unused_field

	// Translations
	@override String get builtin => '默认规则可以编辑、停用或删除。';
	@override String get operator => '每条规则只作用于完全相同的一段文本。';
}

// Path: rules.filterKind
class _Translations$rules$filterKind$zh_CN implements Translations$rules$filterKind$en {
	_Translations$rules$filterKind$zh_CN._(this._root);

	final TranslationsZhCn _root; // ignore: unused_field

	// Translations
	@override String get entity => '类别';
	@override String get keyword => '文本';
	@override String get characterLength => '字符数';
	@override String get confidence => '置信度';
}

/// The flat map containing all translations for locale <zh-CN>.
/// Only for edge cases! For simple maps, use the map function of this library.
///
/// The Dart AOT compiler has issues with very large switch statements,
/// so the map is split into smaller functions (512 entries each).
extension on TranslationsZhCn {
	dynamic _flatMapFunction(String path) {
		return switch (path) {
			'demo.banner' => '交互演示 · 示例数据',
			'demo.detail' => '刷新后重置修改。不会向上游发送请求。',
			'demo.credentials' => ({required Object username, required Object password}) => '演示账号：${username} / ${password}，无需真实凭据。',
			'demo.reset' => '重置演示',
			'demo.enter' => '进入演示',
			'demo.loginTitle' => '体验 Privacy Router',
			'demo.tagline' => '在示例工作区中体验隐私管理功能',
			'language' => '语言',
			'app.title' => 'Privacy Router',
			'app.tagline' => '本地隐私识别与脱敏转发',
			'common.search' => '搜索',
			'common.cancel' => '取消',
			'common.delete' => '删除',
			'common.deleteConfirmation' => '删除这条规则？此操作无法撤销。',
			'common.retry' => '重试',
			'common.save' => '保存',
			'common.deleteFailed' => ({required Object error}) => '删除失败：${error}',
			'common.saveFailed' => ({required Object error}) => '保存失败：${error}',
			'common.enabled' => '已启用',
			'common.disabled' => '已停用',
			'common.account' => '账户',
			'common.accountMenu' => ({required Object username}) => '${username}，账户菜单',
			'common.appearance' => ({required Object value}) => '外观：${value}',
			'common.signOut' => '退出登录',
			'appearance.system' => '跟随系统',
			'appearance.light' => '浅色',
			'appearance.dark' => '深色',
			'navigation.overview.label' => '概览',
			'navigation.overview.description' => '流量规模、判定结果与类别分布',
			'navigation.pool.label' => '内容池',
			'navigation.pool.description' => '复核还没登记过决定的内容，一次登记对每一次出现生效',
			'navigation.rules.label' => '规则',
			'navigation.rules.description' => '决定什么被抹去、什么被放行；没有命中时按兜底策略',
			'navigation.providers.label' => '上游',
			'navigation.providers.description' => '转发目标与协议；客户端凭证原样透传',
			'navigation.performance.label' => '性能',
			'navigation.performance.description' => '推理与转发耗时、结果缓存命中率',
			'navigation.expandSidebar' => '展开侧边栏',
			'navigation.collapseSidebar' => '收起侧边栏',
			'model.entity.accountNumber' => '智能识别·账号',
			'model.entity.bankCard' => '确定性规则·银行卡',
			'model.entity.creditCode' => '确定性规则·中国大陆统一社会信用代码',
			'model.entity.idCard' => '确定性规则·中国大陆身份证',
			'model.entity.ipAddress' => '确定性规则·IP 地址',
			'model.entity.macAddress' => '确定性规则·MAC 地址',
			'model.entity.privateAddress' => '智能识别·地址',
			'model.entity.privateDate' => '智能识别·日期',
			'model.entity.privateEmail' => '确定性规则·邮箱',
			'model.entity.privatePerson' => '智能识别·姓名',
			'model.entity.privatePhone' => '智能识别·电话',
			'model.entity.privateUrl' => '智能识别·网址',
			'model.entity.secret' => '智能识别·密钥',
			'model.entity.unknown' => '未知',
			'model.decision.release' => '放行',
			'model.decision.redact' => '抹去',
			'model.source.builtin' => '内置',
			'model.source.console' => '自定义',
			'model.source.operator' => '管理员登记',
			'model.pattern.exact' => '完全相同',
			'model.pattern.substring' => '包含文本',
			'model.pattern.glob' => '通配模式',
			'model.comparison.greater' => '大于',
			'model.comparison.equal' => '等于',
			'model.comparison.less' => '小于',
			'model.logic.all' => '全部满足',
			'model.logic.any' => '任一满足',
			'model.apiFormat.openaiResponses' => 'OpenAI Responses',
			'model.apiFormat.openaiChat' => 'OpenAI Chat',
			'model.apiFormat.anthropicMessages' => 'Anthropic Messages',
			'model.filterSummary.entity' => ({required Object group}) => '类别是${group}',
			'model.filterSummary.keyword' => ({required Object operator, required Object text}) => '文本${operator}「${text}」',
			'model.filterSummary.characterLength' => ({required Object operator, required Object value}) => '字符数${operator}${value}',
			'model.filterSummary.confidence' => ({required Object operator, required Object value}) => '置信度${operator}${value}',
			'model.filterSummary.allJoiner' => ' 且 ',
			'model.filterSummary.anyJoiner' => ' 或 ',
			'model.unknownFilter' => ({required Object kind}) => '未知规则过滤器：${kind}',
			'login.username' => '管理员账号',
			'login.password' => '口令',
			'login.submit' => '登录',
			'login.unreachable' => '无法连接到代理。请确认它已在运行。',
			'login.providerMissing' => '尚未配置上游 provider，登录后在「上游」页添加。',
			'setup.title' => '创建管理员',
			'setup.subtitle' => '这个代理还没有管理员账号，在这里设置一个即可开始使用。',
			'setup.username' => '管理员账号',
			'setup.password' => '口令',
			'setup.confirmation' => '重复口令',
			'setup.submit' => '创建并登录',
			'setup.usernameRequired' => '管理员账号不能为空',
			'setup.passwordRequired' => '口令不能为空',
			'setup.passwordMismatch' => '两次输入的口令不一致',
			'setup.localOnly' => '初始化只能在本机完成：如果你不在运行代理的这台电脑上，请回到那台机器上打开这个页面。',
			'overview.window.oneHour' => '1 小时',
			'overview.window.day' => '24 小时',
			'overview.window.week' => '7 天',
			'overview.window.month' => '30 天',
			'overview.loading' => '正在加载统计',
			'overview.loadFailed' => ({required Object error}) => '无法加载统计：${error}',
			'overview.requests' => '请求',
			'overview.requestsSubtitle' => '窗口内完成的代理请求',
			'overview.fragments' => '判定片段',
			'overview.fragmentsSubtitle' => '进入内容池的文本段',
			'overview.detections' => '命中',
			'overview.detectionBreakdown' => ({required Object redacted, required Object released}) => '抹去 ${redacted} · 放行 ${released}',
			'overview.redactedRatio' => '抹去占比',
			'overview.redactedRatioSubtitle' => '命中中未放行的比例',
			'overview.distribution' => '类别分布',
			'overview.distributionSubtitle' => '窗口内每一类被识别出来的次数',
			'overview.empty' => '窗口内还没有命中',
			'overview.emptySubtitle' => '流量经过代理后会在这里出现',
			'performance.firstResult' => '首结果耗时',
			'performance.firstResultSubtitle' => '本地推理开始至首个识别结果的平均耗时',
			'performance.throughput' => '模型吞吐',
			'performance.throughputSubtitle' => '处理 token 数 / 模型前向执行时间',
			'performance.modelTokens' => '已处理 Token',
			'performance.modelTokensSubtitle' => '实际模型工作量，不计缓存命中',
			'performance.tokenization' => '分词',
			'performance.validation' => '校验与批次规划',
			'performance.forward' => '模型前向计算',
			'performance.decoding' => '实体解码',
			'performance.loading' => '正在加载性能数据',
			'performance.loadFailed' => ({required Object error}) => '无法加载性能数据：${error}',
			'performance.average' => '平均端到端',
			'performance.averageSubtitle' => '排队 + 识别 + 转发',
			'performance.percentileSubtitle' => '最近秩法分位',
			'performance.maximum' => '最长',
			'performance.maximumSubtitle' => '窗口内最慢的一次',
			'performance.breakdown' => '耗时分解',
			'performance.inference' => '识别（推理）',
			'performance.inferenceSubtitle' => '本地模型前向 + 结果缓存命中',
			'performance.upstream' => '转发（上游）',
			'performance.upstreamSubtitle' => '脱敏后的请求送达上游并取得响应',
			'performance.cacheHit' => '缓存命中',
			'performance.cacheBreakdown' => ({required Object cached, required Object inferred}) => '命中 ${cached} · 推理 ${inferred} 个片段',
			'performance.note' => '模型指标仅统计实际推理，缓存命中不计入吞吐。转发耗时记录到上游响应头，响应体保持流式透传。',
			'pool.filterUnreviewed' => '未审核',
			'pool.filterReviewed' => '已审核',
			'pool.filterAll' => '全部内容',
			'pool.previous' => '上一页',
			'pool.next' => '下一页',
			'pool.refresh' => '刷新',
			'pool.range' => ({required Object start, required Object end, required Object total}) => '第 ${start}–${end} 条，共 ${total} 条',
			'pool.emptyPage' => '本页暂无记录，请返回上一页或刷新。',
			'pool.emptyUnreviewed' => '没有待审核的内容。\n判定过的内容都已经登记过决定。',
			'pool.emptyReviewed' => '还没有已审核的内容。\n登记过放行或禁止的内容会出现在这里。',
			'pool.empty' => '内容池是空的。\n流量经过代理后，被判定过的内容会出现在这里。',
			'pool.loading' => '正在加载内容池',
			'pool.loadFailed' => ({required Object error}) => '无法加载内容池：${error}',
			'pool.footer' => ({required Object count}) => '共 ${count} 行。每一行是同一段内容的全部判定：先看摇摆的，登记一次即对这段内容的每一次出现生效。',
			'pool.occurrenceFailed' => ({required Object error}) => '无法加载出现记录：${error}',
			'pool.occurrencesLabel' => '出现',
			'pool.occurrences' => ({required Object count, required Object turns}) => '${count} 次出现 · ${turns} 个 turn',
			'pool.decisionsLabel' => '判定',
			'pool.decisions' => ({required Object redacted, required Object released}) => '抹去 ${redacted} · 放行 ${released}',
			'pool.confidenceLabel' => '置信度',
			'pool.confidence' => ({required Object value}) => '置信度 ${value}',
			'pool.confidenceRange' => ({required Object min, required Object max}) => '置信度 ${min}–${max}',
			'pool.lastSeenLabel' => '最近出现',
			'pool.lastSeen' => ({required Object time}) => '最近 ${time}',
			'pool.categories' => '识别为',
			'pool.firstSeenLabel' => '首次出现',
			'pool.registered' => '已登记',
			'pool.registeredNone' => '未登记',
			'pool.registeredRelease' => '已登记：放行',
			'pool.registeredRedact' => '已登记：禁止',
			'pool.release' => '放行',
			'pool.deny' => '禁止',
			'pool.releasedCreated' => '已放行，并登记为放行规则',
			'pool.releasedExisting' => '该内容此前已放行，无需重复登记',
			'pool.releaseFailed' => ({required Object error}) => '放行失败：${error}',
			'pool.deniedCreated' => '已禁止，并登记为强制抹去规则',
			'pool.deniedExisting' => '该内容此前已禁止，无需重复登记',
			'pool.denyFailed' => ({required Object error}) => '禁止失败：${error}',
			'pool.occurrenceCount' => '出现记录',
			'pool.noOccurrences' => '这段内容没有留下出现记录。',
			'pool.occurrenceSource' => ({required Object protocol, required Object path}) => '${protocol} · ${path}',
			'pool.detailHint' => '登记一次，这段内容的每一次出现都按同一方向处理。',
			'pool.contentTitle' => '内容详情',
			'pool.turnTitle' => '命中详情',
			'pool.original' => '原文',
			'pool.forwarded' => '转发内容',
			'pool.detectedSpans' => '识别到的内容',
			'pool.noDetections' => '这一次没有命中任何类别。',
			'pool.characters' => ({required Object count}) => '${count} 字符',
			'pool.matchedRule' => ({required Object id}) => '命中规则：${id}',
			'pool.time' => '时间',
			'pool.protocol' => '协议',
			'pool.detailFailed' => ({required Object error}) => '无法加载详情：${error}',
			'rules.listTitle' => '规则',
			'rules.empty' => '暂无规则',
			'rules.editList' => '编辑',
			'rules.unmatched' => '未匹配任何规则时',
			'rules.loading' => '正在加载规则',
			'rules.loadFailed' => ({required Object error}) => '无法加载规则：${error}',
			'rules.kNew' => '新建规则',
			'rules.edit' => '编辑规则',
			'rules.prioritySummary' => ({required Object priority, required Object condition}) => '优先级 ${priority} · ${condition}',
			'rules.toggleFailed' => ({required Object error}) => '切换失败：${error}',
			'rules.group.builtin' => '内置规则',
			'rules.group.console' => '自定义规则',
			'rules.group.operator' => '管理员登记',
			'rules.footer.builtin' => '默认规则可以编辑、停用或删除。',
			'rules.footer.operator' => '每条规则只作用于完全相同的一段文本。',
			'rules.defaultName' => ({required Object group}) => '拦截：${group}',
			'rules.operatorReleaseName' => '逐条放行',
			'rules.operatorRedactName' => '逐条禁止',
			'rules.evaluationRedact' => '判定按优先级从大到小进行；没有规则命中时默认抹去，这是兜底行为，不出现在上面的列表里。',
			'rules.evaluationRelease' => '判定按优先级从大到小进行；没有规则命中时默认放行。',
			'rules.nameRequired' => '规则名称不能为空',
			'rules.name' => '规则名称',
			'rules.matchingLogic' => '匹配逻辑',
			'rules.matchingLogicSubtitle' => '条件组可以继续嵌套；同一组按“全部满足”或“任一满足”计算。',
			'rules.result' => '规则结果',
			'rules.priority' => '优先级（数值越大越先判定）',
			'rules.filterKind.entity' => '类别',
			'rules.filterKind.keyword' => '文本',
			'rules.filterKind.characterLength' => '字符数',
			'rules.filterKind.confidence' => '置信度',
			'rules.groupEmpty' => '每个条件组至少需要一个条件',
			'rules.keywordRequired' => '文本过滤器的匹配内容不能为空',
			'rules.lengthInvalid' => '字符数必须是大于等于 0 的整数',
			'rules.confidenceInvalid' => '置信度必须是 0 到 1 之间的数字',
			'rules.addFilter' => '添加过滤器',
			'rules.addGroup' => '添加条件组',
			'rules.keywordPlaceholder' => '匹配文本或 Glob 模式',
			'rules.caseSensitive' => '区分大小写',
			'rules.lengthPlaceholder' => '字符数',
			'providers.loading' => '正在加载上游',
			'providers.loadFailed' => ({required Object error}) => '无法加载上游：${error}',
			'providers.empty' => '还没有配置上游。\n至少添加一个，代理才能转发请求。',
			'providers.routingNote' => '脱敏接口按协议选择第一个启用的上游；其他 /v1/* 接口直接透传到第一个启用的上游。可用 x-privacy-router-provider 请求头指定上游。',
			'providers.add' => '添加上游',
			'providers.edit' => '编辑上游',
			'providers.required' => '名称与 base url 都不能为空',
			'providers.name' => '名称（唯一）',
			'providers.baseUrl' => 'Base URL，例如 https://api.openai.com/v1',
			'providers.format' => '协议格式',
			'providers.endpoint' => ({required Object endpoint}) => '入站端点：${endpoint}',
			'providers.credentialNote' => '客户端凭证原样透传。代理不保存或注入 API key。',
			_ => null,
		};
	}
}
