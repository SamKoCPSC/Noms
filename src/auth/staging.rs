//! Secret-protected test sessions for one explicitly approved Railway deployment.
use axum::{
    body::to_bytes,
    extract::{Extension, Request},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

pub(crate) const ENVIRONMENT_ID: &str = "196c1b53-86cd-451f-81d0-aaf9cadc2e9a";
const PROJECT_ID: &str = "8f16a6e9-6089-4088-9307-8ca7c29b12e2";
const SERVICE_ID: &str = "199b610b-f88e-4c4d-a9f1-6120cbf3d1ae";
pub(crate) const ORIGIN: &str = "https://staging.nomsrecipes.com";
// This identity is reserved exclusively for the staging bootstrap. Never look up
// an existing normal account by a client-supplied ID, username or email.
const TEST_USER_ID: uuid::Uuid = uuid::Uuid::from_u128(0x38d6fd8a_1ec5_4791_b782_51ea46e11c7c);

pub(crate) fn is_approved_environment(lookup: &impl Fn(&str) -> Option<String>) -> bool {
    lookup("RAILWAY_ENVIRONMENT_ID").as_deref() == Some(ENVIRONMENT_ID)
        && lookup("RAILWAY_PROJECT_ID").as_deref() == Some(PROJECT_ID)
        && lookup("RAILWAY_SERVICE_ID").as_deref() == Some(SERVICE_ID)
        && lookup("BASE_URL").is_some_and(|v| v == ORIGIN || v == format!("{ORIGIN}/"))
}

#[derive(Clone)]
pub(crate) struct StagingLoginConfig {
    digest: [u8; 32],
}
impl std::fmt::Debug for StagingLoginConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("StagingLoginConfig([redacted])")
    }
}
impl StagingLoginConfig {
    pub(crate) fn new(secret: &str) -> Self {
        Self {
            digest: Sha256::digest(secret.as_bytes()).into(),
        }
    }
    fn accepts(&self, headers: &axum::http::HeaderMap) -> bool {
        let mut values = headers.get_all(header::AUTHORIZATION).iter();
        let Some(value) = values.next().and_then(|v| v.to_str().ok()) else {
            return false;
        };
        if values.next().is_some() {
            return false;
        }
        let Some(secret) = value.strip_prefix("Bearer ") else {
            return false;
        };
        if secret.len() != 64 {
            return false;
        }
        let actual: [u8; 32] = Sha256::digest(secret.as_bytes()).into();
        bool::from(self.digest.ct_eq(&actual))
    }
}

pub(crate) async fn create_test_session(
    Extension(pool): Extension<sqlx::PgPool>,
    config: Option<Extension<StagingLoginConfig>>,
    request: Request,
) -> Response {
    let mut response = handle(pool, config, request).await;
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, "private, no-store".parse().unwrap());
    response
        .headers_mut()
        .insert(header::PRAGMA, "no-cache".parse().unwrap());
    response
}

async fn handle(
    pool: sqlx::PgPool,
    config: Option<Extension<StagingLoginConfig>>,
    request: Request,
) -> Response {
    let Some(Extension(config)) = config else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if !config.accepts(request.headers()) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let headers = request.headers();
    let mut origins = headers.get_all(header::ORIGIN).iter();
    if let Some(origin) = origins.next() {
        if origin.to_str().ok() != Some(ORIGIN) || origins.next().is_some() {
            return StatusCode::FORBIDDEN.into_response();
        }
    }
    if request.uri().query().is_some() {
        return StatusCode::BAD_REQUEST.into_response();
    }
    if to_bytes(request.into_body(), 0).await.is_err() {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let result = crate::security::run_with_database_budget(crate::security::DATABASE_EXECUTION_BUDGET, async {
        let mut tx = pool.begin().await?;
        sqlx::query("INSERT INTO users (id, username, display_name, email) VALUES ($1, 'codex_staging_tester', 'Codex staging tester', 'codex-staging@noms.invalid') ON CONFLICT (id) DO NOTHING")
            .bind(TEST_USER_ID).execute(&mut *tx).await?;
        let token = super::session::create_session(&mut *tx, TEST_USER_ID).await.map_err(|_| sqlx::Error::Protocol("Could not create staging session".into()))?;
        tx.commit().await?;
        Ok::<_, sqlx::Error>(token)
    }).await;
    match result {
        Ok(Ok(token)) => {
            let mut cookie = super::session::build_session_cookie(&token);
            cookie.set_secure(true);
            cookie.unset_domain();
            (
                StatusCode::NO_CONTENT,
                [(header::SET_COOKIE, cookie.to_string())],
            )
                .into_response()
        }
        _ => (
            StatusCode::SERVICE_UNAVAILABLE,
            "Could not create a test session. Try again shortly.",
        )
            .into_response(),
    }
}

#[cfg(test)]
#[path = "staging_tests.rs"]
mod tests;
