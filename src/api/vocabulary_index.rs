//! Authenticated, cache-aware downloads for the maintained ingredient vocabulary.
use crate::auth::context::AuthUser;
use axum::{
    body::{Body, Bytes},
    extract::{Extension, Path},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use noms::vocabulary::{index::valid_version, index_store::IndexService};
use std::sync::Arc;

#[cfg(test)]
#[path = "vocabulary_index_tests.rs"]
mod tests;

fn error(status: StatusCode, text: &'static str) -> Response {
    let mut response = (status, text).into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    response
}

fn authorize(viewer: Option<Extension<AuthUser>>) -> Result<(), (StatusCode, &'static str)> {
    let Some(Extension(viewer)) = viewer else {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Sign in to download ingredient suggestions.",
        ));
    };
    crate::security::enforce_user_request_budget(
        viewer.user_id,
        crate::security::RequestClass::Autocomplete,
    )
    .map_err(|_| {
        (
            StatusCode::TOO_MANY_REQUESTS,
            "Too many vocabulary requests.",
        )
    })
}

async fn bounded<T>(
    operation: impl std::future::Future<Output = Result<T, Box<dyn std::error::Error + Send + Sync>>>,
) -> Result<T, Response> {
    crate::security::run_with_expensive_read_work_budget(crate::security::run_with_database_budget(
        crate::security::DATABASE_EXECUTION_BUDGET,
        operation,
    ))
    .await
    .map_err(|_| error(StatusCode::SERVICE_UNAVAILABLE, "Vocabulary is busy."))?
    .map_err(|_| error(StatusCode::SERVICE_UNAVAILABLE, "Vocabulary timed out."))?
    .map_err(|e| {
        tracing::error!(%e,"Vocabulary snapshot failed");
        error(StatusCode::SERVICE_UNAVAILABLE, "Vocabulary unavailable.")
    })
}

fn response(
    headers: &HeaderMap,
    body: Arc<[u8]>,
    version: &str,
    immutable: bool,
    gzip: bool,
) -> Response {
    let tag = format!("W/\"{version}\"");
    let matches = headers
        .get(header::IF_NONE_MATCH)
        .and_then(|x| x.to_str().ok())
        .is_some_and(|s| {
            s.split(',').any(|s| {
                let s = s.trim();
                s == "*" || s == tag || s == format!("\"{version}\"")
            })
        });
    let mut response = if matches {
        StatusCode::NOT_MODIFIED.into_response()
    } else {
        Body::from(Bytes::from_owner(body)).into_response()
    };
    let h = response.headers_mut();
    h.insert(
        header::CONTENT_TYPE,
        "application/json; charset=utf-8".parse().unwrap(),
    );
    h.insert(header::ETAG, tag.parse().unwrap());
    h.insert(
        header::CACHE_CONTROL,
        if immutable {
            "private, max-age=31536000, immutable"
        } else {
            "private, no-cache"
        }
        .parse()
        .unwrap(),
    );
    h.insert(header::VARY, "Accept-Encoding, Cookie".parse().unwrap());
    if gzip && !matches {
        h.insert(header::CONTENT_ENCODING, "gzip".parse().unwrap());
    }
    response
}

fn accepts_gzip(headers: &HeaderMap) -> bool {
    headers
        .get(header::ACCEPT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|h| {
            h.split(',').any(|item| {
                let mut parts = item.trim().split(';');
                let encoding = parts.next().unwrap_or_default().trim();
                let q = parts
                    .find_map(|p| p.trim().strip_prefix("q="))
                    .and_then(|q| q.parse::<f32>().ok())
                    .unwrap_or(1.0);
                encoding.eq_ignore_ascii_case("gzip") && q > 0.0
            })
        })
}

pub(crate) async fn manifest(
    Extension(service): Extension<IndexService>,
    viewer: Option<Extension<AuthUser>>,
    headers: HeaderMap,
) -> Response {
    if let Err((status, message)) = authorize(viewer) {
        return error(status, message);
    }
    match bounded(service.current()).await {
        Ok(s) => response(
            &headers,
            s.manifest.clone(),
            &s.manifest_version,
            false,
            false,
        ),
        Err(e) => e,
    }
}

pub(crate) async fn catalog(
    Path(file): Path<String>,
    Extension(service): Extension<IndexService>,
    viewer: Option<Extension<AuthUser>>,
    headers: HeaderMap,
) -> Response {
    if let Err((status, message)) = authorize(viewer) {
        return error(status, message);
    }
    let Some(version) = file.strip_suffix(".json").filter(|v| valid_version(v)) else {
        return error(StatusCode::BAD_REQUEST, "Invalid vocabulary version.");
    };
    match bounded(service.version(version)).await {
        Ok(Some(s)) => {
            let gzip = accepts_gzip(&headers);
            response(
                &headers,
                if gzip { s.gzip.clone() } else { s.body.clone() },
                &s.version,
                true,
                gzip,
            )
        }
        Ok(None) => error(
            StatusCode::GONE,
            "Vocabulary changed. Refresh the version and try again.",
        ),
        Err(e) => e,
    }
}
