//! Deployment-mode and production security configuration.

use std::collections::HashSet;

use axum::extract::{Request, State};
use axum::http::{header::STRICT_TRANSPORT_SECURITY, HeaderValue};
use axum::middleware::Next;
use axum::response::Response;
use url::{Host, Url};

const LOCAL_BASE_URL: &str = "http://localhost:8080";
const HSTS_VALUE: HeaderValue = HeaderValue::from_static("max-age=31536000");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeploymentMode {
    Local,
    Production,
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeConfig {
    mode: DeploymentMode,
    base_url: String,
    cookie_domain: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SecurityHeaders {
    hsts_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConfigError {
    variable: &'static str,
    reason: &'static str,
}

impl ConfigError {
    fn new(variable: &'static str, reason: &'static str) -> Self {
        Self { variable, reason }
    }
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "invalid production configuration for {}: {}",
            self.variable, self.reason
        )
    }
}

impl std::error::Error for ConfigError {}

impl RuntimeConfig {
    pub(crate) fn from_environment() -> Result<Self, ConfigError> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    pub(crate) fn from_lookup<F>(lookup: F) -> Result<Self, ConfigError>
    where
        F: Fn(&str) -> Option<String>,
    {
        let local_requested = lookup("NOMS_ENV").as_deref() == Some("local");
        let railway_deployment = ["RAILWAY_ENVIRONMENT_ID", "RAILWAY_PROJECT_ID"]
            .into_iter()
            .any(|name| lookup(name).is_some_and(|value| !value.trim().is_empty()));
        if local_requested && railway_deployment {
            return Err(ConfigError::new(
                "NOMS_ENV",
                "local mode is forbidden in a Railway deployment",
            ));
        }

        let mode = if local_requested {
            DeploymentMode::Local
        } else {
            DeploymentMode::Production
        };

        if mode == DeploymentMode::Local {
            let base_url = lookup("BASE_URL").unwrap_or_else(|| LOCAL_BASE_URL.to_string());
            validate_local_base_url(&base_url)?;
            return Ok(Self {
                mode,
                base_url: base_url.trim_end_matches('/').to_string(),
                cookie_domain: normalized_cookie_domain(lookup("COOKIE_DOMAIN"))?,
            });
        }

        let base_url = required(&lookup, "BASE_URL")?;
        validate_base_url("BASE_URL", &base_url, "https")?;

        let session_secret = required(&lookup, "SESSION_SECRET")?;
        validate_session_secret(&session_secret)?;

        let database_url = required(&lookup, "DATABASE_URL")?;
        let runtime_database = validate_database_url("DATABASE_URL", &database_url)?;
        if let Some(migration_database_url) = lookup("MIGRATION_DATABASE_URL") {
            let migration_database =
                validate_database_url("MIGRATION_DATABASE_URL", &migration_database_url)?;
            if runtime_database.username() == migration_database.username() {
                return Err(ConfigError::new(
                    "MIGRATION_DATABASE_URL",
                    "must use a different, schema-owning role from DATABASE_URL",
                ));
            }
            if runtime_database.host_str() != migration_database.host_str()
                || runtime_database.port_or_known_default()
                    != migration_database.port_or_known_default()
                || runtime_database.path() != migration_database.path()
            {
                return Err(ConfigError::new(
                    "MIGRATION_DATABASE_URL",
                    "must target the same database server and database as DATABASE_URL",
                ));
            }
        } else if lookup("NOMS_MIGRATIONS_APPLIED").as_deref() != Some("1") {
            return Err(ConfigError::new(
                "MIGRATION_DATABASE_URL",
                "value is required until the production entrypoint applies migrations",
            ));
        }

        for name in ["GOOGLE_CLIENT_ID", "GITHUB_CLIENT_ID"] {
            validate_identifier(name, &required(&lookup, name)?)?;
        }
        for name in [
            "GOOGLE_CLIENT_SECRET",
            "GITHUB_CLIENT_SECRET",
            "R2_SECRET_ACCESS_KEY",
        ] {
            validate_secret(name, &required(&lookup, name)?, 16)?;
        }
        validate_identifier("R2_ACCESS_KEY_ID", &required(&lookup, "R2_ACCESS_KEY_ID")?)?;
        validate_identifier("R2_BUCKET", &required(&lookup, "R2_BUCKET")?)?;

        for name in [
            "GOOGLE_AUTH_URL",
            "GOOGLE_TOKEN_URL",
            "GOOGLE_USERINFO_URL",
            "GITHUB_AUTH_URL",
            "GITHUB_TOKEN_URL",
            "GITHUB_USERINFO_URL",
            "GITHUB_EMAILS_URL",
            "R2_ENDPOINT",
            "R2_PUBLIC_URL",
        ] {
            validate_https_url(name, &required(&lookup, name)?)?;
        }

        Ok(Self {
            mode,
            base_url: base_url.trim_end_matches('/').to_string(),
            cookie_domain: normalized_cookie_domain(lookup("COOKIE_DOMAIN"))?,
        })
    }

    pub(crate) fn mode(&self) -> DeploymentMode {
        self.mode
    }

    pub(crate) fn base_url(&self) -> &str {
        &self.base_url
    }

    pub(crate) fn cookie_domain(&self) -> Option<&str> {
        self.cookie_domain.as_deref()
    }

    pub(crate) fn hsts_enabled(&self) -> bool {
        self.mode == DeploymentMode::Production
    }

    pub(crate) fn security_headers(&self) -> SecurityHeaders {
        SecurityHeaders {
            hsts_enabled: self.hsts_enabled(),
        }
    }
}

pub(crate) async fn security_headers_middleware(
    State(config): State<SecurityHeaders>,
    request: Request,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;
    if config.hsts_enabled {
        response
            .headers_mut()
            .insert(STRICT_TRANSPORT_SECURITY, HSTS_VALUE);
    }
    response
}

fn required<F>(lookup: &F, name: &'static str) -> Result<String, ConfigError>
where
    F: Fn(&str) -> Option<String>,
{
    lookup(name)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| ConfigError::new(name, "value is required"))
}

fn validate_session_secret(secret: &str) -> Result<(), ConfigError> {
    validate_secret("SESSION_SECRET", secret, 32)?;
    let unique_bytes = secret.as_bytes().iter().copied().collect::<HashSet<_>>();
    if unique_bytes.len() < 12 {
        return Err(ConfigError::new(
            "SESSION_SECRET",
            "value does not have enough character diversity",
        ));
    }
    Ok(())
}

fn validate_secret(
    name: &'static str,
    secret: &str,
    minimum_length: usize,
) -> Result<(), ConfigError> {
    let normalized = secret.trim().to_ascii_lowercase();
    let known_placeholder = normalized == "secret"
        || normalized.contains("change-me")
        || normalized.contains("mock-")
        || normalized.contains("local-secret")
        || normalized.contains("example");
    if secret.trim().len() < minimum_length || known_placeholder {
        return Err(ConfigError::new(
            name,
            "value is too short or matches a development placeholder",
        ));
    }
    Ok(())
}

fn validate_identifier(name: &'static str, value: &str) -> Result<(), ConfigError> {
    let normalized = value.trim().to_ascii_lowercase();
    if normalized.is_empty()
        || matches!(normalized.as_str(), "google" | "github" | "secret")
        || normalized.starts_with("mock-")
        || normalized.contains("local-key")
        || normalized.contains("example")
    {
        return Err(ConfigError::new(
            name,
            "value matches a development placeholder",
        ));
    }
    Ok(())
}

fn validate_local_base_url(value: &str) -> Result<(), ConfigError> {
    let url = Url::parse(value).map_err(|_| ConfigError::new("BASE_URL", "must be a URL"))?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(ConfigError::new(
            "BASE_URL",
            "must use HTTP or HTTPS and include a host",
        ));
    }
    Ok(())
}

fn validate_base_url(
    name: &'static str,
    value: &str,
    required_scheme: &str,
) -> Result<(), ConfigError> {
    let url = Url::parse(value).map_err(|_| ConfigError::new(name, "must be a URL"))?;
    if url.scheme() != required_scheme
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || !matches!(url.path(), "" | "/")
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(ConfigError::new(
            name,
            "must be an HTTPS origin without credentials, path, query, or fragment",
        ));
    }
    Ok(())
}

fn validate_https_url(name: &'static str, value: &str) -> Result<(), ConfigError> {
    let url = Url::parse(value).map_err(|_| ConfigError::new(name, "must be a URL"))?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(ConfigError::new(
            name,
            "must be HTTPS and must not contain credentials or a fragment",
        ));
    }
    Ok(())
}

fn validate_database_url(name: &'static str, value: &str) -> Result<Url, ConfigError> {
    let url = Url::parse(value).map_err(|_| ConfigError::new(name, "must be a PostgreSQL URL"))?;
    if !matches!(url.scheme(), "postgres" | "postgresql")
        || url.host_str().is_none()
        || url.username().is_empty()
        || url.password().is_none()
    {
        return Err(ConfigError::new(
            name,
            "must be a credentialed PostgreSQL URL with a host",
        ));
    }
    Ok(url)
}

fn normalized_cookie_domain(value: Option<String>) -> Result<Option<String>, ConfigError> {
    let Some(domain) = value.filter(|domain| !domain.trim().is_empty()) else {
        return Ok(None);
    };
    let candidate = domain.trim().trim_start_matches('.');
    if candidate.contains(['/', ':', '@'])
        || candidate.chars().any(char::is_whitespace)
        || !matches!(Host::parse(candidate), Ok(Host::Domain(_)))
    {
        return Err(ConfigError::new(
            "COOKIE_DOMAIN",
            "must be a DNS domain without a scheme, port, path, or credentials",
        ));
    }
    Ok(Some(domain.trim().to_string()))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{security_headers_middleware, DeploymentMode, RuntimeConfig};

    fn valid_production_environment() -> HashMap<String, String> {
        HashMap::from([
            ("NOMS_ENV".into(), "production".into()),
            ("BASE_URL".into(), "https://noms.example.com".into()),
            (
                "SESSION_SECRET".into(),
                "R7uZ_93pLm2Kx8Qw4Nc6Vt1Hs5Jb0DfE".into(),
            ),
            (
                "DATABASE_URL".into(),
                "postgres://noms:strong-password@db.internal:5432/noms".into(),
            ),
            (
                "MIGRATION_DATABASE_URL".into(),
                "postgres://noms_owner:owner-password@db.internal:5432/noms".into(),
            ),
            (
                "GOOGLE_CLIENT_ID".into(),
                "real-google-client-id.apps.googleusercontent.com".into(),
            ),
            (
                "GOOGLE_CLIENT_SECRET".into(),
                "google-production-secret-value".into(),
            ),
            (
                "GOOGLE_AUTH_URL".into(),
                "https://accounts.google.com/o/oauth2/v2/auth".into(),
            ),
            (
                "GOOGLE_TOKEN_URL".into(),
                "https://oauth2.googleapis.com/token".into(),
            ),
            (
                "GOOGLE_USERINFO_URL".into(),
                "https://openidconnect.googleapis.com/v1/userinfo".into(),
            ),
            ("GITHUB_CLIENT_ID".into(), "real-github-client-id".into()),
            (
                "GITHUB_CLIENT_SECRET".into(),
                "github-production-secret-value".into(),
            ),
            (
                "GITHUB_AUTH_URL".into(),
                "https://github.com/login/oauth/authorize".into(),
            ),
            (
                "GITHUB_TOKEN_URL".into(),
                "https://github.com/login/oauth/access_token".into(),
            ),
            (
                "GITHUB_USERINFO_URL".into(),
                "https://api.github.com/user".into(),
            ),
            (
                "GITHUB_EMAILS_URL".into(),
                "https://api.github.com/user/emails".into(),
            ),
            (
                "R2_ENDPOINT".into(),
                "https://tenant.r2.cloudflarestorage.com".into(),
            ),
            ("R2_ACCESS_KEY_ID".into(), "production-access-key".into()),
            (
                "R2_SECRET_ACCESS_KEY".into(),
                "production-storage-secret".into(),
            ),
            ("R2_BUCKET".into(), "noms-production-media".into()),
            (
                "R2_PUBLIC_URL".into(),
                "https://media.noms.example.com".into(),
            ),
        ])
    }

    fn load(environment: &HashMap<String, String>) -> Result<RuntimeConfig, String> {
        RuntimeConfig::from_lookup(|name| environment.get(name).cloned())
            .map_err(|error| error.to_string())
    }

    #[test]
    fn local_mode_keeps_isolated_mock_defaults_available() {
        let environment = HashMap::from([("NOMS_ENV".into(), "local".into())]);
        let config = load(&environment).expect("local development should retain mock defaults");

        assert_eq!(config.mode(), DeploymentMode::Local);
        assert_eq!(config.base_url(), "http://localhost:8080");
        assert!(!config.hsts_enabled());
    }

    #[test]
    fn railway_deployments_cannot_enable_local_security_bypasses() {
        let environment = HashMap::from([
            ("NOMS_ENV".into(), "local".into()),
            (
                "RAILWAY_ENVIRONMENT_ID".into(),
                "production-environment".into(),
            ),
        ]);

        let error = load(&environment).expect_err("Railway must never accept local mode");
        assert!(error.contains("NOMS_ENV"));
    }

    #[test]
    fn production_requires_every_security_critical_value() {
        for required_name in [
            "BASE_URL",
            "SESSION_SECRET",
            "DATABASE_URL",
            "MIGRATION_DATABASE_URL",
            "GOOGLE_CLIENT_ID",
            "GOOGLE_CLIENT_SECRET",
            "GOOGLE_AUTH_URL",
            "GOOGLE_TOKEN_URL",
            "GOOGLE_USERINFO_URL",
            "GITHUB_CLIENT_ID",
            "GITHUB_CLIENT_SECRET",
            "GITHUB_AUTH_URL",
            "GITHUB_TOKEN_URL",
            "GITHUB_USERINFO_URL",
            "GITHUB_EMAILS_URL",
            "R2_ENDPOINT",
            "R2_ACCESS_KEY_ID",
            "R2_SECRET_ACCESS_KEY",
            "R2_BUCKET",
            "R2_PUBLIC_URL",
        ] {
            let mut environment = valid_production_environment();
            environment.remove(required_name);
            let error = load(&environment).expect_err("production must fail closed");
            assert!(error.contains(required_name), "unexpected error: {error}");
        }
    }

    #[test]
    fn production_rejects_insecure_public_and_provider_endpoints() {
        for endpoint_name in [
            "BASE_URL",
            "GOOGLE_AUTH_URL",
            "GOOGLE_TOKEN_URL",
            "GOOGLE_USERINFO_URL",
            "GITHUB_AUTH_URL",
            "GITHUB_TOKEN_URL",
            "GITHUB_USERINFO_URL",
            "GITHUB_EMAILS_URL",
            "R2_ENDPOINT",
            "R2_PUBLIC_URL",
        ] {
            let mut environment = valid_production_environment();
            environment.insert(endpoint_name.into(), "http://insecure.example.com".into());
            let error = load(&environment).expect_err("production endpoints require HTTPS");
            assert!(error.contains(endpoint_name), "unexpected error: {error}");
        }
    }

    #[test]
    fn production_rejects_weak_or_mock_secrets_without_echoing_them() {
        for (name, value) in [
            (
                "SESSION_SECRET",
                "change-me-in-production-use-rust-random-uuid",
            ),
            ("SESSION_SECRET", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            ("GOOGLE_CLIENT_SECRET", "mock-google-client-secret"),
            ("GITHUB_CLIENT_SECRET", "secret"),
            ("R2_SECRET_ACCESS_KEY", "noms-local-secret"),
        ] {
            let mut environment = valid_production_environment();
            environment.insert(name.into(), value.into());
            let error = load(&environment).expect_err("weak production secret must be rejected");
            assert!(error.contains(name), "unexpected error: {error}");
            assert!(
                !error.contains(value),
                "configuration errors must not echo secrets"
            );
        }
    }

    #[test]
    fn production_uses_host_only_cookies_unless_a_valid_domain_is_explicit() {
        let host_only = load(&valid_production_environment()).expect("valid production config");
        assert_eq!(host_only.cookie_domain(), None);

        let mut explicit = valid_production_environment();
        explicit.insert("COOKIE_DOMAIN".into(), ".example.com".into());
        let domain_config = load(&explicit).expect("an explicit parent domain should be valid");
        assert_eq!(domain_config.cookie_domain(), Some(".example.com"));

        explicit.insert("COOKIE_DOMAIN".into(), "https://example.com/path".into());
        let error = load(&explicit).expect_err("cookie domains cannot contain URL components");
        assert!(error.contains("COOKIE_DOMAIN"));
    }

    #[test]
    fn valid_production_configuration_enables_hsts() {
        let config = load(&valid_production_environment()).expect("production config should pass");

        assert_eq!(config.mode(), DeploymentMode::Production);
        assert_eq!(config.base_url(), "https://noms.example.com");
        assert!(config.hsts_enabled());
    }

    #[test]
    fn production_requires_separate_runtime_and_migration_database_roles() {
        let mut environment = valid_production_environment();
        environment.insert(
            "MIGRATION_DATABASE_URL".into(),
            environment["DATABASE_URL"].clone(),
        );

        let error = load(&environment).expect_err("the application role cannot own its schema");
        assert!(error.contains("MIGRATION_DATABASE_URL"));
        assert!(!error.contains("strong-password"));

        environment = valid_production_environment();
        environment.insert(
            "MIGRATION_DATABASE_URL".into(),
            "postgres://noms_owner:owner-password@other-db.internal:5432/noms".into(),
        );
        let error = load(&environment).expect_err("both roles must target one database server");
        assert!(error.contains("MIGRATION_DATABASE_URL"));
    }

    #[test]
    fn production_entrypoint_marker_allows_migration_secret_removal() {
        let mut environment = valid_production_environment();
        environment.remove("MIGRATION_DATABASE_URL");
        environment.insert("NOMS_MIGRATIONS_APPLIED".into(), "1".into());

        load(&environment).expect("the reviewed entrypoint removes migration credentials");
    }

    #[tokio::test]
    async fn production_responses_include_hsts_without_affecting_local_http() {
        use axum::body::Body;
        use axum::http::{header::STRICT_TRANSPORT_SECURITY, Request};
        use axum::routing::get;
        use tower::ServiceExt;

        async fn ok() -> &'static str {
            "ok"
        }

        for (config, expected_header) in [
            (
                load(&valid_production_environment()).unwrap(),
                Some("max-age=31536000"),
            ),
            (
                load(&HashMap::from([("NOMS_ENV".into(), "local".into())])).unwrap(),
                None,
            ),
        ] {
            let app = axum::Router::new().route("/", get(ok)).layer(
                axum::middleware::from_fn_with_state(
                    config.security_headers(),
                    security_headers_middleware,
                ),
            );
            let response = app
                .oneshot(Request::get("/").body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(
                response
                    .headers()
                    .get(STRICT_TRANSPORT_SECURITY)
                    .and_then(|value| value.to_str().ok()),
                expected_header
            );
        }
    }
}
