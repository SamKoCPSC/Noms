use dioxus::prelude::*;

use crate::auth::context::{use_auth, UserProfile};
use crate::components::base::{Button, ButtonVariant, Card, Input, PageHeader};

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
    use dioxus::fullstack::FullstackContext;
    use dioxus::server::axum::Extension;
    use sqlx::PgPool;

    let fsc = FullstackContext::current().ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let (user_id, pool) = {
        let parts = fsc.parts_mut();
        let user_id = crate::auth::session::extract_user_id_from_headers(&parts.headers)
            .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
        let pool = parts
            .extensions
            .get::<Extension<PgPool>>()
            .ok_or_else(|| ServerFnError::new("Database pool not available"))?
            .0
            .clone();
        (user_id, pool)
    };

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
) -> Result<UserProfile, ServerFnError> {
    use dioxus::fullstack::FullstackContext;
    use dioxus::server::axum::Extension;
    use sqlx::PgPool;

    let fsc = FullstackContext::current().ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let user_id = {
        let parts = fsc.parts_mut();
        crate::auth::session::extract_user_id_from_headers(&parts.headers)
            .ok_or_else(|| ServerFnError::new("Not authenticated"))?
    };

    let pool = fsc
        .parts_mut()
        .extensions
        .get::<Extension<PgPool>>()
        .ok_or_else(|| ServerFnError::new("Database pool not available"))?
        .0
        .clone();

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

    // Delete account state
    let mut delete_step = use_signal(|| None::<DeleteStep>);
    let mut delete_input = use_signal(String::new);
    let mut deleting = use_signal(|| false);
    let mut delete_error = use_signal(|| Option::<String>::None);

    // Extract username for typed confirmation (before use_hook consumes auth.current_user)
    let username = auth
        .current_user
        .as_ref()
        .map(|u| u.username.clone())
        .unwrap_or_default();

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

    // Current username for typed confirmation
    let expected_confirm = format!("delete {}", username);
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
                    // Logout to clear session cookie, then hard-navigate home
                    let _ = gloo_net::http::Request::post("/auth/logout").send().await;
                    if let Some(window) = web_sys::window() {
                        let _ = window.location().set_href("/");
                    }
                }
                Err(e) => {
                    delete_error.set(Some(e.to_string()));
                    deleting.set(false);
                }
            }
        });
    };

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
                                    "`delete {username}`"
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
