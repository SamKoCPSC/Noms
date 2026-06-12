//! Shared types used across the client-server boundary.
//!
//! These types are serialized/deserialized by Dioxus `#[server]` functions
//! and must be available on both the client (wasm) and server.
#![allow(dead_code)]

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// A recipe saved by a user.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Recipe {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub prep_time_minutes: Option<i32>,
    pub cook_time_minutes: Option<i32>,
    pub servings: Option<i32>,
    pub instructions: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Paginated recipe list response for server functions.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RecipeListResponse {
    pub recipes: Vec<Recipe>,
    pub total_count: i64,
    pub has_more: bool,
}
