use crate::{Error, Result, Store, codec};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use sqlx::sqlite::SqliteRow;
use uuid::Uuid;

/// 上游报文格式。决定入站路由、上游路径与需要脱敏的字段位置。
///
/// 序列化名称是数据库 `CHECK` 约束与线上重命名的共同来源，因此逐条显式给出：
/// `snake_case` 会把 `OpenAiChat` 拆成 `open_ai_chat`，与协议自身的写法不符。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApiFormat {
    #[serde(rename = "openai_responses")]
    OpenAiResponses,
    #[serde(rename = "openai_chat")]
    OpenAiChat,
    #[serde(rename = "anthropic_messages")]
    AnthropicMessages,
}

/// 上游 provider 配置。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api_format: ApiFormat,
    pub enabled: bool,
}

/// 控制台提交的新建或修改内容；`id` 由仓储生成。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderDraft {
    pub name: String,
    pub base_url: String,
    pub api_format: ApiFormat,
    pub enabled: bool,
}

impl ProviderDraft {
    /// 校验配置自洽性。跨字段约束与数据库 `CHECK` 使用同一套规则。
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(Error::InvalidProvider("名称不能为空".into()));
        }
        let url = self.base_url.trim();
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            return Err(Error::InvalidProvider(
                "base_url 必须以 http:// 或 https:// 开头".into(),
            ));
        }
        Ok(())
    }
}

/// provider 表的读写。
pub struct ProviderRepository<'a> {
    store: &'a Store,
}

impl<'a> ProviderRepository<'a> {
    pub(crate) fn new(store: &'a Store) -> Self {
        Self { store }
    }

    pub async fn list(&self) -> Result<Vec<Provider>> {
        let rows = sqlx::query(
            "SELECT id, name, base_url, api_format, enabled
             FROM providers ORDER BY name",
        )
        .fetch_all(self.store.pool())
        .await?;
        rows.iter().map(decode_provider).collect()
    }

    pub async fn find(&self, id: &str) -> Result<Option<Provider>> {
        let row = sqlx::query(
            "SELECT id, name, base_url, api_format, enabled
             FROM providers WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(self.store.pool())
        .await?;
        row.as_ref().map(decode_provider).transpose()
    }

    /// 按名称解析上游。名称在库内唯一，因此结果确定。
    pub async fn find_by_name(&self, name: &str) -> Result<Option<Provider>> {
        let row = sqlx::query(
            "SELECT id, name, base_url, api_format, enabled
             FROM providers WHERE name = ?1",
        )
        .bind(name)
        .fetch_optional(self.store.pool())
        .await?;
        row.as_ref().map(decode_provider).transpose()
    }

    pub async fn create(&self, draft: &ProviderDraft) -> Result<Provider> {
        draft.validate()?;
        let now = self.store.now_ms();
        let provider = Provider {
            id: Uuid::new_v4().to_string(),
            name: draft.name.trim().to_owned(),
            base_url: draft.base_url.trim().trim_end_matches('/').to_owned(),
            api_format: draft.api_format,
            enabled: draft.enabled,
        };

        sqlx::query(
            "INSERT INTO providers (id, name, base_url, api_format, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        )
        .bind(&provider.id)
        .bind(&provider.name)
        .bind(&provider.base_url)
        .bind(codec::encode(&provider.api_format)?)
        .bind(i64::from(provider.enabled))
        .bind(now)
        .execute(self.store.pool())
        .await
        .map_err(unique_name_conflict(&provider.name))?;

        Ok(provider)
    }

    pub async fn update(&self, id: &str, draft: &ProviderDraft) -> Result<Provider> {
        draft.validate()?;
        let provider = Provider {
            id: id.to_owned(),
            name: draft.name.trim().to_owned(),
            base_url: draft.base_url.trim().trim_end_matches('/').to_owned(),
            api_format: draft.api_format,
            enabled: draft.enabled,
        };

        let result = sqlx::query(
            "UPDATE providers
             SET name = ?1, base_url = ?2, api_format = ?3, enabled = ?4, updated_at = ?5
             WHERE id = ?6",
        )
        .bind(&provider.name)
        .bind(&provider.base_url)
        .bind(codec::encode(&provider.api_format)?)
        .bind(i64::from(provider.enabled))
        .bind(self.store.now_ms())
        .bind(id)
        .execute(self.store.pool())
        .await
        .map_err(unique_name_conflict(&provider.name))?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound(format!("provider {id}")));
        }
        Ok(provider)
    }

    pub async fn delete(&self, id: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM providers WHERE id = ?1")
            .bind(id)
            .execute(self.store.pool())
            .await?;
        if result.rows_affected() == 0 {
            return Err(Error::NotFound(format!("provider {id}")));
        }
        Ok(())
    }
}

/// 名称唯一约束的冲突要变成可读错误，而不是把驱动错误原样抛给控制台。
fn unique_name_conflict(name: &str) -> impl Fn(sqlx::Error) -> Error + '_ {
    move |error| match &error {
        sqlx::Error::Database(database) if database.is_unique_violation() => {
            Error::ProviderNameTaken(name.to_owned())
        }
        _ => Error::Database(error),
    }
}

fn decode_provider(row: &SqliteRow) -> Result<Provider> {
    Ok(Provider {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        base_url: row.try_get("base_url")?,
        api_format: codec::decode(&row.try_get::<String, _>("api_format")?)?,
        enabled: row.try_get::<i64, _>("enabled")? != 0,
    })
}
