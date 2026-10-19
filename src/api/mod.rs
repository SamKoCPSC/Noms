pub mod collection;
pub mod recipe;

pub mod search;

pub mod upload;

#[cfg(feature = "server")]
pub mod recipe_archive;

#[cfg(feature = "server")]
mod recipe_json_media;

#[cfg(feature = "server")]
pub(crate) mod recipe_html;
