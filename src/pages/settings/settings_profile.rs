use dioxus::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

use crate::auth::context::{use_auth, AuthContext, UserProfile};
use crate::components::base::{
    Avatar, AvatarSize, Button, ButtonVariant, Card, Input, PageHeader, SettingsTab, SettingsTabs,
};
use crate::components::AuthRequired;

/// Steps in the 3-layer account deletion confirmation flow.
#[derive(Debug, Clone, Copy, PartialEq)]
enum DeleteStep {
    /// Layer 1: Initial confirmation dialog.
    Confirming,
    /// Layer 2: Typed confirmation ("delete <username>").
    Typing,
    /// Layer 3: Final confirmation ("This cannot be undone").
    Final,
}

/// Delete the authenticated user's account.
///
/// Deletes the user row from the database (oauth_accounts cascade automatically).
/// On the client side, the caller is responsible for logging out and redirecting.
#[server]
pub async fn delete_account() -> Result<(), ServerFnError> {
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let pool = crate::db::get_pool();

    crate::db::delete_user(&pool, user_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(())
}

/// Save user profile via server function.
#[server]
pub async fn save_profile(
    display_name: String,
    bio: Option<String>,
    new_username: Option<String>,
) -> Result<UserProfile, ServerFnError> {
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let pool = crate::db::get_pool();

    // --- Username validation ---
    let trimmed_username = new_username.as_ref().map(|u| u.trim().to_string());
    if let Some(ref username) = trimmed_username {
        if username.len() < 3 || username.len() > 30 {
            return Err(ServerFnError::new("Username must be 3-30 characters"));
        }
        if !username
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(ServerFnError::new(
                "Username can only contain letters, numbers, hyphens, and underscores",
            ));
        }
        if username.starts_with(['-', '_']) || username.ends_with(['-', '_']) {
            return Err(ServerFnError::new(
                "Username cannot start or end with a hyphen or underscore",
            ));
        }
    }

    // --- Display name validation ---
    let trimmed_name = display_name.trim().to_string();
    if trimmed_name.len() < 2 || trimmed_name.len() > 30 {
        return Err(ServerFnError::new("Display name must be 2-30 characters"));
    }

    // --- Bio validation ---
    let trimmed_bio = bio.map(|b| b.trim().to_string());
    if let Some(ref b) = trimmed_bio {
        if b.len() > 160 {
            return Err(ServerFnError::new("Bio must be 160 characters or less"));
        }
    }

    // --- Apply username change if provided ---
    if let Some(ref username) = trimmed_username {
        crate::db::update_username(&pool, user_id, username)
            .await
            .map_err(|e| match &e {
                crate::db::DbError::UsernameTaken => {
                    ServerFnError::new("That username is already taken. Please choose another.")
                }
                _ => ServerFnError::new(e.to_string()),
            })?;
    }

    // --- Apply display_name + bio update ---
    let updated =
        crate::db::update_user_profile(&pool, user_id, &trimmed_name, trimmed_bio.as_deref())
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(UserProfile {
        id: updated.id,
        username: updated.username,
        display_name: updated.display_name,
        email: updated.email,
        avatar_url: updated.avatar_url,
        bio: updated.bio,
    })
}

/// Upload a new avatar image for the authenticated user.
///
/// Expects raw WebP image bytes for both the full-size (400x400) and
/// thumbnail (100x100) variants. The images have already been
/// processed client-side before being sent here.
///
/// If the user already has an avatar, the old image files are deleted
/// from storage (best-effort) before the new ones are uploaded.
#[server]
pub async fn upload_avatar(
    full_bytes: Vec<u8>,
    thumb_bytes: Vec<u8>,
) -> Result<UserProfile, ServerFnError> {
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let pool = crate::db::get_pool();
    let storage = crate::storage::get_storage_client();

    // Validate WebP magic bytes (both variants)
    if !crate::api::upload::validate_webp_magic(&full_bytes) {
        return Err(ServerFnError::new("Invalid image format: must be WebP"));
    }
    if !crate::api::upload::validate_webp_magic(&thumb_bytes) {
        return Err(ServerFnError::new("Invalid thumbnail format: must be WebP"));
    }

    // Enforce max file size (2MB for avatars)
    const MAX_AVATAR_SIZE: usize = 2 * 1024 * 1024;
    if full_bytes.len() > MAX_AVATAR_SIZE {
        return Err(ServerFnError::new(format!(
            "Image too large: {} bytes (max {} bytes)",
            full_bytes.len(),
            MAX_AVATAR_SIZE
        )));
    }
    if thumb_bytes.len() > MAX_AVATAR_SIZE {
        return Err(ServerFnError::new(format!(
            "Thumbnail too large: {} bytes (max {} bytes)",
            thumb_bytes.len(),
            MAX_AVATAR_SIZE
        )));
    }

    // Fetch current user to get existing avatar URL for cleanup
    let current_user = crate::db::get_user_by_id(&pool, user_id)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch user: {e}")))?
        .ok_or_else(|| ServerFnError::new("User not found"))?;

    // Generate storage keys: avatars/{user_id}/{uuid}.webp and -thumb.webp
    let unique_id = uuid::Uuid::new_v4();
    let full_key = format!("avatars/{}/{}.webp", user_id, unique_id);
    let thumb_key = format!("avatars/{}/{}-thumb.webp", user_id, unique_id);

    // Upload new avatar to storage
    let full_url = storage
        .put_object(&full_key, &full_bytes, "image/webp")
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to upload avatar: {e}")))?;
    let _thumb_url = storage
        .put_object(&thumb_key, &thumb_bytes, "image/webp")
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to upload avatar thumbnail: {e}")))?;

    // Persist URL to database
    let updated = crate::db::update_avatar_url(&pool, user_id, Some(full_url.clone()))
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to save avatar URL: {e}")))?;

    // Delete old avatar from storage (best-effort)
    if let Some(ref old_url) = current_user.avatar_url {
        let public_base = storage.public_url().trim_end_matches('/');
        if let Some(old_key) = old_url
            .strip_prefix(public_base)
            .map(|s| s.trim_start_matches('/'))
        {
            let _ = storage.delete_object(old_key).await;
            // Also try to delete old thumbnail
            if let Some(old_base) = old_key.strip_suffix(".webp") {
                let _ = storage
                    .delete_object(&format!("{}-thumb.webp", old_base))
                    .await;
            }
        }
    }

    Ok(UserProfile {
        id: updated.id,
        username: updated.username,
        display_name: updated.display_name,
        email: updated.email,
        avatar_url: updated.avatar_url,
        bio: updated.bio,
    })
}

/// Remove the authenticated user's avatar.
///
/// Deletes the avatar image files from storage (best-effort) before
/// clearing the avatar_url in the database.
#[server]
pub async fn remove_avatar() -> Result<UserProfile, ServerFnError> {
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let pool = crate::db::get_pool();
    let storage = crate::storage::get_storage_client();

    // Fetch current user to get existing avatar URL for cleanup
    let current_user = crate::db::get_user_by_id(&pool, user_id)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch user: {e}")))?
        .ok_or_else(|| ServerFnError::new("User not found"))?;

    // Delete old avatar from storage (best-effort)
    if let Some(ref old_url) = current_user.avatar_url {
        let public_base = storage.public_url().trim_end_matches('/');
        if let Some(old_key) = old_url
            .strip_prefix(public_base)
            .map(|s| s.trim_start_matches('/'))
        {
            let _ = storage.delete_object(old_key).await;
            // Also try to delete old thumbnail
            if let Some(old_base) = old_key.strip_suffix(".webp") {
                let _ = storage
                    .delete_object(&format!("{}-thumb.webp", old_base))
                    .await;
            }
        }
    }

    // Clear avatar_url in database
    let updated = crate::db::update_avatar_url(&pool, user_id, None)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to remove avatar: {e}")))?;

    Ok(UserProfile {
        id: updated.id,
        username: updated.username,
        display_name: updated.display_name,
        email: updated.email,
        avatar_url: updated.avatar_url,
        bio: updated.bio,
    })
}

/// Processed avatar result: full-size (400x400) and thumbnail (100x100) WebP bytes.
#[cfg(target_arch = "wasm32")]
struct ProcessedAvatar {
    /// Full-size WebP bytes (400x400, center-cropped square)
    full_bytes: Vec<u8>,
    /// Thumbnail WebP bytes (100x100, center-cropped square)
    thumb_bytes: Vec<u8>,
}

/// Process an avatar image client-side: load, center-crop to square,
/// generate full-size (400x400) and thumbnail (100x100) WebP variants.
///
/// Only available on wasm32 target.
#[cfg(target_arch = "wasm32")]
async fn process_avatar_image(web_file: web_sys::File) -> Result<ProcessedAvatar, String> {
    use wasm_bindgen::JsCast;
    use web_sys::HtmlCanvasElement;

    let window = web_sys::window().ok_or("No window")?;
    let document = window.document().ok_or("No document")?;

    // Load the image using the existing utility
    let img = crate::utils::image_processing_client::load_image_from_file(&web_file).await?;

    // Center-crop: calculate source rectangle from the center of the image
    let natural_width = img.natural_width();
    let natural_height = img.natural_height();
    let min_dim = natural_width.min(natural_height);
    let sx = (natural_width as f64 - min_dim as f64) / 2.0;
    let sy = (natural_height as f64 - min_dim as f64) / 2.0;

    // Generate full-size (400x400) variant
    let full_bytes = {
        let canvas = document
            .create_element("canvas")
            .map_err(|e| format!("Failed to create canvas: {:?}", e))?
            .dyn_into::<HtmlCanvasElement>()
            .map_err(|_| "Failed to cast to HtmlCanvasElement")?;

        let size: u32 = 400;
        canvas.set_width(size);
        canvas.set_height(size);

        let ctx = canvas
            .get_context("2d")
            .map_err(|e| format!("Failed to get 2D context: {:?}", e))?
            .and_then(|v| v.dyn_into::<web_sys::CanvasRenderingContext2d>().ok())
            .ok_or("Failed to cast context")?;

        ctx.set_fill_style_str("white");
        ctx.fill_rect(0.0, 0.0, size as f64, size as f64);
        ctx.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
            &img,
            sx,
            sy,
            min_dim as f64,
            min_dim as f64,
            0.0,
            0.0,
            size as f64,
            size as f64,
        )
        .map_err(|e| format!("draw_image error: {:?}", e))?;

        crate::utils::image_processing_client::blob_from_canvas(&canvas, 0.9).await?
    };

    // Generate thumbnail (100x100) variant
    let thumb_bytes = {
        let canvas = document
            .create_element("canvas")
            .map_err(|e| format!("Failed to create canvas: {:?}", e))?
            .dyn_into::<HtmlCanvasElement>()
            .map_err(|_| "Failed to cast to HtmlCanvasElement")?;

        let size: u32 = 100;
        canvas.set_width(size);
        canvas.set_height(size);

        let ctx = canvas
            .get_context("2d")
            .map_err(|e| format!("Failed to get 2D context: {:?}", e))?
            .and_then(|v| v.dyn_into::<web_sys::CanvasRenderingContext2d>().ok())
            .ok_or("Failed to cast context")?;

        ctx.set_fill_style_str("white");
        ctx.fill_rect(0.0, 0.0, size as f64, size as f64);
        ctx.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
            &img,
            sx,
            sy,
            min_dim as f64,
            min_dim as f64,
            0.0,
            0.0,
            size as f64,
            size as f64,
        )
        .map_err(|e| format!("draw_image error: {:?}", e))?;

        crate::utils::image_processing_client::blob_from_canvas(&canvas, 0.8).await?
    };

    Ok(ProcessedAvatar {
        full_bytes,
        thumb_bytes,
    })
}

/// Programmatically trigger a click on a file input element by ID.
/// Only active on wasm32; no-op during SSR.
#[cfg(target_arch = "wasm32")]
fn trigger_file_input(id: &str) {
    if let Some(window) = web_sys::window() {
        if let Some(doc) = window.document() {
            if let Ok(Some(input)) = doc.query_selector(id) {
                if let Some(file_input) = input.dyn_ref::<web_sys::HtmlInputElement>() {
                    file_input.click();
                }
            }
        }
    }
}

/// No-op stub for non-wasm32 targets (SSR).
#[cfg(not(target_arch = "wasm32"))]
fn trigger_file_input(_id: &str) {
    // No DOM available during server-side rendering
}

/// User profile settings page.
///
/// Displays editable display name and bio fields with validation,
/// optimistic UI updates, and rollback on failure.
#[component]
pub fn SettingsProfile() -> Element {
    let auth = use_auth();

    let mut username = use_signal(String::new);
    let mut display_name = use_signal(String::new);
    let mut bio = use_signal(String::new);
    let mut is_saving = use_signal(|| false);
    let mut error = use_signal(|| Option::<String>::None);
    let mut saved_message = use_signal(|| Option::<String>::None);

    // Delete account state
    let mut delete_step = use_signal(|| None::<DeleteStep>);
    let mut delete_input = use_signal(String::new);
    let mut deleting = use_signal(|| false);
    let mut delete_error = use_signal(|| Option::<String>::None);

    // Avatar upload state
    let avatar_uploading = use_signal(|| false);
    let mut avatar_url_signal = use_signal(|| Option::<String>::None);

    // Extract username for typed confirmation (before use_effect consumes auth.current_user)
    let delete_username = auth
        .current_user
        .as_ref()
        .map(|u| u.username.clone())
        .unwrap_or_default();

    // Extract email for read-only display
    let email = auth
        .current_user
        .as_ref()
        .map(|u| u.email.clone())
        .unwrap_or_default();

    // Extract avatar URL for display - initialize signal from auth context
    let initial_avatar_url = auth
        .current_user
        .as_ref()
        .and_then(|u| u.avatar_url.clone());
    if let Some(ref url) = initial_avatar_url {
        if avatar_url_signal.read().is_none() {
            avatar_url_signal.set(Some(url.clone()));
        }
    }
    let avatar_url = avatar_url_signal.read().clone();

    // Avatar name: use display_name, fall back to username if empty
    let avatar_name = {
        let dn = display_name();
        if dn.trim().is_empty() {
            username()
        } else {
            dn
        }
    };

    // Hold a reference to the auth context signal for updating after save (Issue #5)
    let mut auth_context = use_context::<Signal<AuthContext>>();

    // Track whether form fields have been initialized from auth context
    let mut initialized = use_signal(|| false);

    // Load profile from auth context when it becomes available.
    // Reading `auth_context.read()` inside the effect closure subscribes the
    // effect to the signal, so it re-runs when auth data arrives asynchronously.
    // The `initialized` guard ensures we only populate once.
    use_effect(move || {
        if !initialized() {
            if let Some(user) = auth_context.read().current_user.as_ref() {
                username.set(user.username.clone());
                display_name.set(user.display_name.clone());
                bio.set(user.bio.clone().unwrap_or_default());
                initialized.set(true);
            }
        }
    });

    let on_save = move |_| {
        let new_username = username().clone();
        let new_name = display_name().clone();
        let new_bio = bio().clone();
        let trimmed_username = new_username.trim().to_string();
        let trimmed_name = new_name.trim().to_string();
        let trimmed_bio = if new_bio.trim().is_empty() {
            None
        } else {
            Some(new_bio.trim().to_string())
        };

        // Read current username from auth context FRESH at handler time,
        // not captured at render time. This avoids the issue where
        // `current_username` is `None` on initial client render. (Issue #4)
        let current_user = auth_context.read().current_user.clone();
        let username_changed = current_user
            .as_ref()
            .is_some_and(|cu| cu.username != trimmed_username);

        let username_to_send = if username_changed {
            Some(trimmed_username.clone())
        } else {
            None
        };

        // --- Username validation (client-side) ---
        if username_changed {
            if trimmed_username.len() < 3 || trimmed_username.len() > 30 {
                error.set(Some("Username must be 3-30 characters".to_string()));
                return;
            }
            if !trimmed_username
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
            {
                error.set(Some(
                    "Username can only contain letters, numbers, hyphens, and underscores"
                        .to_string(),
                ));
                return;
            }
            if trimmed_username.starts_with(['-', '_']) || trimmed_username.ends_with(['-', '_']) {
                error.set(Some(
                    "Username cannot start or end with a hyphen or underscore".to_string(),
                ));
                return;
            }
        }

        // Validate client-side
        if trimmed_name.len() < 2 || trimmed_name.len() > 30 {
            error.set(Some("Display name must be 2-30 characters".to_string()));
            return;
        }
        if let Some(ref b) = trimmed_bio {
            if b.len() > 160 {
                error.set(Some("Bio must be 160 characters or less".to_string()));
                return;
            }
        }

        // Capture committed values from auth context for reliable rollback.
        // These are the last server-authoritative values, not the current
        // form signals which may contain uncommitted invalid input.
        let committed = auth_context.read().current_user.as_ref().map(|u| {
            (
                u.username.clone(),
                u.display_name.clone(),
                u.bio.clone().unwrap_or_default(),
            )
        });

        // Optimistic update: apply trimmed values immediately
        username.set(trimmed_username.clone());
        display_name.set(trimmed_name.clone());
        bio.set(trimmed_bio.clone().unwrap_or_default());

        is_saving.set(true);
        error.set(None);

        // Spawn async save
        let username_for_server = username_to_send.clone();
        let name_for_server = trimmed_name.clone();
        let bio_for_server = trimmed_bio.clone();
        spawn(async move {
            match save_profile(name_for_server, bio_for_server, username_for_server).await {
                Ok(profile) => {
                    // Apply server-authoritative values
                    username.set(profile.username.clone());
                    display_name.set(profile.display_name.clone());
                    bio.set(profile.bio.clone().unwrap_or_default());

                    // Update the global auth context so navbar reflects changes immediately
                    auth_context.write().current_user = Some(profile);

                    saved_message.set(Some("Profile saved successfully!".to_string()));
                    is_saving.set(false);
                }
                Err(e) => {
                    // Extract clean error message without Dioxus wrapper text
                    let msg = match &e {
                        ServerFnError::ServerError { message, .. } => message.clone(),
                        _ => e.to_string(),
                    };

                    // Clear any previous success message so it doesn't show alongside error
                    saved_message.set(None);

                    // Rollback to last committed values from auth context
                    if let Some((cu, cn, cb)) = committed {
                        username.set(cu);
                        display_name.set(cn);
                        bio.set(cb);
                    }
                    error.set(Some(msg));
                    is_saving.set(false);
                }
            }
        });
    };

    let is_valid = {
        let u = username().trim().to_string();
        let n = display_name().trim().to_string();
        let b = bio().trim().to_string();

        // Username validity: if changed, must meet constraints; if unchanged, always valid.
        // Read current username from auth context FRESH (not captured at render time). (Issue #4)
        let username_valid = {
            let current = auth_context
                .read()
                .current_user
                .as_ref()
                .map(|u| u.username.clone());
            if current.as_ref().is_some_and(|cu| cu.as_str() != u.as_str()) {
                u.len() >= 3
                    && u.len() <= 30
                    && u.chars()
                        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
                    && !u.starts_with(['-', '_'])
                    && !u.ends_with(['-', '_'])
            } else {
                true
            }
        };

        n.len() >= 2 && n.len() <= 30 && b.len() <= 160 && username_valid
    };

    // Current username for typed confirmation
    let expected_confirm = format!("delete {}", delete_username);
    let input_matches = delete_input() == expected_confirm;

    // Delete account handlers
    let on_delete_confirm = move |_| {
        delete_step.set(Some(DeleteStep::Typing));
        delete_input.set(String::new());
        delete_error.set(None);
    };

    let on_delete_cancel = move |_| {
        delete_step.set(None);
        delete_input.set(String::new());
        delete_error.set(None);
    };

    let on_delete_continue = move |_| {
        delete_step.set(Some(DeleteStep::Final));
    };

    let on_delete_go_back = move |_| {
        delete_step.set(Some(DeleteStep::Typing));
    };

    let on_delete_final = move |_| {
        deleting.set(true);
        delete_error.set(None);

        spawn(async move {
            match delete_account().await {
                Ok(()) => {
                    // Full-page navigation to logout endpoint (clears cookie + redirects to /)
                    // Browsers ignore Set-Cookie from XHR, so we navigate directly
                    #[cfg(target_arch = "wasm32")]
                    {
                        if let Some(window) = web_sys::window() {
                            let _ = window.location().set_href("/auth/logout?redirect_uri=/");
                        }
                    }
                }
                Err(e) => {
                    let msg = match &e {
                        ServerFnError::ServerError { message, .. } => message.clone(),
                        _ => e.to_string(),
                    };
                    delete_error.set(Some(msg));
                    deleting.set(false);
                }
            }
        });
    };

    // Avatar upload handler: process image client-side, then upload
    // Uses the same pattern as image_upload.rs for file handling
    let on_avatar_file_change = move |files: Vec<dioxus::html::FileData>| {
        if files.is_empty() {
            return;
        }

        #[cfg(target_arch = "wasm32")]
        {
            let mut auth_context = auth_context;
            let mut avatar_url = avatar_url_signal;
            let mut avatar_uploading = avatar_uploading;
            let mut error = error;
            let mut saved_message = saved_message;

            spawn(async move {
                use dioxus_web::WebFileExt;

                avatar_uploading.set(true);
                error.set(None);

                let file_data = &files[0];
                let web_file = match file_data.get_web_file() {
                    Some(f) => f,
                    None => {
                        error.set(Some("Could not read file".to_string()));
                        avatar_uploading.set(false);
                        return;
                    }
                };

                // Process: read file, center-crop on canvas, encode as WebP bytes
                let processed = process_avatar_image(web_file).await;
                let processed = match processed {
                    Ok(data) => data,
                    Err(e) => {
                        error.set(Some(format!("Failed to process image: {e}")));
                        avatar_uploading.set(false);
                        return;
                    }
                };

                // Upload to server (both full-size and thumbnail)
                match upload_avatar(processed.full_bytes, processed.thumb_bytes).await {
                    Ok(profile) => {
                        avatar_url.set(profile.avatar_url.clone());
                        auth_context.write().current_user = Some(profile);
                        saved_message.set(Some("Avatar updated successfully!".to_string()));
                    }
                    Err(e) => {
                        let msg = match &e {
                            ServerFnError::ServerError { message, .. } => message.clone(),
                            _ => e.to_string(),
                        };
                        error.set(Some(msg));
                    }
                }

                avatar_uploading.set(false);
            });
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = &files;
        }
    };

    // Avatar remove handler
    let on_remove_avatar = move |_| {
        let mut auth_context = auth_context;
        let mut avatar_url = avatar_url_signal;
        let mut avatar_uploading = avatar_uploading;
        let mut error = error;
        let mut saved_message = saved_message;

        spawn(async move {
            avatar_uploading.set(true);
            error.set(None);

            match remove_avatar().await {
                Ok(profile) => {
                    avatar_url.set(profile.avatar_url.clone());
                    auth_context.write().current_user = Some(profile);
                    saved_message.set(Some("Avatar removed successfully!".to_string()));
                }
                Err(e) => {
                    let msg = match &e {
                        ServerFnError::ServerError { message, .. } => message.clone(),
                        _ => e.to_string(),
                    };
                    error.set(Some(msg));
                }
            }

            avatar_uploading.set(false);
        });
    };

    rsx! {
        AuthRequired {
            div { class: "container",
            PageHeader {
                title: "Profile Settings",
            }
            SettingsTabs {
                active: SettingsTab::Profile,
            }
            div {
                max_width: "500px",
                Card {
                    div {
                        display: "flex",
                        flex_direction: "column",
                        gap: "var(--space-md)",
                        // Avatar section
                        div {
                            display: "flex",
                            flex_direction: "column",
                            align_items: "center",
                            gap: "var(--space-sm)",
                            label {
                                font_size: "14px",
                                font_weight: "600",
                                color: "var(--text-secondary)",
                                align_self: "flex-start",
                                "Avatar"
                            }
                            Avatar {
                                size: AvatarSize::Large,
                                src: avatar_url.clone(),
                                username: avatar_name.clone(),
                            }
                            if avatar_uploading() {
                                span {
                                    font_size: "12px",
                                    color: "var(--text-tertiary)",
                                    "Uploading..."
                                }
                            } else if avatar_url.is_some() {
                                div {
                                    display: "flex",
                                    gap: "var(--space-sm)",
                                    Button {
                                        variant: ButtonVariant::Secondary,
                                        onclick: move |_| trigger_file_input("#avatar-upload-input"),
                                        "Change Photo"
                                    }
                                    Button {
                                        variant: ButtonVariant::Danger,
                                        onclick: on_remove_avatar,
                                        "Remove"
                                    }
                                }
                            } else {
                                Button {
                                        variant: ButtonVariant::Secondary,
                                        onclick: move |_| trigger_file_input("#avatar-upload-input"),
                                        "Choose Photo"
                                    }
                            }
                            // Hidden file input for avatar upload
                            input {
                                id: "avatar-upload-input",
                                r#type: "file",
                                accept: "image/png,image/jpeg,image/webp,image/gif",
                                display: "none",
                                onchange: move |evt| {
                                    on_avatar_file_change(evt.files());
                                },
                            }
                        }
                        div {
                            display: "flex",
                            flex_direction: "column",
                            gap: "var(--space-sm)",
                            label {
                                font_size: "14px",
                                font_weight: "600",
                                color: "var(--text-secondary)",
                                "Display Name"
                            }
                            Input {
                                value: display_name().clone(),
                                placeholder: "Your name",
                                oninput: move |v| {
                                    display_name.set(v);
                                    error.set(None);
                                },
                            }
                            span {
                                font_size: "12px",
                                color: "var(--text-tertiary)",
                                "{display_name().trim().len()}/30"
                            }
                        }
                        // Username field
                        div {
                            display: "flex",
                            flex_direction: "column",
                            gap: "var(--space-sm)",
                            label {
                                font_size: "14px",
                                font_weight: "600",
                                color: "var(--text-secondary)",
                                "Username"
                            }
                            div {
                                display: "flex",
                                align_items: "center",
                                span {
                                    padding: "var(--space-sm) 0 var(--space-sm) var(--space-md)",
                                    font_family: "var(--font-body)",
                                    font_size: "14px",
                                    color: "var(--text-tertiary)",
                                    font_style: "italic",
                                    "@ "
                                }
                                Input {
                                    value: username().clone(),
                                    placeholder: "username",
                                    oninput: move |v| {
                                        username.set(v);
                                        error.set(None);
                                    },
                                }
                            }
                            span {
                                font_size: "12px",
                                color: "var(--text-tertiary)",
                                "{username().trim().len()}/30"
                            }
                        }
                        // Email field (read-only)
                        div {
                            display: "flex",
                            flex_direction: "column",
                            gap: "var(--space-sm)",
                            label {
                                font_size: "14px",
                                font_weight: "600",
                                color: "var(--text-secondary)",
                                "Email"
                            }
                            div {
                                padding: "var(--space-sm) var(--space-md)",
                                background_color: "var(--surface)",
                                border: "1px solid var(--border)",
                                border_radius: "var(--radius-md)",
                                font_family: "var(--font-body)",
                                font_size: "14px",
                                color: "var(--text-tertiary)",
                                "{email}"
                            }
                        }
                        div {
                            display: "flex",
                            flex_direction: "column",
                            gap: "var(--space-sm)",
                            label {
                                font_size: "14px",
                                font_weight: "600",
                                color: "var(--text-secondary)",
                                "Bio"
                            }
                            textarea {
                                class: "neumo-inset input",
                                placeholder: "Tell us about yourself...",
                                rows: "3",
                                maxlength: "160",
                                value: "{bio()}",
                                oninput: move |evt| {
                                    bio.set(evt.value());
                                    error.set(None);
                                },
                                padding: "var(--space-sm) var(--space-md)",
                                font_family: "var(--font-body)",
                                font_size: "14px",
                                color: "var(--text-primary)",
                                background_color: "var(--surface)",
                                outline: "none",
                                resize: "vertical",
                                width: "100%",
                            }
                            span {
                                font_size: "12px",
                                color: "var(--text-tertiary)",
                                "{bio().trim().len()}/160"
                            }
                        }
                        // Error message
                        if let Some(err) = error() {
                            div {
                                padding: "var(--space-sm) var(--space-md)",
                                background_color: "var(--error-bg)",
                                border_radius: "var(--radius-md)",
                                color: "var(--error)",
                                font_size: "14px",
                                "{err}"
                            }
                        }
                        // Saved message
                        if let Some(msg) = saved_message() {
                            div {
                                padding: "var(--space-sm) var(--space-md)",
                                background_color: "var(--success-bg)",
                                border_radius: "var(--radius-md)",
                                color: "var(--success)",
                                font_size: "14px",
                                "{msg}"
                            }
                        }
                        // Save button
                        Button {
                            variant: ButtonVariant::Primary,
                            disabled: is_saving() || !is_valid,
                            onclick: on_save,
                            if is_saving() {
                                "Saving..."
                            } else {
                                "Save Changes"
                            }
                        }
                    }
                }
            }

            // ── Danger Zone ──────────────────────────────────────────────
            div {
                class: "danger-zone",
                margin_top: "var(--space-xl)",
                max_width: "500px",
                Card {
                    div {
                        display: "flex",
                        flex_direction: "column",
                        gap: "var(--space-sm)",
                        h3 {
                            color: "var(--error)",
                            font_size: "16px",
                            "Danger Zone"
                        }
                        p {
                            font_size: "14px",
                            color: "var(--text-secondary)",
                            "Permanently delete your account and all associated data."
                        }
                        Button {
                            variant: ButtonVariant::Danger,
                            onclick: move |_| {
                                delete_step.set(Some(DeleteStep::Confirming));
                                delete_error.set(None);
                            },
                            "Delete Account"
                        }
                    }
                }
            }

            // ── Modal Overlays ───────────────────────────────────────────
            if let Some(step) = delete_step() {
                div {
                    class: "modal-overlay",
                    onclick: move |_| {
                        // Close on backdrop click (only for Confirming and Final steps)
                        if step == DeleteStep::Confirming || step == DeleteStep::Final {
                            delete_step.set(None);
                        }
                    },

                    // Layer 1: Initial Confirmation
                    if step == DeleteStep::Confirming {
                        div {
                            class: "modal-card",
                            onclick: move |evt| evt.stop_propagation(),
                            display: "flex",
                            flex_direction: "column",
                            gap: "var(--space-md)",
                            h2 {
                                "Are you sure?"
                            }
                            p {
                                font_size: "14px",
                                color: "var(--text-secondary)",
                                "This will permanently delete your account and all associated data."
                            }
                            div {
                                display: "flex",
                                justify_content: "flex-end",
                                gap: "var(--space-sm)",
                                Button {
                                    variant: ButtonVariant::Ghost,
                                    onclick: on_delete_cancel,
                                    "Cancel"
                                }
                                Button {
                                    variant: ButtonVariant::Danger,
                                    onclick: on_delete_confirm,
                                    "Confirm"
                                }
                            }
                        }
                    }

                    // Layer 2: Typed Confirmation
                    if step == DeleteStep::Typing {
                        div {
                            class: "modal-card",
                            onclick: move |evt| evt.stop_propagation(),
                            display: "flex",
                            flex_direction: "column",
                            gap: "var(--space-md)",
                            p {
                                font_size: "14px",
                                color: "var(--text-secondary)",
                                "Type "
                                code {
                                    "`delete {delete_username}`"
                                }
                                " to continue"
                            }
                            Input {
                                value: delete_input().clone(),
                                placeholder: "delete ...",
                                oninput: move |v| {
                                    delete_input.set(v);
                                    delete_error.set(None);
                                },
                            }
                            div {
                                display: "flex",
                                justify_content: "flex-end",
                                gap: "var(--space-sm)",
                                Button {
                                    variant: ButtonVariant::Ghost,
                                    onclick: on_delete_cancel,
                                    "Cancel"
                                }
                                Button {
                                    variant: ButtonVariant::Danger,
                                    disabled: !input_matches,
                                    onclick: on_delete_continue,
                                    "Continue"
                                }
                            }
                        }
                    }

                    // Layer 3: Final Confirmation
                    if step == DeleteStep::Final {
                        div {
                            class: "modal-card",
                            onclick: move |evt| evt.stop_propagation(),
                            display: "flex",
                            flex_direction: "column",
                            gap: "var(--space-md)",
                            h2 {
                                "This cannot be undone"
                            }
                            p {
                                font_size: "14px",
                                color: "var(--text-secondary)",
                                "All your data will be permanently deleted."
                            }
                            if let Some(err) = delete_error() {
                                div {
                                    padding: "var(--space-sm) var(--space-md)",
                                    background_color: "var(--error-bg)",
                                    border_radius: "var(--radius-md)",
                                    color: "var(--error)",
                                    font_size: "14px",
                                    "{err}"
                                }
                            }
                            div {
                                display: "flex",
                                justify_content: "flex-end",
                                gap: "var(--space-sm)",
                                Button {
                                    variant: ButtonVariant::Ghost,
                                    onclick: on_delete_go_back,
                                    "Go Back"
                                }
                                Button {
                                    variant: ButtonVariant::Danger,
                                    disabled: deleting(),
                                    onclick: on_delete_final,
                                    if deleting() {
                                        "Deleting..."
                                    } else {
                                        "Delete My Account"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        }
    }
}
