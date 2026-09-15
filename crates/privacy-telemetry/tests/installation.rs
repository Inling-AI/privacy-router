//! 进程级安装的行为。这里刻意只保留一个测试函数：全局订阅器在一个进程内只能安装一次，
//! 拆成多个测试会因并行执行而互相干扰。

use privacy_telemetry::{Error, LogFormat, Telemetry, TelemetryConfig};
use std::path::PathBuf;

fn scratch_directory() -> PathBuf {
    // 每个测试二进制进程使用独立目录，避免残留文件互相影响。
    std::env::temp_dir().join(format!("privacy-telemetry-{}", std::process::id()))
}

#[test]
fn installing_once_writes_a_rolling_file_and_rejects_a_second_install() {
    let directory = scratch_directory();
    let _ = std::fs::remove_dir_all(&directory);

    let config = TelemetryConfig {
        format: LogFormat::Json,
        filter: "info".to_owned(),
        log_dir: Some(directory.clone()),
        ansi: Some(false),
    };

    let guard = Telemetry::init(&config).expect("首次安装必须成功");
    tracing::info!(request_id = "r-file", action = "redact", "written to disk");
    // 丢弃 guard 会冲刷非阻塞写入器；日志文件在此之后才完整。
    drop(guard);

    let entries: Vec<_> = std::fs::read_dir(&directory)
        .expect("日志目录已创建")
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("privacy-router.log")
        })
        .collect();
    assert_eq!(entries.len(), 1, "滚动文件应已创建：{directory:?}");

    let content = std::fs::read_to_string(entries[0].path()).expect("日志文件可读");
    assert!(
        content.contains("written to disk"),
        "文件必须收到日志：{content}"
    );
    assert!(
        content.contains("r-file"),
        "文件必须保留结构化字段：{content}"
    );
    assert!(
        serde_json::from_str::<serde_json::Value>(content.lines().next().expect("有记录")).is_ok(),
        "文件内容必须与终端格式一致：{content}"
    );

    // 全局订阅器已存在，再次安装必须被明确拒绝而不是覆盖或 panic。
    let error = Telemetry::init(&config).expect_err("重复安装必须失败");
    assert!(matches!(error, Error::AlreadyInstalled(_)));

    let _ = std::fs::remove_dir_all(&directory);
}
