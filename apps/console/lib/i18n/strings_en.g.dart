///
/// Generated file. Do not edit.
///
// coverage:ignore-file
// ignore_for_file: type=lint, unused_import
// dart format off

part of 'strings.g.dart';

// Path: <root>
typedef TranslationsEn = Translations; // ignore: unused_element
class Translations with BaseTranslations<AppLocale, Translations> {
	/// Returns the current translations of the given [context].
	///
	/// Usage:
	/// final t = Translations.of(context);
	static Translations of(BuildContext context) => InheritedLocaleData.of<AppLocale, Translations>(context).translations;

	/// You can call this constructor and build your own translation instance of this locale.
	/// Constructing via the enum [AppLocale.build] is preferred.
	Translations({Map<String, Node>? overrides, PluralResolver? cardinalResolver, PluralResolver? ordinalResolver, TranslationMetadata<AppLocale, Translations>? meta})
		: assert(overrides == null, 'Set "translation_overrides: true" in order to enable this feature.'),
		  _meta = meta ?? TranslationMetadata(
		    locale: AppLocale.en,
		    overrides: overrides ?? {},
		    cardinalResolver: cardinalResolver,
		    ordinalResolver: ordinalResolver,
		  ) {
		_meta.setFlatMapFunction(_flatMapFunction);
	}

	/// Metadata for the translations of <en>.
	final TranslationMetadata<AppLocale, Translations> _meta;
	@override TranslationMetadata<AppLocale, Translations> get $meta => _meta;

	/// Access flat map
	dynamic operator[](String key) => _meta.getTranslation(key);

	late final Translations _root = this; // ignore: unused_field

	Translations $copyWith({TranslationMetadata<AppLocale, Translations>? meta}) => Translations(meta: meta ?? this.$meta);

	// Translations
	late final Translations$demo$en demo = Translations$demo$en._(_root);

	/// en: 'Language'
	String get language => 'Language';

	late final Translations$app$en app = Translations$app$en._(_root);
	late final Translations$common$en common = Translations$common$en._(_root);
	late final Translations$appearance$en appearance = Translations$appearance$en._(_root);
	late final Translations$navigation$en navigation = Translations$navigation$en._(_root);
	late final Translations$model$en model = Translations$model$en._(_root);
	late final Translations$login$en login = Translations$login$en._(_root);
	late final Translations$setup$en setup = Translations$setup$en._(_root);
	late final Translations$overview$en overview = Translations$overview$en._(_root);
	late final Translations$performance$en performance = Translations$performance$en._(_root);
	late final Translations$pool$en pool = Translations$pool$en._(_root);
	late final Translations$rules$en rules = Translations$rules$en._(_root);
	late final Translations$providers$en providers = Translations$providers$en._(_root);
}

// Path: demo
class Translations$demo$en {
	Translations$demo$en._(this._root);

	final Translations _root; // ignore: unused_field

	// Translations

	/// en: 'Interactive demo · Sample data'
	String get banner => 'Interactive demo · Sample data';

	/// en: 'Changes reset on reload. No requests are sent to providers.'
	String get detail => 'Changes reset on reload. No requests are sent to providers.';

	/// en: 'Demo login: $username / $password. No real credentials needed.'
	String credentials({required Object username, required Object password}) => 'Demo login: ${username} / ${password}. No real credentials needed.';

	/// en: 'Reset demo'
	String get reset => 'Reset demo';

	/// en: 'Enter demo'
	String get enter => 'Enter demo';

	/// en: 'Try Privacy Router'
	String get loginTitle => 'Try Privacy Router';

	/// en: 'Explore privacy controls with a sample workspace'
	String get tagline => 'Explore privacy controls with a sample workspace';
}

// Path: app
class Translations$app$en {
	Translations$app$en._(this._root);

	final Translations _root; // ignore: unused_field

	// Translations

	/// en: 'Privacy Router'
	String get title => 'Privacy Router';

	/// en: 'Local privacy detection and redaction proxy'
	String get tagline => 'Local privacy detection and redaction proxy';
}

// Path: common
class Translations$common$en {
	Translations$common$en._(this._root);

	final Translations _root; // ignore: unused_field

	// Translations

	/// en: 'Search'
	String get search => 'Search';

	/// en: 'Cancel'
	String get cancel => 'Cancel';

	/// en: 'Delete'
	String get delete => 'Delete';

	/// en: 'Delete this rule? This cannot be undone.'
	String get deleteConfirmation => 'Delete this rule? This cannot be undone.';

	/// en: 'Retry'
	String get retry => 'Retry';

	/// en: 'Save'
	String get save => 'Save';

	/// en: 'Delete failed: $error'
	String deleteFailed({required Object error}) => 'Delete failed: ${error}';

	/// en: 'Save failed: $error'
	String saveFailed({required Object error}) => 'Save failed: ${error}';

	/// en: 'Enabled'
	String get enabled => 'Enabled';

	/// en: 'Disabled'
	String get disabled => 'Disabled';

	/// en: 'Account'
	String get account => 'Account';

	/// en: '$username, account menu'
	String accountMenu({required Object username}) => '${username}, account menu';

	/// en: 'Appearance: $value'
	String appearance({required Object value}) => 'Appearance: ${value}';

	/// en: 'Sign out'
	String get signOut => 'Sign out';
}

// Path: appearance
class Translations$appearance$en {
	Translations$appearance$en._(this._root);

	final Translations _root; // ignore: unused_field

	// Translations

	/// en: 'System'
	String get system => 'System';

	/// en: 'Light'
	String get light => 'Light';

	/// en: 'Dark'
	String get dark => 'Dark';
}

// Path: navigation
class Translations$navigation$en {
	Translations$navigation$en._(this._root);

	final Translations _root; // ignore: unused_field

	// Translations
	late final Translations$navigation$overview$en overview = Translations$navigation$overview$en._(_root);
	late final Translations$navigation$pool$en pool = Translations$navigation$pool$en._(_root);
	late final Translations$navigation$rules$en rules = Translations$navigation$rules$en._(_root);
	late final Translations$navigation$providers$en providers = Translations$navigation$providers$en._(_root);
	late final Translations$navigation$performance$en performance = Translations$navigation$performance$en._(_root);

	/// en: 'Expand sidebar'
	String get expandSidebar => 'Expand sidebar';

	/// en: 'Collapse sidebar'
	String get collapseSidebar => 'Collapse sidebar';
}

// Path: model
class Translations$model$en {
	Translations$model$en._(this._root);

	final Translations _root; // ignore: unused_field

	// Translations
	late final Translations$model$entity$en entity = Translations$model$entity$en._(_root);
	late final Translations$model$decision$en decision = Translations$model$decision$en._(_root);
	late final Translations$model$source$en source = Translations$model$source$en._(_root);
	late final Translations$model$pattern$en pattern = Translations$model$pattern$en._(_root);
	late final Translations$model$comparison$en comparison = Translations$model$comparison$en._(_root);
	late final Translations$model$logic$en logic = Translations$model$logic$en._(_root);
	late final Translations$model$apiFormat$en apiFormat = Translations$model$apiFormat$en._(_root);
	late final Translations$model$filterSummary$en filterSummary = Translations$model$filterSummary$en._(_root);

	/// en: 'Unknown rule filter: $kind'
	String unknownFilter({required Object kind}) => 'Unknown rule filter: ${kind}';
}

// Path: login
class Translations$login$en {
	Translations$login$en._(this._root);

	final Translations _root; // ignore: unused_field

	// Translations

	/// en: 'Administrator account'
	String get username => 'Administrator account';

	/// en: 'Password'
	String get password => 'Password';

	/// en: 'Sign in'
	String get submit => 'Sign in';

	/// en: 'Cannot connect to the proxy. Make sure it is running.'
	String get unreachable => 'Cannot connect to the proxy. Make sure it is running.';

	/// en: 'No upstream is configured. Add one on the Upstreams page after signing in.'
	String get providerMissing => 'No upstream is configured. Add one on the Upstreams page after signing in.';
}

// Path: setup
class Translations$setup$en {
	Translations$setup$en._(this._root);

	final Translations _root; // ignore: unused_field

	// Translations

	/// en: 'Create administrator'
	String get title => 'Create administrator';

	/// en: 'This proxy has no administrator account yet. Set one up here.'
	String get subtitle => 'This proxy has no administrator account yet. Set one up here.';

	/// en: 'Administrator account'
	String get username => 'Administrator account';

	/// en: 'Password'
	String get password => 'Password';

	/// en: 'Repeat password'
	String get confirmation => 'Repeat password';

	/// en: 'Create and sign in'
	String get submit => 'Create and sign in';

	/// en: 'Administrator account must not be empty'
	String get usernameRequired => 'Administrator account must not be empty';

	/// en: 'Password must not be empty'
	String get passwordRequired => 'Password must not be empty';

	/// en: 'The two passwords do not match'
	String get passwordMismatch => 'The two passwords do not match';

	/// en: 'Setup can only be completed on the machine running the proxy. Open this page there if you are on another device.'
	String get localOnly => 'Setup can only be completed on the machine running the proxy. Open this page there if you are on another device.';
}

// Path: overview
class Translations$overview$en {
	Translations$overview$en._(this._root);

	final Translations _root; // ignore: unused_field

	// Translations
	late final Translations$overview$window$en window = Translations$overview$window$en._(_root);

	/// en: 'Loading statistics'
	String get loading => 'Loading statistics';

	/// en: 'Unable to load statistics: $error'
	String loadFailed({required Object error}) => 'Unable to load statistics: ${error}';

	/// en: 'Requests'
	String get requests => 'Requests';

	/// en: 'Proxy requests completed in this window'
	String get requestsSubtitle => 'Proxy requests completed in this window';

	/// en: 'Evaluated fragments'
	String get fragments => 'Evaluated fragments';

	/// en: 'Text fragments sent to the content pool'
	String get fragmentsSubtitle => 'Text fragments sent to the content pool';

	/// en: 'Detections'
	String get detections => 'Detections';

	/// en: 'Redacted $redacted · Released $released'
	String detectionBreakdown({required Object redacted, required Object released}) => 'Redacted ${redacted} · Released ${released}';

	/// en: 'Redacted'
	String get redactedRatio => 'Redacted';

	/// en: 'Share of detections not released'
	String get redactedRatioSubtitle => 'Share of detections not released';

	/// en: 'Category distribution'
	String get distribution => 'Category distribution';

	/// en: 'Detections by category in this window'
	String get distributionSubtitle => 'Detections by category in this window';

	/// en: 'No detections in this window'
	String get empty => 'No detections in this window';

	/// en: 'Detections appear here after traffic passes through the proxy'
	String get emptySubtitle => 'Detections appear here after traffic passes through the proxy';
}

// Path: performance
class Translations$performance$en {
	Translations$performance$en._(this._root);

	final Translations _root; // ignore: unused_field

	// Translations

	/// en: 'First result'
	String get firstResult => 'First result';

	/// en: 'Mean local inference time to first result'
	String get firstResultSubtitle => 'Mean local inference time to first result';

	/// en: 'Model throughput'
	String get throughput => 'Model throughput';

	/// en: 'Processed tokens / forward execution time'
	String get throughputSubtitle => 'Processed tokens / forward execution time';

	/// en: 'Processed tokens'
	String get modelTokens => 'Processed tokens';

	/// en: 'Actual model work; cache hits excluded'
	String get modelTokensSubtitle => 'Actual model work; cache hits excluded';

	/// en: 'Tokenization'
	String get tokenization => 'Tokenization';

	/// en: 'Validation and batch planning'
	String get validation => 'Validation and batch planning';

	/// en: 'Model forward pass'
	String get forward => 'Model forward pass';

	/// en: 'Entity decoding'
	String get decoding => 'Entity decoding';

	/// en: 'Loading performance data'
	String get loading => 'Loading performance data';

	/// en: 'Unable to load performance data: $error'
	String loadFailed({required Object error}) => 'Unable to load performance data: ${error}';

	/// en: 'Average end-to-end'
	String get average => 'Average end-to-end';

	/// en: 'Queue + detection + forwarding'
	String get averageSubtitle => 'Queue + detection + forwarding';

	/// en: 'Nearest-rank percentile'
	String get percentileSubtitle => 'Nearest-rank percentile';

	/// en: 'Maximum'
	String get maximum => 'Maximum';

	/// en: 'Slowest request in this window'
	String get maximumSubtitle => 'Slowest request in this window';

	/// en: 'Latency breakdown'
	String get breakdown => 'Latency breakdown';

	/// en: 'Detection (inference)'
	String get inference => 'Detection (inference)';

	/// en: 'Local model forward pass + result cache hit'
	String get inferenceSubtitle => 'Local model forward pass + result cache hit';

	/// en: 'Forwarding (upstream)'
	String get upstream => 'Forwarding (upstream)';

	/// en: 'Send redacted request upstream and receive its response'
	String get upstreamSubtitle => 'Send redacted request upstream and receive its response';

	/// en: 'Cache hit'
	String get cacheHit => 'Cache hit';

	/// en: 'Cached $cached · Inferred $inferred fragments'
	String cacheBreakdown({required Object cached, required Object inferred}) => 'Cached ${cached} · Inferred ${inferred} fragments';

	/// en: 'Detection latency includes cached fragments; cache hits do not enter the model and therefore do not increase it. Forwarding latency is one upstream round trip. Responses, including SSE, are streamed unchanged without buffering or rewriting.'
	String get note => 'Detection latency includes cached fragments; cache hits do not enter the model and therefore do not increase it. Forwarding latency is one upstream round trip. Responses, including SSE, are streamed unchanged without buffering or rewriting.';
}

// Path: pool
class Translations$pool$en {
	Translations$pool$en._(this._root);

	final Translations _root; // ignore: unused_field

	// Translations

	/// en: 'Unreviewed'
	String get filterUnreviewed => 'Unreviewed';

	/// en: 'Reviewed'
	String get filterReviewed => 'Reviewed';

	/// en: 'All content'
	String get filterAll => 'All content';

	/// en: 'Previous'
	String get previous => 'Previous';

	/// en: 'Next'
	String get next => 'Next';

	/// en: 'Refresh'
	String get refresh => 'Refresh';

	/// en: '$start–$end of $total'
	String range({required Object start, required Object end, required Object total}) => '${start}–${end} of ${total}';

	/// en: 'No records on this page. Return to the previous page or refresh.'
	String get emptyPage => 'No records on this page. Return to the previous page or refresh.';

	/// en: 'Nothing is waiting for review. Everything evaluated already has a registered decision.'
	String get emptyUnreviewed => 'Nothing is waiting for review. Everything evaluated already has a registered decision.';

	/// en: 'Nothing has been reviewed yet. Content you release or deny shows up here.'
	String get emptyReviewed => 'Nothing has been reviewed yet. Content you release or deny shows up here.';

	/// en: 'The content pool is empty. Evaluated content appears here after traffic passes through the proxy.'
	String get empty => 'The content pool is empty.\nEvaluated content appears here after traffic passes through the proxy.';

	/// en: 'Loading content'
	String get loading => 'Loading content';

	/// en: 'Unable to load content: $error'
	String loadFailed({required Object error}) => 'Unable to load content: ${error}';

	/// en: '$count rows. Each row is every decision about one piece of content: flipping first, and one registration covers every occurrence.'
	String footer({required Object count}) => '${count} rows. Each row is every decision about one piece of content: flipping first, and one registration covers every occurrence.';

	/// en: 'Unable to load occurrences: $error'
	String occurrenceFailed({required Object error}) => 'Unable to load occurrences: ${error}';

	/// en: 'Occurrences'
	String get occurrencesLabel => 'Occurrences';

	/// en: '$count occurrences · $turns turns'
	String occurrences({required Object count, required Object turns}) => '${count} occurrences · ${turns} turns';

	/// en: 'Decisions'
	String get decisionsLabel => 'Decisions';

	/// en: '$redacted redacted · $released released'
	String decisions({required Object redacted, required Object released}) => '${redacted} redacted · ${released} released';

	/// en: 'Confidence'
	String get confidenceLabel => 'Confidence';

	/// en: 'Confidence $value'
	String confidence({required Object value}) => 'Confidence ${value}';

	/// en: 'Confidence $min–$max'
	String confidenceRange({required Object min, required Object max}) => 'Confidence ${min}–${max}';

	/// en: 'Last seen'
	String get lastSeenLabel => 'Last seen';

	/// en: 'Last $time'
	String lastSeen({required Object time}) => 'Last ${time}';

	/// en: 'Detected as'
	String get categories => 'Detected as';

	/// en: 'First seen'
	String get firstSeenLabel => 'First seen';

	/// en: 'Registered'
	String get registered => 'Registered';

	/// en: 'Not registered'
	String get registeredNone => 'Not registered';

	/// en: 'Registered: release'
	String get registeredRelease => 'Registered: release';

	/// en: 'Registered: deny'
	String get registeredRedact => 'Registered: deny';

	/// en: 'Release'
	String get release => 'Release';

	/// en: 'Deny'
	String get deny => 'Deny';

	/// en: 'Released and registered as a release rule'
	String get releasedCreated => 'Released and registered as a release rule';

	/// en: 'This content was already released; nothing changed'
	String get releasedExisting => 'This content was already released; nothing changed';

	/// en: 'Release failed: $error'
	String releaseFailed({required Object error}) => 'Release failed: ${error}';

	/// en: 'Denied and registered as a force-redact rule'
	String get deniedCreated => 'Denied and registered as a force-redact rule';

	/// en: 'This content was already denied; nothing changed'
	String get deniedExisting => 'This content was already denied; nothing changed';

	/// en: 'Deny failed: $error'
	String denyFailed({required Object error}) => 'Deny failed: ${error}';

	/// en: 'Occurrences'
	String get occurrenceCount => 'Occurrences';

	/// en: 'No occurrence was recorded for this content.'
	String get noOccurrences => 'No occurrence was recorded for this content.';

	/// en: '$protocol · $path'
	String occurrenceSource({required Object protocol, required Object path}) => '${protocol} · ${path}';

	/// en: 'One registration decides every occurrence of this content.'
	String get detailHint => 'One registration decides every occurrence of this content.';

	/// en: 'Content details'
	String get contentTitle => 'Content details';

	/// en: 'Detection details'
	String get turnTitle => 'Detection details';

	/// en: 'Original'
	String get original => 'Original';

	/// en: 'Forwarded content'
	String get forwarded => 'Forwarded content';

	/// en: 'Detected content'
	String get detectedSpans => 'Detected content';

	/// en: 'This turn has no detected categories.'
	String get noDetections => 'This turn has no detected categories.';

	/// en: '$count characters'
	String characters({required Object count}) => '${count} characters';

	/// en: 'Matched rule: $id'
	String matchedRule({required Object id}) => 'Matched rule: ${id}';

	/// en: 'Time'
	String get time => 'Time';

	/// en: 'Protocol'
	String get protocol => 'Protocol';

	/// en: 'Unable to load details: $error'
	String detailFailed({required Object error}) => 'Unable to load details: ${error}';
}

// Path: rules
class Translations$rules$en {
	Translations$rules$en._(this._root);

	final Translations _root; // ignore: unused_field

	// Translations

	/// en: 'Rules'
	String get listTitle => 'Rules';

	/// en: 'No rules'
	String get empty => 'No rules';

	/// en: 'Edit'
	String get editList => 'Edit';

	/// en: 'When no rule matches'
	String get unmatched => 'When no rule matches';

	/// en: 'Loading rules'
	String get loading => 'Loading rules';

	/// en: 'Unable to load rules: $error'
	String loadFailed({required Object error}) => 'Unable to load rules: ${error}';

	/// en: 'New rule'
	String get kNew => 'New rule';

	/// en: 'Edit rule'
	String get edit => 'Edit rule';

	/// en: 'Priority $priority · $condition'
	String prioritySummary({required Object priority, required Object condition}) => 'Priority ${priority} · ${condition}';

	/// en: 'Update failed: $error'
	String toggleFailed({required Object error}) => 'Update failed: ${error}';

	late final Translations$rules$group$en group = Translations$rules$group$en._(_root);
	late final Translations$rules$footer$en footer = Translations$rules$footer$en._(_root);

	/// en: 'Redact: $group'
	String defaultName({required Object group}) => 'Redact: ${group}';

	/// en: 'Released content'
	String get operatorReleaseName => 'Released content';

	/// en: 'Denied content'
	String get operatorRedactName => 'Denied content';

	/// en: 'Rules are evaluated from highest to lowest priority. If none match, content is redacted as the fallback; the fallback is not listed above.'
	String get evaluationRedact => 'Rules are evaluated from highest to lowest priority. If none match, content is redacted as the fallback; the fallback is not listed above.';

	/// en: 'Rules are evaluated from highest to lowest priority. If none match, content is released.'
	String get evaluationRelease => 'Rules are evaluated from highest to lowest priority. If none match, content is released.';

	/// en: 'Rule name is required'
	String get nameRequired => 'Rule name is required';

	/// en: 'Rule name'
	String get name => 'Rule name';

	/// en: 'Matching logic'
	String get matchingLogic => 'Matching logic';

	/// en: 'Groups may be nested. Each group matches all or any of its children.'
	String get matchingLogicSubtitle => 'Groups may be nested. Each group matches all or any of its children.';

	/// en: 'Rule result'
	String get result => 'Rule result';

	/// en: 'Priority (higher values run first)'
	String get priority => 'Priority (higher values run first)';

	late final Translations$rules$filterKind$en filterKind = Translations$rules$filterKind$en._(_root);

	/// en: 'Each condition group must contain at least one condition'
	String get groupEmpty => 'Each condition group must contain at least one condition';

	/// en: 'Text filter value is required'
	String get keywordRequired => 'Text filter value is required';

	/// en: 'Character count must be an integer greater than or equal to 0'
	String get lengthInvalid => 'Character count must be an integer greater than or equal to 0';

	/// en: 'Confidence must be a number from 0 to 1'
	String get confidenceInvalid => 'Confidence must be a number from 0 to 1';

	/// en: 'Add filter'
	String get addFilter => 'Add filter';

	/// en: 'Add group'
	String get addGroup => 'Add group';

	/// en: 'Text or glob pattern'
	String get keywordPlaceholder => 'Text or glob pattern';

	/// en: 'Case sensitive'
	String get caseSensitive => 'Case sensitive';

	/// en: 'Characters'
	String get lengthPlaceholder => 'Characters';
}

// Path: providers
class Translations$providers$en {
	Translations$providers$en._(this._root);

	final Translations _root; // ignore: unused_field

	// Translations

	/// en: 'Loading upstreams'
	String get loading => 'Loading upstreams';

	/// en: 'Unable to load upstreams: $error'
	String loadFailed({required Object error}) => 'Unable to load upstreams: ${error}';

	/// en: 'No upstream is configured. Add at least one before the proxy can forward requests.'
	String get empty => 'No upstream is configured.\nAdd at least one before the proxy can forward requests.';

	/// en: 'Redaction endpoints select the first enabled upstream for their protocol. Other /v1/* endpoints pass through to the first enabled upstream. Set x-privacy-router-provider to select one explicitly.'
	String get routingNote => 'Redaction endpoints select the first enabled upstream for their protocol. Other /v1/* endpoints pass through to the first enabled upstream. Set x-privacy-router-provider to select one explicitly.';

	/// en: 'Add upstream'
	String get add => 'Add upstream';

	/// en: 'Edit upstream'
	String get edit => 'Edit upstream';

	/// en: 'Name and base URL are required'
	String get required => 'Name and base URL are required';

	/// en: 'Unique name'
	String get name => 'Unique name';

	/// en: 'Base URL, for example https://api.openai.com/v1'
	String get baseUrl => 'Base URL, for example https://api.openai.com/v1';

	/// en: 'Protocol format'
	String get format => 'Protocol format';

	/// en: 'Inbound endpoint: $endpoint'
	String endpoint({required Object endpoint}) => 'Inbound endpoint: ${endpoint}';

	/// en: 'Client credentials pass through unchanged. This proxy does not store or inject API keys.'
	String get credentialNote => 'Client credentials pass through unchanged. This proxy does not store or inject API keys.';
}

// Path: navigation.overview
class Translations$navigation$overview$en {
	Translations$navigation$overview$en._(this._root);

	final Translations _root; // ignore: unused_field

	// Translations

	/// en: 'Overview'
	String get label => 'Overview';

	/// en: 'Traffic, decisions, and detected categories'
	String get description => 'Traffic, decisions, and detected categories';
}

// Path: navigation.pool
class Translations$navigation$pool$en {
	Translations$navigation$pool$en._(this._root);

	final Translations _root; // ignore: unused_field

	// Translations

	/// en: 'Content'
	String get label => 'Content';

	/// en: 'Review content with no registered decision yet; one registration covers every occurrence'
	String get description => 'Review content with no registered decision yet; one registration covers every occurrence';
}

// Path: navigation.rules
class Translations$navigation$rules$en {
	Translations$navigation$rules$en._(this._root);

	final Translations _root; // ignore: unused_field

	// Translations

	/// en: 'Rules'
	String get label => 'Rules';

	/// en: 'Define what is redacted or released; unmatched content follows the fallback'
	String get description => 'Define what is redacted or released; unmatched content follows the fallback';
}

// Path: navigation.providers
class Translations$navigation$providers$en {
	Translations$navigation$providers$en._(this._root);

	final Translations _root; // ignore: unused_field

	// Translations

	/// en: 'Upstreams'
	String get label => 'Upstreams';

	/// en: 'Destinations and protocols; client credentials pass through unchanged'
	String get description => 'Destinations and protocols; client credentials pass through unchanged';
}

// Path: navigation.performance
class Translations$navigation$performance$en {
	Translations$navigation$performance$en._(this._root);

	final Translations _root; // ignore: unused_field

	// Translations

	/// en: 'Performance'
	String get label => 'Performance';

	/// en: 'Inference and upstream latency, plus result cache hits'
	String get description => 'Inference and upstream latency, plus result cache hits';
}

// Path: model.entity
class Translations$model$entity$en {
	Translations$model$entity$en._(this._root);

	final Translations _root; // ignore: unused_field

	// Translations

	/// en: 'Model · Account number'
	String get accountNumber => 'Model · Account number';

	/// en: 'Deterministic · Bank card'
	String get bankCard => 'Deterministic · Bank card';

	/// en: 'Deterministic · Mainland China credit code'
	String get creditCode => 'Deterministic · Mainland China credit code';

	/// en: 'Deterministic · Mainland China ID card'
	String get idCard => 'Deterministic · Mainland China ID card';

	/// en: 'Deterministic · IP address'
	String get ipAddress => 'Deterministic · IP address';

	/// en: 'Deterministic · MAC address'
	String get macAddress => 'Deterministic · MAC address';

	/// en: 'Model · Address'
	String get privateAddress => 'Model · Address';

	/// en: 'Model · Date'
	String get privateDate => 'Model · Date';

	/// en: 'Deterministic · Email'
	String get privateEmail => 'Deterministic · Email';

	/// en: 'Model · Person'
	String get privatePerson => 'Model · Person';

	/// en: 'Model · Phone'
	String get privatePhone => 'Model · Phone';

	/// en: 'Model · URL'
	String get privateUrl => 'Model · URL';

	/// en: 'Model · Secret'
	String get secret => 'Model · Secret';

	/// en: 'Unknown'
	String get unknown => 'Unknown';
}

// Path: model.decision
class Translations$model$decision$en {
	Translations$model$decision$en._(this._root);

	final Translations _root; // ignore: unused_field

	// Translations

	/// en: 'Release'
	String get release => 'Release';

	/// en: 'Redact'
	String get redact => 'Redact';
}

// Path: model.source
class Translations$model$source$en {
	Translations$model$source$en._(this._root);

	final Translations _root; // ignore: unused_field

	// Translations

	/// en: 'Built-in'
	String get builtin => 'Built-in';

	/// en: 'Custom'
	String get console => 'Custom';

	/// en: 'Operator record'
	String get operator => 'Operator record';
}

// Path: model.pattern
class Translations$model$pattern$en {
	Translations$model$pattern$en._(this._root);

	final Translations _root; // ignore: unused_field

	// Translations

	/// en: 'equals'
	String get exact => 'equals';

	/// en: 'contains'
	String get substring => 'contains';

	/// en: 'matches glob'
	String get glob => 'matches glob';
}

// Path: model.comparison
class Translations$model$comparison$en {
	Translations$model$comparison$en._(this._root);

	final Translations _root; // ignore: unused_field

	// Translations

	/// en: 'greater than'
	String get greater => 'greater than';

	/// en: 'equal to'
	String get equal => 'equal to';

	/// en: 'less than'
	String get less => 'less than';
}

// Path: model.logic
class Translations$model$logic$en {
	Translations$model$logic$en._(this._root);

	final Translations _root; // ignore: unused_field

	// Translations

	/// en: 'Match all'
	String get all => 'Match all';

	/// en: 'Match any'
	String get any => 'Match any';
}

// Path: model.apiFormat
class Translations$model$apiFormat$en {
	Translations$model$apiFormat$en._(this._root);

	final Translations _root; // ignore: unused_field

	// Translations

	/// en: 'OpenAI Responses'
	String get openaiResponses => 'OpenAI Responses';

	/// en: 'OpenAI Chat'
	String get openaiChat => 'OpenAI Chat';

	/// en: 'Anthropic Messages'
	String get anthropicMessages => 'Anthropic Messages';
}

// Path: model.filterSummary
class Translations$model$filterSummary$en {
	Translations$model$filterSummary$en._(this._root);

	final Translations _root; // ignore: unused_field

	// Translations

	/// en: 'category is $group'
	String entity({required Object group}) => 'category is ${group}';

	/// en: 'text $operator "$text"'
	String keyword({required Object operator, required Object text}) => 'text ${operator} "${text}"';

	/// en: 'character count is $operator $value'
	String characterLength({required Object operator, required Object value}) => 'character count is ${operator} ${value}';

	/// en: 'confidence is $operator $value'
	String confidence({required Object operator, required Object value}) => 'confidence is ${operator} ${value}';

	/// en: ' and '
	String get allJoiner => ' and ';

	/// en: ' or '
	String get anyJoiner => ' or ';
}

// Path: overview.window
class Translations$overview$window$en {
	Translations$overview$window$en._(this._root);

	final Translations _root; // ignore: unused_field

	// Translations

	/// en: '1 hour'
	String get oneHour => '1 hour';

	/// en: '24 hours'
	String get day => '24 hours';

	/// en: '7 days'
	String get week => '7 days';

	/// en: '30 days'
	String get month => '30 days';
}

// Path: rules.group
class Translations$rules$group$en {
	Translations$rules$group$en._(this._root);

	final Translations _root; // ignore: unused_field

	// Translations

	/// en: 'Built-in rules'
	String get builtin => 'Built-in rules';

	/// en: 'Custom rules'
	String get console => 'Custom rules';

	/// en: 'Operator records'
	String get operator => 'Operator records';
}

// Path: rules.footer
class Translations$rules$footer$en {
	Translations$rules$footer$en._(this._root);

	final Translations _root; // ignore: unused_field

	// Translations

	/// en: 'Default rules may be edited, disabled, or deleted.'
	String get builtin => 'Default rules may be edited, disabled, or deleted.';

	/// en: 'Each rule applies to that exact text only.'
	String get operator => 'Each rule applies to that exact text only.';
}

// Path: rules.filterKind
class Translations$rules$filterKind$en {
	Translations$rules$filterKind$en._(this._root);

	final Translations _root; // ignore: unused_field

	// Translations

	/// en: 'Category'
	String get entity => 'Category';

	/// en: 'Text'
	String get keyword => 'Text';

	/// en: 'Length'
	String get characterLength => 'Length';

	/// en: 'Confidence'
	String get confidence => 'Confidence';
}

/// The flat map containing all translations for locale <en>.
/// Only for edge cases! For simple maps, use the map function of this library.
///
/// The Dart AOT compiler has issues with very large switch statements,
/// so the map is split into smaller functions (512 entries each).
extension on Translations {
	dynamic _flatMapFunction(String path) {
		return switch (path) {
			'demo.banner' => 'Interactive demo · Sample data',
			'demo.detail' => 'Changes reset on reload. No requests are sent to providers.',
			'demo.credentials' => ({required Object username, required Object password}) => 'Demo login: ${username} / ${password}. No real credentials needed.',
			'demo.reset' => 'Reset demo',
			'demo.enter' => 'Enter demo',
			'demo.loginTitle' => 'Try Privacy Router',
			'demo.tagline' => 'Explore privacy controls with a sample workspace',
			'language' => 'Language',
			'app.title' => 'Privacy Router',
			'app.tagline' => 'Local privacy detection and redaction proxy',
			'common.search' => 'Search',
			'common.cancel' => 'Cancel',
			'common.delete' => 'Delete',
			'common.deleteConfirmation' => 'Delete this rule? This cannot be undone.',
			'common.retry' => 'Retry',
			'common.save' => 'Save',
			'common.deleteFailed' => ({required Object error}) => 'Delete failed: ${error}',
			'common.saveFailed' => ({required Object error}) => 'Save failed: ${error}',
			'common.enabled' => 'Enabled',
			'common.disabled' => 'Disabled',
			'common.account' => 'Account',
			'common.accountMenu' => ({required Object username}) => '${username}, account menu',
			'common.appearance' => ({required Object value}) => 'Appearance: ${value}',
			'common.signOut' => 'Sign out',
			'appearance.system' => 'System',
			'appearance.light' => 'Light',
			'appearance.dark' => 'Dark',
			'navigation.overview.label' => 'Overview',
			'navigation.overview.description' => 'Traffic, decisions, and detected categories',
			'navigation.pool.label' => 'Content',
			'navigation.pool.description' => 'Review content with no registered decision yet; one registration covers every occurrence',
			'navigation.rules.label' => 'Rules',
			'navigation.rules.description' => 'Define what is redacted or released; unmatched content follows the fallback',
			'navigation.providers.label' => 'Upstreams',
			'navigation.providers.description' => 'Destinations and protocols; client credentials pass through unchanged',
			'navigation.performance.label' => 'Performance',
			'navigation.performance.description' => 'Inference and upstream latency, plus result cache hits',
			'navigation.expandSidebar' => 'Expand sidebar',
			'navigation.collapseSidebar' => 'Collapse sidebar',
			'model.entity.accountNumber' => 'Model · Account number',
			'model.entity.bankCard' => 'Deterministic · Bank card',
			'model.entity.creditCode' => 'Deterministic · Mainland China credit code',
			'model.entity.idCard' => 'Deterministic · Mainland China ID card',
			'model.entity.ipAddress' => 'Deterministic · IP address',
			'model.entity.macAddress' => 'Deterministic · MAC address',
			'model.entity.privateAddress' => 'Model · Address',
			'model.entity.privateDate' => 'Model · Date',
			'model.entity.privateEmail' => 'Deterministic · Email',
			'model.entity.privatePerson' => 'Model · Person',
			'model.entity.privatePhone' => 'Model · Phone',
			'model.entity.privateUrl' => 'Model · URL',
			'model.entity.secret' => 'Model · Secret',
			'model.entity.unknown' => 'Unknown',
			'model.decision.release' => 'Release',
			'model.decision.redact' => 'Redact',
			'model.source.builtin' => 'Built-in',
			'model.source.console' => 'Custom',
			'model.source.operator' => 'Operator record',
			'model.pattern.exact' => 'equals',
			'model.pattern.substring' => 'contains',
			'model.pattern.glob' => 'matches glob',
			'model.comparison.greater' => 'greater than',
			'model.comparison.equal' => 'equal to',
			'model.comparison.less' => 'less than',
			'model.logic.all' => 'Match all',
			'model.logic.any' => 'Match any',
			'model.apiFormat.openaiResponses' => 'OpenAI Responses',
			'model.apiFormat.openaiChat' => 'OpenAI Chat',
			'model.apiFormat.anthropicMessages' => 'Anthropic Messages',
			'model.filterSummary.entity' => ({required Object group}) => 'category is ${group}',
			'model.filterSummary.keyword' => ({required Object operator, required Object text}) => 'text ${operator} "${text}"',
			'model.filterSummary.characterLength' => ({required Object operator, required Object value}) => 'character count is ${operator} ${value}',
			'model.filterSummary.confidence' => ({required Object operator, required Object value}) => 'confidence is ${operator} ${value}',
			'model.filterSummary.allJoiner' => ' and ',
			'model.filterSummary.anyJoiner' => ' or ',
			'model.unknownFilter' => ({required Object kind}) => 'Unknown rule filter: ${kind}',
			'login.username' => 'Administrator account',
			'login.password' => 'Password',
			'login.submit' => 'Sign in',
			'login.unreachable' => 'Cannot connect to the proxy. Make sure it is running.',
			'login.providerMissing' => 'No upstream is configured. Add one on the Upstreams page after signing in.',
			'setup.title' => 'Create administrator',
			'setup.subtitle' => 'This proxy has no administrator account yet. Set one up here.',
			'setup.username' => 'Administrator account',
			'setup.password' => 'Password',
			'setup.confirmation' => 'Repeat password',
			'setup.submit' => 'Create and sign in',
			'setup.usernameRequired' => 'Administrator account must not be empty',
			'setup.passwordRequired' => 'Password must not be empty',
			'setup.passwordMismatch' => 'The two passwords do not match',
			'setup.localOnly' => 'Setup can only be completed on the machine running the proxy. Open this page there if you are on another device.',
			'overview.window.oneHour' => '1 hour',
			'overview.window.day' => '24 hours',
			'overview.window.week' => '7 days',
			'overview.window.month' => '30 days',
			'overview.loading' => 'Loading statistics',
			'overview.loadFailed' => ({required Object error}) => 'Unable to load statistics: ${error}',
			'overview.requests' => 'Requests',
			'overview.requestsSubtitle' => 'Proxy requests completed in this window',
			'overview.fragments' => 'Evaluated fragments',
			'overview.fragmentsSubtitle' => 'Text fragments sent to the content pool',
			'overview.detections' => 'Detections',
			'overview.detectionBreakdown' => ({required Object redacted, required Object released}) => 'Redacted ${redacted} · Released ${released}',
			'overview.redactedRatio' => 'Redacted',
			'overview.redactedRatioSubtitle' => 'Share of detections not released',
			'overview.distribution' => 'Category distribution',
			'overview.distributionSubtitle' => 'Detections by category in this window',
			'overview.empty' => 'No detections in this window',
			'overview.emptySubtitle' => 'Detections appear here after traffic passes through the proxy',
			'performance.firstResult' => 'First result',
			'performance.firstResultSubtitle' => 'Mean local inference time to first result',
			'performance.throughput' => 'Model throughput',
			'performance.throughputSubtitle' => 'Processed tokens / forward execution time',
			'performance.modelTokens' => 'Processed tokens',
			'performance.modelTokensSubtitle' => 'Actual model work; cache hits excluded',
			'performance.tokenization' => 'Tokenization',
			'performance.validation' => 'Validation and batch planning',
			'performance.forward' => 'Model forward pass',
			'performance.decoding' => 'Entity decoding',
			'performance.loading' => 'Loading performance data',
			'performance.loadFailed' => ({required Object error}) => 'Unable to load performance data: ${error}',
			'performance.average' => 'Average end-to-end',
			'performance.averageSubtitle' => 'Queue + detection + forwarding',
			'performance.percentileSubtitle' => 'Nearest-rank percentile',
			'performance.maximum' => 'Maximum',
			'performance.maximumSubtitle' => 'Slowest request in this window',
			'performance.breakdown' => 'Latency breakdown',
			'performance.inference' => 'Detection (inference)',
			'performance.inferenceSubtitle' => 'Local model forward pass + result cache hit',
			'performance.upstream' => 'Forwarding (upstream)',
			'performance.upstreamSubtitle' => 'Send redacted request upstream and receive its response',
			'performance.cacheHit' => 'Cache hit',
			'performance.cacheBreakdown' => ({required Object cached, required Object inferred}) => 'Cached ${cached} · Inferred ${inferred} fragments',
			'performance.note' => 'Detection latency includes cached fragments; cache hits do not enter the model and therefore do not increase it. Forwarding latency is one upstream round trip. Responses, including SSE, are streamed unchanged without buffering or rewriting.',
			'pool.filterUnreviewed' => 'Unreviewed',
			'pool.filterReviewed' => 'Reviewed',
			'pool.filterAll' => 'All content',
			'pool.previous' => 'Previous',
			'pool.next' => 'Next',
			'pool.refresh' => 'Refresh',
			'pool.range' => ({required Object start, required Object end, required Object total}) => '${start}–${end} of ${total}',
			'pool.emptyPage' => 'No records on this page. Return to the previous page or refresh.',
			'pool.emptyUnreviewed' => 'Nothing is waiting for review. Everything evaluated already has a registered decision.',
			'pool.emptyReviewed' => 'Nothing has been reviewed yet. Content you release or deny shows up here.',
			'pool.empty' => 'The content pool is empty.\nEvaluated content appears here after traffic passes through the proxy.',
			'pool.loading' => 'Loading content',
			'pool.loadFailed' => ({required Object error}) => 'Unable to load content: ${error}',
			'pool.footer' => ({required Object count}) => '${count} rows. Each row is every decision about one piece of content: flipping first, and one registration covers every occurrence.',
			'pool.occurrenceFailed' => ({required Object error}) => 'Unable to load occurrences: ${error}',
			'pool.occurrencesLabel' => 'Occurrences',
			'pool.occurrences' => ({required Object count, required Object turns}) => '${count} occurrences · ${turns} turns',
			'pool.decisionsLabel' => 'Decisions',
			'pool.decisions' => ({required Object redacted, required Object released}) => '${redacted} redacted · ${released} released',
			'pool.confidenceLabel' => 'Confidence',
			'pool.confidence' => ({required Object value}) => 'Confidence ${value}',
			'pool.confidenceRange' => ({required Object min, required Object max}) => 'Confidence ${min}–${max}',
			'pool.lastSeenLabel' => 'Last seen',
			'pool.lastSeen' => ({required Object time}) => 'Last ${time}',
			'pool.categories' => 'Detected as',
			'pool.firstSeenLabel' => 'First seen',
			'pool.registered' => 'Registered',
			'pool.registeredNone' => 'Not registered',
			'pool.registeredRelease' => 'Registered: release',
			'pool.registeredRedact' => 'Registered: deny',
			'pool.release' => 'Release',
			'pool.deny' => 'Deny',
			'pool.releasedCreated' => 'Released and registered as a release rule',
			'pool.releasedExisting' => 'This content was already released; nothing changed',
			'pool.releaseFailed' => ({required Object error}) => 'Release failed: ${error}',
			'pool.deniedCreated' => 'Denied and registered as a force-redact rule',
			'pool.deniedExisting' => 'This content was already denied; nothing changed',
			'pool.denyFailed' => ({required Object error}) => 'Deny failed: ${error}',
			'pool.occurrenceCount' => 'Occurrences',
			'pool.noOccurrences' => 'No occurrence was recorded for this content.',
			'pool.occurrenceSource' => ({required Object protocol, required Object path}) => '${protocol} · ${path}',
			'pool.detailHint' => 'One registration decides every occurrence of this content.',
			'pool.contentTitle' => 'Content details',
			'pool.turnTitle' => 'Detection details',
			'pool.original' => 'Original',
			'pool.forwarded' => 'Forwarded content',
			'pool.detectedSpans' => 'Detected content',
			'pool.noDetections' => 'This turn has no detected categories.',
			'pool.characters' => ({required Object count}) => '${count} characters',
			'pool.matchedRule' => ({required Object id}) => 'Matched rule: ${id}',
			'pool.time' => 'Time',
			'pool.protocol' => 'Protocol',
			'pool.detailFailed' => ({required Object error}) => 'Unable to load details: ${error}',
			'rules.listTitle' => 'Rules',
			'rules.empty' => 'No rules',
			'rules.editList' => 'Edit',
			'rules.unmatched' => 'When no rule matches',
			'rules.loading' => 'Loading rules',
			'rules.loadFailed' => ({required Object error}) => 'Unable to load rules: ${error}',
			'rules.kNew' => 'New rule',
			'rules.edit' => 'Edit rule',
			'rules.prioritySummary' => ({required Object priority, required Object condition}) => 'Priority ${priority} · ${condition}',
			'rules.toggleFailed' => ({required Object error}) => 'Update failed: ${error}',
			'rules.group.builtin' => 'Built-in rules',
			'rules.group.console' => 'Custom rules',
			'rules.group.operator' => 'Operator records',
			'rules.footer.builtin' => 'Default rules may be edited, disabled, or deleted.',
			'rules.footer.operator' => 'Each rule applies to that exact text only.',
			'rules.defaultName' => ({required Object group}) => 'Redact: ${group}',
			'rules.operatorReleaseName' => 'Released content',
			'rules.operatorRedactName' => 'Denied content',
			'rules.evaluationRedact' => 'Rules are evaluated from highest to lowest priority. If none match, content is redacted as the fallback; the fallback is not listed above.',
			'rules.evaluationRelease' => 'Rules are evaluated from highest to lowest priority. If none match, content is released.',
			'rules.nameRequired' => 'Rule name is required',
			'rules.name' => 'Rule name',
			'rules.matchingLogic' => 'Matching logic',
			'rules.matchingLogicSubtitle' => 'Groups may be nested. Each group matches all or any of its children.',
			'rules.result' => 'Rule result',
			'rules.priority' => 'Priority (higher values run first)',
			'rules.filterKind.entity' => 'Category',
			'rules.filterKind.keyword' => 'Text',
			'rules.filterKind.characterLength' => 'Length',
			'rules.filterKind.confidence' => 'Confidence',
			'rules.groupEmpty' => 'Each condition group must contain at least one condition',
			'rules.keywordRequired' => 'Text filter value is required',
			'rules.lengthInvalid' => 'Character count must be an integer greater than or equal to 0',
			'rules.confidenceInvalid' => 'Confidence must be a number from 0 to 1',
			'rules.addFilter' => 'Add filter',
			'rules.addGroup' => 'Add group',
			'rules.keywordPlaceholder' => 'Text or glob pattern',
			'rules.caseSensitive' => 'Case sensitive',
			'rules.lengthPlaceholder' => 'Characters',
			'providers.loading' => 'Loading upstreams',
			'providers.loadFailed' => ({required Object error}) => 'Unable to load upstreams: ${error}',
			'providers.empty' => 'No upstream is configured.\nAdd at least one before the proxy can forward requests.',
			'providers.routingNote' => 'Redaction endpoints select the first enabled upstream for their protocol. Other /v1/* endpoints pass through to the first enabled upstream. Set x-privacy-router-provider to select one explicitly.',
			'providers.add' => 'Add upstream',
			'providers.edit' => 'Edit upstream',
			'providers.required' => 'Name and base URL are required',
			'providers.name' => 'Unique name',
			'providers.baseUrl' => 'Base URL, for example https://api.openai.com/v1',
			'providers.format' => 'Protocol format',
			'providers.endpoint' => ({required Object endpoint}) => 'Inbound endpoint: ${endpoint}',
			'providers.credentialNote' => 'Client credentials pass through unchanged. This proxy does not store or inject API keys.',
			_ => null,
		};
	}
}
