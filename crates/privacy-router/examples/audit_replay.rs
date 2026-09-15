//! 只读回放审计数据库，报告修改前后数量，不输出原文或凭据。
use privacy_filter::Entity;
use privacy_router::redaction::redact;
use privacy_rules::{Action, RuleSet};
use sqlx::{
    Row,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use std::collections::{BTreeMap, HashSet};

struct Audit;
impl Audit {
    async fn run() -> Result<(), Box<dyn std::error::Error>> {
        let path = std::env::args().nth(1).ok_or("需要数据库路径")?;
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(SqliteConnectOptions::new().filename(path).read_only(true))
            .await?;
        let mut transaction = pool.begin().await?;
        let rows = sqlx::query("SELECT f.content_hash,f.original_text AS text,s.entity_group,s.score,s.byte_start,s.byte_end,s.original_text,s.action FROM fragments f JOIN spans s ON s.fragment_id=f.id ORDER BY f.created_at DESC")
            .fetch_all(&mut *transaction).await?;
        let mut seen = HashSet::new();
        let rules = RuleSet::builtin().with_fallback(Action::Redact);
        let mut counts = BTreeMap::<String, usize>::new();
        for row in rows {
            let start = row.get::<i64, _>("byte_start") as usize;
            let end = row.get::<i64, _>("byte_end") as usize;
            let group: String = row.get("entity_group");
            if !seen.insert((
                row.get::<Vec<u8>, _>("content_hash"),
                start,
                end,
                group.clone(),
            )) {
                continue;
            }
            let text: String = row.get("text");
            let entity = Entity {
                entity_group: group.parse()?,
                score: row.get("score"),
                start,
                end,
                word: row.get("original_text"),
            };
            let outcome = redact(&text, &[entity], &rules);
            let old: String = row.get("action");
            let new = if outcome.changed() {
                "redact"
            } else {
                "release"
            };
            *counts.entry(format!("{old}->{new}")).or_default() += 1;
            if outcome
                .spans
                .iter()
                .any(|s| s.action == Action::Redact && (s.byte_start < start || s.byte_end > end))
            {
                *counts.entry("expanded_redactions".into()).or_default() += 1;
            }
            for span in outcome.spans {
                if let Some(id) = span
                    .matched_rule_id
                    .filter(|id| id.starts_with("builtin.release.technical"))
                {
                    *counts.entry(id).or_default() += 1;
                }
            }
        }
        transaction.rollback().await?;
        println!("{}", serde_json::to_string_pretty(&counts)?);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Audit::run().await
}
