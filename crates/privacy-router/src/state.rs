//! 共享状态。规则集合放在可整体替换的槽里：控制台改动规则后立即可见，读路径不加锁等待
//! 数据库。

use crate::classifier::Classifier;
use crate::config::ServerArgs;
use privacy_rules::{Action, Rule, RuleId, RuleSet};
use privacy_store::Store;
use std::sync::{Arc, RwLock};

/// 进程级共享状态。
pub struct AppState {
    pub store: Arc<Store>,
    pub classifier: Arc<dyn Classifier>,
    pub http: reqwest::Client,
    pub args: Arc<ServerArgs>,
    rules: RwLock<Arc<RuleSet>>,
}

impl AppState {
    pub fn new(
        store: Arc<Store>,
        classifier: Arc<dyn Classifier>,
        http: reqwest::Client,
        args: Arc<ServerArgs>,
        rules: RuleSet,
    ) -> Self {
        Self {
            store,
            classifier,
            http,
            args,
            rules: RwLock::new(Arc::new(rules)),
        }
    }

    /// 取得当前规则集合的快照。判定路径只做一次 Arc 克隆，不持有锁。
    pub fn rules(&self) -> Arc<RuleSet> {
        self.rules.read().expect("规则槽未被毒化").clone()
    }

    /// 规则改动后从数据库重新装配。控制台的每次写操作都会调用它。
    ///
    /// 先在锁外完成读取与装配，再整体替换：写锁只覆盖一次指针交换，判定路径不会被
    /// 数据库读取阻塞。
    pub async fn reload_rules(&self) -> Result<(), privacy_store::Error> {
        let set = self.store.rules().load_set().await?;
        *self.rules.write().expect("规则槽未被毒化") = Arc::new(set);
        Ok(())
    }

    /// 登记管理员对一条命中的决定：之后完全相同的一段文本按 `action` 处置。
    ///
    /// 命中不存在时返回 `None`。登记的身份是这段文本本身，方向不是：同一段内容只会有一条
    /// 登记，重复同一个方向复用既有规则，改主意则改写它——否则列表里会同时躺着同一段文本的
    /// 两个方向，谁算数只能靠优先级去推。布尔值表示这次调用有没有改动登记。
    ///
    /// 早先「改主意」是再加一条反方向的规则，因此这段文本可能已经躺着多条登记。这一次点击
    /// 顺带把它们收敛成一条：留下的那条按 `action` 改写，其余全部删除。
    ///
    /// 决定登记完立即重新装配，对后续请求即时生效。
    pub async fn record_span_decision(
        &self,
        span_id: &str,
        action: Action,
    ) -> Result<Option<(RuleId, bool)>, privacy_store::Error> {
        let Some(span) = self.store.pool_records().find_span(span_id).await? else {
            return Ok(None);
        };

        // 判定、展示、去重都问同一个快照，避免彼此看到不同的登记集合。
        let rules = self.rules();
        if let Some(existing) = rules.operator_decision(&span.original_text) {
            let rule_id = existing.id.clone();
            let superseded: Vec<RuleId> = rules
                .registrations_of(&span.original_text)
                .map(|rule| rule.id.clone())
                .filter(|id| *id != rule_id)
                .collect();

            if existing.action == action && superseded.is_empty() {
                return Ok(Some((rule_id, false)));
            }

            let rule = existing.clone().redecided(action);
            self.store.rules().update(&rule).await?;
            for stale in &superseded {
                self.store.rules().delete(stale).await?;
            }
            self.reload_rules().await?;
            tracing::info!(
                stage = "console",
                entity_group = %span.entity_group,
                rule = %rule.id,
                action = ?rule.action,
                superseded = superseded.len(),
                "content decision changed by operator"
            );
            return Ok(Some((rule_id, true)));
        }

        // 标识只需唯一且非空：内容本身不进标识，规则列表就不会成为泄露渠道。
        let id = RuleId::new(format!("operator.{}", uuid::Uuid::new_v4())).expect("生成的标识非空");
        let rule = match action {
            Action::Release => {
                Rule::operator_release(id, span.original_text.clone(), span.entity_group)
            }
            Action::Redact => {
                Rule::operator_redact(id, span.original_text.clone(), span.entity_group)
            }
        };
        self.store.rules().create(&rule).await?;
        self.reload_rules().await?;

        tracing::info!(
            stage = "console",
            entity_group = %span.entity_group,
            rule = %rule.id,
            action = ?rule.action,
            "content decision recorded by operator"
        );
        Ok(Some((rule.id, true)))
    }
}
