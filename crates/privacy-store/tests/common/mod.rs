//! 各测试目标共用的临时数据库。集成测试使用真实 SQLite 文件，不用内存库。
//!
//! 本模块被多个测试目标分别编译，每个目标只用到其中一部分，因此整体关掉死代码检查。
#![allow(dead_code)]

use privacy_store::{Clock, Store};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// 可推进的测试时钟，使审计时间戳与过期判断都不依赖宿主机时钟。
pub struct TestClock(AtomicI64);

impl TestClock {
    pub fn new(now_ms: i64) -> Arc<Self> {
        Arc::new(Self(AtomicI64::new(now_ms)))
    }

    pub fn advance(&self, milliseconds: i64) {
        self.0.fetch_add(milliseconds, Ordering::Relaxed);
    }
}

impl Clock for TestClock {
    fn now_ms(&self) -> i64 {
        self.0.load(Ordering::Relaxed)
    }
}

/// 每个测试独占一个文件路径；析构时清掉 SQLite 的 WAL 附属文件。
pub struct TempDatabase {
    path: PathBuf,
}

impl TempDatabase {
    pub fn new() -> Self {
        let sequence = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "privacy-store-test-{}-{sequence}.db",
            std::process::id()
        ));
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", path.display()));
        }
        Self { path }
    }

    pub async fn open(&self) -> Store {
        self.open_with(TestClock::new(1_700_000_000_000)).await
    }

    pub async fn open_with(&self, clock: Arc<TestClock>) -> Store {
        Store::open_with_clock(&self.path, clock)
            .await
            .expect("临时数据库必须可打开并完成迁移")
    }
}

impl Drop for TempDatabase {
    fn drop(&mut self) {
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", self.path.display()));
        }
    }
}
