use crate::components::base::{PageIcon, PageIconKind};
use chrono::DateTime;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::mutation_recovery::{
    settings_mutation_feedback, SettingsMutation, SettingsMutationControl,
};
use crate::auth::context::{use_auth, AuthContext};
use crate::components::base::{
    ButtonVariant, Card, EmptyState, LoadRecovery, LoadingSpinner, SettingsTab, SettingsTabs,
    SharedModal,
};
use crate::components::AuthRequired;

#[cfg(test)]
#[path = "unlink_recovery_tests.rs"]
mod unlink_recovery_tests;

/// Extract a query parameter from the current URL.
///
/// On WASM, reads from `window.location.search`. On the server, reads
/// from the `FullstackContext` request URI.
fn extract_query_param(name: &str) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            if let Ok(search) = window.location().search() {
                // Parse query string manually (avoids url crate dependency)
                for param in search.trim_start_matches('?').split('&') {
                    if let Some(value) = param.strip_prefix(&format!("{name}=")) {
                        return Some(value.to_string());
                    }
                }
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Some(fsc) = dioxus_fullstack::FullstackContext::current() {
            if let Some(query) = fsc.parts_mut().uri.query() {
                for param in query.split('&') {
                    if let Some(value) = param.strip_prefix(&format!("{name}=")) {
                        return Some(value.to_string());
                    }
                }
            }
        }
    }
    None
}

// ── Serializable response type ───────────────────────────────────────────────

/// A linked OAuth account, serializable for server functions.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LinkedAccount {
    pub id: Uuid,
    pub provider: String,
    pub email: Option<String>,
    pub last_used_at: DateTime<chrono::Utc>,
}

/// Client confirmation/pending authority is separate from server ownership and
/// last-account checks. Keep the named dialog open until the outcome is known.
#[derive(Default)]
struct AccountUnlinkState {
    confirmation: Option<(Uuid, String)>,
    pending: bool,
    error: Option<String>,
    success: Option<String>,
}

impl AccountUnlinkState {
    fn open(&mut self, id: Uuid, provider: String) -> bool {
        if self.pending {
            return false;
        }
        self.confirmation = Some((id, provider));
        self.error = None;
        self.success = None;
        true
    }

    fn begin(&mut self) -> Option<(Uuid, String)> {
        if self.pending {
            return None;
        }
        let candidate = self.confirmation.clone()?;
        self.pending = true;
        self.error = None;
        Some(candidate)
    }

    fn cancel(&mut self) {
        if !self.pending {
            self.confirmation = None;
        }
    }

    fn finish(&mut self, result: Result<(), String>) {
        if !self.pending {
            return;
        }
        self.pending = false;
        match result {
            Ok(()) => {
                if let Some((_, provider)) = self.confirmation.take() {
                    self.success = Some(format!("{provider} account unlinked successfully"));
                }
            }
            Err(error) => self.error = Some(error),
        }
    }
}

fn can_unlink_account(accounts: &[LinkedAccount], id: Uuid) -> bool {
    accounts.len() > 1 && accounts.iter().any(|account| account.id == id)
}

fn oauth_link_feedback(error: Option<&str>, provider: Option<&str>) -> Option<String> {
    if error != Some("account_already_linked") {
        return None;
    }
    let provider = match provider {
        Some("google") => "Google",
        Some("github") => "GitHub",
        _ => "Sign-in provider",
    };
    Some(format!("This {provider} account is already linked to another user. Use a different provider account or keep your current sign-in methods."))
}

#[component]
fn AccountUnlinkDialog(
    provider: String,
    pending: bool,
    error: Option<String>,
    on_cancel: EventHandler<()>,
    on_confirm: EventHandler<()>,
) -> Element {
    rsx! {
        SharedModal {
            aria_labelledby: "unlink-account-title",
            focus_selector: "#unlink-account-cancel button", pending,
            on_dismiss: move |_| { if !pending { on_cancel.call(()); } },
            div {
                class: "modal-card", onclick: move |event| event.stop_propagation(),
                display: "flex", flex_direction: "column", gap: "var(--space-md)",
                h2 { id: "unlink-account-title", "Unlink {provider}?" }
                p { "You will need to sign in again with your remaining accounts." }
                if pending {
                    p { role: "status", "Unlinking {provider}…" }
                } else if let Some(error) = error {
                    p { role: "alert", overflow_wrap: "anywhere", "{error}" }
                    a { href: "/settings/accounts", target: "_blank", rel: "noopener", "Check linked accounts in another tab" }
                }
                div {
                    display: "flex", flex_wrap: "wrap", justify_content: "flex-end", gap: "var(--space-sm)",
                    span { id: "unlink-account-cancel",
                        SettingsMutationControl {
                            id: "unlink-account-cancel-button", variant: ButtonVariant::Secondary,
                            pending, label: "Cancel", pending_label: "Cancel", on_action: on_cancel,
                        }
                    }
                    SettingsMutationControl {
                        id: "unlink-account-confirm", variant: ButtonVariant::Danger,
                        pending, label: "Unlink", pending_label: "Unlinking…", on_action: on_confirm,
                    }
                }
            }
        }
    }
}

// ── Server functions ─────────────────────────────────────────────────────────

/// Fetch all OAuth accounts linked to the current user.
#[server]
pub async fn get_linked_accounts() -> Result<Vec<LinkedAccount>, ServerFnError> {
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
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
        .await
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
    let auth_context = use_context::<Signal<AuthContext>>();

    // Preserve failure while retrying so the focused recovery control remains
    // mounted. Subscribe to state explicitly for immediate pending feedback.
    let mut accounts = use_resource(get_linked_accounts);

    let mut unlink_state = use_signal(AccountUnlinkState::default);

    // Extract error params from URL for OAuth conflict notification
    let error_type = use_hook(|| extract_query_param("error"));
    let error_provider = use_hook(|| extract_query_param("provider"));
    let mut error =
        use_signal(move || oauth_link_feedback(error_type.as_deref(), error_provider.as_deref()));

    // Closure to clear error params from the URL (WASM only)
    let clear_error_params = move || {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(window) = web_sys::window() {
                if let Ok(history) = window.history() {
                    let _ = history.replace_state_with_url(
                        &wasm_bindgen::JsValue::NULL,
                        "",
                        Some("/settings/accounts"),
                    );
                }
            }
        }
    };

    // Build OAuth connect URLs
    let google_url = "/auth/google/start?redirect_uri=/settings/accounts&purpose=link";
    let github_url = "/auth/github/start?redirect_uri=/settings/accounts&purpose=link";

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
        if !auth_context.peek().is_authenticated
            || accounts.pending()
            || unlink_state.peek().pending
        {
            return;
        }
        let eligible = accounts
            .peek()
            .as_ref()
            .and_then(|result| result.as_ref().ok())
            .is_some_and(|rows| can_unlink_account(rows, account_id));
        if eligible && unlink_state.write().open(account_id, provider) {
            error.set(None);
        }
    };

    let on_confirm_unlink = move |_| {
        if !auth_context.peek().is_authenticated
            || accounts.pending()
            || unlink_state.peek().pending
        {
            return;
        }
        let Some((account_id, _)) = unlink_state.peek().confirmation.clone() else {
            return;
        };
        // Revalidate the current loaded list, not a captured last-account flag.
        let eligible = accounts
            .peek()
            .as_ref()
            .and_then(|result| result.as_ref().ok())
            .is_some_and(|rows| can_unlink_account(rows, account_id));
        if !eligible {
            unlink_state.write().error = Some("Your linked accounts changed. Cancel and reload the account list before trying again.".into());
            return;
        }
        if unlink_state.write().begin().is_none() {
            return;
        }
        error.set(None);
        spawn(async move {
            match unlink_account(account_id).await {
                Ok(()) => {
                    unlink_state.write().finish(Ok(()));
                    accounts.restart();
                }
                Err(e) => {
                    unlink_state.write().finish(Err(settings_mutation_feedback(
                        SettingsMutation::Unlink,
                        &e,
                    )
                    .into()));
                }
            }
        });
    };

    let on_cancel_unlink = move |_| unlink_state.write().cancel();

    // Extract data from resource before rendering to avoid borrow issues in rsx!
    let is_pending = *accounts.state().read() == UseResourceState::Pending;
    let accounts_result = accounts.read().clone();
    let accounts_data = accounts_result
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .cloned();

    // Determine which providers are already linked
    let linked_providers: std::collections::HashSet<String> = accounts_data
        .as_ref()
        .map(|accs| accs.iter().map(|a| a.provider.to_lowercase()).collect())
        .unwrap_or_default();

    // Build the main content
    let content = if is_pending && !matches!(accounts_result, Some(Err(_))) {
        rsx! {
            div {
                role: "status",
                display: "flex",
                justify_content: "center",
                padding: "var(--space-2xl)",
                LoadingSpinner {}
                span { "Loading linked accounts…" }
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
            let last_account = accs.len() == 1;
            rsx! {
                div { class: "user-accounts__list",
                    // Linked account cards
                    for account in owned_accounts.into_iter() {
                        Card {
                            div { class: "user-accounts__row",
                                div { class: "user-accounts__identity",
                                    // Provider icon
                                    div {
                                        width: "40px",
                                        height: "40px",
                                        border_radius: "50%",
                                        background_color: "var(--accent-bg)",
                                        color: "var(--accent-ink)",
                                        display: "flex",
                                        align_items: "center",
                                        justify_content: "center",
                                        font_weight: "bold",
                                        font_size: "18px",
                                        flex_shrink: "0",
                                        "{provider_icon(&account.provider)}"
                                    }
                                    // Account details
                                    div { class: "user-accounts__details",
                                        div { class: "user-accounts__provider",
                                            h2 { "{provider_label(&account.provider)}" }
                                            span { class: "user-accounts__status", "Connected" }
                                        }
                                        if let Some(ref email) = account.email {
                                            p {
                                                font_size: "14px",
                                                color: "var(--text-secondary)",
                                                "Associated email: {email}"
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
                                SettingsMutationControl {
                                    id: format!("unlink-account-{}", account.id),
                                    variant: ButtonVariant::Secondary,
                                    pending: unlink_state.read().pending,
                                    unavailable: last_account,
                                    label: "Unlink", pending_label: "Unlinking…",
                                    on_action: move |_| {
                                        on_request_unlink(
                                            account.id,
                                            provider_label(&account.provider),
                                        );
                                    },
                                }
                            }
                        }
                    }

                    // Connect additional providers
                    section { class: "user-accounts__connect",
                        h2 { "Sign-in options" }
                        if last_account {
                            p { "This is your only sign-in method. Connect another account before unlinking it." }
                        } else {
                            p { "Keep at least one sign-in method linked so you can always access your recipes." }
                        }
                        div { class: "user-accounts__connect-links",
                            if !linked_providers.contains("google") {
                                a {
                                    href: "{google_url}",
                                    class: "btn btn-secondary touch-target",
                                    aria_disabled: unlink_state.read().pending,
                                    onclick: move |event| { if unlink_state.peek().pending { event.prevent_default(); } },
                                    "Connect Google"
                                }
                            }
                            if !linked_providers.contains("github") {
                                a {
                                    href: "{github_url}",
                                    class: "btn btn-secondary touch-target",
                                    aria_disabled: unlink_state.read().pending,
                                    onclick: move |event| { if unlink_state.peek().pending { event.prevent_default(); } },
                                    "Connect GitHub"
                                }
                            }
                        }
                    }
                }
            }
        }
    } else {
        rsx! {
            LoadRecovery {
                pending: is_pending,
                message: "Your linked accounts couldn’t be loaded. Please try again.",
                retry_label: "Retry linked accounts",
                onretry: move |_| { if !accounts.pending() { accounts.restart(); } },
            }
        }
    };

    rsx! {
        document::Stylesheet { href: asset!("/assets/user-pages.css") }
        AuthRequired {
            div { class: "container user-settings user-accounts",
            header { class: "user-page-heading",
                p { class: "user-page-eyebrow", "Your account" }
                h1 { id: "linked-accounts-title", tabindex: "-1", class: "brand-title", PageIcon { kind: PageIconKind::Settings } span { "Linked accounts" } }
                p { "Choose how you sign in to Noms." }
            }
            SettingsTabs {
                active: SettingsTab::Accounts,
            }
            p { class: "user-accounts__guidance", "Connected accounts are private. Keep at least one sign-in method linked." }

            // Error message
            if let Some(err) = error().or_else(|| {
                let state = unlink_state.read();
                state.confirmation.is_none().then(|| state.error.clone()).flatten()
            }) {
                div {
                    role: "alert",
                    padding: "var(--space-sm) var(--space-md)",
                    background_color: "var(--error-bg)",
                    border_radius: "var(--radius-md)",
                    color: "var(--error-ink)",
                    font_size: "14px",
                    margin_bottom: "var(--space-md)",
                    display: "flex",
                    align_items: "flex-start",
                    gap: "var(--space-sm)",
                    span { flex: "1", "{err}" }
                    button {
                        aria_label: "Dismiss account error",
                        r#type: "button", class: "touch-target",
                        background: "transparent",
                        border: "none",
                        color: "var(--error-ink)",
                        cursor: "pointer",
                        font_size: "18px",
                        line_height: "1",
                        padding: "0",
                        margin_left: "auto",
                        flex_shrink: "0",
                        onclick: move |_| {
                            clear_error_params();
                            error.set(None);
                            unlink_state.write().error = None;
                            #[cfg(target_arch = "wasm32")]
                            {
                                use wasm_bindgen::JsCast;
                                if let Some(heading) = web_sys::window()
                                    .and_then(|window| window.document())
                                    .and_then(|document| document.get_element_by_id("linked-accounts-title"))
                                    .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
                                {
                                    let _ = heading.focus();
                                }
                            }
                        },
                        "×"
                    }
                }
            }

            // Success message
            if let Some(msg) = unlink_state.read().success.clone() {
                div {
                    role: "status",
                    padding: "var(--space-sm) var(--space-md)",
                    background_color: "var(--success-bg)",
                    border_radius: "var(--radius-md)",
                    color: "var(--success-ink)",
                    font_size: "14px",
                    margin_bottom: "var(--space-md)",
                    "{msg}"
                }
            }

            {content}

            // Confirmation modal
            if let Some((_, provider)) = unlink_state.read().confirmation.clone() {
                AccountUnlinkDialog {
                    provider, pending: unlink_state.read().pending,
                    error: unlink_state.read().error.clone(),
                    on_cancel: on_cancel_unlink, on_confirm: on_confirm_unlink,
                }
            }
        }
        }
    }
}
