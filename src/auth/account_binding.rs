//! Bind account-scoped forms to their initiating identity, not a replaceable cookie.
use dioxus::prelude::*;
use uuid::Uuid;

pub const ACCOUNT_CHANGED_MESSAGE: &str = "Account changed";
pub const ACCOUNT_CHANGED_FEEDBACK: &str = "Your signed-in account changed. Nothing was saved by this request. Your entries and image edits are kept here. Sign in to the original account in another tab before retrying, or open a fresh page for the new account.";
pub const ACCOUNT_COMPLETION_CHANGED_FEEDBACK: &str = "The request completed for the original account, but this page's account changed while it was running. Your local entries are kept here. Check the original account in another tab before continuing.";

/// The expected ID is only a consistency check: authority always comes from
/// the verified request session. Reject before budgets, storage or database writes.
pub(crate) fn check_account_binding(
    expected: Option<Uuid>,
    verified: Option<Uuid>,
) -> Result<Uuid, ServerFnError> {
    let verified = verified.ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    if expected != Some(verified) {
        return Err(ServerFnError::new(ACCOUNT_CHANGED_MESSAGE));
    }
    Ok(verified)
}

#[cfg(feature = "server")]
pub(crate) async fn require_form_account(expected: Option<Uuid>) -> Result<Uuid, ServerFnError> {
    check_account_binding(
        expected,
        super::session::extract_user_id_from_fullstack().await,
    )
}

fn latch_form_account(binding: &mut Option<Uuid>, current: Option<Uuid>) {
    if binding.is_none() {
        *binding = current;
    }
}

/// Capture once when authentication becomes available. Refreshing authentication
/// must never silently reassign fields or selected uploads to another account.
pub(crate) fn use_form_account() -> Signal<Option<Uuid>> {
    let auth = use_context::<Signal<super::context::AuthContext>>();
    let initial = auth
        .read()
        .is_authenticated
        .then_some(auth.read().current_user_id)
        .flatten();
    let mut binding = use_signal(move || initial);
    use_effect(move || {
        let current = auth.read();
        if binding.peek().is_none() && current.is_authenticated {
            let mut latched = *binding.peek();
            latch_form_account(&mut latched, current.current_user_id);
            if latched.is_some() {
                binding.set(latched);
            }
        }
    });
    binding
}

pub(crate) fn account_completion_matches(
    initiating: Option<Uuid>,
    current: Option<Uuid>,
    returned: Uuid,
) -> bool {
    initiating == Some(returned) && current == initiating
}

pub(crate) fn account_error_feedback(error: &ServerFnError) -> Option<&'static str> {
    matches!(error, ServerFnError::ServerError { message, .. } if message == ACCOUNT_CHANGED_MESSAGE)
        .then_some(ACCOUNT_CHANGED_FEEDBACK)
}

#[cfg(test)]
#[path = "account_binding_tests.rs"]
mod tests;
