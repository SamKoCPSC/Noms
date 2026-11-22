//! Authentication module.

pub mod account_binding;
pub mod context;
pub(crate) mod local_origin;
pub mod redirect;
pub mod refresh;
pub mod session_page;

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

#[cfg(feature = "server")]
pub(crate) mod staging;
pub(crate) mod staging_context;
