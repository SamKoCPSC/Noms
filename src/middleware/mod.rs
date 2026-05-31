//! HTTP middleware for authentication and route protection.
//!
//! Only compiled when the `server` feature is enabled.
#![cfg(feature = "server")]

pub mod auth;
