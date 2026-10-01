use crate::components::base::ButtonVariant;
use dioxus::prelude::*;

#[derive(Clone, Copy)]
pub(super) enum SettingsMutation {
    Unlink,
    RemoveAvatar,
    DeleteAccount,
}

/// A failed response does not prove a mutation rolled back. Only known domain
/// errors are shown verbatim; transport/database details never reach the UI.
pub(super) fn settings_mutation_feedback(
    action: SettingsMutation,
    error: &ServerFnError,
) -> &'static str {
    if let ServerFnError::ServerError { message, .. } = error {
        if message == "Not authenticated" {
            return "Sign in again in another tab, then check your account before retrying. Keep this page open to retain your entries and confirmation.";
        }
        if matches!(action, SettingsMutation::Unlink)
            && message == "You must have at least one linked account"
        {
            return "Keep at least one sign-in method linked. Check your linked accounts before trying again.";
        }
    }
    match action {
        SettingsMutation::Unlink => "Could not confirm unlinking. The account may already be unlinked; check your linked accounts in another tab before retrying.",
        SettingsMutation::RemoveAvatar => "Could not confirm photo removal. It may already have been removed; check your profile in another tab before retrying. Your entries and selected image edits are kept here.",
        SettingsMutation::DeleteAccount => "Could not confirm account deletion. Your account may already have been deleted; check whether you can still sign in in another tab before retrying. Keep this confirmation open.",
    }
}

/// Scoped Settings actions retain one focusable node during requests. Callers
/// additionally claim live pending state before starting asynchronous work.
#[component]
pub(super) fn SettingsMutationControl(
    id: String,
    variant: ButtonVariant,
    pending: bool,
    #[props(default)] unavailable: bool,
    label: String,
    pending_label: String,
    on_action: EventHandler<()>,
) -> Element {
    let variant_class = match variant {
        ButtonVariant::Primary => "btn-primary",
        ButtonVariant::Secondary => "btn-secondary",
        ButtonVariant::Ghost => "btn-ghost",
        ButtonVariant::Danger => "btn-danger",
    };
    rsx! {
        button {
            id, r#type: "button", class: "btn {variant_class} touch-target",
            aria_disabled: pending || unavailable, aria_busy: pending,
            onclick: move |_| { if !pending && !unavailable { on_action.call(()); } },
            if pending { "{pending_label}" } else { "{label}" }
        }
    }
}

#[cfg(test)]
#[path = "mutation_recovery_tests.rs"]
mod tests;
