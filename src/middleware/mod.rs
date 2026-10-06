//! HTTP middleware for authentication and route protection.
//!
//! Only compiled when the `server` feature is enabled.
#![cfg(feature = "server")]

pub mod auth;
pub(crate) mod local_origin;
pub mod rate_limit;
