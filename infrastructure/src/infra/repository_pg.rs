//! sqlx implementation of the [`AuthStore`] port (PostgreSQL).
//!
//! Tables come from `migrations/0001_platform_users.sql` (`auth_users`,
//! `auth_sessions`, `auth_tokens`). Soft-deleted users are invisible to
//! every lookup. Raw tokens/cookies never reach this module — callers pass
//! SHA-256 hashes.

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool};
use ulid::Ulid;

use crate::domain::auth::entities::{
    AuthToken, AuthTokenId, Session, SessionId, TokenPurpose, User, UserId,
};
use crate::ports::auth_store::{
    AuthStore, BoxFut, OnboardingInput, StoreError,
};
use crate::domain::auth::value_objects::Email;

/// Seeded system `owner` role (migrations/0002_platform_orgs.sql).
const OWNER_ROLE_ID: &str = "01KZ8KAFYKD2RFQB4481AR5EV3";

/// PostgreSQL-backed auth store.
pub struct PgAuthStore {
    pool: PgPool,
}

impl PgAuthStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: String,
    name: String,
    email: String,
    password_hash: String,
    avatar_url: Option<String>,
    oauth_provider: Option<String>,
    oauth_subject: Option<String>,
    email_verified_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    last_login_at: Option<DateTime<Utc>>,
}

impl TryFrom<UserRow> for User {
    type Error = StoreError;

    fn try_from(row: UserRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: parse_id(&row.id)?,
            name: row.name,
            email: Email::parse(&row.email).map_err(|e| StoreError::Storage(e.to_string()))?,
            password_hash: row.password_hash,
            avatar_url: row.avatar_url,
            oauth_provider: row.oauth_provider,
            oauth_subject: row.oauth_subject,
            email_verified_at: row.email_verified_at,
            created_at: row.created_at,
            last_login_at: row.last_login_at,
        })
    }
}

fn parse_id<T>(raw: &str) -> Result<T, StoreError>
where
    T: std::str::FromStr<Err = ulid::DecodeError>,
{
    raw.parse::<T>()
        .map_err(|e| StoreError::Storage(format!("invalid ULID '{raw}': {e}")))
}

/// Maps driver errors; unique violations become [`StoreError::Conflict`].
fn map_err(err: sqlx::Error) -> StoreError {
    if let sqlx::Error::Database(db) = &err
        && (db.constraint().is_some() || db.code().as_deref() == Some("23505"))
    {
        return StoreError::Conflict;
    }
    StoreError::Storage(err.to_string())
}

/// Lowercase ASCII slug from a display name (`"Acme Corp!"` → `acme-corp`).
fn slugify(name: &str) -> String {
    let mut slug = String::new();
    for ch in name.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
        } else if matches!(ch, ' ' | '-' | '_' | '.' | '/') && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let trimmed = slug.trim_matches('-').to_string();
    if trimmed.is_empty() { "org".to_string() } else { trimmed }
}

/// Appends `-2`, `-3`, ... until the candidate is free; falls back to a
/// ULID suffix for pathological collisions.
const MAX_SLUG_ATTEMPTS: u32 = 50;

async fn unique_org_slug(tx: &mut PgConnection, base: &str) -> Result<String, StoreError> {
    for n in 0..MAX_SLUG_ATTEMPTS {
        let candidate = if n == 0 { base.to_string() } else { format!("{base}-{n}") };
        let taken = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM platform_organizations WHERE slug = $1)",
        )
        .bind(&candidate)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_err)?;
        if !taken {
            return Ok(candidate);
        }
    }
    Ok(format!("{base}-{}", Ulid::new()))
}

/// Project slugs are unique per organization (`UNIQUE (org_id, slug)`).
async fn unique_project_slug(
    tx: &mut PgConnection,
    base: &str,
    org_id: &str,
) -> Result<String, StoreError> {
    for n in 0..MAX_SLUG_ATTEMPTS {
        let candidate = if n == 0 { base.to_string() } else { format!("{base}-{n}") };
        let taken = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM platform_projects WHERE org_id = $1 AND slug = $2)",
        )
        .bind(org_id)
        .bind(&candidate)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_err)?;
        if !taken {
            return Ok(candidate);
        }
    }
    Ok(format!("{base}-{}", Ulid::new()))
}

impl AuthStore for PgAuthStore {
    // -- users ------------------------------------------------------------

    fn create_user(&self, user: &User) -> BoxFut<'_, Result<(), StoreError>> {
        let pool = self.pool.clone();
        let user = user.clone();
        Box::pin(async move {
            sqlx::query(
                "INSERT INTO auth_users (id, name, email, password_hash, avatar_url,
                                        oauth_provider, oauth_subject)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(user.id.to_string())
            .bind(&user.name)
            .bind(user.email.as_str())
            .bind(&user.password_hash)
            .bind(&user.avatar_url)
            .bind(&user.oauth_provider)
            .bind(&user.oauth_subject)
            .execute(&pool)
            .await
            .map_err(map_err)?;
            Ok(())
        })
    }

    fn find_user_by_email(&self, email: &str) -> BoxFut<'_, Result<Option<User>, StoreError>> {
        let pool = self.pool.clone();
        let email = email.to_string();
        Box::pin(async move {
            let row = sqlx::query_as::<_, UserRow>(
                "SELECT id, name, email, password_hash, avatar_url,
                        email_verified_at, oauth_provider, oauth_subject,
                        created_at, last_login_at
                 FROM auth_users WHERE email = $1 AND deleted_at IS NULL",
            )
            .bind(email)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;
            row.map(User::try_from).transpose()
        })
    }

    fn find_user_by_id(&self, id: UserId) -> BoxFut<'_, Result<Option<User>, StoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let row = sqlx::query_as::<_, UserRow>(
                "SELECT id, name, email, password_hash, avatar_url,
                        email_verified_at, oauth_provider, oauth_subject,
                        created_at, last_login_at
                 FROM auth_users WHERE id = $1 AND deleted_at IS NULL",
            )
            .bind(id.to_string())
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;
            row.map(User::try_from).transpose()
        })
    }

    fn set_email_verified(
        &self,
        user_id: UserId,
        verified_at: DateTime<Utc>,
    ) -> BoxFut<'_, Result<(), StoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            sqlx::query(
                "UPDATE auth_users SET email_verified_at = $2, updated_at = now()
                 WHERE id = $1",
            )
            .bind(user_id.to_string())
            .bind(verified_at)
            .execute(&pool)
            .await
            .map_err(map_err)?;
            Ok(())
        })
    }

    fn update_password(
        &self,
        user_id: UserId,
        password_hash: String,
    ) -> BoxFut<'_, Result<(), StoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            sqlx::query(
                "UPDATE auth_users SET password_hash = $2, updated_at = now()
                 WHERE id = $1",
            )
            .bind(user_id.to_string())
            .bind(password_hash)
            .execute(&pool)
            .await
            .map_err(map_err)?;
            Ok(())
        })
    }

    fn touch_last_login(
        &self,
        user_id: UserId,
        at: DateTime<Utc>,
    ) -> BoxFut<'_, Result<(), StoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            sqlx::query("UPDATE auth_users SET last_login_at = $2 WHERE id = $1")
                .bind(user_id.to_string())
                .bind(at)
                .execute(&pool)
                .await
                .map_err(map_err)?;
            Ok(())
        })
    }

    // -- sessions ---------------------------------------------------------

    fn create_session(&self, session: &Session) -> BoxFut<'_, Result<(), StoreError>> {
        let pool = self.pool.clone();
        let session = session.clone();
        Box::pin(async move {
            sqlx::query(
                "INSERT INTO auth_sessions (id, user_id, token_hash, expires_at)
                 VALUES ($1, $2, $3, $4)",
            )
            .bind(session.id.to_string())
            .bind(session.user_id.to_string())
            .bind(&session.token_hash)
            .bind(session.expires_at)
            .execute(&pool)
            .await
            .map_err(map_err)?;
            Ok(())
        })
    }

    fn find_session_by_hash(
        &self,
        token_hash: &str,
    ) -> BoxFut<'_, Result<Option<Session>, StoreError>> {
        let pool = self.pool.clone();
        let token_hash = token_hash.to_string();
        Box::pin(async move {
            let row = sqlx::query_as::<_, SessionRow>(
                "SELECT id, user_id, token_hash, expires_at, revoked_at, created_at
                 FROM auth_sessions WHERE token_hash = $1",
            )
            .bind(token_hash)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;
            row.map(Session::try_from).transpose()
        })
    }

    fn revoke_session(&self, id: SessionId) -> BoxFut<'_, Result<(), StoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            sqlx::query(
                "UPDATE auth_sessions SET revoked_at = now()
                 WHERE id = $1 AND revoked_at IS NULL",
            )
            .bind(id.to_string())
            .execute(&pool)
            .await
            .map_err(map_err)?;
            Ok(())
        })
    }

    fn revoke_all_sessions_for_user(&self, user_id: UserId) -> BoxFut<'_, Result<(), StoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            sqlx::query(
                "UPDATE auth_sessions SET revoked_at = now()
                 WHERE user_id = $1 AND revoked_at IS NULL",
            )
            .bind(user_id.to_string())
            .execute(&pool)
            .await
            .map_err(map_err)?;
            Ok(())
        })
    }

    // -- one-time tokens ----------------------------------------------------

    fn create_token(&self, token: &AuthToken) -> BoxFut<'_, Result<(), StoreError>> {
        let pool = self.pool.clone();
        let token = token.clone();
        Box::pin(async move {
            sqlx::query(
                "INSERT INTO auth_tokens (id, user_id, purpose, token_hash, expires_at)
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(token.id.to_string())
            .bind(token.user_id.to_string())
            .bind(token.purpose.as_str())
            .bind(&token.token_hash)
            .bind(token.expires_at)
            .execute(&pool)
            .await
            .map_err(map_err)?;
            Ok(())
        })
    }

    fn find_token_by_hash(
        &self,
        token_hash: &str,
        purpose: TokenPurpose,
    ) -> BoxFut<'_, Result<Option<AuthToken>, StoreError>> {
        let pool = self.pool.clone();
        let token_hash = token_hash.to_string();
        Box::pin(async move {
            let row = sqlx::query_as::<_, TokenRow>(
                "SELECT id, user_id, purpose, token_hash, expires_at, consumed_at, created_at
                 FROM auth_tokens WHERE token_hash = $1 AND purpose = $2",
            )
            .bind(token_hash)
            .bind(purpose.as_str())
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;
            row.map(AuthToken::try_from).transpose()
        })
    }

    fn consume_token(&self, id: AuthTokenId) -> BoxFut<'_, Result<(), StoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            sqlx::query(
                "UPDATE auth_tokens SET consumed_at = now()
                 WHERE id = $1 AND consumed_at IS NULL",
            )
            .bind(id.to_string())
            .execute(&pool)
            .await
            .map_err(map_err)?;
            Ok(())
        })
    }

    fn consume_tokens_for_user(
        &self,
        user_id: UserId,
        purpose: TokenPurpose,
    ) -> BoxFut<'_, Result<(), StoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            sqlx::query(
                "UPDATE auth_tokens SET consumed_at = now()
                 WHERE user_id = $1 AND purpose = $2 AND consumed_at IS NULL",
            )
            .bind(user_id.to_string())
            .bind(purpose.as_str())
            .execute(&pool)
            .await
            .map_err(map_err)?;
            Ok(())
        })
    }

    // -- oauth ---------------------------------------------------------------

    fn find_user_by_oauth(
        &self,
        provider: &str,
        subject: &str,
    ) -> BoxFut<'_, Result<Option<User>, StoreError>> {
        let pool = self.pool.clone();
        let provider = provider.to_string();
        let subject = subject.to_string();
        Box::pin(async move {
            let row = sqlx::query_as::<_, UserRow>(
                "SELECT id, name, email, password_hash, avatar_url,
                        email_verified_at, oauth_provider, oauth_subject,
                        created_at, last_login_at
                 FROM auth_users
                 WHERE oauth_provider = $1 AND oauth_subject = $2 AND deleted_at IS NULL",
            )
            .bind(provider)
            .bind(subject)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;
            row.map(User::try_from).transpose()
        })
    }

    fn link_oauth_to_user(
        &self,
        user_id: UserId,
        provider: &str,
        subject: &str,
    ) -> BoxFut<'_, Result<(), StoreError>> {
        let pool = self.pool.clone();
        let provider = provider.to_string();
        let subject = subject.to_string();
        Box::pin(async move {
            sqlx::query(
                "UPDATE auth_users SET oauth_provider = $2, oauth_subject = $3, updated_at = now()
                 WHERE id = $1 AND deleted_at IS NULL",
            )
            .bind(user_id.to_string())
            .bind(provider)
            .bind(subject)
            .execute(&pool)
            .await
            .map_err(map_err)?;
            Ok(())
        })
    }

    // -- onboarding ---------------------------------------------------------

    fn has_org_and_project(&self, user_id: UserId) -> BoxFut<'_, Result<bool, StoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let has = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS (
                    SELECT 1 FROM platform_members pm
                    JOIN platform_organizations o ON o.id = pm.org_id
                    WHERE pm.user_id = $1
                      AND pm.deleted_at IS NULL AND o.deleted_at IS NULL
                ) AND EXISTS (
                    SELECT 1 FROM platform_projects p
                    JOIN platform_members pm ON pm.org_id = p.org_id
                    WHERE pm.user_id = $1
                      AND pm.deleted_at IS NULL AND p.deleted_at IS NULL
                )",
            )
            .bind(user_id.to_string())
            .fetch_one(&pool)
            .await
            .map_err(map_err)?;
            Ok(has)
        })
    }

    fn complete_onboarding(
        &self,
        user_id: UserId,
        input: OnboardingInput,
    ) -> BoxFut<'_, Result<(), StoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let mut tx = pool.begin().await.map_err(map_err)?;

            let org_id = Ulid::new().to_string();
            let org_base = slugify(&input.org_name);
            let org_slug = unique_org_slug(&mut tx, &org_base).await?;
            sqlx::query(
                "INSERT INTO platform_organizations
                    (id, name, slug, logo_url, region, timezone, created_by)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(&org_id)
            .bind(input.org_name.trim())
            .bind(&org_slug)
            .bind(&input.org_logo_url)
            .bind(&input.org_region)
            .bind(&input.org_timezone)
            .bind(user_id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;

            // owner membership via the seeded system role (migrations/0002)
            sqlx::query(
                "INSERT INTO platform_members (id, org_id, user_id, role_id, status, joined_at)
                 VALUES ($1, $2, $3, $4, 'active', now())",
            )
            .bind(Ulid::new().to_string())
            .bind(&org_id)
            .bind(user_id.to_string())
            .bind(OWNER_ROLE_ID)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;

            let project_id = Ulid::new().to_string();
            let project_base = slugify(&input.project_name);
            let project_slug = unique_project_slug(&mut tx, &project_base, &org_id).await?;
            sqlx::query(
                "INSERT INTO platform_projects (id, org_id, name, slug, description, created_by)
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(&project_id)
            .bind(&org_id)
            .bind(input.project_name.trim())
            .bind(&project_slug)
            .bind(&input.project_description)
            .bind(user_id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;

            sqlx::query(
                "INSERT INTO platform_environments (id, project_id, name, slug)
                 VALUES ($1, $2, $3, $3)",
            )
            .bind(Ulid::new().to_string())
            .bind(&project_id)
            .bind(input.environment)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;

            tx.commit().await.map_err(map_err)?;
            Ok(())
        })
    }
}

// -- row types ----------------------------------------------------------

#[derive(sqlx::FromRow)]
struct SessionRow {
    id: String,
    user_id: String,
    token_hash: String,
    expires_at: DateTime<Utc>,
    revoked_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl TryFrom<SessionRow> for Session {
    type Error = StoreError;

    fn try_from(row: SessionRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: parse_id(&row.id)?,
            user_id: parse_id(&row.user_id)?,
            token_hash: row.token_hash,
            expires_at: row.expires_at,
            revoked_at: row.revoked_at,
            created_at: row.created_at,
        })
    }
}

#[derive(sqlx::FromRow)]
struct TokenRow {
    id: String,
    user_id: String,
    purpose: String,
    token_hash: String,
    expires_at: DateTime<Utc>,
    consumed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl TryFrom<TokenRow> for AuthToken {
    type Error = StoreError;

    fn try_from(row: TokenRow) -> Result<Self, Self::Error> {
        let purpose = match row.purpose.as_str() {
            "email_verification" => TokenPurpose::EmailVerification,
            "password_reset" => TokenPurpose::PasswordReset,
            other => {
                return Err(StoreError::Storage(format!(
                    "unknown token purpose '{other}'"
                )));
            }
        };
        Ok(Self {
            id: parse_id(&row.id)?,
            user_id: parse_id(&row.user_id)?,
            purpose,
            token_hash: row.token_hash,
            expires_at: row.expires_at,
            consumed_at: row.consumed_at,
            created_at: row.created_at,
        })
    }
}

#[cfg(test)]
mod pg_tests {
    //! Integration tests against a live Postgres. Run after filling in
    //! docker-compose values:
    //! `NOTIFI_TEST_DATABASE_URL=postgres://user:pass@localhost:5432/notifi \
    //!  cargo test -p notifi-auth -- --ignored`
    use super::*;
    use chrono::Utc;
    use ulid::Ulid;

    fn test_pool() -> PgPool {
        let url = std::env::var("NOTIFI_TEST_DATABASE_URL")
            .expect("NOTIFI_TEST_DATABASE_URL must be set for ignored pg tests");
        PgPool::connect_lazy(&url).unwrap()
    }

    #[tokio::test]
    #[ignore = "requires NOTIFI_TEST_DATABASE_URL pointing at a live Postgres"]
    async fn user_roundtrip() {
        let store = PgAuthStore::new(test_pool());
        let email = format!("roundtrip-{}@example.com", Ulid::new());
        let user = User {
            id: UserId::new(),
            name: "Roundtrip".to_string(),
            email: Email::parse(&email).unwrap(),
            password_hash: "phc$placeholder".to_string(),
            avatar_url: None,
            email_verified_at: None,
            oauth_provider: None,
            oauth_subject: None,
            created_at: Utc::now(),
            last_login_at: None,
        };

        store.create_user(&user).await.unwrap();

        // duplicate insert conflicts
        assert!(matches!(
            store.create_user(&user).await,
            Err(StoreError::Conflict)
        ));

        // NOTE: lookups use the normalized (lowercased) address — exactly what
        // AuthService passes after Email::parse.
        let found = store
            .find_user_by_email(user.email.as_str())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.id, user.id);
        assert_eq!(found.email.as_str(), user.email.as_str());

        store.touch_last_login(user.id, Utc::now()).await.unwrap();
        store.set_email_verified(user.id, Utc::now()).await.unwrap();
        let verified = store.find_user_by_id(user.id).await.unwrap().unwrap();
        assert!(verified.email_verified());

        // cleanup so the suite stays rerunnable
        sqlx::query("DELETE FROM auth_users WHERE id = $1")
            .bind(user.id.to_string())
            .execute(&store.pool)
            .await
            .unwrap();
    }
}
