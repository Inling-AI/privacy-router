use crate::{Error, Result, Store};
use argon2::Argon2;
use argon2::password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash};
use sqlx::Row;

/// 会话令牌的明文只在创建时返回一次；库内只保存摘要。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub token: String,
    pub expires_at: i64,
}

/// 管理员凭据。构造即校验，因此「账号与口令都不能为空」只在这一处表达：
/// 创建与轮换都只接受本类型，任何入口都不必（也不可能）自行重复检查。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credentials {
    username: String,
    password: String,
}

impl Credentials {
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Result<Self> {
        let username = username.into();
        if username.trim().is_empty() {
            return Err(Error::InvalidCredentials(
                "the administrator name must not be empty".into(),
            ));
        }
        let password = password.into();
        if password.is_empty() {
            return Err(Error::InvalidCredentials(
                "the administrator password must not be empty".into(),
            ));
        }
        Ok(Self {
            username: username.trim().to_owned(),
            password,
        })
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn password(&self) -> &str {
        &self.password
    }
}

/// 管理员账号（单行）与会话。
pub struct AdminRepository<'a> {
    store: &'a Store,
}

impl<'a> AdminRepository<'a> {
    pub(crate) fn new(store: &'a Store) -> Self {
        Self { store }
    }

    pub async fn is_configured(&self) -> Result<bool> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM admin")
            .fetch_one(self.store.pool())
            .await?;
        Ok(count > 0)
    }

    /// 创建唯一的管理员账号。账号已存在时返回错误，不会静默覆盖。
    ///
    /// 唯一性由数据库约束裁决，而不是「先查再写」：并发调用时只有一个会插入成功，
    /// 落败者拿到的是明确的 [`Error::AdminAlreadyExists`]，而不是主键冲突。
    pub async fn create(&self, credentials: &Credentials) -> Result<()> {
        let now = self.store.now_ms();
        let result = sqlx::query(
            "INSERT INTO admin (id, username, password_hash, created_at, updated_at)
             VALUES (1, ?1, ?2, ?3, ?3)
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(credentials.username())
        .bind(hash_password(credentials.password())?)
        .bind(now)
        .execute(self.store.pool())
        .await?;
        if result.rows_affected() == 0 {
            return Err(Error::AdminAlreadyExists);
        }
        Ok(())
    }

    /// 重置既有账号的凭据；账号不存在时返回错误。
    pub async fn replace_credentials(&self, credentials: &Credentials) -> Result<()> {
        let result = sqlx::query(
            "UPDATE admin SET username = ?1, password_hash = ?2, updated_at = ?3 WHERE id = 1",
        )
        .bind(credentials.username())
        .bind(hash_password(credentials.password())?)
        .bind(self.store.now_ms())
        .execute(self.store.pool())
        .await?;

        if result.rows_affected() == 0 {
            return Err(Error::AdminMissing);
        }
        // 凭据轮换必须让既有会话立即失效。
        sqlx::query("DELETE FROM sessions")
            .execute(self.store.pool())
            .await?;
        Ok(())
    }

    /// 校验账号密码。用户名或密码任一不符都返回 `false`。
    pub async fn authenticate(&self, username: &str, password: &str) -> Result<bool> {
        let row = sqlx::query("SELECT username, password_hash FROM admin WHERE id = 1")
            .fetch_optional(self.store.pool())
            .await?
            .ok_or(Error::AdminMissing)?;

        let stored: String = row.try_get("username")?;
        let hash: String = row.try_get("password_hash")?;
        if stored != username {
            return Ok(false);
        }
        Ok(verify_password(password, &hash))
    }

    /// 签发会话。`ttl_ms` 由调用方给出，便于测试注入确定性的过期时间。
    pub async fn create_session(&self, ttl_ms: i64) -> Result<Session> {
        // 128 位 CSPRNG 随机数；明文只在此返回一次，库内只留摘要。
        let token = uuid::Uuid::new_v4().simple().to_string();
        let now = self.store.now_ms();
        let expires_at = now + ttl_ms;

        sqlx::query(
            "INSERT INTO sessions (token_hash, admin_id, created_at, expires_at) VALUES (?1, 1, ?2, ?3)",
        )
        .bind(digest(&token))
        .bind(now)
        .bind(expires_at)
        .execute(self.store.pool())
        .await?;

        Ok(Session { token, expires_at })
    }

    /// 会话是否有效。过期即视为无效，并顺手清理该行。
    pub async fn validate_session(&self, token: &str) -> Result<bool> {
        let now = self.store.now_ms();
        let expires_at: Option<i64> =
            sqlx::query_scalar("SELECT expires_at FROM sessions WHERE token_hash = ?1")
                .bind(digest(token))
                .fetch_optional(self.store.pool())
                .await?;

        match expires_at {
            None => Ok(false),
            Some(expires_at) if expires_at <= now => {
                self.revoke_session(token).await?;
                Ok(false)
            }
            Some(_) => Ok(true),
        }
    }

    pub async fn revoke_session(&self, token: &str) -> Result<()> {
        sqlx::query("DELETE FROM sessions WHERE token_hash = ?1")
            .bind(digest(token))
            .execute(self.store.pool())
            .await?;
        Ok(())
    }

    pub async fn purge_expired_sessions(&self) -> Result<u64> {
        let result = sqlx::query("DELETE FROM sessions WHERE expires_at <= ?1")
            .bind(self.store.now_ms())
            .execute(self.store.pool())
            .await?;
        Ok(result.rows_affected())
    }
}

fn hash_password(password: &str) -> Result<String> {
    // Argon2 自行生成随机盐并写入 PHC 字符串，调用方不需要单独管理盐值。
    Argon2::default()
        .hash_password(password.as_bytes())
        .map(|hash| hash.to_string())
        .map_err(|error| Error::Credential(error.to_string()))
}

fn verify_password(password: &str, encoded: &str) -> bool {
    PasswordHash::new(encoded)
        .map(|parsed| {
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok()
        })
        .unwrap_or(false)
}

/// 会话令牌的存储形态：只存摘要，库被读取也无法直接冒充会话。
fn digest(token: &str) -> Vec<u8> {
    blake3::hash(token.as_bytes()).as_bytes().to_vec()
}
