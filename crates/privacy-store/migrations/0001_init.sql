-- Privacy Router 初始 schema。
--
-- 时间统一存 unix 毫秒整数：跨驱动与跨语言读取都不需要额外的类型集成。
-- 存放用户内容的两张表（fragments / spans）不设外键到规则，只保留 matched_rule_id，
-- 规则被删除时该列置空——审计记录不因配置变更而级联消失。

-- 判定规则。内置默认、控制台创建、放行登记都在这一张表里，走同一套判定。
CREATE TABLE rules (
    id             TEXT    PRIMARY KEY,
    name           TEXT    NOT NULL,
    priority       INTEGER NOT NULL,
    condition_json TEXT    NOT NULL,
    action         TEXT    NOT NULL CHECK (action IN ('release', 'redact')),
    enabled        INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    source         TEXT    NOT NULL CHECK (source IN ('builtin', 'console', 'approval')),
    created_at     INTEGER NOT NULL,
    updated_at     INTEGER NOT NULL
);

CREATE INDEX rules_priority_index ON rules (priority DESC, id);

-- 初始数据只安装一次。默认规则被编辑或删除后，进程重启不得重新生成它们。
CREATE TABLE initialization (
    key          TEXT PRIMARY KEY,
    completed_at INTEGER NOT NULL
);

-- 上游只配置路由。凭证由客户端携带，代理不管理、不保存、不替换。
CREATE TABLE providers (
    id          TEXT    PRIMARY KEY,
    name        TEXT    NOT NULL UNIQUE,
    base_url    TEXT    NOT NULL,
    api_format  TEXT    NOT NULL CHECK (
        api_format IN ('openai_responses', 'openai_chat', 'anthropic_messages')
    ),
    enabled     INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);

-- 管理员。单行表：CHECK 让「最多一个账号」成为数据库约束而不是应用约定。
CREATE TABLE admin (
    id            INTEGER PRIMARY KEY CHECK (id = 1),
    username      TEXT    NOT NULL,
    password_hash TEXT    NOT NULL,
    created_at    INTEGER NOT NULL,
    updated_at    INTEGER NOT NULL
);

-- 控制台会话。只存令牌摘要，库泄露不等于会话可被直接使用。
CREATE TABLE sessions (
    token_hash BLOB    PRIMARY KEY,
    admin_id   INTEGER NOT NULL REFERENCES admin (id) ON DELETE CASCADE,
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL
);

CREATE INDEX sessions_expiry_index ON sessions (expires_at);

-- 内容池：一段被判定过的文本及其替换结果。放行与抹去的内容都入库。
CREATE TABLE fragments (
    id            TEXT    PRIMARY KEY,
    request_id    TEXT    NOT NULL,
    api_format    TEXT    NOT NULL,
    path          TEXT    NOT NULL,
    content_hash  BLOB    NOT NULL,
    original_text TEXT    NOT NULL,
    redacted_text TEXT    NOT NULL,
    created_at    INTEGER NOT NULL
);

CREATE INDEX fragments_request_index ON fragments (request_id);
CREATE INDEX fragments_created_index ON fragments (created_at DESC);

-- 命中片段。原文在此保留，供控制台逐条复查与放行。
CREATE TABLE spans (
    id              TEXT    PRIMARY KEY,
    fragment_id     TEXT    NOT NULL REFERENCES fragments (id) ON DELETE CASCADE,
    entity_group    TEXT    NOT NULL,
    score           REAL    NOT NULL,
    char_len        INTEGER NOT NULL CHECK (char_len >= 0),
    byte_start      INTEGER NOT NULL CHECK (byte_start >= 0),
    byte_end        INTEGER NOT NULL,
    original_text   TEXT    NOT NULL,
    action          TEXT    NOT NULL CHECK (action IN ('release', 'redact')),
    matched_rule_id TEXT    REFERENCES rules (id) ON DELETE SET NULL,
    created_at      INTEGER NOT NULL,
    CHECK (byte_end >= byte_start)
);

CREATE INDEX spans_fragment_index ON spans (fragment_id);
CREATE INDEX spans_original_index ON spans (original_text);
CREATE INDEX spans_group_index ON spans (entity_group);

-- 单次代理请求的计时与结果。上游或 provider 被删除后仍保留这条记录。
CREATE TABLE requests (
    id               TEXT    PRIMARY KEY,
    request_id       TEXT    NOT NULL UNIQUE,
    provider_id      TEXT    REFERENCES providers (id) ON DELETE SET NULL,
    api_format       TEXT    NOT NULL,
    started_at       INTEGER NOT NULL,
    duration_ms      INTEGER NOT NULL,
    queue_ms         INTEGER NOT NULL,
    inference_ms     INTEGER NOT NULL,
    upstream_ms      INTEGER NOT NULL,
    status           INTEGER NOT NULL,
    cache_usage_json TEXT    NOT NULL DEFAULT '{}',
    error_kind       TEXT
);

CREATE INDEX requests_started_index ON requests (started_at DESC);
