//! 内嵌 SQLite 持久化。
//!
//! 单一二进制自包含：SQLite 随 crate 编译，不依赖系统库，迁移在进程启动时自动执行。
//! 仓储按领域类型组织，本 crate 不包含 HTTP、协议解析或日志格式化。
#![doc = include_str!("../README.md")]

mod admin;
mod codec;
mod inference;
mod metrics;
mod pool;
mod provider;
mod rules;

use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub use admin::{AdminRepository, Credentials, Session};
pub use inference::{InferenceNode, InferenceRepository};
pub use metrics::{
    EntityGroupCount, LatencySummary, MetricsRepository, RequestOutcome, RequestTiming, Statistics,
};
pub use pool::{
    ContentCategory, ContentFilter, ContentOccurrence, ContentSummary, FragmentDetail,
    FragmentRecording, PoolRepository, RecordedFragment, RecordedSpan,
};
pub use provider::{ApiFormat, Provider, ProviderDraft, ProviderRepository};
pub use rules::RuleRepository;

/// 写入时间的来源。注入后，审计记录与统计都能在测试中获得确定性的时间戳。
pub trait Clock: Send + Sync {
    /// 当前的 unix 毫秒时间戳。
    fn now_ms(&self) -> i64;
}

/// 使用宿主机时钟；生产路径的默认实现。
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_ms(&self) -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_millis() as i64)
            .unwrap_or_default()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("value encoding error: {0}")]
    Encoding(String),
    #[error("rule error: {0}")]
    Rule(#[from] privacy_rules::Error),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("the admin account already exists")]
    AdminAlreadyExists,
    #[error("invalid administrator credentials: {0}")]
    InvalidCredentials(String),
    #[error(
        "no admin account is configured; create one in the console or run `privacy-router init-admin`"
    )]
    AdminMissing,
    #[error("a provider named '{0}' already exists")]
    ProviderNameTaken(String),
    #[error("a rule with id '{0}' already exists")]
    RuleIdTaken(String),
    #[error("invalid provider configuration: {0}")]
    InvalidProvider(String),
    #[error("credential error: {0}")]
    Credential(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// 数据库句柄与仓储入口。
pub struct Store {
    pool: SqlitePool,
    clock: Arc<dyn Clock>,
}

impl Store {
    pub fn inference(&self) -> InferenceRepository<'_> {
        InferenceRepository::new(self)
    }
    /// 打开（必要时创建）数据库并执行迁移。
    pub async fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::open_with_clock(path, Arc::new(SystemClock)).await
    }

    /// 用注入的时钟打开数据库；测试据此获得确定性时间戳。
    pub async fn open_with_clock(path: impl AsRef<Path>, clock: Arc<dyn Clock>) -> Result<Self> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            // WAL 让读取不被写入阻塞；写者仍然只有一个，由 busy_timeout 吸收等待。
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_secs(5))
            // 级联删除与置空依赖该开关，默认关闭时会静默失效。
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(8)
            .connect_with(options)
            .await?;
        Self::from_pool(pool, clock).await
    }

    /// 在已有连接池上执行迁移并构造句柄。
    pub async fn from_pool(pool: SqlitePool, clock: Arc<dyn Clock>) -> Result<Self> {
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(Self { pool, clock })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub fn now_ms(&self) -> i64 {
        self.clock.now_ms()
    }

    pub fn rules(&self) -> RuleRepository<'_> {
        RuleRepository::new(self)
    }

    pub fn providers(&self) -> ProviderRepository<'_> {
        ProviderRepository::new(self)
    }

    pub fn admin(&self) -> AdminRepository<'_> {
        AdminRepository::new(self)
    }

    pub fn pool_records(&self) -> PoolRepository<'_> {
        PoolRepository::new(self)
    }

    pub fn metrics(&self) -> MetricsRepository<'_> {
        MetricsRepository::new(self)
    }
}
