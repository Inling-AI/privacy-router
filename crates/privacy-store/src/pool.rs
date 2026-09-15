use crate::provider::ApiFormat;
use crate::{Error, Result, Store, codec};
use privacy_rules::Action;
use privacy_rules::EntityGroup;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use sqlx::sqlite::SqliteRow;
use std::collections::HashMap;
use uuid::Uuid;

/// 内容池的复核过滤：聚合之后哪些行值得人看。
///
/// 复核的对象是**还没被人决定过**的内容。一行内容有没有被登记过，就是它有没有被审过；
/// 判定稳不稳只影响排序——摇摆得越厉害越该先看，但它不是「审没审过」的定义。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ContentFilter {
    /// 还没有登记过决定的内容。这是默认视图。
    #[default]
    Unreviewed,
    /// 已经登记过决定的内容。
    Reviewed,
    /// 全部有命中的内容。
    All,
}

/// 同一段内容在池子里的判定统计。
///
/// 聚合身份是原文本身：登记规则按整段文本逐字匹配，所以「列表里的一行」与「一次登记」必须
/// 是同一个东西。类别不进身份——同一段文本被认成不同类别时仍是一行，类别作为行内证据展示。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContentSummary {
    /// 最近一次出现的命中。详情与登记都用它做锚点，URL 里永远不放被审查的文本本身。
    pub anchor_span_id: String,
    pub original_text: String,
    /// 出现次数，以及它分布在多少个 turn 上。
    pub occurrences: i64,
    pub turns: i64,
    pub released: i64,
    pub redacted: i64,
    /// 置信度区间：它直接解释这段文本为什么摇摆。
    pub score_min: f64,
    pub score_max: f64,
    pub first_seen: i64,
    pub last_seen: i64,
    /// 最近一次的处置。
    pub latest_action: Action,
    /// 这段文本被识别成过哪些类别、各多少次；次数最多的排在最前。
    pub categories: Vec<ContentCategory>,
}

/// 一段内容在一个类别上的出现次数。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContentCategory {
    pub entity_group: EntityGroup,
    pub count: i64,
}

/// 同一段内容的某一次出现。带上来源与判定，但不带整段 turn 原文：复核的对象是这段内容。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContentOccurrence {
    pub span_id: String,
    pub entity_group: EntityGroup,
    pub score: f64,
    pub action: Action,
    pub matched_rule_id: Option<String>,
    pub api_format: ApiFormat,
    pub path: String,
    pub created_at: i64,
}

/// 一段被判定过的文本及其替换结果。放行与抹去的内容都入库。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordedFragment {
    pub id: String,
    pub request_id: String,
    pub api_format: ApiFormat,
    pub path: String,
    pub original_text: String,
    pub redacted_text: String,
    pub created_at: i64,
}

/// 片段内的一条命中。原文保留在此，供控制台逐条复查与放行。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordedSpan {
    pub id: String,
    pub entity_group: EntityGroup,
    pub score: f64,
    pub char_len: usize,
    pub byte_start: usize,
    pub byte_end: usize,
    pub original_text: String,
    pub action: Action,
    pub matched_rule_id: Option<String>,
}

/// 待写入的片段连同它的命中列表。
#[derive(Debug, Clone, PartialEq)]
pub struct FragmentRecording {
    pub request_id: String,
    pub api_format: ApiFormat,
    pub path: String,
    pub content_hash: Vec<u8>,
    pub original_text: String,
    pub redacted_text: String,
    pub spans: Vec<RecordedSpan>,
}

/// 一条命中所属的那一次 turn：进模型前收到的原文、转发出去的文本，以及这次的全部命中。
///
/// 逐条命中都在 `spans` 里，`fragment` 给出两者之差——被抹掉的内容就是原文里不再等于
/// 转发文本的部分。查看一次判定只需要这一份读取，调用方不再自己拼片段与命中。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FragmentDetail {
    pub fragment: RecordedFragment,
    pub spans: Vec<RecordedSpan>,
}

/// 内容池的写入与查询。
pub struct PoolRepository<'a> {
    store: &'a Store,
}

impl<'a> PoolRepository<'a> {
    pub(crate) fn new(store: &'a Store) -> Self {
        Self { store }
    }

    /// 写入片段与它的命中列表。两者在同一事务内提交，避免出现没有命中的片段。
    pub async fn record(&self, recording: &FragmentRecording) -> Result<String> {
        let now = self.store.now_ms();
        let fragment_id = Uuid::new_v4().to_string();
        let mut transaction = self.store.pool().begin().await?;

        sqlx::query(
            "INSERT INTO fragments (id, request_id, api_format, path, content_hash, original_text, redacted_text, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind(&fragment_id)
        .bind(&recording.request_id)
        .bind(codec::encode(&recording.api_format)?)
        .bind(&recording.path)
        .bind(&recording.content_hash)
        .bind(&recording.original_text)
        .bind(&recording.redacted_text)
        .bind(now)
        .execute(&mut *transaction)
        .await?;

        for span in &recording.spans {
            sqlx::query(
                "INSERT INTO spans (id, fragment_id, entity_group, score, char_len, byte_start, byte_end, original_text, action, matched_rule_id, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            )
            .bind(Uuid::new_v4().to_string())
            .bind(&fragment_id)
            .bind(codec::encode(&span.entity_group)?)
            .bind(span.score)
            .bind(span.char_len as i64)
            .bind(span.byte_start as i64)
            .bind(span.byte_end as i64)
            .bind(&span.original_text)
            .bind(codec::encode(&span.action)?)
            .bind(&span.matched_rule_id)
            .bind(now)
            .execute(&mut *transaction)
            .await?;
        }

        transaction.commit().await?;
        Ok(fragment_id)
    }

    pub async fn count(&self) -> Result<i64> {
        Ok(sqlx::query_scalar("SELECT COUNT(*) FROM fragments")
            .fetch_one(self.store.pool())
            .await?)
    }

    pub async fn find(&self, id: &str) -> Result<Option<RecordedFragment>> {
        let row = sqlx::query(
            "SELECT id, request_id, api_format, path, original_text, redacted_text, created_at
             FROM fragments WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(self.store.pool())
        .await?;
        row.as_ref().map(decode_fragment).transpose()
    }

    pub async fn spans_of(&self, fragment_id: &str) -> Result<Vec<RecordedSpan>> {
        let rows = sqlx::query(
            "SELECT id, entity_group, score, char_len, byte_start, byte_end, original_text, action, matched_rule_id
             FROM spans WHERE fragment_id = ?1 ORDER BY byte_start",
        )
        .bind(fragment_id)
        .fetch_all(self.store.pool())
        .await?;
        rows.iter().map(decode_span).collect()
    }

    pub async fn find_span(&self, id: &str) -> Result<Option<RecordedSpan>> {
        let row = sqlx::query(
            "SELECT id, entity_group, score, char_len, byte_start, byte_end, original_text, action, matched_rule_id
             FROM spans WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(self.store.pool())
        .await?;
        row.as_ref().map(decode_span).transpose()
    }

    /// 一条命中所属的 turn；`None` 表示这条命中不存在。
    pub async fn fragment_of_span(&self, span_id: &str) -> Result<Option<FragmentDetail>> {
        let row = sqlx::query(
            "SELECT f.id, f.request_id, f.api_format, f.path, f.original_text, f.redacted_text, f.created_at
               FROM fragments f JOIN spans s ON s.fragment_id = f.id
              WHERE s.id = ?1",
        )
        .bind(span_id)
        .fetch_optional(self.store.pool())
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let fragment = decode_fragment(&row)?;
        let spans = self.spans_of(&fragment.id).await?;
        Ok(Some(FragmentDetail { fragment, spans }))
    }

    /// 符合过滤条件的内容条数。
    ///
    /// `registered` 是规则集合里全部登记过的原文：「这段内容审过没有」只有一个答案，
    /// 它由规则集合给出，SQL 不自己解析规则条件。
    pub async fn content_count(&self, filter: ContentFilter, registered: &[String]) -> Result<i64> {
        let mut query = sqlx::query_scalar::<_, i64>(sqlx::AssertSqlSafe(format!(
            "SELECT COUNT(*) FROM (SELECT original_text, \
                    SUM(action = ?1) AS released, SUM(action = ?2) AS redacted \
               FROM spans GROUP BY original_text) AS g \
              WHERE {}",
            filter.clause(&placeholders(3, registered.len()))
        )))
        .bind(codec::encode(&Action::Release)?)
        .bind(codec::encode(&Action::Redact)?);
        for text in registered {
            query = query.bind(text);
        }
        Ok(query.fetch_one(self.store.pool()).await?)
    }

    /// 按内容聚合的一页，最该先看的排在最前。
    ///
    /// 排序即复核顺序：判定不一致的行永远在前，其次是出现得多的，最后按最近出现的时间。
    /// 原文参与排序只是为了分页确定性——没有它，同分的行在不同次查询里顺序可以不一样。
    pub async fn content_page(
        &self,
        filter: ContentFilter,
        registered: &[String],
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ContentSummary>> {
        // 动态的只有排序片段与占位符个数这类常量：登记过的原文全部走 bind，没有任何数据被拼进 SQL。
        let limit_bind = 3 + registered.len();
        let offset_bind = limit_bind + 1;
        let mut query = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT g.original_text, g.occurrences, g.turns, g.released, g.redacted,
                    g.score_min, g.score_max, g.first_seen, g.last_seen,
                    (SELECT s.id FROM spans s WHERE s.original_text = g.original_text
                      {LATEST_FIRST} LIMIT 1) AS anchor_span_id,
                    (SELECT s.action FROM spans s WHERE s.original_text = g.original_text
                      {LATEST_FIRST} LIMIT 1) AS latest_action
               FROM (SELECT original_text, COUNT(*) AS occurrences,
                            COUNT(DISTINCT fragment_id) AS turns,
                            SUM(action = ?1) AS released, SUM(action = ?2) AS redacted,
                            MIN(score) AS score_min, MAX(score) AS score_max,
                            MIN(created_at) AS first_seen, MAX(created_at) AS last_seen
                       FROM spans GROUP BY original_text) AS g
              WHERE {}
              ORDER BY (g.released > 0 AND g.redacted > 0) DESC, g.occurrences DESC,
                       g.last_seen DESC, g.original_text
              LIMIT ?{limit_bind} OFFSET ?{offset_bind}",
            filter.clause(&placeholders(3, registered.len()))
        )))
        .bind(codec::encode(&Action::Release)?)
        .bind(codec::encode(&Action::Redact)?);
        for text in registered {
            query = query.bind(text);
        }
        let rows = query
            .bind(limit)
            .bind(offset)
            .fetch_all(self.store.pool())
            .await?;

        let texts: Vec<String> = rows
            .iter()
            .map(|row| row.try_get::<String, _>("original_text"))
            .collect::<std::result::Result<_, _>>()?;
        let mut categories = self.categories_of(&texts).await?;

        rows.iter()
            .map(|row| {
                let original_text: String = row.try_get("original_text")?;
                Ok(ContentSummary {
                    anchor_span_id: row.try_get("anchor_span_id")?,
                    categories: categories.remove(&original_text).unwrap_or_default(),
                    original_text,
                    occurrences: row.try_get("occurrences")?,
                    turns: row.try_get("turns")?,
                    released: row.try_get("released")?,
                    redacted: row.try_get("redacted")?,
                    score_min: row.try_get("score_min")?,
                    score_max: row.try_get("score_max")?,
                    first_seen: row.try_get("first_seen")?,
                    last_seen: row.try_get("last_seen")?,
                    latest_action: codec::decode(&row.try_get::<String, _>("latest_action")?)?,
                })
            })
            .collect()
    }

    /// 这些文本各自被识别成过哪些类别。
    ///
    /// 类别不是聚合身份，只是一页之内的证据，所以它是第二条查询：把 IN 列表限制在这一页，
    /// 而不是让聚合本身按 (文本, 类别) 分行。
    async fn categories_of(
        &self,
        texts: &[String],
    ) -> Result<HashMap<String, Vec<ContentCategory>>> {
        if texts.is_empty() {
            return Ok(HashMap::new());
        }
        let placeholders = (1..=texts.len())
            .map(|index| format!("?{index}"))
            .collect::<Vec<_>>()
            .join(", ");
        // 动态的只有占位符个数，文本本身全部走 bind；这里没有拼接任何数据。
        let mut query = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT original_text, entity_group, COUNT(*) AS count FROM spans
              WHERE original_text IN ({placeholders})
              GROUP BY original_text, entity_group
              ORDER BY original_text, count DESC, entity_group"
        )));
        for text in texts {
            query = query.bind(text);
        }

        let mut grouped: HashMap<String, Vec<ContentCategory>> = HashMap::new();
        for row in query.fetch_all(self.store.pool()).await? {
            let text: String = row.try_get("original_text")?;
            grouped.entry(text).or_default().push(ContentCategory {
                entity_group: codec::decode(&row.try_get::<String, _>("entity_group")?)?,
                count: row.try_get("count")?,
            });
        }
        Ok(grouped)
    }

    /// 这段文本的全部出现，最近的在前。
    pub async fn occurrences_of(&self, text: &str) -> Result<Vec<ContentOccurrence>> {
        let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT s.id, s.entity_group, s.score, s.action, s.matched_rule_id, s.created_at,
                        f.api_format, f.path
                   FROM spans s JOIN fragments f ON f.id = s.fragment_id
                  WHERE s.original_text = ?1
                  {LATEST_FIRST}"
        )))
        .bind(text)
        .fetch_all(self.store.pool())
        .await?;
        rows.iter()
            .map(|row| {
                Ok(ContentOccurrence {
                    span_id: row.try_get("id")?,
                    entity_group: codec::decode(&row.try_get::<String, _>("entity_group")?)?,
                    score: row.try_get("score")?,
                    action: codec::decode(&row.try_get::<String, _>("action")?)?,
                    matched_rule_id: row.try_get("matched_rule_id")?,
                    api_format: codec::decode(&row.try_get::<String, _>("api_format")?)?,
                    path: row.try_get("path")?,
                    created_at: row.try_get("created_at")?,
                })
            })
            .collect()
    }
}

impl ContentFilter {
    /// 这段过滤条件在聚合结果上的 SQL 条件。
    ///
    /// 登记过的原文决定「审没审过」，它们由调用方 bind 进来，这里只决定用哪种比较。
    fn clause(self, placeholders: &str) -> String {
        match self {
            Self::All => "1 = 1".to_owned(),
            // 还没有任何登记时，「已审」必然是空集、「未审」必然是全集；空列表写进 IN 会让
            // SQLite 把整段条件判成空集，这两种情况要在这里说清楚。
            Self::Reviewed if placeholders.is_empty() => "1 = 0".to_owned(),
            Self::Unreviewed if placeholders.is_empty() => "1 = 1".to_owned(),
            Self::Reviewed => format!("g.original_text IN ({placeholders})"),
            Self::Unreviewed => format!("g.original_text NOT IN ({placeholders})"),
        }
    }
}

/// 从 `first` 开始编号的占位符列表，供 `IN`/`NOT IN` 使用。
fn placeholders(first: usize, count: usize) -> String {
    (first..first + count)
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn decode_fragment(row: &SqliteRow) -> Result<RecordedFragment> {
    Ok(RecordedFragment {
        id: row.try_get("id")?,
        request_id: row.try_get("request_id")?,
        api_format: codec::decode(&row.try_get::<String, _>("api_format")?)?,
        path: row.try_get("path")?,
        original_text: row.try_get("original_text")?,
        redacted_text: row.try_get("redacted_text")?,
        created_at: row.try_get("created_at")?,
    })
}

fn decode_span(row: &SqliteRow) -> Result<RecordedSpan> {
    let byte_start = row.try_get::<i64, _>("byte_start")?;
    let byte_end = row.try_get::<i64, _>("byte_end")?;
    if byte_start < 0 || byte_end < byte_start {
        return Err(Error::Encoding(format!(
            "span 偏移不自洽：{byte_start}..{byte_end}"
        )));
    }
    Ok(RecordedSpan {
        id: row.try_get("id")?,
        entity_group: codec::decode(&row.try_get::<String, _>("entity_group")?)?,
        score: row.try_get("score")?,
        char_len: row.try_get::<i64, _>("char_len")?.max(0) as usize,
        byte_start: byte_start as usize,
        byte_end: byte_end as usize,
        original_text: row.try_get("original_text")?,
        action: codec::decode(&row.try_get::<String, _>("action")?)?,
        matched_rule_id: row.try_get("matched_rule_id")?,
    })
}

/// 「最近一次出现」的排序：同一毫秒内的两次出现仍有先后，`rowid` 就是写入顺序。
///
/// 时间戳精度是毫秒，一个 turn 里的多段内容必然撞在同一毫秒；用随机标识做次序只会让
/// 「最近一次」变成随机数。
const LATEST_FIRST: &str = "ORDER BY s.created_at DESC, s.rowid DESC";
