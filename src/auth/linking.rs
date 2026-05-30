//! Account linking: match OAuth identities to users.
//!
//! Handles three flows:
//! 1. Returning user — existing provider+uid match
//! 2. New provider for existing user — email match
//! 3. Brand-new user — creates user + OAuth account

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

/// OAuth user info received from an external provider.
pub struct OauthUserInfo {
    pub provider: Provider,
    pub provider_uid: String,
    pub email: Option<String>,
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
}

impl std::fmt::Display for LinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LinkError::Db(e) => write!(f, "database error: {e}"),
            LinkError::UsernameGenerationFailed => {
                write!(f, "failed to generate a unique username after all attempts")
            }
        }
    }
}

impl std::error::Error for LinkError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LinkError::Db(e) => Some(e),
            LinkError::UsernameGenerationFailed => None,
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

        if !db::get_user_by_username(tx.deref_mut(), &candidate).await? {
            return Ok(candidate);
        }
    }

    // Fallback: user-{8hex} with uniqueness check
    for _ in 0..3 {
        let fallback = format!("user-{}", &Uuid::new_v4().to_string()[..8]);
        if !db::get_user_by_username(tx.deref_mut(), &fallback).await? {
            return Ok(fallback);
        }
    }
    Err(LinkError::UsernameGenerationFailed)
}

// ── Main linking function ────────────────────────────────────────────────────

/// Link an OAuth identity to a user, creating a new user if necessary.
///
/// Runs inside a single atomic transaction:
/// 1. Existing provider login → update `last_used_at` and return.
/// 2. Existing user by email  → link new OAuth account to that user.
/// 3. Brand-new user          → create user + OAuth account.
pub async fn link_or_create(pool: &PgPool, info: OauthUserInfo) -> Result<LinkResult, LinkError> {
    let mut tx = pool.begin().await.map_err(db::DbError::Connection)?;

    // (a) Check for existing oauth account by provider+provider_uid
    if let Some(account) = db::get_oauth_account_by_provider(
        tx.deref_mut(),
        info.provider.as_str(),
        &info.provider_uid,
    )
    .await?
    {
        db::update_oauth_last_used(tx.deref_mut(), account.id).await?;
        tx.commit().await.map_err(db::DbError::Connection)?;
        return Ok(LinkResult {
            user_id: account.user_id,
            oauth_account_id: account.id,
            is_new_user: false,
        });
    }

    // (b) Check for existing user by email
    if let Some(ref email) = info.email {
        if let Some(user) = db::get_user_by_email(tx.deref_mut(), email).await? {
            let account = db::insert_oauth_account(
                tx.deref_mut(),
                user.id,
                info.provider.as_str(),
                &info.provider_uid,
                Some(email),
                None,
            )
            .await?;
            tx.commit().await.map_err(db::DbError::Connection)?;
            return Ok(LinkResult {
                user_id: user.id,
                oauth_account_id: account.id,
                is_new_user: false,
            });
        }
    }

    // (c) Brand-new user
    let username = generate_unique_username(&mut tx, &info.display_name).await?;
    let placeholder_email;
    let email = match &info.email {
        Some(e) => e.as_str(),
        None => {
            placeholder_email = format!("noreply+{}@placeholder", Uuid::new_v4());
            placeholder_email.as_str()
        }
    };
    let avatar_url = info.avatar_url.as_deref();
    let user = db::insert_user(
        tx.deref_mut(),
        &username,
        &info.display_name,
        email,
        avatar_url,
    )
    .await?;
    let account = db::insert_oauth_account(
        tx.deref_mut(),
        user.id,
        info.provider.as_str(),
        &info.provider_uid,
        info.email.as_deref(),
        None,
    )
    .await?;
    tx.commit().await.map_err(db::DbError::Connection)?;

    Ok(LinkResult {
        user_id: user.id,
        oauth_account_id: account.id,
        is_new_user: true,
    })
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
        )
        .await
        .unwrap();

        // Now link with the same provider+uid — should return existing account
        let result = link_or_create(
            &pool,
            OauthUserInfo {
                provider: Provider::Google,
                provider_uid: format!("google-{u}"),
                email: Some(format!("test{u}@example.com")),
                display_name: "Test User".to_string(),
                avatar_url: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(result.user_id, user.id);
        assert_eq!(result.oauth_account_id, account.id);
        assert!(!result.is_new_user);
    }

    #[tokio::test]
    async fn test_new_provider_same_email() {
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

        // Link with a different provider but same email
        let result = link_or_create(
            &pool,
            OauthUserInfo {
                provider: Provider::GitHub,
                provider_uid: format!("github-{u}"),
                email: Some(format!("test{u}@example.com")),
                display_name: "Test User".to_string(),
                avatar_url: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(result.user_id, user.id);
        assert!(!result.is_new_user);

        // Verify the new oauth account was created
        let oauth = db::get_oauth_account_by_provider(&pool, "github", &format!("github-{u}"))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(oauth.user_id, user.id);
    }

    #[tokio::test]
    async fn test_brand_new_user() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let result = link_or_create(
            &pool,
            OauthUserInfo {
                provider: Provider::Google,
                provider_uid: format!("google-{u}"),
                email: Some(format!("new{u}@example.com")),
                display_name: "New User".to_string(),
                avatar_url: None,
            },
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
    }

    #[tokio::test]
    async fn test_username_collision_retry() {
        let (_db, pool) = test_utils::setup_test_db().await;

        // Create multiple users with the same display name —
        // each should get a unique username
        let mut usernames = Vec::new();
        for i in 0..3 {
            let result = link_or_create(
                &pool,
                OauthUserInfo {
                    provider: Provider::Google,
                    provider_uid: format!("collision-user-{i}"),
                    email: Some(format!("collision{i}@example.com")),
                    display_name: "Same Name".to_string(),
                    avatar_url: None,
                },
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

        let result = link_or_create(
            &pool,
            OauthUserInfo {
                provider: Provider::GitHub,
                provider_uid: format!("github-noemail-{u}"),
                email: None,
                display_name: "No Email User".to_string(),
                avatar_url: None,
            },
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
        // collision handling extensively. Each call to link_or_create
        // generates a random 4-hex suffix; if that suffix is already
        // taken, it retries up to 5 times, then falls back to user-{8hex}
        // (with 3 fallback attempts). All usernames must remain unique.
        let mut usernames = Vec::new();
        for i in 0..10 {
            let result = link_or_create(
                &pool,
                OauthUserInfo {
                    provider: Provider::Google,
                    provider_uid: format!("fallback-user-{i}"),
                    email: Some(format!("fallback{i}@example.com")),
                    display_name: "Collision Test".to_string(),
                    avatar_url: None,
                },
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
}
