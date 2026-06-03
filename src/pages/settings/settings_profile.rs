use dioxus::prelude::*;

use crate::auth::context::{use_auth, AuthContext, UserProfile};
use crate::components::base::{
    Button, ButtonVariant, Card, Input, PageHeader, SettingsTab, SettingsTabs,
};

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
                    if let Some(window) = web_sys::window() {
                        let _ = window.location().set_href("/auth/logout");
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

    rsx! {
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
