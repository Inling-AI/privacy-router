mod common;

use common::{TempDatabase, TestClock};
use privacy_store::Credentials;
use rstest::rstest;

/// 测试关心的是仓储行为，不是凭据校验，因此在这里一次性构造。
fn credentials(username: &str, password: &str) -> Credentials {
    Credentials::new(username, password).expect("测试凭据非空")
}

#[rstest]
#[tokio::test]
async fn only_one_admin_account_can_be_created() {
    let database = TempDatabase::new();
    let store = database.open().await;

    assert!(!store.admin().is_configured().await.expect("可查询"));
    store
        .admin()
        .create(&credentials("admin", "correct horse battery staple"))
        .await
        .expect("可创建");
    assert!(store.admin().is_configured().await.expect("可查询"));

    assert!(matches!(
        store
            .admin()
            .create(&credentials("other", "another password"))
            .await,
        Err(privacy_store::Error::AdminAlreadyExists)
    ));
}

/// 唯一性由数据库约束裁决：并发创建时只有一个成功，其余得到明确的「已存在」。
#[rstest]
#[tokio::test]
async fn concurrent_creation_yields_exactly_one_admin() {
    let database = TempDatabase::new();
    let store = database.open().await;
    let account = credentials("admin", "correct horse battery staple");
    let admin = store.admin();

    let (first, second) = tokio::join!(admin.create(&account), admin.create(&account));

    let succeeded = [&first, &second]
        .iter()
        .filter(|result| result.is_ok())
        .count();
    assert_eq!(succeeded, 1, "只能有一个创建请求成功");
    let refused = [first, second]
        .into_iter()
        .find(Result::is_err)
        .expect("落败的请求必须报错");
    assert!(matches!(
        refused,
        Err(privacy_store::Error::AdminAlreadyExists)
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM admin")
            .fetch_one(store.pool())
            .await
            .expect("可查询"),
        1
    );
}

/// 空账号与空口令在构造凭据时就已被拒绝，任何入口都拿不到这样的凭据。
#[rstest]
#[case("  ", "correct horse battery staple")]
#[case("admin", "")]
fn blank_credentials_are_rejected(#[case] username: &str, #[case] password: &str) {
    assert!(matches!(
        Credentials::new(username, password),
        Err(privacy_store::Error::InvalidCredentials(_))
    ));
}

#[rstest]
#[tokio::test]
async fn authentication_requires_both_username_and_password() {
    let database = TempDatabase::new();
    let store = database.open().await;
    store
        .admin()
        .create(&credentials("admin", "s3cret-password"))
        .await
        .expect("可创建");

    assert!(
        store
            .admin()
            .authenticate("admin", "s3cret-password")
            .await
            .expect("可校验")
    );
    assert!(
        !store
            .admin()
            .authenticate("admin", "wrong-password")
            .await
            .expect("可校验")
    );
    assert!(
        !store
            .admin()
            .authenticate("someone-else", "s3cret-password")
            .await
            .expect("可校验")
    );
}

#[rstest]
#[tokio::test]
async fn passwords_are_never_stored_in_the_clear() {
    let database = TempDatabase::new();
    let store = database.open().await;
    let password = "unique-plaintext-marker";
    store
        .admin()
        .create(&credentials("admin", password))
        .await
        .expect("可创建");

    let stored: String = sqlx::query_scalar("SELECT password_hash FROM admin WHERE id = 1")
        .fetch_one(store.pool())
        .await
        .expect("可读取");

    assert!(!stored.contains(password), "库内不得出现明文密码");
    assert!(
        stored.starts_with("$argon2"),
        "必须是 PHC 格式的 Argon2 哈希：{stored}"
    );
}

#[rstest]
#[tokio::test]
async fn sessions_are_stored_as_digests_and_validated_by_time() {
    let database = TempDatabase::new();
    let clock = TestClock::new(1_700_000_000_000);
    let store = database.open_with(clock.clone()).await;
    store
        .admin()
        .create(&credentials("admin", "pw"))
        .await
        .expect("可创建");

    let session = store.admin().create_session(1_000).await.expect("可签发");
    assert!(
        store
            .admin()
            .validate_session(&session.token)
            .await
            .expect("可校验")
    );

    // 明文令牌不得出现在库里。
    let stored: Vec<u8> = sqlx::query_scalar("SELECT token_hash FROM sessions LIMIT 1")
        .fetch_one(store.pool())
        .await
        .expect("可读取");
    assert_ne!(stored, session.token.as_bytes());
    assert_eq!(stored.len(), 32, "存的是摘要而不是令牌本身");

    // 过期后立即失效。
    clock.advance(1_001);
    assert!(
        !store
            .admin()
            .validate_session(&session.token)
            .await
            .expect("可校验")
    );
}

#[rstest]
#[tokio::test]
async fn expired_sessions_are_purged() {
    let database = TempDatabase::new();
    let clock = TestClock::new(1_700_000_000_000);
    let store = database.open_with(clock.clone()).await;
    store
        .admin()
        .create(&credentials("admin", "pw"))
        .await
        .expect("可创建");
    store.admin().create_session(500).await.expect("可签发");

    assert_eq!(
        store
            .admin()
            .purge_expired_sessions()
            .await
            .expect("可清理"),
        0
    );
    clock.advance(501);
    assert_eq!(
        store
            .admin()
            .purge_expired_sessions()
            .await
            .expect("可清理"),
        1
    );
}

#[rstest]
#[tokio::test]
async fn revoking_a_session_invalidates_it_immediately() {
    let database = TempDatabase::new();
    let store = database.open().await;
    store
        .admin()
        .create(&credentials("admin", "pw"))
        .await
        .expect("可创建");

    let session = store.admin().create_session(60_000).await.expect("可签发");
    store
        .admin()
        .revoke_session(&session.token)
        .await
        .expect("可注销");
    assert!(
        !store
            .admin()
            .validate_session(&session.token)
            .await
            .expect("可校验")
    );
}

#[rstest]
#[tokio::test]
async fn rotating_credentials_drops_every_existing_session() {
    let database = TempDatabase::new();
    let store = database.open().await;
    store
        .admin()
        .create(&credentials("admin", "old-password"))
        .await
        .expect("可创建");
    let session = store.admin().create_session(60_000).await.expect("可签发");

    store
        .admin()
        .replace_credentials(&credentials("admin", "new-password"))
        .await
        .expect("可轮换");

    assert!(
        !store
            .admin()
            .validate_session(&session.token)
            .await
            .expect("可校验"),
        "凭据轮换后旧会话必须失效"
    );
    assert!(
        store
            .admin()
            .authenticate("admin", "new-password")
            .await
            .expect("可校验")
    );
    assert!(
        !store
            .admin()
            .authenticate("admin", "old-password")
            .await
            .expect("可校验")
    );
}

#[rstest]
#[tokio::test]
async fn operating_without_an_admin_reports_a_specific_error() {
    let database = TempDatabase::new();
    let store = database.open().await;

    assert!(matches!(
        store.admin().authenticate("admin", "pw").await,
        Err(privacy_store::Error::AdminMissing)
    ));
    assert!(matches!(
        store
            .admin()
            .replace_credentials(&credentials("admin", "pw"))
            .await,
        Err(privacy_store::Error::AdminMissing)
    ));
}
