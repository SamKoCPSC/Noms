use dioxus::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

use crate::auth::context::{use_auth, AuthContext, UserProfile};
use crate::components::base::{
    Avatar, AvatarSize, Button, ButtonVariant, Card, ImageEditorCropMode, ImageEditorModal, Input,
    PageHeader, SettingsTab, SettingsTabs,
};
use crate::components::AuthRequired;
use crate::types::RecipeImageEdit;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AvatarEditSource {
    pub original_url: String,
    pub edit: RecipeImageEdit,
}

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

fn avatar_photo_actions(has_avatar: bool) -> &'static [&'static str] {
    if has_avatar {
        &["Edit Photo", "Change Photo", "Remove"]
    } else {
        &["Choose Photo"]
    }
}

#[cfg(test)]
mod avatar_action_tests {
    use super::avatar_photo_actions;

    #[test]
    fn existing_avatar_offers_edit_without_requiring_replacement() {
        assert_eq!(
            avatar_photo_actions(true),
            &["Edit Photo", "Change Photo", "Remove"]
        );
        assert_eq!(avatar_photo_actions(false), &["Choose Photo"]);
    }
}

/// Delete the authenticated user's account.
///
/// Atomically stages durable storage cleanup authority before deleting the user
/// row (oauth_accounts cascade automatically). On the client side, the caller
/// is responsible for logging out and redirecting.
#[server]
pub async fn delete_account() -> Result<(), ServerFnError> {
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let pool = crate::db::get_pool();
    let storage = crate::storage::get_storage_client();
    let _media_lifecycle_guard = crate::api::upload::acquire_media_lifecycle_guard(user_id).await;

    crate::db::delete_user_with_media_cleanup(&pool, user_id, &storage)
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
/// If the user already has an avatar, its old image files are deleted
/// best-effort only after the new avatar is uploaded and persisted.
#[cfg(feature = "server")]
fn validate_avatar_payload<'a>(
    original_bytes: &'a [u8],
    full_bytes: &'a [u8],
    thumbnail_bytes: &'a [u8],
    edit: &RecipeImageEdit,
) -> Result<crate::api::upload::MediaUploadRequest<'a>, String> {
    if original_bytes.is_empty() || full_bytes.is_empty() || thumbnail_bytes.is_empty() {
        return Err("Avatar image variants must not be empty".to_string());
    }
    crate::security::validate_image_edit_metadata(edit)
        .map_err(|_| "Avatar edit metadata is invalid".to_string())?;
    crate::api::upload::MediaUploadRequest::avatar(original_bytes, full_bytes, thumbnail_bytes)
}

#[cfg(feature = "server")]
fn map_avatar_upload_error(
    error: crate::api::upload::MediaUploadError,
    user_id: uuid::Uuid,
) -> ServerFnError {
    match error {
        crate::api::upload::MediaUploadError::RateLimited => {
            ServerFnError::new("Upload rate limit exceeded")
        }
        crate::api::upload::MediaUploadError::UsageUnavailable => {
            ServerFnError::new("Unable to verify media quota")
        }
        crate::api::upload::MediaUploadError::QuotaExceeded => {
            ServerFnError::new("Media storage quota exceeded")
        }
        crate::api::upload::MediaUploadError::Storage(error) => {
            tracing::error!(%error, %user_id, "Failed to upload avatar media variants");
            ServerFnError::new("Failed to upload avatar")
        }
    }
}

#[cfg(feature = "server")]
async fn persist_uploaded_avatar(
    pool: &sqlx::PgPool,
    storage: &crate::storage::StorageClient,
    user_id: uuid::Uuid,
    uploaded: crate::api::upload::UploadedMedia,
    edit_json: serde_json::Value,
) -> Result<(crate::db::User, crate::api::upload::UploadedMedia), ServerFnError> {
    let uploaded_urls = (
        uploaded.url(crate::storage::MediaObjectKind::AvatarOriginal),
        uploaded.url(crate::storage::MediaObjectKind::AvatarFull),
        uploaded.url(crate::storage::MediaObjectKind::AvatarThumbnail),
    );
    let (Some(original_url), Some(full_url), Some(_thumbnail_url)) = uploaded_urls else {
        uploaded.compensate(storage).await;
        return Err(ServerFnError::new(
            "Uploaded avatar response was incomplete",
        ));
    };
    let original_url = original_url.to_string();
    let full_url = full_url.to_string();

    let updated = sqlx::query_as::<_, crate::db::User>(
        "UPDATE users SET avatar_url = $2, avatar_original_url = $3, avatar_edit = $4, updated_at = NOW() \
         WHERE id = $1 \
         RETURNING id, username, display_name, email, avatar_url, bio, created_at, updated_at",
    )
    .bind(user_id)
    .bind(Some(full_url))
    .bind(Some(original_url))
    .bind(Some(edit_json))
    .fetch_one(pool)
    .await;
    match updated {
        Ok(updated) => Ok((updated, uploaded)),
        Err(error) => {
            uploaded.compensate(storage).await;
            Err(ServerFnError::new(format!(
                "Failed to save avatar URL: {error}"
            )))
        }
    }
}

#[server]
pub async fn upload_avatar(
    original_bytes: Vec<u8>,
    full_bytes: Vec<u8>,
    thumb_bytes: Vec<u8>,
    edit: RecipeImageEdit,
) -> Result<UserProfile, ServerFnError> {
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let pool = crate::db::get_pool();
    let storage = crate::storage::get_storage_client();

    let upload_request = validate_avatar_payload(&original_bytes, &full_bytes, &thumb_bytes, &edit)
        .map_err(ServerFnError::new)?;
    // Serialize the complete read-upload-write-retire lifecycle. Acquiring the
    // permit after reading the old URLs would let concurrent replacements both
    // retire the same predecessor and leave the first new avatar unreferenced.
    let upload_permit = crate::api::upload::acquire_media_upload_permit(user_id)
        .await
        .map_err(|error| map_avatar_upload_error(error, user_id))?;

    let (previous_avatar, previous_original): (Option<String>, Option<String>) =
        sqlx::query_as("SELECT avatar_url, avatar_original_url FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&pool)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch current avatar: {e}")))?
            .ok_or_else(|| ServerFnError::new("User not found"))?;

    let edit_json = serde_json::to_value(&edit)
        .map_err(|e| ServerFnError::new(format!("Failed to serialize avatar edit: {e}")))?;
    let uploaded = upload_permit
        .upload(&storage, upload_request)
        .await
        .map_err(|error| map_avatar_upload_error(error, user_id))?;
    let (updated, _uploaded_lifecycle_guard) =
        persist_uploaded_avatar(&pool, &storage, user_id, uploaded, edit_json).await?;

    // Stored URLs are legacy/user-influenced data, so cleanup requires exact
    // typed ownership proof even though failures remain best-effort.
    delete_owned_avatar_objects(
        &storage,
        user_id,
        previous_avatar.as_deref(),
        previous_original.as_deref(),
    )
    .await;

    Ok(UserProfile {
        id: updated.id,
        username: updated.username,
        display_name: updated.display_name,
        email: updated.email,
        avatar_url: updated.avatar_url,
        bio: updated.bio,
    })
}

#[server]
pub async fn get_avatar_edit_source() -> Result<Option<AvatarEditSource>, ServerFnError> {
    use sqlx::Row;

    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let pool = crate::db::get_pool();
    let row =
        sqlx::query("SELECT avatar_url, avatar_original_url, avatar_edit FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&pool)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to load avatar edit: {e}")))?
            .ok_or_else(|| ServerFnError::new("User not found"))?;
    let avatar_url: Option<String> = row.try_get("avatar_url").ok();
    let original_url: Option<String> = row.try_get("avatar_original_url").ok();
    let edit_json: Option<serde_json::Value> = row.try_get("avatar_edit").ok();
    let Some(original_url) = original_url.or(avatar_url) else {
        return Ok(None);
    };
    let edit = edit_json
        .map(serde_json::from_value)
        .transpose()
        .map_err(|e| ServerFnError::new(format!("Invalid saved avatar edit: {e}")))?
        .unwrap_or_else(RecipeImageEdit::default_ellipse_1x1);

    Ok(Some(AvatarEditSource { original_url, edit }))
}

/// Remove the authenticated user's avatar.
///
/// Clears the database fields first, then deletes avatar storage objects on a
/// best-effort basis so a database failure never leaves dangling URLs.
#[server]
pub async fn remove_avatar() -> Result<UserProfile, ServerFnError> {
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    crate::security::enforce_user_request_budget(user_id, crate::security::RequestClass::Mutation)
        .map_err(|_| ServerFnError::new("Mutation rate limit exceeded"))?;
    let pool = crate::db::get_pool();
    let storage = crate::storage::get_storage_client();
    let _media_lifecycle_guard = crate::api::upload::acquire_media_lifecycle_guard(user_id).await;

    let (avatar_url, original_url): (Option<String>, Option<String>) =
        sqlx::query_as("SELECT avatar_url, avatar_original_url FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&pool)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch current avatar: {e}")))?
            .ok_or_else(|| ServerFnError::new("User not found"))?;

    let updated = sqlx::query_as::<_, crate::db::User>(
        "UPDATE users SET avatar_url = NULL, avatar_original_url = NULL, avatar_edit = NULL, updated_at = NOW() \
         WHERE id = $1 \
         RETURNING id, username, display_name, email, avatar_url, bio, created_at, updated_at",
    )
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to remove avatar: {e}")))?;

    // Delete old avatar objects only after the database update succeeds.
    delete_owned_avatar_objects(
        &storage,
        user_id,
        avatar_url.as_deref(),
        original_url.as_deref(),
    )
    .await;

    Ok(UserProfile {
        id: updated.id,
        username: updated.username,
        display_name: updated.display_name,
        email: updated.email,
        avatar_url: updated.avatar_url,
        bio: updated.bio,
    })
}

#[cfg(feature = "server")]
async fn delete_owned_avatar_objects(
    storage: &crate::storage::StorageClient,
    owner_id: uuid::Uuid,
    avatar_url: Option<&str>,
    original_url: Option<&str>,
) {
    if let Some(avatar_url) = avatar_url {
        match storage.parse_media_url(avatar_url) {
            Ok(object)
                if object.owner_id() == owner_id
                    && object.kind() == crate::storage::MediaObjectKind::AvatarFull =>
            {
                let _ = storage.delete_object(object.key()).await;
                if let Some(thumbnail_key) = object.paired_thumbnail_key() {
                    let _ = storage.delete_object(&thumbnail_key).await;
                }
            }
            Ok(object) => {
                tracing::warn!(
                    owner_id = %owner_id,
                    object_owner_id = %object.owner_id(),
                    object_key = %object.key(),
                    "Skipping foreign or non-avatar media during avatar cleanup"
                );
            }
            Err(error) => {
                tracing::warn!(
                    owner_id = %owner_id,
                    error = %error,
                    "Skipping unverified avatar media during cleanup"
                );
            }
        }
    }

    if let Some(original_url) = original_url {
        match storage.parse_media_url(original_url) {
            Ok(object)
                if object.owner_id() == owner_id
                    && object.kind() == crate::storage::MediaObjectKind::AvatarOriginal =>
            {
                let _ = storage.delete_object(object.key()).await;
            }
            Ok(object) => {
                tracing::warn!(
                    owner_id = %owner_id,
                    object_owner_id = %object.owner_id(),
                    object_key = %object.key(),
                    "Skipping foreign or non-original media during avatar cleanup"
                );
            }
            Err(error) => {
                tracing::warn!(
                    owner_id = %owner_id,
                    error = %error,
                    "Skipping unverified avatar original during cleanup"
                );
            }
        }
    }
}

/// Source rectangle in original-image pixels used when rendering an avatar.
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub(crate) struct AvatarSourceRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Convert the editor's normalized crop metadata into a safe pixel source
/// rectangle for canvas rendering.
#[allow(dead_code)]
pub(crate) fn avatar_source_rect(
    edit: &RecipeImageEdit,
    image_width: f64,
    image_height: f64,
) -> AvatarSourceRect {
    let image_width = if image_width.is_finite() {
        image_width.max(0.0)
    } else {
        0.0
    };
    let image_height = if image_height.is_finite() {
        image_height.max(0.0)
    } else {
        0.0
    };
    let width = edit.crop.width.clamp(0.0, 1.0);
    let height = edit.crop.height.clamp(0.0, 1.0);
    let x = edit.crop.x.clamp(0.0, 1.0 - width);
    let y = edit.crop.y.clamp(0.0, 1.0 - height);

    AvatarSourceRect {
        x: x * image_width,
        y: y * image_height,
        width: width * image_width,
        height: height * image_height,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg(any(target_arch = "wasm32", test))]
struct AvatarTransform {
    rotation_radians: f64,
    scale_x: f64,
    scale_y: f64,
}

#[cfg(any(target_arch = "wasm32", test))]
fn avatar_transform(
    edit: &RecipeImageEdit,
    image_width: f64,
    image_height: f64,
) -> AvatarTransform {
    let stretch_x = edit.transform.stretch_x.abs();
    let stretch_y = edit.transform.stretch_y.abs();
    let radians = edit.transform.rotation.to_radians();
    let cos = radians.cos().abs();
    let sin = radians.sin().abs();
    let transformed_width = image_width * stretch_x * cos + image_height * stretch_y * sin;
    let transformed_height = image_width * stretch_x * sin + image_height * stretch_y * cos;
    let fit_x = if transformed_width > 0.0 {
        image_width / transformed_width
    } else {
        1.0
    };
    let fit_y = if transformed_height > 0.0 {
        image_height / transformed_height
    } else {
        1.0
    };
    let fit_scale = fit_x.min(fit_y).clamp(0.1, 1.0);

    AvatarTransform {
        rotation_radians: radians,
        scale_x: edit.transform.stretch_x
            * fit_scale
            * if edit.transform.flip_x { -1.0 } else { 1.0 },
        scale_y: edit.transform.stretch_y
            * fit_scale
            * if edit.transform.flip_y { -1.0 } else { 1.0 },
    }
}

#[cfg(any(target_arch = "wasm32", test))]
fn apply_avatar_color(red: u8, green: u8, blue: u8, edit: &RecipeImageEdit) -> (u8, u8, u8) {
    let apply_contrast = |channel: u8, contrast: f64| {
        (channel as f64 * contrast.max(0.0))
            .round()
            .clamp(0.0, 255.0)
    };
    let red = apply_contrast(red, edit.color.contrast_r);
    let green = apply_contrast(green, edit.color.contrast_g);
    let blue = apply_contrast(blue, edit.color.contrast_b);
    let saturation = edit.color.saturation.max(0.0);
    let luminance = red * 0.2126 + green * 0.7152 + blue * 0.0722;
    let apply_saturation = |channel: f64| {
        (luminance + (channel - luminance) * saturation)
            .round()
            .clamp(0.0, 255.0) as u8
    };

    (
        apply_saturation(red),
        apply_saturation(green),
        apply_saturation(blue),
    )
}

#[cfg(test)]
mod avatar_crop_tests {
    use super::{apply_avatar_color, avatar_source_rect, avatar_transform};
    use crate::types::{ImageCropRect, RecipeImageEdit};

    #[test]
    fn avatar_source_rect_applies_the_selected_normalized_crop() {
        let mut edit = RecipeImageEdit::default_ellipse_1x1();
        edit.crop = ImageCropRect {
            x: 0.25,
            y: 0.1,
            width: 0.5,
            height: 0.8,
        };

        let source = avatar_source_rect(&edit, 800.0, 600.0);

        assert_eq!(source.x, 200.0);
        assert_eq!(source.y, 60.0);
        assert_eq!(source.width, 400.0);
        assert_eq!(source.height, 480.0);
    }

    #[test]
    fn avatar_source_rect_clamps_crop_to_image_bounds() {
        let mut edit = RecipeImageEdit::default_ellipse_1x1();
        edit.crop = ImageCropRect {
            x: 0.9,
            y: -0.5,
            width: 0.4,
            height: 2.0,
        };

        let source = avatar_source_rect(&edit, 100.0, 200.0);

        assert_eq!(source.x, 60.0);
        assert_eq!(source.y, 0.0);
        assert_eq!(source.width, 40.0);
        assert_eq!(source.height, 200.0);
    }

    #[test]
    fn avatar_transform_applies_rotation_stretch_and_flips() {
        let mut edit = RecipeImageEdit::default_ellipse_1x1();
        edit.transform.rotation = 90.0;
        edit.transform.stretch_x = 0.75;
        edit.transform.stretch_y = 0.5;
        edit.transform.flip_x = true;

        let transform = avatar_transform(&edit, 800.0, 600.0);

        assert!((transform.rotation_radians - std::f64::consts::FRAC_PI_2).abs() < 0.0001);
        assert_eq!(transform.scale_x, -0.75);
        assert_eq!(transform.scale_y, 0.5);
    }

    #[test]
    fn avatar_transform_fits_a_square_rotated_45_degrees() {
        let mut edit = RecipeImageEdit::default_ellipse_1x1();
        edit.transform.rotation = 45.0;

        let transform = avatar_transform(&edit, 400.0, 400.0);

        let expected = std::f64::consts::FRAC_1_SQRT_2;
        assert!((transform.scale_x - expected).abs() < 0.0001);
        assert!((transform.scale_y - expected).abs() < 0.0001);
    }

    #[test]
    fn avatar_color_applies_saturation_and_channel_contrast() {
        let mut edit = RecipeImageEdit::default_ellipse_1x1();
        edit.color.saturation = 0.0;
        edit.color.contrast_r = 0.5;
        edit.color.contrast_g = 1.0;
        edit.color.contrast_b = 1.5;

        assert_eq!(apply_avatar_color(100, 150, 200, &edit), (136, 136, 136));
    }
}

/// Processed avatar result: full-size (400x400) and thumbnail (100x100) WebP bytes.
#[cfg(target_arch = "wasm32")]
struct ProcessedAvatar {
    /// Full-size circular WebP bytes (400x400)
    full_bytes: Vec<u8>,
    /// Circular thumbnail WebP bytes (100x100)
    thumb_bytes: Vec<u8>,
}

#[cfg(target_arch = "wasm32")]
async fn render_avatar_variant(
    document: &web_sys::Document,
    img: &web_sys::HtmlImageElement,
    source: AvatarSourceRect,
    edit: &RecipeImageEdit,
    size: u32,
    quality: f64,
) -> Result<Vec<u8>, String> {
    use wasm_bindgen::{Clamped, JsCast};

    let canvas = document
        .create_element("canvas")
        .map_err(|e| format!("Failed to create canvas: {:?}", e))?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| "Failed to cast to HtmlCanvasElement")?;
    canvas.set_width(size);
    canvas.set_height(size);

    let ctx = canvas
        .get_context("2d")
        .map_err(|e| format!("Failed to get 2D context: {:?}", e))?
        .and_then(|value| value.dyn_into::<web_sys::CanvasRenderingContext2d>().ok())
        .ok_or("Failed to cast context")?;
    let half_size = size as f64 / 2.0;

    ctx.begin_path();
    ctx.arc(half_size, half_size, half_size, 0.0, std::f64::consts::TAU)
        .map_err(|e| format!("Failed to create avatar mask: {:?}", e))?;
    ctx.clip();

    let transform = avatar_transform(
        edit,
        img.natural_width() as f64,
        img.natural_height() as f64,
    );
    ctx.save();
    ctx.translate(half_size, half_size)
        .map_err(|e| format!("Failed to position avatar: {:?}", e))?;
    ctx.rotate(transform.rotation_radians)
        .map_err(|e| format!("Failed to rotate avatar: {:?}", e))?;
    ctx.scale(transform.scale_x, transform.scale_y)
        .map_err(|e| format!("Failed to scale avatar: {:?}", e))?;
    let draw_result = ctx
        .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
            img,
            source.x,
            source.y,
            source.width,
            source.height,
            -half_size,
            -half_size,
            size as f64,
            size as f64,
        );
    ctx.restore();
    draw_result.map_err(|e| format!("draw_image error: {:?}", e))?;

    let image_data = ctx
        .get_image_data(0.0, 0.0, size as f64, size as f64)
        .map_err(|e| format!("Failed to read avatar pixels: {:?}", e))?;
    let mut pixels = image_data.data().0;
    for pixel in pixels.chunks_exact_mut(4) {
        let (red, green, blue) = apply_avatar_color(pixel[0], pixel[1], pixel[2], edit);
        pixel[0] = red;
        pixel[1] = green;
        pixel[2] = blue;
    }
    let adjusted = web_sys::ImageData::new_with_u8_clamped_array_and_sh(
        Clamped(pixels.as_slice()),
        size,
        size,
    )
    .map_err(|e| format!("Failed to create adjusted avatar pixels: {:?}", e))?;
    ctx.put_image_data(&adjusted, 0.0, 0.0)
        .map_err(|e| format!("Failed to write avatar pixels: {:?}", e))?;

    crate::utils::image_processing_client::blob_from_canvas(&canvas, quality).await
}

/// Process an avatar image client-side using the editor's selected crop and
/// generate transparent circular 400px and 100px WebP variants.
///
/// Only available on wasm32 target.
#[cfg(target_arch = "wasm32")]
async fn process_avatar_image(
    image_blob: web_sys::Blob,
    edit: &RecipeImageEdit,
) -> Result<ProcessedAvatar, String> {
    let window = web_sys::window().ok_or("No window")?;
    let document = window.document().ok_or("No document")?;

    // Load the image using the existing utility
    let img = crate::utils::image_processing_client::load_image_from_blob(&image_blob).await?;

    let source = avatar_source_rect(
        edit,
        img.natural_width() as f64,
        img.natural_height() as f64,
    );
    if source.width <= 0.0 || source.height <= 0.0 {
        return Err("Selected crop is empty".to_string());
    }

    let full_bytes = render_avatar_variant(&document, &img, source, edit, 400, 0.9).await?;
    let thumb_bytes = render_avatar_variant(&document, &img, source, edit, 100, 0.8).await?;

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

#[cfg(target_arch = "wasm32")]
fn reset_file_input(id: &str) {
    if let Some(input) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.query_selector(id).ok().flatten())
        .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
    {
        input.set_value("");
    }
}

/// No-op stub for non-wasm32 targets (SSR).
#[cfg(not(target_arch = "wasm32"))]
fn trigger_file_input(_id: &str) {
    // No DOM available during server-side rendering
}

#[cfg(not(target_arch = "wasm32"))]
fn reset_file_input(_id: &str) {}

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
    let mut avatar_editor_url = use_signal(|| Option::<String>::None);
    let mut avatar_editor_initial_edit = use_signal(|| Option::<RecipeImageEdit>::None);
    #[cfg(target_arch = "wasm32")]
    let mut avatar_editor_blob = use_signal(|| Option::<web_sys::Blob>::None);

    // Revoke a selected file's preview URL if the settings page unmounts
    // while its editor is open.
    #[cfg(target_arch = "wasm32")]
    {
        use dioxus::prelude::use_drop;
        use_drop(move || {
            if let Some(url) = avatar_editor_url() {
                let _ = web_sys::Url::revoke_object_url(&url);
            }
        });
    }

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
                    // Account deletion invalidates the database session. POST logout as
                    // a final browser-side cleanup so the host-only cookie is removed too.
                    #[cfg(target_arch = "wasm32")]
                    {
                        let _ = gloo_net::http::Request::post("/auth/logout?redirect_uri=/")
                            .send()
                            .await;
                        if let Some(window) = web_sys::window() {
                            let _ = window.location().set_href("/");
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

    // Selecting an avatar opens the editor. Processing and upload only begin
    // after the user confirms their crop.
    #[allow(unused_mut)]
    let mut on_avatar_file_change = move |files: Vec<dioxus::html::FileData>| {
        if files.is_empty() {
            return;
        }

        #[cfg(target_arch = "wasm32")]
        {
            use dioxus_web::WebFileExt;

            error.set(None);
            saved_message.set(None);

            let Some(web_file) = files[0].get_web_file() else {
                error.set(Some("Could not read file".to_string()));
                return;
            };

            let object_url = match web_sys::Url::create_object_url_with_blob(&web_file) {
                Ok(url) => url,
                Err(_) => {
                    error.set(Some("Could not preview the selected image".to_string()));
                    return;
                }
            };

            if let Some(previous_url) = avatar_editor_url() {
                let _ = web_sys::Url::revoke_object_url(&previous_url);
            }
            avatar_editor_blob.set(Some(web_file.into()));
            avatar_editor_initial_edit.set(Some(RecipeImageEdit::default_ellipse_1x1()));
            avatar_editor_url.set(Some(object_url));
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = &files;
        }
    };

    let on_edit_existing_avatar = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            spawn(async move {
                error.set(None);
                let source = match get_avatar_edit_source().await {
                    Ok(Some(source)) => source,
                    Ok(None) => {
                        error.set(Some("No avatar is available to edit".to_string()));
                        return;
                    }
                    Err(_) => {
                        error.set(Some("Could not load the saved avatar edit".to_string()));
                        return;
                    }
                };
                let response = match gloo_net::http::Request::get(&source.original_url)
                    .send()
                    .await
                {
                    Ok(response) if response.ok() => response,
                    Ok(response) => {
                        error.set(Some(format!(
                            "Could not load the current avatar (HTTP {})",
                            response.status()
                        )));
                        return;
                    }
                    Err(_) => {
                        error.set(Some(
                            "Could not load the current avatar for editing".to_string(),
                        ));
                        return;
                    }
                };
                let bytes = match response.binary().await {
                    Ok(bytes) => bytes,
                    Err(_) => {
                        error.set(Some("Could not read the current avatar".to_string()));
                        return;
                    }
                };
                let byte_array = js_sys::Uint8Array::from(bytes.as_slice());
                let parts = js_sys::Array::new();
                parts.push(&byte_array);
                let blob = match web_sys::Blob::new_with_u8_array_sequence(&parts) {
                    Ok(blob) => blob,
                    Err(_) => {
                        error.set(Some(
                            "Could not prepare the current avatar for editing".to_string(),
                        ));
                        return;
                    }
                };
                let object_url = match web_sys::Url::create_object_url_with_blob(&blob) {
                    Ok(url) => url,
                    Err(_) => {
                        error.set(Some("Could not preview the current avatar".to_string()));
                        return;
                    }
                };

                if let Some(previous_url) = avatar_editor_url() {
                    let _ = web_sys::Url::revoke_object_url(&previous_url);
                }
                avatar_editor_blob.set(Some(blob));
                avatar_editor_initial_edit.set(Some(source.edit));
                avatar_editor_url.set(Some(object_url));
            });
        }
    };

    let on_avatar_editor_close = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(url) = avatar_editor_url() {
                let _ = web_sys::Url::revoke_object_url(&url);
            }
            avatar_editor_blob.set(None);
        }
        avatar_editor_initial_edit.set(None);
        avatar_editor_url.set(None);
        reset_file_input("#avatar-upload-input");
    };

    let on_avatar_edit_save = move |edit: RecipeImageEdit| {
        #[cfg(target_arch = "wasm32")]
        {
            let Some(image_blob) = avatar_editor_blob() else {
                error.set(Some(
                    "The selected image is no longer available".to_string(),
                ));
                return;
            };
            let mut auth_context = auth_context;
            let mut avatar_url = avatar_url_signal;
            let mut avatar_uploading = avatar_uploading;
            let mut error = error;
            let mut saved_message = saved_message;

            spawn(async move {
                avatar_uploading.set(true);
                error.set(None);

                let original_bytes =
                    match crate::utils::image_processing_client::blob_to_bytes(&image_blob).await {
                        Ok(bytes) => bytes,
                        Err(e) => {
                            error.set(Some(format!("Failed to read original image: {e}")));
                            avatar_uploading.set(false);
                            return;
                        }
                    };

                let processed = match process_avatar_image(image_blob, &edit).await {
                    Ok(data) => data,
                    Err(e) => {
                        error.set(Some(format!("Failed to process image: {e}")));
                        avatar_uploading.set(false);
                        return;
                    }
                };

                match upload_avatar(
                    original_bytes,
                    processed.full_bytes,
                    processed.thumb_bytes,
                    edit,
                )
                .await
                {
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
            let _ = edit;
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
                                        onclick: on_edit_existing_avatar,
                                        "{avatar_photo_actions(true)[0]}"
                                    }
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
            ImageEditorModal {
                image_url: avatar_editor_url,
                crop_mode: ImageEditorCropMode::Ellipse1x1,
                initial_edit: avatar_editor_initial_edit(),
                on_save: on_avatar_edit_save,
                on_close: on_avatar_editor_close,
            }

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

#[cfg(all(feature = "server", test))]
mod avatar_media_ownership_tests {
    use super::{delete_owned_avatar_objects, persist_uploaded_avatar, validate_avatar_payload};
    use crate::types::RecipeImageEdit;

    const VALID_WEBP_BYTES: &[u8] = b"RIFF\x24\x00\x00\x00WEBP";

    #[test]
    fn avatar_payload_rejects_every_empty_variant() {
        let edit = RecipeImageEdit::default_ellipse_1x1();
        for result in [
            validate_avatar_payload(&[], VALID_WEBP_BYTES, VALID_WEBP_BYTES, &edit),
            validate_avatar_payload(b"original", &[], VALID_WEBP_BYTES, &edit),
            validate_avatar_payload(b"original", VALID_WEBP_BYTES, &[], &edit),
        ] {
            assert_eq!(
                result.unwrap_err(),
                "Avatar image variants must not be empty"
            );
        }
        assert!(
            validate_avatar_payload(b"original", VALID_WEBP_BYTES, VALID_WEBP_BYTES, &edit).is_ok()
        );
    }

    #[test]
    fn avatar_payload_rejects_oversized_edit_metadata() {
        let mut edit = RecipeImageEdit::default_ellipse_1x1();
        edit.name = "e".repeat(crate::security::MAX_IMAGE_EDIT_NAME_CHARS + 1);
        assert!(
            validate_avatar_payload(b"original", VALID_WEBP_BYTES, VALID_WEBP_BYTES, &edit,)
                .is_err()
        );
    }

    #[test]
    fn avatar_payload_preserves_format_and_size_error_contracts() {
        let edit = RecipeImageEdit::default_ellipse_1x1();
        assert_eq!(
            validate_avatar_payload(b"original", b"not webp", VALID_WEBP_BYTES, &edit).unwrap_err(),
            "Invalid image format: must be WebP"
        );

        let mut oversized = vec![0_u8; 2 * 1024 * 1024 + 1];
        oversized[0..4].copy_from_slice(b"RIFF");
        oversized[8..12].copy_from_slice(b"WEBP");
        assert_eq!(
            validate_avatar_payload(b"original", &oversized, VALID_WEBP_BYTES, &edit).unwrap_err(),
            format!(
                "Image too large: {} bytes (max {} bytes)",
                oversized.len(),
                2 * 1024 * 1024
            )
        );
    }

    #[tokio::test]
    async fn avatar_cleanup_deletes_only_exact_owned_avatar_keys() {
        let owner_id = uuid::Uuid::new_v4();
        let victim_id = uuid::Uuid::new_v4();
        let owned_object = uuid::Uuid::new_v4();
        let victim_object = uuid::Uuid::new_v4();
        let mock_server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("DELETE"))
            .respond_with(wiremock::ResponseTemplate::new(204))
            .expect(3)
            .mount(&mock_server)
            .await;
        let storage = crate::storage::StorageClient::for_test_with_endpoint_and_public_url(
            &mock_server.uri(),
            "https://cdn.example.com/media",
        );
        let owned_full = storage.object_url(&format!("avatars/{owner_id}/{owned_object}.webp"));
        let owned_original =
            storage.object_url(&format!("avatars/{owner_id}/{owned_object}-original"));
        let victim_full = storage.object_url(&format!("avatars/{victim_id}/{victim_object}.webp"));
        let victim_original =
            storage.object_url(&format!("avatars/{victim_id}/{victim_object}-original"));

        delete_owned_avatar_objects(
            &storage,
            owner_id,
            Some(&victim_full),
            Some(&victim_original),
        )
        .await;
        delete_owned_avatar_objects(&storage, owner_id, Some(&owned_full), Some(&owned_original))
            .await;

        let requests = mock_server.received_requests().await.unwrap();
        let deleted_paths: Vec<_> = requests
            .iter()
            .map(|request| request.url.path().to_string())
            .collect();
        assert_eq!(requests.len(), 3);
        assert!(deleted_paths
            .iter()
            .any(|path| path.ends_with(&format!("/{owned_object}.webp"))));
        assert!(deleted_paths
            .iter()
            .any(|path| path.ends_with(&format!("/{owned_object}-thumb.webp"))));
        assert!(deleted_paths
            .iter()
            .any(|path| path.ends_with(&format!("/{owned_object}-original"))));
        assert!(deleted_paths
            .iter()
            .all(|path| !path.contains(&victim_id.to_string())));
    }

    #[tokio::test]
    async fn avatar_database_failure_compensates_new_objects_and_preserves_previous_state() {
        let (_database, pool) = crate::test_utils::setup_test_db().await;
        let user = crate::db::insert_user(
            &pool,
            "avatar_db_failure",
            "Avatar DB Failure",
            "avatar-db-failure@example.com",
            None,
        )
        .await
        .unwrap();
        let owner_id = user.id;
        let previous_object = uuid::Uuid::new_v4();
        let previous_full =
            format!("https://cdn.example.com/media/avatars/{owner_id}/{previous_object}.webp");
        let previous_original =
            format!("https://cdn.example.com/media/avatars/{owner_id}/{previous_object}-original");
        sqlx::query("UPDATE users SET avatar_url = $2, avatar_original_url = $3 WHERE id = $1")
            .bind(owner_id)
            .bind(&previous_full)
            .bind(&previous_original)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "CREATE FUNCTION reject_avatar_update() RETURNS trigger AS $$ \
             BEGIN RAISE EXCEPTION 'injected avatar update failure'; END; \
             $$ LANGUAGE plpgsql",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "CREATE TRIGGER reject_avatar_update_trigger BEFORE UPDATE OF avatar_url ON users \
             FOR EACH ROW EXECUTE FUNCTION reject_avatar_update()",
        )
        .execute(&pool)
        .await
        .unwrap();

        let mock_server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("PUT"))
            .respond_with(wiremock::ResponseTemplate::new(200))
            .expect(3)
            .mount(&mock_server)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("DELETE"))
            .respond_with(wiremock::ResponseTemplate::new(204))
            .expect(3)
            .mount(&mock_server)
            .await;
        let storage = crate::storage::StorageClient::for_test_with_endpoint_and_public_url(
            &mock_server.uri(),
            "https://cdn.example.com/media",
        );
        let request = crate::api::upload::MediaUploadRequest::avatar(
            b"original image bytes",
            VALID_WEBP_BYTES,
            VALID_WEBP_BYTES,
        )
        .unwrap();
        let uploaded =
            crate::api::upload::upload_media_transaction_for_test(&storage, owner_id, request)
                .await
                .unwrap();

        let result =
            persist_uploaded_avatar(&pool, &storage, owner_id, uploaded, serde_json::json!({}))
                .await;
        assert!(result.is_err());

        let preserved: (Option<String>, Option<String>) =
            sqlx::query_as("SELECT avatar_url, avatar_original_url FROM users WHERE id = $1")
                .bind(owner_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(preserved.0.as_deref(), Some(previous_full.as_str()));
        assert_eq!(preserved.1.as_deref(), Some(previous_original.as_str()));
        let requests = mock_server.received_requests().await.unwrap();
        let deleted_paths: Vec<_> = requests
            .iter()
            .filter(|request| request.method.as_str() == "DELETE")
            .map(|request| request.url.path().to_string())
            .collect();
        assert_eq!(deleted_paths.len(), 3);
        assert!(deleted_paths
            .iter()
            .all(|path| !path.contains(&previous_object.to_string())));
    }
}
