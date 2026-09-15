mod common;

use common::capture;
use privacy_telemetry::{LogFormat, TelemetryConfig};
use rstest::{fixture, rstest};
use serde_json::Value;

#[fixture]
fn json_config() -> TelemetryConfig {
    TelemetryConfig {
        format: LogFormat::Json,
        ansi: Some(false),
        ..TelemetryConfig::default()
    }
}

#[fixture]
fn human_config() -> TelemetryConfig {
    TelemetryConfig {
        format: LogFormat::Human,
        ansi: Some(false),
        ..TelemetryConfig::default()
    }
}

#[rstest]
fn json_emits_one_parseable_object_per_record(json_config: TelemetryConfig) {
    let output = capture(&json_config, || {
        tracing::info!(request_id = "r-1", "first");
        tracing::info!(request_id = "r-2", "second");
    });

    let lines: Vec<&str> = output.lines().filter(|line| !line.is_empty()).collect();
    assert_eq!(lines.len(), 2, "每条记录必须独占一行：{output}");
    for line in lines {
        serde_json::from_str::<Value>(line).expect("每行都是合法 JSON");
    }
}

#[rstest]
fn json_carries_the_documented_field_vocabulary(json_config: TelemetryConfig) {
    let output = capture(&json_config, || {
        tracing::info!(
            request_id = "r-1",
            provider = "openai",
            api_format = "openai_chat",
            stage = "classify",
            entity_group = "private_email",
            action = "redact",
            tokens = 42,
            inference_ms = 17,
            "span decided"
        );
    });

    let record: Value =
        serde_json::from_str(output.lines().next().expect("有输出")).expect("合法 JSON");
    let fields = &record["fields"];

    assert_eq!(fields["message"], "span decided");
    assert_eq!(fields["request_id"], "r-1");
    assert_eq!(fields["provider"], "openai");
    assert_eq!(fields["api_format"], "openai_chat");
    assert_eq!(fields["stage"], "classify");
    assert_eq!(fields["entity_group"], "private_email");
    assert_eq!(fields["action"], "redact");
    assert_eq!(fields["tokens"], 42);
    assert_eq!(fields["inference_ms"], 17);
}

#[rstest]
fn json_records_level_and_target(json_config: TelemetryConfig) {
    let output = capture(&json_config, || {
        tracing::warn!(stage = "forward", "upstream unavailable");
    });

    let record: Value =
        serde_json::from_str(output.lines().next().expect("有输出")).expect("合法 JSON");
    assert_eq!(record["level"], "WARN");
    // target 标识产生该日志的模块，采集端据此定位来源。
    assert_eq!(record["target"], env!("CARGO_CRATE_NAME"));
}

#[rstest]
fn json_attaches_the_current_span(json_config: TelemetryConfig) {
    let output = capture(&json_config, || {
        let span = tracing::info_span!("request", request_id = "r-9");
        let _entered = span.enter();
        tracing::info!(stage = "forward", "forwarding");
    });

    let record: Value =
        serde_json::from_str(output.lines().next().expect("有输出")).expect("合法 JSON");
    assert_eq!(record["span"]["request_id"], "r-9");
    assert_eq!(record["spans"][0]["name"], "request");
}

#[rstest]
fn human_output_stays_readable_and_uncoded(human_config: TelemetryConfig) {
    let output = capture(&human_config, || {
        tracing::info!(request_id = "r-1", "request completed");
    });

    assert!(output.contains("request completed"), "应包含消息：{output}");
    assert!(output.contains("r-1"), "应包含字段：{output}");
    assert!(
        serde_json::from_str::<Value>(output.trim()).is_err(),
        "人类可读格式不应输出 JSON：{output}"
    );
}

#[rstest]
fn human_output_omits_ansi_when_disabled(human_config: TelemetryConfig) {
    let output = capture(&human_config, || {
        tracing::info!("plain");
    });

    assert!(
        !output.contains('\u{1b}'),
        "禁用着色后不得出现转义序列：{output:?}"
    );
}

#[rstest]
fn filter_directives_control_what_is_emitted() {
    // 只有 warn 及以上通过，info 必须被过滤掉。
    let quiet = TelemetryConfig {
        format: LogFormat::Json,
        filter: "warn".to_owned(),
        ansi: Some(false),
        ..TelemetryConfig::default()
    };

    let output = capture(&quiet, || {
        tracing::info!(stage = "parse", "dropped");
        tracing::warn!(stage = "parse", "kept");
    });

    assert!(!output.contains("dropped"), "info 应被过滤：{output}");
    assert!(output.contains("kept"), "warn 应保留：{output}");
}

#[rstest]
fn invalid_filter_directives_are_reported_not_panicked() {
    let invalid = TelemetryConfig {
        filter: "level===nonsense".to_owned(),
        ..TelemetryConfig::default()
    };

    let error = privacy_telemetry::Telemetry::subscriber(&invalid, std::io::sink)
        .err()
        .expect("非法过滤串必须返回错误");
    assert!(matches!(error, privacy_telemetry::Error::InvalidFilter(_)));
}

#[rstest]
fn the_same_field_produces_the_same_value_in_both_formats(
    json_config: TelemetryConfig,
    human_config: TelemetryConfig,
) {
    let emit = || {
        tracing::info!(request_id = "shared-id", action = "release", "compared");
    };

    let json = capture(&json_config, emit);
    let human = capture(&human_config, emit);

    let record: Value =
        serde_json::from_str(json.lines().next().expect("有输出")).expect("合法 JSON");
    assert_eq!(record["fields"]["request_id"], "shared-id");
    assert!(human.contains("shared-id"));
    assert!(human.contains("release"));
}
