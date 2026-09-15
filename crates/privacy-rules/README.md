# privacy-rules

Privacy Router's rule evaluation component. Takes one detected entity and returns an action with the matching rule ID. The console and persisted policies use these same rule types.

- `condition.rs`: label, confidence, Unicode character-length, and keyword filters; recursive `All`/`Any` expressions. Keywords match the entity text. Numeric comparisons are strict; glob patterns support `*` and `?`.
- `rule.rs`: rule identity, validation, source, and priority constants.
- `ruleset.rs`: descending priority, ascending ID tie-break, first enabled match wins. Unmatched entities use the configured fallback, defaulting to release.
- `builtin.rs`: per-label protection thresholds and default redaction rules. A rule only gates what recognition leaves open. Categories whose shape is defined by an authoritative deterministic filter (`Filters::kind_of`) ask for nothing but the category: their hits are facts, so a confidence threshold would re-derive the constant `1.0` and a length threshold a shape that is already complete. Categories the model still speaks for (for example phone numbers) keep both thresholds, because it is the probabilistic hits that need them. The distinction is read from the filters, not restated here.

When changing policy behavior, preserve deterministic ordering and validate externally supplied rules before evaluation. `RuleSet::new` sorts without validating. Rule source records provenance and does not change precedence. Persistence, built-in installation, and console editing are handled by the application and `privacy-store`.

Behavior tests in `tests/` cover nested expressions, threshold boundaries, precedence, and default policy using in-memory entities.
