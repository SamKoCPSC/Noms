use crate::auth::context::AuthContext;
use dioxus::prelude::*;

/// A failed refresh is not evidence of sign-out. Keep the last context until
/// a valid response arrives; preserve the recovery control through completion.
#[derive(Default)]
struct AuthRefreshState {
    pending: bool,
    failed: bool,
    recovered: bool,
}

impl AuthRefreshState {
    fn visible(&self) -> bool {
        self.failed || self.recovered
    }

    fn begin(&mut self) -> bool {
        if self.pending {
            return false;
        }
        self.pending = true;
        true
    }

    fn finish(
        &mut self,
        started: &AuthContext,
        current: &AuthContext,
        result: Result<AuthContext, ()>,
    ) -> Option<AuthContext> {
        if !self.pending {
            return None;
        }
        self.pending = false;
        // Settings can update identity/profile while this request is in flight.
        // A stale response must not replace those authoritative local updates.
        if started != current {
            self.failed = true;
            return None;
        }
        match result {
            Ok(context) => {
                self.recovered = self.visible();
                self.failed = false;
                Some(context)
            }
            Err(()) => {
                self.failed = true;
                None
            }
        }
    }

    fn dismiss(&mut self) {
        if !self.pending && !self.failed {
            self.recovered = false;
        }
    }
}

/// Treat malformed/inconsistent payloads like a failed check, never like a
/// confirmed guest response. Do not expose endpoint/transport bodies in the UI.
#[cfg(any(target_arch = "wasm32", test))]
fn parse_auth_refresh(body: &str) -> Result<AuthContext, ()> {
    let context: AuthContext = serde_json::from_str(body).map_err(|_| ())?;
    let valid = if context.is_authenticated {
        matches!((&context.current_user_id, &context.current_user), (Some(id), Some(profile)) if *id == profile.id)
    } else {
        context.current_user_id.is_none() && context.current_user.is_none()
    };
    valid.then_some(context).ok_or(())
}

#[cfg(target_arch = "wasm32")]
async fn fetch_auth_refresh() -> Result<AuthContext, ()> {
    let response = gloo_net::http::Request::get("/api/user_profile")
        .send()
        .await
        .map_err(|_| ())?;
    if !response.ok() {
        return Err(());
    }
    parse_auth_refresh(&response.text().await.map_err(|_| ())?)
}

#[component]
fn AuthRefreshFeedback(pending: bool, failed: bool, on_action: EventHandler<()>) -> Element {
    rsx! {
        section {
            class: "container", aria_label: "Sign-in status check",
            padding_top: "var(--space-md)", padding_bottom: "var(--space-md)",
            div {
                class: "neumo-raised", padding: "var(--space-md)",
                border_radius: "var(--radius-lg)", overflow_wrap: "anywhere",
                display: "flex", flex_wrap: "wrap", align_items: "center", gap: "var(--space-md)",
                p {
                    role: if failed && !pending { "alert" } else { "status" },
                    flex: "1 1 240px", margin: "0",
                    if pending { "Checking sign-in status…" }
                    else if failed { "We couldn’t check your sign-in status. Your current page is kept open; retry before relying on account actions." }
                    else { "Sign-in status checked." }
                }
                // One node across failure/pending/completion: no focus loss when
                // a retry completes, and no automatic page reload or dismissal.
                button {
                    id: "auth-refresh-action", r#type: "button",
                    class: "btn btn-secondary touch-target", aria_disabled: pending,
                    aria_busy: pending,
                    onclick: move |_| { if !pending { on_action.call(()); } },
                    if pending { "Checking…" } else if failed { "Retry sign-in check" } else { "Dismiss" }
                }
            }
        }
    }
}

/// Mounted once in the shared layout. SSR retains its request-authenticated
/// context; only the hydrated client performs the background session check.
#[component]
pub fn AuthRefresh() -> Element {
    #[allow(unused_mut)]
    let mut auth = use_context::<Signal<AuthContext>>();
    let mut state = use_signal(AuthRefreshState::default);
    let refresh = use_callback(move |_: ()| {
        if !state.write().begin() {
            return;
        }
        let started = auth.peek().clone();
        spawn(async move {
            #[cfg(target_arch = "wasm32")]
            let result = fetch_auth_refresh().await;
            #[cfg(not(target_arch = "wasm32"))]
            let result = Err(());
            let current = auth.peek().clone();
            if let Some(context) = state.write().finish(&started, &current, result) {
                auth.set(context);
            }
        });
    });
    #[cfg(target_arch = "wasm32")]
    use_hook(move || refresh.call(()));

    rsx! {
        if state.read().visible() {
            AuthRefreshFeedback {
                pending: state.read().pending, failed: state.read().failed,
                on_action: move |_| {
                    if state.peek().pending { return; }
                    if state.peek().failed {
                        refresh.call(());
                    } else {
                        state.write().dismiss();
                        // Explicit dismissal returns focus to a visible shared
                        // navigation control on every viewport and route.
                        #[cfg(target_arch = "wasm32")]
                        {
                            use wasm_bindgen::JsCast;
                            if let Some(button) = web_sys::window()
                                .and_then(|window| window.document())
                                .and_then(|document| document.get_element_by_id("navigation-menu-trigger"))
                                .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
                            {
                                let _ = button.focus();
                            }
                        }
                    }
                },
            }
        }
    }
}

#[cfg(test)]
#[path = "refresh_tests.rs"]
mod refresh_tests;
