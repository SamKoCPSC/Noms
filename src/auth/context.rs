//! Authentication context for Dioxus components.
//!
//! Server side: populates context from request extensions (set by auth middleware).
//! Client side: provides a default-unauthenticated context that hydrates
//! from the SSR-rendered initial state.

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Shared types ────────────────────────────────────────────────────────────

/// Verified user injected into request extensions by the auth middleware.
///
/// Shared between `middleware/auth.rs` (which inserts it) and this module
/// (which reads it via `FullstackContext::extension`). Only used on the server.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
}

/// User profile exposed to Dioxus components.
///
/// Populated by a server function that fetches the full user record from the
/// database. Currently defined for future use (login page, profile display).
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: Uuid,
    pub username: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
}

/// Authentication context consumed by Dioxus components via [`use_auth`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuthContext {
    pub current_user_id: Option<Uuid>,
    pub is_authenticated: bool,
}

/// Hook to consume the authentication context.
///
/// Works on both server (SSR) and client (hydration). On the server,
/// the context is populated per-request from the auth middleware's
/// request extensions via `FullstackContext::extension`. On the client,
/// it hydrates from the server-rendered initial state.
#[allow(dead_code)] // Consumed by Dioxus page components
pub fn use_auth() -> AuthContext {
    use_context::<AuthContext>()
}

// ── Server-side context provider ─────────────────────────────────────────────

/// Build an `AuthContext` from the current `FullstackContext`.
///
/// Reads the `AuthUser` extension that the auth middleware injected into
/// the request. Returns an unauthenticated context if no extension is found
/// (graceful degradation — the page still renders, just without the user).
///
/// This is called synchronously from `ServeConfig::context_provider`, which
/// runs during SSR rendering when `FullstackContext::current()` is available.
#[cfg(feature = "server")]
pub fn build_context_from_fullstack() -> AuthContext {
    use dioxus_fullstack::FullstackContext;

    let Some(fsc) = FullstackContext::current() else {
        return AuthContext::default();
    };

    let Some(auth_user) = fsc.extension::<AuthUser>() else {
        return AuthContext::default();
    };

    AuthContext {
        current_user_id: Some(auth_user.user_id),
        is_authenticated: true,
    }
}
