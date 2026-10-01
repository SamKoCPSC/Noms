//! Authentication module.

pub mod context;
pub mod redirect;
pub mod refresh;

#[cfg(feature = "server")]
pub mod linking;
#[cfg(feature = "server")]
pub mod logout;
#[cfg(feature = "server")]
pub mod oauth;
#[cfg(feature = "server")]
pub mod revoke;
#[cfg(feature = "server")]
pub mod session;
#[cfg(feature = "server")]
pub mod user_profile;
