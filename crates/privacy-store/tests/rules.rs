mod common;

use common::TempDatabase;
use privacy_filter::{Entity, EntityGroup};
use privacy_rules::{
    Action, ComparisonOperator, Rule, RuleExpression, RuleId, RuleSet, RuleSource, priority,
};
use rstest::rstest;
use sqlx::Row;

#[rstest]
#[case(Action::Redact)]
#[case(Action::Release)]
#[tokio::test]
async fn fallback_policy_survives_reopening_and_controls_unmatched_entities(
    #[case] action: Action,
) {
    let database = TempDatabase::new();
    let store = database.open().await;
    store.rules().set_fallback(action).await.unwrap();
    drop(store);
    let reopened = database.open().await;
    let rules = reopened.rules().load_set().await.unwrap();
    assert_eq!(rules.fallback(), action);
}

async fn table_names(database: &TempDatabase) -> Vec<String> {
    let store = database.open().await;
    let rows = sqlx::query("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .fetch_all(store.pool())
        .await
        .expect("可读取 schema");
    rows.iter()
        .map(|row| row.get::<String, _>("name"))
        .collect()
}

#[rstest]
#[tokio::test]
async fn migrations_create_every_table() {
    let database = TempDatabase::new();
    let tables = table_names(&database).await;

    for expected in [
        "admin",
        "fragments",
        "initialization",
        "providers",
        "requests",
        "rules",
        "sessions",
        "spans",
    ] {
        assert!(
            tables.contains(&expected.to_owned()),
            "缺少表 {expected}：{tables:?}"
        );
    }
}

#[rstest]
#[tokio::test]
async fn reopening_an_existing_database_is_idempotent() {
    let database = TempDatabase::new();
    let first = database.open().await;
    let rules = first.rules().list().await.expect("可列举");
    drop(first);

    // 第二次打开会重新执行迁移；重复执行不得失败或产生副作用。
    let second = database.open().await;
    assert_eq!(second.rules().list().await.expect("可列举"), rules);
}

#[rstest]
#[tokio::test]
async fn foreign_keys_are_enforced() {
    let database = TempDatabase::new();
    let store = database.open().await;

    // spans.fragment_id 指向不存在的片段必须被拒绝，否则级联删除会静默失效。
    let error = sqlx::query(
        "INSERT INTO spans (id, fragment_id, entity_group, score, char_len, byte_start, byte_end, original_text, action, created_at)
         VALUES ('s1', 'missing', 'secret', 0.9, 1, 0, 1, 'x', 'redact', 0)",
    )
    .execute(store.pool())
    .await;

    assert!(error.is_err(), "外键约束必须生效");
}

#[rstest]
#[tokio::test]
async fn the_admin_table_admits_only_one_row() {
    let database = TempDatabase::new();
    let store = database.open().await;
    let mut transaction = store.pool().begin().await.expect("可开启事务");

    for id in [1, 2] {
        let result = sqlx::query(
            "INSERT INTO admin (id, username, password_hash, created_at, updated_at)
             VALUES (?1, 'admin', 'hash', 0, 0)",
        )
        .bind(id)
        .execute(&mut *transaction)
        .await;
        if id == 2 {
            assert!(result.is_err(), "单行约束必须拒绝第二个管理员");
        } else {
            result.expect("第一个管理员可写入");
        }
    }
}

#[rstest]
#[tokio::test]
async fn default_rules_are_seeded_once_and_deleted_rules_stay_deleted() {
    let database = TempDatabase::new();
    let store = database.open().await;
    let builtin: Vec<Rule> = RuleSet::builtin().rules().to_vec();

    let first = store
        .rules()
        .seed_builtin(&builtin)
        .await
        .expect("首次补齐成功");
    assert!(first <= builtin.len(), "不得重复插入：{first}");
    for rule in &builtin {
        assert_eq!(
            store.rules().find(&rule.id).await.expect("可查询"),
            Some(rule.clone()),
            "补齐之后每条预置规则都必须在库里"
        );
    }

    // 管理员删除其中一条，再次启动时不得被重新生成。
    let deleted = &builtin[0].id;
    store.rules().delete(deleted).await.expect("可删除");

    let second = store
        .rules()
        .seed_builtin(&builtin)
        .await
        .expect("再次补齐成功");
    assert_eq!(second, 0, "已存在的规则不应重复插入");

    assert_eq!(
        store.rules().find(deleted).await.expect("可查询"),
        None,
        "管理员删除的默认规则不得在重启路径中复活"
    );
}

#[rstest]
#[tokio::test]
async fn console_rules_round_trip_through_storage() {
    let database = TempDatabase::new();
    let store = database.open().await;

    let rule = Rule {
        id: RuleId::new("console.block-corp").expect("非空"),
        name: "拦截企业邮箱".to_owned(),
        priority: priority::RELEASE_KEYWORD + 1,
        condition: RuleExpression::entity(EntityGroup::PrivateEmail),
        action: Action::Redact,
        enabled: true,
        source: RuleSource::Console,
        category: None,
    };

    store.rules().create(&rule).await.expect("可创建");
    assert_eq!(
        store.rules().find(&rule.id).await.expect("可查询"),
        Some(rule.clone())
    );

    let mut updated = rule.clone();
    updated.name = "改名后".to_owned();
    updated.enabled = false;
    store.rules().update(&updated).await.expect("可更新");
    assert_eq!(
        store.rules().find(&rule.id).await.expect("可查询"),
        Some(updated)
    );

    store.rules().delete(&rule.id).await.expect("可删除");
    assert_eq!(store.rules().find(&rule.id).await.expect("可查询"), None);
}

#[rstest]
#[tokio::test]
async fn default_rules_can_be_rewritten_and_deleted() {
    let database = TempDatabase::new();
    let store = database.open().await;
    let builtin = RuleSet::builtin();
    store
        .rules()
        .seed_builtin(builtin.rules())
        .await
        .expect("补齐成功");

    let target = &builtin.rules()[0];

    let mut rewritten = target.clone();
    rewritten.priority = 1;
    store
        .rules()
        .update(&rewritten)
        .await
        .expect("默认规则可改写");
    let reloaded = store
        .rules()
        .find(&target.id)
        .await
        .expect("可查询")
        .expect("存在");
    assert_eq!(reloaded.priority, 1);

    store
        .rules()
        .delete(&target.id)
        .await
        .expect("默认规则可删除");
    assert_eq!(store.rules().find(&target.id).await.expect("可查询"), None);
}

#[rstest]
#[tokio::test]
async fn missing_rules_report_not_found_rather_than_a_row_count() {
    let database = TempDatabase::new();
    let store = database.open().await;
    let absent = RuleId::new("does-not-exist").expect("非空");

    assert!(matches!(
        store.rules().delete(&absent).await,
        Err(privacy_store::Error::NotFound(_))
    ));
    assert!(matches!(
        store.rules().set_enabled(&absent, false).await,
        Err(privacy_store::Error::NotFound(_))
    ));
}

#[rstest]
#[tokio::test]
async fn duplicate_console_rule_ids_are_rejected() {
    let database = TempDatabase::new();
    let store = database.open().await;
    let rule = Rule {
        id: RuleId::new("dup").expect("非空"),
        name: "第一条".to_owned(),
        priority: 1,
        condition: RuleExpression::all([]),
        action: Action::Release,
        enabled: true,
        source: RuleSource::Console,
        category: None,
    };

    store.rules().create(&rule).await.expect("首次创建成功");
    assert!(matches!(
        store.rules().create(&rule).await,
        Err(privacy_store::Error::RuleIdTaken(_))
    ));
}

#[rstest]
#[tokio::test]
async fn the_loaded_rule_set_merges_every_source_into_one_evaluation_order() {
    let database = TempDatabase::new();
    let store = database.open().await;
    store
        .rules()
        .seed_builtin(RuleSet::builtin().rules())
        .await
        .expect("补齐成功");

    // 管理员登记与内置规则进入同一个集合，且沿优先级排列。
    let released = Rule::operator_release(
        RuleId::new("approval.1").expect("非空"),
        "TimmyOVO",
        EntityGroup::PrivatePerson,
    );
    let denied = Rule::operator_redact(
        RuleId::new("approval.2").expect("非空"),
        "秘密项目名",
        EntityGroup::Secret,
    );
    store.rules().create(&released).await.expect("可创建");
    store.rules().create(&denied).await.expect("可创建");

    let set = store.rules().load_set().await.expect("可装配");
    let order: Vec<i32> = set.rules().iter().map(|rule| rule.priority).collect();
    assert!(
        order.windows(2).all(|pair| pair[0] >= pair[1]),
        "规则集合必须按优先级降序：{order:?}"
    );

    let sources: Vec<RuleSource> = set.rules().iter().map(|rule| rule.source).collect();
    assert!(sources.contains(&RuleSource::Builtin));
    assert!(sources.contains(&RuleSource::Operator));
}

const PROTECTION_PROFILE_MIGRATION: &str =
    include_str!("../migrations/0005_builtin_protection_profile.sql");

const EMAIL_MIGRATION: &str =
    include_str!("../migrations/0006_builtin_email_has_no_length_gate.sql");

const DETERMINISTIC_CATEGORIES_MIGRATION: &str =
    include_str!("../migrations/0007_deterministic_categories.sql");

const CATEGORY_ONLY_RULES_MIGRATION: &str =
    include_str!("../migrations/0008_authoritative_rules_ask_only_for_the_category.sql");

/// 把预置规则升级到当前档位所要执行的迁移，按先后顺序排列。
const CURRENT_DEFAULTS_MIGRATIONS: [&str; 3] = [
    EMAIL_MIGRATION,
    DETERMINISTIC_CATEGORIES_MIGRATION,
    CATEGORY_ONLY_RULES_MIGRATION,
];

/// 迁移前的旧版预置档位：每个类别一条拦截门槛、一条短匹配放行。
///
/// 结构与数值逐字冻结自迁移前的 builtin.rs，是 0005 迁移必须能识别的历史输入。它和当前的
/// builtin.rs 是两个不同时期的事实：这里回答「当时库里是什么样」，那边回答「现在该是什么样」。
const LEGACY_MIN_CONFIDENCE: f64 = 0.8;

/// 当时存在的八个类别与它们各自的字符门槛。0005 迁移只认识这些类别；后来才有的类别
/// 在这份历史输入里不存在，因此这里是一张冻结的表，不是对当前类别集合的遍历。
const LEGACY_GROUPS: [(EntityGroup, usize); 8] = [
    (EntityGroup::AccountNumber, 4),
    (EntityGroup::PrivateAddress, 5),
    (EntityGroup::PrivateDate, 4),
    (EntityGroup::PrivateEmail, 5),
    (EntityGroup::PrivatePerson, 2),
    (EntityGroup::PrivatePhone, 7),
    (EntityGroup::PrivateUrl, 5),
    (EntityGroup::Secret, 8),
];

fn legacy_rule(group: EntityGroup, action: Action, threshold: usize) -> Rule {
    let (id, rank, condition) = match action {
        Action::Redact => (
            format!("builtin.redact.{group}"),
            priority::DEFAULT_REDACT,
            RuleExpression::all([
                RuleExpression::entity(group),
                RuleExpression::confidence(ComparisonOperator::Greater, LEGACY_MIN_CONFIDENCE),
                RuleExpression::character_length(ComparisonOperator::Greater, threshold),
            ]),
        ),
        Action::Release => (
            format!("builtin.release.short.{group}"),
            priority::RELEASE_NOISE,
            RuleExpression::all([
                RuleExpression::entity(group),
                RuleExpression::character_length(ComparisonOperator::Less, threshold),
            ]),
        ),
    };
    let id = RuleId::new(id).expect("内置标识非空");
    Rule {
        name: id.to_string(),
        id,
        priority: rank,
        condition,
        action,
        enabled: true,
        source: RuleSource::Builtin,
        category: None,
    }
}

fn legacy_default_rules() -> Vec<Rule> {
    LEGACY_GROUPS
        .into_iter()
        .flat_map(|(group, minimum)| {
            [
                legacy_rule(group, Action::Redact, minimum - 1),
                legacy_rule(group, Action::Release, minimum),
            ]
        })
        .collect()
}

fn entity(group: EntityGroup, score: f64, word: &str) -> Entity {
    Entity {
        entity_group: group,
        score,
        start: 0,
        end: word.len(),
        word: word.to_owned(),
    }
}

/// 0005 迁移把已存在的库升级到新的预置保护档位。
///
/// 迁移本身是一段数据变换，验收点只能在「旧库 → 执行迁移 → 判定结果」这条链路上：造一个装
/// 着旧版预置的库，原样执行迁移文件，再用规则集合的公开判定观察结果。管理员改过的规则用
/// 「只改名」和「只改条件」两条分别钉住，确认升级不会把他们的配置当预置处理。
#[rstest]
#[tokio::test]
async fn protection_profile_migration_upgrades_legacy_defaults_in_place() {
    let database = TempDatabase::new();
    let store = database.open().await;

    let mut renamed = legacy_rule(EntityGroup::Secret, Action::Release, 8);
    renamed.name = "我的密钥放行".to_owned();
    let mut reconditioned = legacy_rule(EntityGroup::PrivateUrl, Action::Release, 5);
    reconditioned.condition =
        RuleExpression::all([RuleExpression::entity(EntityGroup::PrivateUrl)]);

    for rule in legacy_default_rules() {
        if rule.id == renamed.id || rule.id == reconditioned.id {
            continue;
        }
        store.rules().create(&rule).await.expect("可写入旧版预置");
    }
    store
        .rules()
        .create(&renamed)
        .await
        .expect("可写入改名规则");
    store
        .rules()
        .create(&reconditioned)
        .await
        .expect("可写入改条件规则");

    // 迁移前：两个字的姓名由「大于 1」的预置规则拦截。
    let short_name = entity(EntityGroup::PrivatePerson, 0.9, "张三");
    let before = store.rules().load_set().await.expect("可装配");
    assert_eq!(before.evaluate(&short_name).action, Action::Redact);

    sqlx::raw_sql(PROTECTION_PROFILE_MIGRATION)
        .execute(store.pool())
        .await
        .expect("迁移必须可执行");

    let after = store.rules().load_set().await.expect("可装配");

    // 姓名门槛抬到 4 个字符：短的不再被认领，够长的仍然拦截。
    assert_eq!(after.evaluate(&short_name).rule, None);
    assert_eq!(
        after
            .evaluate(&entity(EntityGroup::PrivatePerson, 0.9, "张三四五六"))
            .action,
        Action::Redact
    );
    // 日期不再被任何预置规则认领。
    assert_eq!(
        after
            .evaluate(&entity(EntityGroup::PrivateDate, 0.99, "2026-09-11"))
            .rule,
        None
    );

    // 除管理员改过的两条之外，短匹配放行规则必须消失。
    for (group, _) in LEGACY_GROUPS {
        let id = RuleId::new(format!("builtin.release.short.{group}")).expect("非空");
        if id == renamed.id || id == reconditioned.id {
            continue;
        }
        assert_eq!(
            store.rules().find(&id).await.expect("可查询"),
            None,
            "{id} 应被迁移删除"
        );
    }
    let date = RuleId::new("builtin.redact.private_date").expect("非空");
    assert_eq!(store.rules().find(&date).await.expect("可查询"), None);

    // 没被改版碰到的拦截规则一条都不能少，也不能变：预置只安装一次，删掉就再也回不来。
    for (group, minimum) in LEGACY_GROUPS {
        if matches!(group, EntityGroup::PrivatePerson | EntityGroup::PrivateDate) {
            continue;
        }
        let untouched = legacy_rule(group, Action::Redact, minimum - 1);
        assert_eq!(
            store.rules().find(&untouched.id).await.expect("可查询"),
            Some(untouched)
        );
    }

    assert_eq!(
        store.rules().find(&renamed.id).await.expect("可查询"),
        Some(renamed)
    );
    assert_eq!(
        store.rules().find(&reconditioned.id).await.expect("可查询"),
        Some(reconditioned)
    );

    // 同一份数据再跑一遍不改变任何东西。
    let rules = store.rules().list().await.expect("可列举");
    sqlx::raw_sql(PROTECTION_PROFILE_MIGRATION)
        .execute(store.pool())
        .await
        .expect("重复执行必须安全");
    assert_eq!(store.rules().list().await.expect("可列举"), rules);
}

/// 迁移之前的预置档位：邮箱与手机号还带着长度门槛，后来才有的类别尚不存在。
///
/// 逐字冻结自当时的 builtin.rs：这是迁移必须能识别的历史输入。形状改由确定性过滤器产出之后，
/// 这些长度门槛只是把识别阶段的结论重算一遍。
fn legacy_email_rule() -> Rule {
    let id = RuleId::new("builtin.redact.private_email").expect("内置标识非空");
    Rule {
        name: id.to_string(),
        id,
        priority: priority::DEFAULT_REDACT,
        condition: RuleExpression::all([
            RuleExpression::entity(EntityGroup::PrivateEmail),
            RuleExpression::confidence(ComparisonOperator::Greater, LEGACY_MIN_CONFIDENCE),
            RuleExpression::character_length(ComparisonOperator::Greater, 4),
        ]),
        action: Action::Redact,
        enabled: true,
        source: RuleSource::Builtin,
        category: None,
    }
}

/// 迁移之前存在的类别：身份证、银行卡、统一社会信用代码、MAC、IP 当时都还不存在。
const LEGACY_CATEGORIES: [EntityGroup; 8] = [
    EntityGroup::AccountNumber,
    EntityGroup::PrivateAddress,
    EntityGroup::PrivateDate,
    EntityGroup::PrivateEmail,
    EntityGroup::PrivatePerson,
    EntityGroup::PrivatePhone,
    EntityGroup::PrivateUrl,
    EntityGroup::Secret,
];

/// 装着迁移之前那份预置档位的库。
fn legacy_store_rules() -> Vec<Rule> {
    let reverted = legacy_email_rule();
    RuleSet::builtin()
        .rules()
        .iter()
        .filter(|rule| {
            LEGACY_CATEGORIES
                .into_iter()
                .any(|group| rule.id.as_str() == format!("builtin.redact.{group}"))
        })
        .map(|rule| {
            if rule.id == reverted.id {
                reverted.clone()
            } else {
                rule.clone()
            }
        })
        .collect()
}

/// 把库恢复成迁移之前的样子。`open` 已经跑过最新迁移，这里先把它们留下的痕迹清掉。
async fn install_legacy_defaults(store: &privacy_store::Store, rules: Vec<Rule>) {
    for existing in store.rules().list().await.expect("可列举") {
        store.rules().delete(&existing.id).await.expect("可删除");
    }
    for rule in rules {
        store.rules().create(&rule).await.expect("可写入旧版预置");
    }
}

/// 判定结果的身份：谁认领的、怎么处置。规则的内部结构不属于验收对象。
fn verdict(
    rules: &RuleSet,
    group: EntityGroup,
    score: f64,
    word: &str,
) -> (Option<String>, Action) {
    let decision = rules.evaluate(&entity(group, score, word));
    (decision.rule.map(|id| id.to_string()), decision.action)
}

/// 确定性类别的迁移把已存在的库升级到「形状由过滤器保证、规则只认类别」的档位。
///
/// 验收点是「旧库 + 迁移」与「今天全新安装」判定完全一致：这既钉住了迁移本身，也钉住了迁移
/// 文件里冻结的条件 JSON 与 builtin.rs 今天生成的那一份是同一个事实——两者一旦分家，条件就
/// 不再逐字相等，UPDATE 不命中、INSERT 也不会补上，判定会立刻出现差异。
#[rstest]
#[tokio::test]
async fn deterministic_migrations_converge_on_todays_defaults() {
    let legacy_database = TempDatabase::new();
    let legacy = legacy_database.open().await;
    install_legacy_defaults(&legacy, legacy_store_rules()).await;

    for migration in CURRENT_DEFAULTS_MIGRATIONS {
        sqlx::raw_sql(migration)
            .execute(legacy.pool())
            .await
            .expect("迁移必须可执行");
    }

    let fresh_database = TempDatabase::new();
    let fresh = fresh_database.open().await;
    fresh
        .rules()
        .seed_builtin(RuleSet::builtin().rules())
        .await
        .expect("全新安装成功");

    let migrated = legacy.rules().load_set().await.expect("可装配");
    let installed = fresh.rules().load_set().await.expect("可装配");
    for group in EntityGroup::all() {
        for score in [0.5, LEGACY_MIN_CONFIDENCE, 0.99] {
            for word in [
                "a@b",
                "a@b.co",
                "alice@example.com",
                "13812345678",
                "1234567",
                "Alice",
            ] {
                assert_eq!(
                    verdict(&migrated, group, score, word),
                    verdict(&installed, group, score, word),
                    "{group} / {score} / {word} 的判定必须与全新安装一致"
                );
            }
        }
    }

    // 同一份数据再跑一遍不改变任何东西。
    let rules = legacy.rules().list().await.expect("可列举");
    for migration in CURRENT_DEFAULTS_MIGRATIONS {
        sqlx::raw_sql(migration)
            .execute(legacy.pool())
            .await
            .expect("重复执行必须安全");
    }
    assert_eq!(legacy.rules().list().await.expect("可列举"), rules);
}

/// 管理员动过的预置规则属于他们的配置：改名或改条件之后，迁移既不覆盖也不删除。
#[rstest]
#[case(EditedDefault::Renamed)]
#[case(EditedDefault::Reconditioned)]
#[tokio::test]
async fn migrations_leave_edited_defaults_alone(#[case] edit: EditedDefault) {
    let database = TempDatabase::new();
    let store = database.open().await;
    let email = edit.apply(legacy_email_rule());
    let mut rules = legacy_store_rules();
    for rule in &mut rules {
        if rule.id == email.id {
            *rule = email.clone();
        }
    }
    install_legacy_defaults(&store, rules).await;

    for migration in CURRENT_DEFAULTS_MIGRATIONS {
        sqlx::raw_sql(migration)
            .execute(store.pool())
            .await
            .expect("迁移必须可执行");
    }

    assert_eq!(
        store.rules().find(&email.id).await.expect("可查询"),
        Some(email),
        "管理员改过的规则不得被迁移碰"
    );
}

/// 管理员可能改动的那一维。两种改法都让这一行不再是「逐字等于预置」。
#[derive(Debug, Clone, Copy)]
enum EditedDefault {
    Renamed,
    Reconditioned,
}

impl EditedDefault {
    fn apply(self, mut rule: Rule) -> Rule {
        match self {
            Self::Renamed => rule.name = "我的拦截规则".to_owned(),
            Self::Reconditioned => {
                rule.condition = RuleExpression::all([
                    RuleExpression::entity(rule_group(&rule)),
                    RuleExpression::character_length(ComparisonOperator::Greater, 9),
                ])
            }
        }
        rule
    }
}

/// 规则标识里唯一编码的类别；测试自己构造的规则标识一律是 `builtin.redact.<group>`。
fn rule_group(rule: &Rule) -> EntityGroup {
    EntityGroup::all()
        .find(|group| rule.id.as_str() == format!("builtin.redact.{group}"))
        .expect("预置标识里带着类别")
}
