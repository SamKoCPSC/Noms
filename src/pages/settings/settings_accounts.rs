use chrono::DateTime;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::context::use_auth;
use crate::components::base::{
    Button, ButtonVariant, Card, EmptyState, LoadingSpinner, PageHeader,
};

// ── Serializable response type ───────────────────────────────────────────────

/// A linked OAuth account, serializable for server functions.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LinkedAccount {
    pub id: Uuid,
    pub provider: String,
    pub email: Option<String>,
    pub last_used_at: DateTime<chrono::Utc>,
}

// ── Server functions ─────────────────────────────────────────────────────────

/// Fetch all OAuth accounts linked to the current user.
#[server]
pub async fn get_linked_accounts() -> Result<Vec<LinkedAccount>, ServerFnError> {
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let pool = crate::db::get_pool();

    let rows = crate::db::get_oauth_accounts_by_user(&pool, user_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(rows
        .into_iter()
        .map(|row| LinkedAccount {
            id: row.id,
            provider: row.provider,
            email: row.email,
            last_used_at: row.last_used_at,
        })
        .collect())
}

/// Unlink (delete) a single OAuth account for the current user.
///
/// Returns an error if the account doesn't belong to the user, or if it
/// would be the last remaining linked account.
#[server]
pub async fn unlink_account(account_id: Uuid) -> Result<(), ServerFnError> {
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let pool = crate::db::get_pool();

    // Guard: must have at least one account remaining after deletion
    let count = crate::db::count_oauth_accounts(&pool, user_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    if count <= 1 {
        return Err(ServerFnError::new(
            "You must have at least one linked account".to_string(),
        ));
    }

    crate::db::delete_oauth_account(&pool, account_id, user_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(())
}

// ── Component ────────────────────────────────────────────────────────────────

/// Linked OAuth accounts settings page.
///
/// Displays all OAuth accounts linked to the current user with the ability
/// to unlink them (with confirmation) and connect new providers.
#[component]
pub fn SettingsAccounts() -> Element {
    let _auth = use_auth();

    // Resource returns Option<Vec<LinkedAccount>> (None if server function errored)
    let accounts = use_resource(move || async move { get_linked_accounts().await.ok() });

    let mut unlinking_id = use_signal(|| Option::<Uuid>::None);
    let mut error = use_signal(|| Option::<String>::None);
    let mut success = use_signal(|| Option::<String>::None);
    let mut show_confirm = use_signal(|| Option::<(Uuid, String)>::None);

    // Build OAuth connect URLs
    let google_url = "/auth/google/start?redirect_uri=/settings/accounts";
    let github_url = "/auth/github/start?redirect_uri=/settings/accounts";

    // Format a relative time string for last used date
    let format_date = |dt: DateTime<chrono::Utc>| -> String {
        let now = chrono::Utc::now();
        let diff = now.signed_duration_since(dt);
        if diff.num_days() > 365 {
            format!("{} years ago", diff.num_days() / 365)
        } else if diff.num_days() > 30 {
            format!("{} months ago", diff.num_days() / 30)
        } else if diff.num_days() > 0 {
            format!("{} days ago", diff.num_days())
        } else if diff.num_hours() > 0 {
            format!("{} hours ago", diff.num_hours())
        } else if diff.num_minutes() > 0 {
            format!("{} minutes ago", diff.num_minutes())
        } else {
            "Just now".to_string()
        }
    };

    // Provider display helpers
    let provider_label = |p: &str| -> String {
        match p.to_lowercase().as_str() {
            "google" => "Google".to_string(),
            "github" => "GitHub".to_string(),
            other => other.to_string(),
        }
    };

    let provider_icon = |p: &str| -> &'static str {
        match p.to_lowercase().as_str() {
            "google" => "G",
            "github" => "⌨",
            _ => "●",
        }
    };

    let mut on_request_unlink = move |account_id: Uuid, provider: String| {
        error.set(None);
        success.set(None);
        show_confirm.set(Some((account_id, provider)));
    };

    let mut on_confirm_unlink = move |_| {
        let Some((account_id, provider)) = show_confirm().clone() else {
            return;
        };
        show_confirm.set(None);
        unlinking_id.set(Some(account_id));
        error.set(None);

        let mut accounts_clone = accounts;
        spawn(async move {
            match unlink_account(account_id).await {
                Ok(()) => {
                    success.set(Some(format!("{} account unlinked successfully", provider)));
                    // Restart the resource to refetch
                    accounts_clone.restart();
                }
                Err(e) => {
                    error.set(Some(e.to_string()));
                }
            }
            unlinking_id.set(None);
        });
    };

    let mut on_cancel_unlink = move |_| {
        show_confirm.set(None);
    };

    // Extract data from resource before rendering to avoid borrow issues in rsx!
    let is_pending = accounts.pending();
    let accounts_data: Option<Vec<LinkedAccount>> = accounts.read().clone().flatten();

    // Determine which providers are already linked
    let linked_providers: std::collections::HashSet<String> = accounts_data
        .as_ref()
        .map(|accs| accs.iter().map(|a| a.provider.to_lowercase()).collect())
        .unwrap_or_default();

    // Build the main content
    let content = if is_pending {
        rsx! {
            div {
                display: "flex",
                justify_content: "center",
                padding: "var(--space-2xl)",
                LoadingSpinner {}
            }
        }
    } else if let Some(accs) = accounts_data {
        if accs.is_empty() {
            // No accounts — show empty state with connect buttons
            rsx! {
                EmptyState {
                    icon: rsx! { "🔗" },
                    title: "No linked accounts",
                    description: "Connect your Google or GitHub account for easy sign-in.",
                    action: rsx! {
                        div {
                            display: "flex",
                            gap: "var(--space-md)",
                            flex_wrap: "wrap",
                            justify_content: "center",
                            a {
                                href: "{google_url}",
                                class: "btn btn-secondary touch-target",
                                "Connect Google"
                            }
                            a {
                                href: "{github_url}",
                                class: "btn btn-secondary touch-target",
                                "Connect GitHub"
                            }
                        }
                    },
                }
            }
        } else {
            // Has accounts — show list + connect buttons for missing providers
            // Clone accounts for owned iteration in rsx!
            let owned_accounts = accs.clone();
            rsx! {
                div {
                    display: "flex",
                    flex_direction: "column",
                    gap: "var(--space-md)",
                    // Linked account cards
                    for account in owned_accounts.into_iter() {
                        Card {
                            div {
                                display: "flex",
                                align_items: "center",
                                justify_content: "space-between",
                                gap: "var(--space-md)",
                                div {
                                    display: "flex",
                                    align_items: "center",
                                    gap: "var(--space-md)",
                                    // Provider icon
                                    div {
                                        width: "40px",
                                        height: "40px",
                                        border_radius: "50%",
                                        background_color: "var(--accent-bg)",
                                        color: "var(--accent)",
                                        display: "flex",
                                        align_items: "center",
                                        justify_content: "center",
                                        font_weight: "bold",
                                        font_size: "18px",
                                        flex_shrink: "0",
                                        "{provider_icon(&account.provider)}"
                                    }
                                    // Account details
                                    div {
                                        div {
                                            font_weight: "600",
                                            font_size: "15px",
                                            color: "var(--text-primary)",
                                            "{provider_label(&account.provider)}"
                                        }
                                        if let Some(ref email) = account.email {
                                            p {
                                                font_size: "14px",
                                                color: "var(--text-secondary)",
                                                "{email}"
                                            }
                                        }
                                        span {
                                            font_size: "12px",
                                            color: "var(--text-tertiary)",
                                            "Last used: {format_date(account.last_used_at)}"
                                        }
                                    }
                                }
                                // Unlink button
                                Button {
                                    variant: ButtonVariant::Danger,
                                    disabled: unlinking_id().is_some(),
                                    onclick: move |_| {
                                        on_request_unlink(
                                            account.id,
                                            provider_label(&account.provider),
                                        );
                                    },
                                    "Unlink"
                                }
                            }
                        }
                    }

                    // Connect additional providers
                    div {
                        margin_top: "var(--space-md)",
                        border_top: "1px solid var(--border)",
                        padding_top: "var(--space-md)",
                        h3 {
                            font_size: "14px",
                            font_weight: "600",
                            color: "var(--text-secondary)",
                            margin_bottom: "var(--space-sm)",
                            "Connect additional accounts"
                        }
                        div {
                            display: "flex",
                            gap: "var(--space-md)",
                            flex_wrap: "wrap",
                            if !linked_providers.contains("google") {
                                a {
                                    href: "{google_url}",
                                    class: "btn btn-secondary touch-target",
                                    "Connect Google"
                                }
                            }
                            if !linked_providers.contains("github") {
                                a {
                                    href: "{github_url}",
                                    class: "btn btn-secondary touch-target",
                                    "Connect GitHub"
                                }
                            }
                        }
                    }
                }
            }
        }
    } else {
        // Fetch completed but returned None (error)
        rsx! {
            div {
                padding: "var(--space-sm) var(--space-md)",
                background_color: "var(--error-bg)",
                border_radius: "var(--radius-md)",
                color: "var(--error)",
                font_size: "14px",
                "Failed to load linked accounts."
            }
        }
    };

    rsx! {
        div { class: "container",
            PageHeader {
                title: "Linked Accounts",
            }

            // Error message
            if let Some(err) = error() {
                div {
                    padding: "var(--space-sm) var(--space-md)",
                    background_color: "var(--error-bg)",
                    border_radius: "var(--radius-md)",
                    color: "var(--error)",
                    font_size: "14px",
                    margin_bottom: "var(--space-md)",
                    "{err}"
                }
            }

            // Success message
            if let Some(msg) = success() {
                div {
                    padding: "var(--space-sm) var(--space-md)",
                    background_color: "var(--success-bg)",
                    border_radius: "var(--radius-md)",
                    color: "var(--success)",
                    font_size: "14px",
                    margin_bottom: "var(--space-md)",
                    "{msg}"
                }
            }

            {content}

            // Confirmation modal
            if let Some((confirm_id, confirm_provider)) = show_confirm() {
                div {
                    position: "fixed",
                    top: "0",
                    left: "0",
                    right: "0",
                    bottom: "0",
                    background_color: "rgba(0, 0, 0, 0.5)",
                    display: "flex",
                    align_items: "center",
                    justify_content: "center",
                    z_index: "1000",
                    onclick: move |_| {
                        on_cancel_unlink(());
                    },
                    div {
                        class: "neumo-card",
                        padding: "var(--space-xl)",
                        max_width: "400px",
                        width: "90%",
                        background_color: "var(--surface)",
                        onclick: move |evt| {
                            evt.stop_propagation();
                        },
                        h3 {
                            margin_bottom: "var(--space-sm)",
                            "Unlink {confirm_provider}?"
                        }
                        p {
                            font_size: "14px",
                            color: "var(--text-secondary)",
                            margin_bottom: "var(--space-lg)",
                            "You will need to sign in again with your remaining accounts."
                        }
                        div {
                            display: "flex",
                            gap: "var(--space-sm)",
                            justify_content: "flex-end",
                            Button {
                                variant: ButtonVariant::Secondary,
                                onclick: move |_| {
                                    on_cancel_unlink(());
                                },
                                "Cancel"
                            }
                            Button {
                                variant: ButtonVariant::Danger,
                                disabled: unlinking_id().is_some(),
                                onclick: move |_| {
                                    on_confirm_unlink(());
                                },
                                if unlinking_id() == Some(confirm_id) {
                                    "Unlinking..."
                                } else {
                                    "Unlink"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
