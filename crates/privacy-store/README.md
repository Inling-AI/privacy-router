# privacy-store

Privacy Router's SQLite persistence layer. `Store` owns the pool and exposes repositories for rules, providers, administrator sessions, audit fragments, and request metrics.

Schema DDL lives in `migrations/` and runs through SQLx when the store opens. Keep schema creation out of repository methods and startup code. `Store::open` configures WAL, foreign keys, a five-second busy timeout, and up to eight connections. Callers supplying a pool configure its connections themselves.

Maintenance constraints:

- `rules.rs` installs built-ins once, transactionally. Restarting must preserve administrator edits and deletions. The fallback policy is persisted separately from individual rules.
- `pool.rs` retains original text and redacted text. Deleting a rule clears its audit references; deleting a fragment removes its spans.
- `admin.rs` stores password hashes and session-token digests. `Credentials` is the only way to construct an account, so blank names and passwords never reach storage; creation is arbitrated by the primary-key constraint rather than a check-then-write, so concurrent callers cannot both succeed. Credential rotation invalidates sessions.
- `provider.rs` stores routing configuration; client upstream credentials stay outside this database.
- `metrics.rs` aggregates persisted request outcomes. Timestamps use Unix milliseconds; tests can inject `Clock`.

The database contains sensitive plaintext and is not encrypted by this crate. Protect the file, WAL, and backups. Integration tests in `tests/` use isolated SQLite databases to verify transactions, constraints, repository behavior, and metrics.
