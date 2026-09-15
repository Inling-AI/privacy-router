use crate::provider::ApiFormat;
use crate::{Result, Store, codec};
use privacy_filter::performance::ModelPerformance;
use privacy_rules::Action;
use privacy_rules::EntityGroup;
use serde::{Deserialize, Serialize};
use sqlx::Row;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ModelSummary {
    pub measured_requests: u64,
    pub tokens: u64,
    pub tokens_per_second: Option<f64>,
    pub average_first_result_ms: Option<f64>,
    pub average_tokenization_ms: Option<f64>,
    pub average_validation_ms: Option<f64>,
    pub average_forward_ms: Option<f64>,
    pub average_decoding_ms: Option<f64>,
    pub latest: Option<ModelPerformance>,
    pub latest_tokens_per_second: Option<f64>,
}

impl ModelSummary {
    fn from_samples(samples: &[ModelPerformance]) -> Self {
        let measured: Vec<_> = samples.iter().filter(|s| s.tokens > 0).collect();
        let count = measured.len() as f64;
        let tokens = measured.iter().map(|s| s.tokens).sum();
        let forward: f64 = measured.iter().map(|s| s.forward_ms).sum();
        let first: Vec<_> = measured.iter().filter_map(|s| s.first_result_ms).collect();
        let latest = samples.last().copied();
        Self {
            measured_requests: measured.len() as u64,
            tokens,
            tokens_per_second: (forward > 0.0).then(|| tokens as f64 * 1000.0 / forward),
            average_first_result_ms: (!first.is_empty())
                .then(|| first.iter().sum::<f64>() / first.len() as f64),
            average_tokenization_ms: (count > 0.0)
                .then(|| measured.iter().map(|s| s.tokenization_ms).sum::<f64>() / count),
            average_validation_ms: (count > 0.0)
                .then(|| measured.iter().map(|s| s.validation_ms).sum::<f64>() / count),
            average_forward_ms: (count > 0.0).then(|| forward / count),
            average_decoding_ms: (count > 0.0)
                .then(|| measured.iter().map(|s| s.decoding_ms).sum::<f64>() / count),
            latest,
            latest_tokens_per_second: latest.and_then(|s| s.tokens_per_second()),
        }
    }
}

/// 单次请求的耗时分解（毫秒）。排队时间与推理时间分开记录，避免把锁等待算进推理。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestTiming {
    pub queue_ms: i64,
    pub inference_ms: i64,
    pub upstream_ms: i64,
    pub duration_ms: i64,
}

/// 一次请求的结果。写入失败不影响判定，但会以 `error_kind` 留下痕迹。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestOutcome {
    pub request_id: String,
    pub provider_id: Option<String>,
    pub api_format: ApiFormat,
    pub status: i64,
    pub timing: RequestTiming,
    pub cache_usage_json: String,
    pub error_kind: Option<String>,
}

/// 某一类别命中的条数。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityGroupCount {
    pub entity_group: EntityGroup,
    pub count: i64,
}

/// 控制台概览所需的统计。所有比率都在这里由计数导出，前端不再各算一遍。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Statistics {
    pub model: ModelSummary,
    pub since_ms: i64,
    pub requests: i64,
    pub fragments: i64,
    pub spans: i64,
    pub released_spans: i64,
    pub redacted_spans: i64,
    pub cached_fragments: i64,
    pub inferred_fragments: i64,
    pub latency: LatencySummary,
    pub by_entity_group: Vec<EntityGroupCount>,
}

/// 请求耗时的分位与均值，单位毫秒。
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct LatencySummary {
    pub average_ms: i64,
    pub p50_ms: i64,
    pub p95_ms: i64,
    pub max_ms: i64,
    pub average_inference_ms: i64,
    pub average_upstream_ms: i64,
}

/// 请求计时与统计的读写。
pub struct MetricsRepository<'a> {
    store: &'a Store,
}

impl<'a> MetricsRepository<'a> {
    pub async fn record_with_performance(
        &self,
        outcome: &RequestOutcome,
        performance: &ModelPerformance,
    ) -> Result<()> {
        self.record(outcome).await?;
        sqlx::query("UPDATE requests SET model_performance_json = ? WHERE request_id = ?")
            .bind(
                serde_json::to_string(performance)
                    .map_err(|e| crate::Error::Encoding(e.to_string()))?,
            )
            .bind(&outcome.request_id)
            .execute(self.store.pool())
            .await?;
        Ok(())
    }
    pub(crate) fn new(store: &'a Store) -> Self {
        Self { store }
    }

    pub async fn record(&self, outcome: &RequestOutcome) -> Result<()> {
        sqlx::query(
            "INSERT INTO requests (id, request_id, provider_id, api_format, started_at, duration_ms, queue_ms, inference_ms, upstream_ms, status, cache_usage_json, error_kind)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT (request_id) DO NOTHING",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&outcome.request_id)
        .bind(&outcome.provider_id)
        .bind(codec::encode(&outcome.api_format)?)
        .bind(self.store.now_ms() - outcome.timing.duration_ms)
        .bind(outcome.timing.duration_ms)
        .bind(outcome.timing.queue_ms)
        .bind(outcome.timing.inference_ms)
        .bind(outcome.timing.upstream_ms)
        .bind(outcome.status)
        .bind(&outcome.cache_usage_json)
        .bind(&outcome.error_kind)
        .execute(self.store.pool())
        .await?;
        Ok(())
    }

    /// 汇总 `since_ms` 之后的统计。
    pub async fn summary(&self, since_ms: i64) -> Result<Statistics> {
        let records: Vec<String> = sqlx::query_scalar("SELECT model_performance_json FROM requests WHERE started_at >= ? AND model_performance_json IS NOT NULL ORDER BY started_at, rowid")
            .bind(since_ms).fetch_all(self.store.pool()).await?;
        let samples = records
            .iter()
            .map(|s| {
                serde_json::from_str::<ModelPerformance>(s)
                    .map_err(|e| crate::Error::Encoding(e.to_string()))
            })
            .collect::<Result<Vec<_>>>()?;
        let requests: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM requests WHERE started_at >= ?1")
                .bind(since_ms)
                .fetch_one(self.store.pool())
                .await?;

        let durations: Vec<i64> = sqlx::query_scalar(
            "SELECT duration_ms FROM requests WHERE started_at >= ?1 ORDER BY duration_ms",
        )
        .bind(since_ms)
        .fetch_all(self.store.pool())
        .await?;

        let inference: Vec<i64> =
            sqlx::query_scalar("SELECT inference_ms FROM requests WHERE started_at >= ?1")
                .bind(since_ms)
                .fetch_all(self.store.pool())
                .await?;

        let upstream: Vec<i64> =
            sqlx::query_scalar("SELECT upstream_ms FROM requests WHERE started_at >= ?1")
                .bind(since_ms)
                .fetch_all(self.store.pool())
                .await?;

        let usage: Vec<String> =
            sqlx::query_scalar("SELECT cache_usage_json FROM requests WHERE started_at >= ?1")
                .bind(since_ms)
                .fetch_all(self.store.pool())
                .await?;

        let fragments: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM fragments WHERE created_at >= ?1")
                .bind(since_ms)
                .fetch_one(self.store.pool())
                .await?;

        let spans: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM spans WHERE created_at >= ?1")
            .bind(since_ms)
            .fetch_one(self.store.pool())
            .await?;

        let released_spans = self
            .count_spans_with_action(since_ms, Action::Release)
            .await?;
        let redacted_spans = self
            .count_spans_with_action(since_ms, Action::Redact)
            .await?;

        let group_rows = sqlx::query(
            "SELECT entity_group, COUNT(*) AS count FROM spans
             WHERE created_at >= ?1 GROUP BY entity_group ORDER BY count DESC, entity_group",
        )
        .bind(since_ms)
        .fetch_all(self.store.pool())
        .await?;
        let by_entity_group = group_rows
            .iter()
            .map(|row| {
                Ok(EntityGroupCount {
                    entity_group: codec::decode(&row.try_get::<String, _>("entity_group")?)?,
                    count: row.try_get("count")?,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let (cached_fragments, inferred_fragments) = usage.iter().fold((0, 0), |totals, json| {
            let value: serde_json::Value = serde_json::from_str(json).unwrap_or_default();
            (
                totals.0 + value["cached_fragments"].as_i64().unwrap_or_default(),
                totals.1 + value["inferred_fragments"].as_i64().unwrap_or_default(),
            )
        });

        Ok(Statistics {
            model: ModelSummary::from_samples(&samples),
            since_ms,
            requests,
            fragments,
            spans,
            released_spans,
            redacted_spans,
            cached_fragments,
            inferred_fragments,
            latency: LatencySummary {
                average_ms: mean(&durations),
                p50_ms: percentile(&durations, 0.50),
                p95_ms: percentile(&durations, 0.95),
                max_ms: durations.last().copied().unwrap_or_default(),
                average_inference_ms: mean(&inference),
                average_upstream_ms: mean(&upstream),
            },
            by_entity_group,
        })
    }

    async fn count_spans_with_action(&self, since_ms: i64, action: Action) -> Result<i64> {
        Ok(
            sqlx::query_scalar("SELECT COUNT(*) FROM spans WHERE created_at >= ?1 AND action = ?2")
                .bind(since_ms)
                .bind(codec::encode(&action)?)
                .fetch_one(self.store.pool())
                .await?,
        )
    }
}

fn mean(values: &[i64]) -> i64 {
    if values.is_empty() {
        return 0;
    }
    values.iter().sum::<i64>() / values.len() as i64
}

/// 最近秩法分位：`values` 必须已升序。空集合返回 0，避免把「没有数据」显示成有延迟。
fn percentile(values: &[i64], fraction: f64) -> i64 {
    if values.is_empty() {
        return 0;
    }
    // 最近秩：取第 ceil(p * n) 个观测值，下标从 0 起。
    let rank = (fraction * values.len() as f64).ceil() as usize;
    values[rank.saturating_sub(1).min(values.len() - 1)]
}
