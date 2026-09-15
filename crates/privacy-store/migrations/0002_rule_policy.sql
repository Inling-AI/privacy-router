CREATE TABLE IF NOT EXISTS rule_policy (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    fallback_action TEXT NOT NULL CHECK (fallback_action IN ('release', 'redact'))
);
