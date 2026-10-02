//! Keep read views scoped to the current identity without discarding editor work.
use crate::auth::{account_binding::use_form_account, context::use_auth};
use crate::Route;
use dioxus::prelude::*;

fn retains_local_work(route: &Route) -> bool {
    matches!(
        route,
        Route::RecipeNew {}
            | Route::RecipeEdit { .. }
            | Route::RecipeDraftEdit { .. }
            | Route::CollectionNew { .. }
            | Route::CollectionEdit { .. }
            | Route::SettingsProfile {}
            | Route::SettingsAccounts {}
    )
}

#[component]
pub fn SessionPage() -> Element {
    let auth = use_auth();
    let route = use_route::<Route>();
    let path = router().full_route_string();
    let identity = auth
        .is_authenticated
        .then_some(auth.current_user_id)
        .flatten();
    if retains_local_work(&route) {
        rsx! { RetainedFormPage { key: "form-{path}", path, Outlet::<Route> {} } }
    } else {
        // Cancel old resources and remount every nested private/read-only view
        // together with its profile links. Profile-only updates do not reset it.
        rsx! { Outlet::<Route> { key: "account-{identity:?}" } }
    }
}

#[component]
fn RetainedFormPage(path: String, children: Element) -> Element {
    let binding = use_form_account();
    let auth = use_auth();
    let identity = auth
        .is_authenticated
        .then_some(auth.current_user_id)
        .flatten();
    let blocked = binding().is_some() && binding() != identity;
    #[cfg(target_arch = "wasm32")]
    {
        let mut previously_blocked = use_signal(|| false);
        use_effect(use_reactive(&blocked, move |blocked| {
            crate::components::base::set_form_modals_suspended("session-retained-page", blocked);
            use wasm_bindgen::JsCast;
            if let Some(document) = web_sys::window().and_then(|window| window.document()) {
                if blocked {
                    // Suspend native focus traps before focusing the visible notice.
                    if let Some(heading) = document
                        .get_element_by_id("session-page-notice")
                        .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
                    {
                        let _ = heading.focus();
                    }
                } else if *previously_blocked.peek()
                    && document
                        .query_selector("dialog[open]")
                        .ok()
                        .flatten()
                        .is_none()
                {
                    if let Some(page) = document
                        .get_element_by_id("session-retained-page")
                        .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
                    {
                        let _ = page.focus();
                    }
                }
            }
            previously_blocked.set(blocked);
        }));
    }
    rsx! {
        if blocked {
            section { class: "container", aria_label: "Account changed",
                padding_top: "var(--space-lg)", padding_bottom: "var(--space-lg)",
                div { class: "neumo-raised", padding: "var(--space-lg)", border_radius: "var(--radius-lg)", overflow_wrap: "anywhere",
                    h1 { id: "session-page-notice", tabindex: "-1", "Your signed-in account changed" }
                    p { role: "status", "This editor belongs to the original account. Your local entries and image edits are kept in this tab, but the editor is hidden to avoid mixing accounts." }
                    p { "Sign in to the original account in another tab, then return here to continue. You can also open a fresh page for the current account without closing this tab." }
                    a { href: path, target: "_blank", rel: "noopener noreferrer", class: "btn btn-primary touch-target", "Open current account in a new tab" }
                }
            }
        }
        // Keep the component and owned blobs alive. Hidden/inert also prevents
        // keyboard access; CSS explicitly hides top-layer dialogs/popovers.
        div { id: "session-retained-page", hidden: blocked, inert: blocked, tabindex: "-1", {children} }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::context::{AuthContext, UserProfile};
    use crate::components::AuthRequired;
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    fn identity(id: u128) -> AuthContext {
        let id = uuid::Uuid::from_u128(id);
        AuthContext {
            is_authenticated: true,
            current_user_id: Some(id),
            current_user: Some(UserProfile {
                id,
                username: format!("owner-{id}"),
                display_name: "Owner".into(),
                email: "local@example.test".into(),
                avatar_url: None,
                bio: None,
            }),
        }
    }

    #[component]
    fn LocalEntry(mounts: Rc<Cell<usize>>) -> Element {
        use_hook(move || mounts.set(mounts.get() + 1));
        let entry = use_signal(|| "Retained local entry".to_string());
        rsx! { input { value: entry() } }
    }

    #[component]
    fn TransitionRoot(
        control: Rc<RefCell<Option<Signal<AuthContext>>>>,
        mounts: Rc<Cell<usize>>,
    ) -> Element {
        let auth = use_signal(|| identity(1));
        use_hook(move || *control.borrow_mut() = Some(auth));
        use_context_provider(|| auth);
        rsx! { RetainedFormPage { path: "/settings/profile", AuthRequired { LocalEntry { mounts } } } }
    }

    #[test]
    fn session_page_keeps_one_form_mount_through_signout_replacement_and_recovery() {
        let control = Rc::new(RefCell::new(None));
        let mounts = Rc::new(Cell::new(0));
        let mut dom = VirtualDom::new_with_props(
            TransitionRoot,
            TransitionRootProps {
                control: control.clone(),
                mounts: mounts.clone(),
            },
        );
        dom.rebuild_in_place();
        assert_eq!(mounts.get(), 1);
        let mut auth = control.borrow().unwrap();
        for current in [AuthContext::default(), identity(2), identity(1)] {
            let blocked = current.current_user_id != Some(uuid::Uuid::from_u128(1));
            dom.in_scope(ScopeId::ROOT, || auth.set(current));
            for _ in 0..3 {
                dom.render_immediate(&mut dioxus_core::NoOpMutations);
            }
            let html = dioxus_ssr::render(&dom);
            assert_eq!(mounts.get(), 1, "Form was discarded: {html}");
            assert!(html.contains("Retained local entry"), "{html}");
            assert_eq!(
                html.contains("This editor belongs to the original account"),
                blocked,
                "{html}"
            );
            assert_eq!(
                html.contains("id=\"session-retained-page\" hidden=true inert=true"),
                blocked,
                "{html}"
            );
        }
    }

    #[test]
    fn session_page_retains_only_editor_and_account_forms() {
        for route in [
            Route::RecipeNew {},
            Route::RecipeEdit { id: "one".into() },
            Route::RecipeDraftEdit { id: "two".into() },
            Route::CollectionNew { parent: None },
            Route::CollectionEdit { id: "three".into() },
            Route::SettingsProfile {},
            Route::SettingsAccounts {},
        ] {
            assert!(retains_local_work(&route), "{route:?}");
        }
        for route in [
            Route::Dashboard {},
            Route::CollectionList {},
            Route::Search {
                query: Default::default(),
            },
            Route::RecipeDetail { id: "one".into() },
            Route::RecipeVersionGraph { id: "one".into() },
            Route::Explore {},
        ] {
            assert!(!retains_local_work(&route), "{route:?}");
        }
    }

    #[test]
    fn retained_page_is_inert_and_preserves_top_layer_editors_without_reload() {
        let source = include_str!("session_page.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap();
        assert!(source.contains("hidden: blocked"));
        assert!(source.contains("inert: blocked"));
        assert!(source.contains("target: \"_blank\""));
        assert!(!source.contains("location().reload"));
        let css = include_str!("../../assets/main.css");
        assert!(css.contains("#session-retained-page[hidden] dialog"));
        assert!(css.contains("#session-retained-page[hidden] [popover]"));
    }
}
