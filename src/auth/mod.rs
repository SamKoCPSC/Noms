//! Authentication module.
//! Only compiled when the `server` feature is enabled.
#![cfg(feature = "server")]

pub mod linking;
pub mod oauth;
pub mod session;
