//! Account linking: match OAuth identities to users.
//!
//! Handles three flows:
//! 1. Returning user — existing provider+stable-subject match
//! 2. Explicit link — attach a provider identity to the authenticated user
//! 3. Brand-new identity — creates a user + OAuth account

use std::ops::DerefMut;

use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::db;

// ── Types ────────────────────────────────────────────────────────────────────

/// Supported OAuth providers.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // Apple variant reserved for future use
pub enum Provider {
    Google,
    Apple,
    GitHub,
}

impl Provider {
    /// Return the database string for this provider.
    pub fn as_str(&self) -> &'static str {
        match self {
            Provider::Google => "google",
            Provider::Apple => "apple",
            Provider::GitHub => "github",
        }
    }
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for Provider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "google" => Ok(Provider::Google),
            "github" => Ok(Provider::GitHub),
            "apple" => Ok(Provider::Apple),
            other => Err(format!("Unknown provider: {other}")),
        }
    }
}

/// OAuth user info received from an external provider.
pub struct OauthUserInfo {
    pub provider: Provider,
    pub provider_uid: String,
    pub email: Option<String>,
    pub email_verified: bool,
    pub display_name: String,
    pub avatar_url: Option<String>,
}

/// Result of linking an OAuth identity to a user.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields used by callers at runtime
pub struct LinkResult {
    pub user_id: Uuid,
    pub oauth_account_id: Uuid,
    pub is_new_user: bool,
}

/// Errors from account linking operations.
#[derive(Debug)]
pub enum LinkError {
    /// A database error occurred.
    Db(db::DbError),
    /// All attempts to generate a unique username failed.
    UsernameGenerationFailed,
    /// The provider did not return a usable stable subject identifier.
    InvalidProviderIdentity,
    /// A new provider identity presented an email belonging to another user.
    EmailAlreadyInUse,
    /// OAuth account is already linked to a different user.
    AccountAlreadyLinked(Provider),
}

impl std::fmt::Display for LinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LinkError::Db(e) => write!(f, "database error: {e}"),
            LinkError::UsernameGenerationFailed => {
                write!(f, "failed to generate a unique username after all attempts")
            }
            LinkError::InvalidProviderIdentity => {
                write!(f, "OAuth provider returned an invalid stable identity")
            }
            LinkError::EmailAlreadyInUse => {
                write!(f, "The verified email is already used by another account")
            }
            LinkError::AccountAlreadyLinked(provider) => {
                write!(
                    f,
                    "The {provider} account is already linked to another user"
                )
            }
        }
    }
}

impl std::error::Error for LinkError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LinkError::Db(e) => Some(e),
            LinkError::UsernameGenerationFailed => None,
            LinkError::InvalidProviderIdentity => None,
            LinkError::EmailAlreadyInUse => None,
            LinkError::AccountAlreadyLinked(_) => None,
        }
    }
}

impl From<db::DbError> for LinkError {
    fn from(e: db::DbError) -> Self {
        LinkError::Db(e)
    }
}

// ── Username generation ──────────────────────────────────────────────────────

/// Derive a username from a display name.
///
/// Steps: NFKD normalize → strip non-ASCII → lowercase → replace
/// non-[a-z0-9] with hyphens → collapse consecutive hyphens → truncate
/// to 24 chars → strip trailing hyphens.
fn generate_username_from_display_name(name: &str) -> String {
    use unicode_normalization::UnicodeNormalization;

    // NFKD normalize and strip non-ASCII, then lowercase
    let normalized: String = name
        .nfkd()
        .filter(|c| c.is_ascii())
        .collect::<String>()
        .to_lowercase();

    // Replace non-[a-z0-9] with hyphens, collapsing consecutive hyphens
    let mut result = String::with_capacity(normalized.len());
    let mut prev_hyphen = false;

    for c in normalized.chars() {
        if c.is_ascii_alphanumeric() {
            result.push(c);
            prev_hyphen = false;
        } else if !prev_hyphen {
            result.push('-');
            prev_hyphen = true;
        }
        // If prev_hyphen is true and c is non-alphanumeric, skip (collapse)
    }

    // Truncate to 24 chars
    result.truncate(24);

    // Strip trailing hyphens
    while result.ends_with('-') {
        result.pop();
    }

    result
}

/// Generate a unique username by appending a random 4-hex suffix.
///
/// Retries up to 5 times on collision. Falls back to `user-{8hex}` if all
/// attempts collide.
async fn generate_unique_username(
    tx: &mut Transaction<'_, Postgres>,
    display_name: &str,
) -> Result<String, LinkError> {
    let base = {
        let b = generate_username_from_display_name(display_name);
        if b.is_empty() {
            "user".to_string()
        } else {
            b
        }
    };

    for _ in 0..5 {
        let suffix = &Uuid::new_v4().to_string()[..4];
        let candidate = format!("{base}-{suffix}");

        if !db::username_exists(tx.deref_mut(), &candidate).await? {
            return Ok(candidate);
        }
    }

    // Fallback: user-{8hex} with uniqueness check
    for _ in 0..3 {
        let fallback = format!("user-{}", &Uuid::new_v4().to_string()[..8]);
        if !db::username_exists(tx.deref_mut(), &fallback).await? {
            return Ok(fallback);
        }
    }
    Err(LinkError::UsernameGenerationFailed)
}

// ── Main linking function ────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IdentityResolution {
    Login,
    LinkTo(Uuid),
}

/// Resolve a provider identity for an unauthenticated login.
pub async fn login_or_create(
    pool: &PgPool,
    info: OauthUserInfo,
    refresh_token: Option<String>,
) -> Result<LinkResult, LinkError> {
    resolve_provider_identity(pool, info, IdentityResolution::Login, refresh_token).await
}

/// Link a provider identity to one explicitly authenticated user.
pub async fn link_to_user(
    pool: &PgPool,
    user_id: Uuid,
    info: OauthUserInfo,
    refresh_token: Option<String>,
) -> Result<LinkResult, LinkError> {
    resolve_provider_identity(
        pool,
        info,
        IdentityResolution::LinkTo(user_id),
        refresh_token,
    )
    .await
}

/// Resolve an OAuth identity according to the purpose validated by the callback.
///
/// Runs inside a single atomic transaction:
/// 0. For `LinkTo(user_id)` → link directly to that user.
/// 1. Existing stable provider identity → update its metadata and return.
/// 2. Brand-new identity → create a user + OAuth account.
async fn resolve_provider_identity(
    pool: &PgPool,
    info: OauthUserInfo,
    resolution: IdentityResolution,
    refresh_token: Option<String>,
) -> Result<LinkResult, LinkError> {
    // Reject unusable subjects before acquiring a connection or beginning any
    // mutation. Subjects are otherwise preserved exactly as providers issued
    // them because provider identifiers are case-sensitive opaque values.
    if info.provider_uid.trim().is_empty() {
        return Err(LinkError::InvalidProviderIdentity);
    }

    // Only verified provider email is allowed to become account metadata. The
    // identity itself is always provider + stable subject, never this email.
    let verified_email = info
        .email
        .as_deref()
        .filter(|email| info.email_verified && !email.trim().is_empty());
    let email_verified = verified_email.is_some();

    let mut tx = pool.begin().await.map_err(db::DbError::Connection)?;

    // Serialize resolution of one provider subject across app replicas. This
    // keeps concurrent first-use callbacks from racing to create extra users.
    let identity_lock_key = format!("{}:{}", info.provider.as_str(), info.provider_uid);
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(identity_lock_key)
        .execute(tx.deref_mut())
        .await
        .map_err(db::DbError::Query)?;

    // Linking is bound to the user validated by the OAuth callback. Login never
    // enters this branch, even if some unrelated session exists in the browser.
    if let IdentityResolution::LinkTo(user_id) = resolution {
        let user = db::get_user_by_id(tx.deref_mut(), user_id)
            .await?
            .ok_or(LinkError::Db(db::DbError::Query(sqlx::Error::RowNotFound)))?;

        // Check if this provider is already linked to this user (re-login case)
        if let Some(account) = db::get_oauth_account_by_provider(
            tx.deref_mut(),
            info.provider.as_str(),
            &info.provider_uid,
        )
        .await?
        {
            if account.user_id == user.id {
                db::update_oauth_last_used_and_email(
                    tx.deref_mut(),
                    account.id,
                    verified_email,
                    email_verified,
                )
                .await?;
                tx.commit().await.map_err(db::DbError::Connection)?;
                return Ok(LinkResult {
                    user_id: user.id,
                    oauth_account_id: account.id,
                    is_new_user: false,
                });
            }
            // Provider already linked to a different user — this is a conflict.
            // Use info.provider directly (already the correct enum from the caller)
            // instead of reconstructing from the DB string.
            tracing::warn!(
                event = "oauth_account_link_conflict",
                provider = info.provider.as_str(),
                "OAuth identity is already linked to another local user"
            );
            return Err(LinkError::AccountAlreadyLinked(info.provider.clone()));
        }

        let account = db::insert_oauth_account_with_email_verification(
            tx.deref_mut(),
            user.id,
            info.provider.as_str(),
            &info.provider_uid,
            verified_email,
            email_verified,
            None,
            refresh_token.as_deref(),
        )
        .await?;

        tx.commit().await.map_err(db::DbError::Connection)?;
        return Ok(LinkResult {
            user_id: user.id,
            oauth_account_id: account.id,
            is_new_user: false,
        });
    }

    // (a) Check for existing oauth account by provider+provider_uid
    if let Some(account) = db::get_oauth_account_by_provider(
        tx.deref_mut(),
        info.provider.as_str(),
        &info.provider_uid,
    )
    .await?
    {
        db::update_oauth_last_used_and_email(
            tx.deref_mut(),
            account.id,
            verified_email,
            email_verified,
        )
        .await?;
        tx.commit().await.map_err(db::DbError::Connection)?;
        return Ok(LinkResult {
            user_id: account.user_id,
            oauth_account_id: account.id,
            is_new_user: false,
        });
    }

    // Email is profile metadata, never proof that a new provider subject owns
    // an existing local account. Users can attach that identity only through
    // the authenticated `link_to_user` flow.
    if let Some(email) = verified_email {
        if db::get_user_by_email(tx.deref_mut(), email)
            .await?
            .is_some()
        {
            tracing::warn!(
                event = "oauth_email_conflict",
                provider = info.provider.as_str(),
                "New OAuth identity presented an email already in use"
            );
            return Err(LinkError::EmailAlreadyInUse);
        }
    }

    // (b) Brand-new user
    let username = generate_unique_username(&mut tx, &info.display_name).await?;
    let placeholder_email;
    let email = match verified_email {
        Some(email) => email,
        None => {
            placeholder_email = format!("noreply+{}@placeholder", Uuid::new_v4());
            placeholder_email.as_str()
        }
    };
    let avatar_url = info.avatar_url.as_deref();
    let user = match db::insert_user(
        tx.deref_mut(),
        &username,
        &info.display_name,
        email,
        avatar_url,
    )
    .await
    {
        Ok(user) => user,
        Err(error) if verified_email.is_some() && is_users_email_conflict(&error) => {
            tracing::warn!(
                event = "oauth_email_conflict",
                provider = info.provider.as_str(),
                "Concurrent OAuth identity creation found an email already in use"
            );
            return Err(LinkError::EmailAlreadyInUse);
        }
        Err(error) => return Err(error.into()),
    };
    let account = db::insert_oauth_account_with_email_verification(
        tx.deref_mut(),
        user.id,
        info.provider.as_str(),
        &info.provider_uid,
        verified_email,
        email_verified,
        None,
        refresh_token.as_deref(),
    )
    .await?;
    tx.commit().await.map_err(db::DbError::Connection)?;

    Ok(LinkResult {
        user_id: user.id,
        oauth_account_id: account.id,
        is_new_user: true,
    })
}

fn is_users_email_conflict(error: &db::DbError) -> bool {
    let db::DbError::Query(sqlx::Error::Database(database_error)) = error else {
        return false;
    };

    database_error.code().as_deref() == Some("23505")
        && database_error.constraint() == Some("users_email_key")
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // -- Unit tests for generate_username_from_display_name --

    #[test]
    fn username_from_normal_name() {
        assert_eq!(generate_username_from_display_name("John Doe"), "john-doe");
    }

    #[test]
    fn username_from_single_name() {
        assert_eq!(generate_username_from_display_name("Madonna"), "madonna");
    }

    #[test]
    fn username_from_non_ascii() {
        // é → NFKD: e + combining acute accent → strip combining marks → "e"
        assert_eq!(
            generate_username_from_display_name("José García"),
            "jose-garcia"
        );
    }

    #[test]
    fn username_from_long_name() {
        let long = "A Very Long Display Name That Should Be Truncated";
        let result = generate_username_from_display_name(long);
        assert!(result.len() <= 24, "result: {result}");
        assert!(!result.ends_with('-'));
        // The generated username should be a truncated, hyphenated version
        assert!(result.starts_with("a-very-long-display"));
    }

    #[test]
    fn username_from_empty_name() {
        assert_eq!(generate_username_from_display_name(""), "");
    }

    #[test]
    fn username_from_all_special_chars() {
        assert_eq!(generate_username_from_display_name("@#$%!"), "");
    }

    #[test]
    fn username_from_whitespace_only() {
        assert_eq!(generate_username_from_display_name("   \t\n  "), "");
    }

    #[test]
    fn username_collapse_hyphens() {
        assert_eq!(
            generate_username_from_display_name("Hello   World"),
            "hello-world"
        );
    }

    #[test]
    fn username_strips_trailing_hyphens() {
        assert_eq!(generate_username_from_display_name("John-"), "john");
    }

    #[test]
    fn username_preserves_digits() {
        assert_eq!(generate_username_from_display_name("User123"), "user123");
    }

    // -- Integration tests --

    use crate::test_utils;

    #[tokio::test]
    async fn test_existing_provider_login() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        // Create a user
        let user = db::insert_user(
            &pool,
            &format!("testuser_{u}"),
            "Test User",
            &format!("test{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        // Link a Google account
        let account = db::insert_oauth_account(
            &pool,
            user.id,
            "google",
            &format!("google-{u}"),
            Some(&format!("test{u}@example.com")),
            None,
            None,
        )
        .await
        .unwrap();

        // A provider email change must not change the resolved local identity.
        let result = login_or_create(
            &pool,
            OauthUserInfo {
                provider: Provider::Google,
                provider_uid: format!("google-{u}"),
                email: Some(format!("renamed{u}@example.com")),
                email_verified: true,
                display_name: "Test User".to_string(),
                avatar_url: None,
            },
            None,
        )
        .await
        .unwrap();

        assert_eq!(result.user_id, user.id);
        assert_eq!(result.oauth_account_id, account.id);
        assert!(!result.is_new_user);
    }

    #[tokio::test]
    async fn test_new_provider_same_email_does_not_claim_existing_user() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        // Create a user
        let user = db::insert_user(
            &pool,
            &format!("testuser_{u}"),
            "Test User",
            &format!("test{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        // A different stable provider identity must not inherit this account,
        // even when the provider presents the same verified email.
        let result = login_or_create(
            &pool,
            OauthUserInfo {
                provider: Provider::GitHub,
                provider_uid: format!("github-{u}"),
                email: Some(format!("test{u}@example.com")),
                email_verified: true,
                display_name: "Test User".to_string(),
                avatar_url: None,
            },
            None,
        )
        .await;

        assert!(matches!(result, Err(LinkError::EmailAlreadyInUse)));
        assert!(
            db::get_oauth_account_by_provider(&pool, "github", &format!("github-{u}"))
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(db::count_oauth_accounts(&pool, user.id).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_brand_new_user() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let result = login_or_create(
            &pool,
            OauthUserInfo {
                provider: Provider::Google,
                provider_uid: format!("google-{u}"),
                email: Some(format!("new{u}@example.com")),
                email_verified: true,
                display_name: "New User".to_string(),
                avatar_url: None,
            },
            None,
        )
        .await
        .unwrap();

        assert!(result.is_new_user);

        // Verify user was created
        let user = db::get_user_by_id(&pool, result.user_id)
            .await
            .unwrap()
            .unwrap();
        assert!(user.username.starts_with("new-user"));
        assert_eq!(user.email, format!("new{u}@example.com"));
        let oauth = db::get_oauth_account_by_provider(&pool, "google", &format!("google-{u}"))
            .await
            .unwrap()
            .unwrap();
        assert!(oauth.email_verified);
    }

    #[tokio::test]
    async fn test_username_collision_retry() {
        let (_db, pool) = test_utils::setup_test_db().await;

        // Create multiple users with the same display name —
        // each should get a unique username
        let mut usernames = Vec::new();
        for i in 0..3 {
            let result = login_or_create(
                &pool,
                OauthUserInfo {
                    provider: Provider::Google,
                    provider_uid: format!("collision-user-{i}"),
                    email: Some(format!("collision{i}@example.com")),
                    email_verified: true,
                    display_name: "Same Name".to_string(),
                    avatar_url: None,
                },
                None,
            )
            .await
            .unwrap();

            assert!(result.is_new_user);
            let user = db::get_user_by_id(&pool, result.user_id)
                .await
                .unwrap()
                .unwrap();
            assert!(user.username.starts_with("same-name-"));
            usernames.push(user.username);
        }

        // All usernames should be unique
        let unique_count = usernames
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len();
        assert_eq!(unique_count, 3, "usernames should be unique: {usernames:?}");
    }

    #[tokio::test]
    async fn test_brand_new_user_without_email() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let result = login_or_create(
            &pool,
            OauthUserInfo {
                provider: Provider::GitHub,
                provider_uid: format!("github-noemail-{u}"),
                email: None,
                email_verified: false,
                display_name: "No Email User".to_string(),
                avatar_url: None,
            },
            None,
        )
        .await
        .unwrap();

        assert!(result.is_new_user);

        // Verify user was created with a placeholder email
        let user = db::get_user_by_id(&pool, result.user_id)
            .await
            .unwrap()
            .unwrap();
        assert!(
            user.email.starts_with("noreply+"),
            "email should start with noreply+: {}",
            user.email
        );
        assert!(
            user.email.ends_with("@placeholder"),
            "email should end with @placeholder: {}",
            user.email
        );
        assert!(
            user.username.starts_with("no-email-user"),
            "username should start with no-email-user: {}",
            user.username
        );

        // Verify OAuth account email is None
        let oauth =
            db::get_oauth_account_by_provider(&pool, "github", &format!("github-noemail-{u}"))
                .await
                .unwrap()
                .unwrap();
        assert!(oauth.email.is_none(), "OAuth account email should be None");
    }

    #[tokio::test]
    async fn test_username_fallback_with_many_collisions() {
        let (_db, pool) = test_utils::setup_test_db().await;

        // Create many users with the same display name to exercise
        // collision handling extensively. Each call to login_or_create
        // generates a random 4-hex suffix; if that suffix is already
        // taken, it retries up to 5 times, then falls back to user-{8hex}
        // (with 3 fallback attempts). All usernames must remain unique.
        let mut usernames = Vec::new();
        for i in 0..10 {
            let result = login_or_create(
                &pool,
                OauthUserInfo {
                    provider: Provider::Google,
                    provider_uid: format!("fallback-user-{i}"),
                    email: Some(format!("fallback{i}@example.com")),
                    email_verified: true,
                    display_name: "Collision Test".to_string(),
                    avatar_url: None,
                },
                None,
            )
            .await
            .unwrap();

            assert!(result.is_new_user);
            let user = db::get_user_by_id(&pool, result.user_id)
                .await
                .unwrap()
                .unwrap();
            assert!(
                user.username.starts_with("collision-test-") || user.username.starts_with("user-"),
                "unexpected username: {}",
                user.username
            );
            usernames.push(user.username);
        }

        // All usernames should be unique even with the same display name
        let unique_count = usernames
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len();
        assert_eq!(
            unique_count, 10,
            "usernames should be unique: {usernames:?}"
        );
    }

    // -- Tests for session-aware linking (Issue #1) --

    #[tokio::test]
    async fn test_link_to_user_adds_provider() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        // Create a user with a Google account
        let user = db::insert_user(
            &pool,
            &format!("linkuser_{u}"),
            "Link User",
            &format!("link{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        db::insert_oauth_account(
            &pool,
            user.id,
            "google",
            &format!("google-{u}"),
            Some(&format!("link{u}@example.com")),
            None,
            None,
        )
        .await
        .unwrap();

        // Now link a GitHub account with the existing user's ID
        let result = link_to_user(
            &pool,
            user.id,
            OauthUserInfo {
                provider: Provider::GitHub,
                provider_uid: format!("github-{u}"),
                email: Some(format!("link{u}@example.com")),
                email_verified: true,
                display_name: "Link User".to_string(),
                avatar_url: None,
            },
            None,
        )
        .await
        .unwrap();

        // Should link to the existing user, not create a new one
        assert_eq!(result.user_id, user.id);
        assert!(!result.is_new_user);

        // Verify the GitHub account was created and linked to the existing user
        let github_account =
            db::get_oauth_account_by_provider(&pool, "github", &format!("github-{u}"))
                .await
                .unwrap()
                .unwrap();
        assert_eq!(github_account.user_id, user.id);

        // Verify no new user was created (only one user exists)
        let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(user_count, 1);
    }

    #[tokio::test]
    async fn test_recycled_email_with_new_subject_does_not_authenticate_existing_user() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        // Create a user with a Google account
        let user = db::insert_user(
            &pool,
            &format!("nosession_{u}"),
            "No Session User",
            &format!("nosession{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        db::insert_oauth_account(
            &pool,
            user.id,
            "google",
            &format!("google-nosession-{u}"),
            Some(&format!("nosession{u}@example.com")),
            None,
            None,
        )
        .await
        .unwrap();

        // Resolve a GitHub login with the same email and no linking target.
        let result = login_or_create(
            &pool,
            OauthUserInfo {
                provider: Provider::GitHub,
                provider_uid: format!("github-nosession-{u}"),
                email: Some(format!("nosession{u}@example.com")),
                email_verified: true,
                display_name: "No Session User".to_string(),
                avatar_url: None,
            },
            None,
        )
        .await;

        assert!(matches!(result, Err(LinkError::EmailAlreadyInUse)));
        assert_eq!(db::count_oauth_accounts(&pool, user.id).await.unwrap(), 1);
        assert!(db::get_oauth_account_by_provider(
            &pool,
            "github",
            &format!("github-nosession-{u}")
        )
        .await
        .unwrap()
        .is_none());
    }

    // -- Tests for account conflict detection (AC12 / NOMS-006) --

    #[tokio::test]
    async fn test_link_to_user_conflict_different_user() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        // Create two users
        let user_a = db::insert_user(
            &pool,
            &format!("user_a_{u}"),
            "User A",
            &format!("user_a_{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        let user_b = db::insert_user(
            &pool,
            &format!("user_b_{u}"),
            "User B",
            &format!("user_b_{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        // Link Google account to user A
        db::insert_oauth_account(
            &pool,
            user_a.id,
            "google",
            &format!("google-conflict-{u}"),
            Some(&format!("user_a_{u}@example.com")),
            None,
            None,
        )
        .await
        .unwrap();

        // User B attempts to link the same Google account — should error
        let result = link_to_user(
            &pool,
            user_b.id,
            OauthUserInfo {
                provider: Provider::Google,
                provider_uid: format!("google-conflict-{u}"),
                email: Some(format!("user_b_{u}@example.com")),
                email_verified: true,
                display_name: "User B".to_string(),
                avatar_url: None,
            },
            None,
        )
        .await;

        assert!(
            matches!(
                result,
                Err(LinkError::AccountAlreadyLinked(Provider::Google))
            ),
            "expected AccountAlreadyLinked(Google), got: {result:?}"
        );

        // Verify no new oauth account was created for user B
        let oauth_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM oauth_accounts WHERE user_id = $1 AND provider = 'google'",
        )
        .bind(user_b.id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(oauth_count, 0, "user B should have no google accounts");

        // Verify user A still has the original account
        let oauth_count_a: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM oauth_accounts WHERE user_id = $1 AND provider = 'google'",
        )
        .bind(user_a.id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            oauth_count_a, 1,
            "user A should still have the google account"
        );
    }

    #[tokio::test]
    async fn test_link_to_user_same_user_no_conflict() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        // Create a user with a Google account
        let user = db::insert_user(
            &pool,
            &format!("sameuser_{u}"),
            "Same User",
            &format!("sameuser_{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        let account = db::insert_oauth_account(
            &pool,
            user.id,
            "google",
            &format!("google-sameuser-{u}"),
            Some(&format!("sameuser_{u}@example.com")),
            None,
            None,
        )
        .await
        .unwrap();

        // Same user re-links the same provider — should succeed (re-login case)
        let result = link_to_user(
            &pool,
            user.id,
            OauthUserInfo {
                provider: Provider::Google,
                provider_uid: format!("google-sameuser-{u}"),
                email: Some(format!("sameuser_{u}@example.com")),
                email_verified: true,
                display_name: "Same User".to_string(),
                avatar_url: None,
            },
            None,
        )
        .await
        .unwrap();

        assert_eq!(result.user_id, user.id);
        assert_eq!(result.oauth_account_id, account.id);
        assert!(!result.is_new_user);
    }

    #[tokio::test]
    async fn test_link_to_user_conflict_github() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        // Create two users
        let user_a = db::insert_user(
            &pool,
            &format!("gh_user_a_{u}"),
            "GH User A",
            &format!("gh_a_{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        let user_b = db::insert_user(
            &pool,
            &format!("gh_user_b_{u}"),
            "GH User B",
            &format!("gh_b_{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        // Link GitHub account to user A
        db::insert_oauth_account(
            &pool,
            user_a.id,
            "github",
            &format!("github-conflict-{u}"),
            Some(&format!("gh_a_{u}@example.com")),
            None,
            None,
        )
        .await
        .unwrap();

        // User B attempts to link the same GitHub account — should error
        let result = link_to_user(
            &pool,
            user_b.id,
            OauthUserInfo {
                provider: Provider::GitHub,
                provider_uid: format!("github-conflict-{u}"),
                email: Some(format!("gh_b_{u}@example.com")),
                email_verified: true,
                display_name: "GH User B".to_string(),
                avatar_url: None,
            },
            None,
        )
        .await;

        assert!(
            matches!(
                result,
                Err(LinkError::AccountAlreadyLinked(Provider::GitHub))
            ),
            "expected AccountAlreadyLinked(GitHub), got: {result:?}"
        );
    }

    #[tokio::test]
    async fn test_empty_stable_provider_subject_is_rejected() {
        let (_db, pool) = test_utils::setup_test_db().await;

        let result = login_or_create(
            &pool,
            OauthUserInfo {
                provider: Provider::Google,
                provider_uid: "   ".to_string(),
                email: Some("verified@example.com".to_string()),
                email_verified: true,
                display_name: "Invalid Identity".to_string(),
                avatar_url: None,
            },
            None,
        )
        .await;

        assert!(matches!(result, Err(LinkError::InvalidProviderIdentity)));
        let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(user_count, 0);
    }
}
