use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::auth::context::AuthUser;
use crate::auth::context::{use_auth, UserProfile};
use crate::components::base::{Button, ButtonVariant, Card, Input, PageHeader};

/// Save user profile via server function.
#[server]
pub async fn save_profile(
    display_name: String,
    bio: Option<String>,
) -> Result<UserProfile, ServerFnError> {
    use dioxus::fullstack::FullstackContext;
    use dioxus::server::axum::Extension;
    use sqlx::PgPool;

    let Extension(AuthUser { user_id }): Extension<AuthUser> = FullstackContext::extract().await?;
    let Extension(pool): Extension<PgPool> = FullstackContext::extract().await?;

    let trimmed_name = display_name.trim().to_string();
    if trimmed_name.len() < 2 || trimmed_name.len() > 30 {
        return Err(ServerFnError::new("Display name must be 2-30 characters"));
    }
    let trimmed_bio = bio.map(|b| b.trim().to_string());
    if let Some(ref b) = trimmed_bio {
        if b.len() > 160 {
            return Err(ServerFnError::new("Bio must be 160 characters or less"));
        }
    }

    let updated =
        crate::db::update_user_profile(&pool, user_id, &trimmed_name, trimmed_bio.as_deref())
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(UserProfile {
        id: updated.id,
        username: updated.username,
        display_name: updated.display_name,
        avatar_url: updated.avatar_url,
        bio: updated.bio,
    })
}

/// User profile settings page.
///
/// Displays editable display name and bio fields with validation,
/// optimistic UI updates, and rollback on failure.
#[component]
pub fn SettingsProfile() -> Element {
    let auth = use_auth();

    let mut display_name = use_signal(String::new);
    let mut bio = use_signal(String::new);
    let mut is_saving = use_signal(|| false);
    let mut error = use_signal(|| Option::<String>::None);
    let mut saved_message = use_signal(|| Option::<String>::None);

    // Load profile from auth context on mount (use_hook runs once, not every render)
    use_hook(move || {
        if let Some(user) = &auth.current_user {
            display_name.set(user.display_name.clone());
            bio.set(user.bio.clone().unwrap_or_default());
        }
    });

    let on_save = move |_| {
        let new_name = display_name().clone();
        let new_bio = bio().clone();
        let trimmed_name = new_name.trim().to_string();
        let trimmed_bio = if new_bio.trim().is_empty() {
            None
        } else {
            Some(new_bio.trim().to_string())
        };

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

        // Optimistic update: save old values for rollback, apply trimmed values immediately
        let old_name = display_name().clone();
        let old_bio = bio().clone();

        display_name.set(trimmed_name.clone());
        bio.set(trimmed_bio.clone().unwrap_or_default());

        is_saving.set(true);
        error.set(None);

        // Spawn async save
        let name_for_server = trimmed_name.clone();
        let bio_for_server = trimmed_bio.clone();
        spawn(async move {
            match save_profile(name_for_server, bio_for_server).await {
                Ok(profile) => {
                    // Apply server-authoritative values
                    display_name.set(profile.display_name);
                    bio.set(profile.bio.unwrap_or_default());
                    saved_message.set(Some("Profile saved successfully!".to_string()));
                    is_saving.set(false);
                }
                Err(e) => {
                    // Rollback to old values
                    display_name.set(old_name);
                    bio.set(old_bio);
                    error.set(Some(e.to_string()));
                    is_saving.set(false);
                }
            }
        });
    };

    let is_valid = display_name().trim().len() >= 2
        && display_name().trim().len() <= 30
        && bio().trim().len() <= 160;

    rsx! {
        div { class: "container",
            PageHeader {
                title: "Profile Settings",
            }
            div {
                max_width: "500px",
                Card {
                    div {
                        display: "flex",
                        flex_direction: "column",
                        gap: "var(--space-md)",
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
        }
    }
}
