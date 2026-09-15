use crate::{Error, Result, Store, codec};
use privacy_rules::{Rule, RuleId, RuleSet};
use sqlx::Row;
use sqlx::sqlite::SqliteRow;

/// 规则表的读写。
pub struct RuleRepository<'a> {
    store: &'a Store,
}

impl<'a> RuleRepository<'a> {
    pub(crate) fn new(store: &'a Store) -> Self {
        Self { store }
    }

    /// 在数据库生命周期内安装一次默认规则。之后的编辑与删除都由管理员决定。
    pub async fn seed_builtin(&self, rules: &[Rule]) -> Result<usize> {
        let now = self.store.now_ms();
        let mut transaction = self.store.pool().begin().await?;
        let claimed = sqlx::query(
            "INSERT INTO initialization (key, completed_at) VALUES ('builtin-rules-v1', ?1)
             ON CONFLICT (key) DO NOTHING",
        )
        .bind(now)
        .execute(&mut *transaction)
        .await?;
        if claimed.rows_affected() == 0 {
            return Ok(0);
        }

        let mut inserted = 0;
        for rule in rules {
            let result = sqlx::query(
                "INSERT INTO rules (id, name, priority, condition_json, action, enabled, source, category, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
                 ON CONFLICT (id) DO NOTHING",
            )
            .bind(rule.id.as_str())
            .bind(&rule.name)
            .bind(i64::from(rule.priority))
            .bind(encode_condition(rule)?)
            .bind(codec::encode(&rule.action)?)
            .bind(i64::from(rule.enabled))
            .bind(codec::encode(&rule.source)?)
            .bind(encode_category(rule)?)
            .bind(now)
            .execute(&mut *transaction)
            .await?;
            inserted += result.rows_affected() as usize;
        }
        transaction.commit().await?;
        Ok(inserted)
    }

    pub async fn list(&self) -> Result<Vec<Rule>> {
        let rows = sqlx::query(
            "SELECT id, name, priority, condition_json, action, enabled, source, category
             FROM rules ORDER BY priority DESC, id",
        )
        .fetch_all(self.store.pool())
        .await?;
        rows.iter().map(decode_rule).collect()
    }

    pub async fn find(&self, id: &RuleId) -> Result<Option<Rule>> {
        let row = sqlx::query(
            "SELECT id, name, priority, condition_json, action, enabled, source, category
             FROM rules WHERE id = ?1",
        )
        .bind(id.as_str())
        .fetch_optional(self.store.pool())
        .await?;
        row.as_ref().map(decode_rule).transpose()
    }

    /// 构造供判定使用的规则集合。控制台规则、管理员登记与内置规则在此汇合，只有一条判定路径。
    pub async fn load_set(&self) -> Result<RuleSet> {
        let rules = RuleSet::new(self.list().await?);
        let action: Option<String> =
            sqlx::query_scalar("SELECT fallback_action FROM rule_policy WHERE id = 1")
                .fetch_optional(self.store.pool())
                .await?;
        Ok(match action {
            Some(action) => rules.with_fallback(codec::decode(&action)?),
            None => rules,
        })
    }

    pub async fn set_fallback(&self, action: privacy_rules::Action) -> Result<()> {
        sqlx::query("INSERT INTO rule_policy (id, fallback_action) VALUES (1, ?1) ON CONFLICT(id) DO UPDATE SET fallback_action = excluded.fallback_action")
            .bind(codec::encode(&action)?).execute(self.store.pool()).await?;
        Ok(())
    }

    pub async fn create(&self, rule: &Rule) -> Result<()> {
        rule.validate()?;
        let now = self.store.now_ms();
        let result = sqlx::query(
            "INSERT INTO rules (id, name, priority, condition_json, action, enabled, source, category, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(rule.id.as_str())
        .bind(&rule.name)
        .bind(i64::from(rule.priority))
        .bind(encode_condition(rule)?)
        .bind(codec::encode(&rule.action)?)
        .bind(i64::from(rule.enabled))
        .bind(codec::encode(&rule.source)?)
        .bind(encode_category(rule)?)
        .bind(now)
        .execute(self.store.pool())
        .await?;

        if result.rows_affected() == 0 {
            return Err(Error::RuleIdTaken(rule.id.to_string()));
        }
        Ok(())
    }

    /// 覆盖式更新。规则来源保留不变，其余字段均可由管理员修改。
    pub async fn update(&self, rule: &Rule) -> Result<()> {
        rule.validate()?;
        let result = sqlx::query(
            "UPDATE rules
             SET name = ?1, priority = ?2, condition_json = ?3, action = ?4, enabled = ?5,
                 category = ?6, updated_at = ?7
             WHERE id = ?8",
        )
        .bind(&rule.name)
        .bind(i64::from(rule.priority))
        .bind(encode_condition(rule)?)
        .bind(codec::encode(&rule.action)?)
        .bind(i64::from(rule.enabled))
        .bind(encode_category(rule)?)
        .bind(self.store.now_ms())
        .bind(rule.id.as_str())
        .execute(self.store.pool())
        .await?;

        self.explain_absent_row(&rule.id, result.rows_affected())
            .await
    }

    pub async fn set_enabled(&self, id: &RuleId, enabled: bool) -> Result<()> {
        let result = sqlx::query("UPDATE rules SET enabled = ?1, updated_at = ?2 WHERE id = ?3")
            .bind(i64::from(enabled))
            .bind(self.store.now_ms())
            .bind(id.as_str())
            .execute(self.store.pool())
            .await?;

        self.explain_absent_row(id, result.rows_affected()).await
    }

    /// 删除任意规则。
    pub async fn delete(&self, id: &RuleId) -> Result<()> {
        let result = sqlx::query("DELETE FROM rules WHERE id = ?1")
            .bind(id.as_str())
            .execute(self.store.pool())
            .await?;

        self.explain_absent_row(id, result.rows_affected()).await
    }

    /// 把「没有行被影响」转换为稳定的未找到错误。
    async fn explain_absent_row(&self, id: &RuleId, affected: u64) -> Result<()> {
        if affected > 0 {
            return Ok(());
        }
        match self.find(id).await? {
            None => Err(Error::NotFound(format!("rule {id}"))),
            Some(_) => Err(Error::NotFound(format!("rule {id}"))),
        }
    }
}

fn encode_condition(rule: &Rule) -> Result<String> {
    serde_json::to_string(&rule.condition).map_err(|error| Error::Encoding(error.to_string()))
}

fn encode_category(rule: &Rule) -> Result<Option<String>> {
    rule.category.map(|group| codec::encode(&group)).transpose()
}

fn decode_rule(row: &SqliteRow) -> Result<Rule> {
    let rule = Rule {
        id: RuleId::new(row.try_get::<String, _>("id")?)?,
        name: row.try_get("name")?,
        priority: row.try_get::<i64, _>("priority")? as i32,
        condition: serde_json::from_str(&row.try_get::<String, _>("condition_json")?)
            .map_err(|error| Error::Encoding(error.to_string()))?,
        action: codec::decode(&row.try_get::<String, _>("action")?)?,
        enabled: row.try_get::<i64, _>("enabled")? != 0,
        source: codec::decode(&row.try_get::<String, _>("source")?)?,
        category: row
            .try_get::<Option<String>, _>("category")?
            .map(|category| codec::decode(&category))
            .transpose()?,
    };
    // 读出来就要能用：缺少类别的登记会让「禁止」少拦东西，宁可在加载时失败。
    rule.validate()?;
    Ok(rule)
}
