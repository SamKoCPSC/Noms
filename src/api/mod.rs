pub mod collection;
pub mod collection_portability;
pub mod grocery_list;
pub mod pantry;
pub mod recipe;

pub mod search;

pub mod upload;

#[cfg(feature = "server")]
mod unit_validation;
pub mod vocabulary;
#[cfg(feature = "server")]
pub mod vocabulary_index;

#[cfg(feature = "server")]
pub mod recipe_archive;

#[cfg(feature = "server")]
mod recipe_json_media;

#[cfg(feature = "server")]
pub(crate) mod recipe_html;

#[cfg(feature = "server")]
pub(crate) mod collection_html;
