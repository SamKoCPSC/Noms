//! Authentication module.
//! Only compiled when the `server` feature is enabled.
#![cfg(feature = "server")]
#![allow(dead_code)]

pub mod session;
