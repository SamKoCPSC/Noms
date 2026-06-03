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

/// User profile extension injected into request extensions by the auth middleware.
///
/// Contains the full user profile fetched from the database. Read by
/// `build_context_from_fullstack()` to populate the Dioxus `AuthContext`.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AuthUserProfile {
    pub profile: UserProfile,
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
    pub bio: Option<String>,
}

/// Authentication context consumed by Dioxus components via [`use_auth`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuthContext {
    pub current_user_id: Option<Uuid>,
    /// Full user profile when available (populated via async fetch in future).
    pub current_user: Option<UserProfile>,
    pub is_authenticated: bool,
}

/// Hook to consume the authentication context.
///
/// Works on both server (SSR) and client (hydration). On the server,
/// the context is populated per-request from the auth middleware's
/// request extensions via `FullstackContext::extension`. On the client,
/// it hydrates from the server-rendered initial state.
pub fn use_auth() -> AuthContext {
    use_context::<AuthContext>()
}

// ── Context builder (server + client) ────────────────────────────────────────

/// Build an `AuthContext` from the current `FullstackContext`.
///
/// Reads the `AuthUser` extension that the auth middleware injected into
/// the request. Returns an unauthenticated context if no extension is found
/// (graceful degradation — the page still renders, just without the user).
///
/// On the server: `FullstackContext::current()` returns `Some`, so the
/// middleware-injected extensions are available.
/// On the client: `FullstackContext::current()` returns `None`, so a
/// default unauthenticated context is returned.
pub fn build_context_from_fullstack() -> AuthContext {
    #[cfg(feature = "server")]
    {
        use dioxus_fullstack::FullstackContext;

        let Some(fsc) = FullstackContext::current() else {
            return AuthContext::default();
        };

        let Some(auth_user) = fsc.extension::<AuthUser>() else {
            return AuthContext::default();
        };

        let Some(profile_ext) = fsc.extension::<AuthUserProfile>() else {
            return AuthContext {
                current_user_id: Some(auth_user.user_id),
                current_user: None,
                is_authenticated: true,
            };
        };

        AuthContext {
            current_user_id: Some(auth_user.user_id),
            current_user: Some(profile_ext.profile),
            is_authenticated: true,
        }
    }
    #[cfg(not(feature = "server"))]
    {
        AuthContext::default()
    }
}
