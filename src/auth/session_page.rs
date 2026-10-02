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

/// Section links change focus/scroll position, not the form being edited.
/// Ignore fragments and the empty query delimiter SSR can retain during
/// hydration, but keep nonempty queries (such as a collection parent) distinct.
fn retained_form_visit_key(path: &str) -> String {
    let before_fragment = path
        .split_once('#')
        .map_or(path, |(before_fragment, _)| before_fragment);
    match before_fragment.split_once('?') {
        Some((path, "")) => path,
        _ => before_fragment,
    }
    .to_string()
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
        let visit_key = retained_form_visit_key(&path);
        rsx! {
            for visit in [path] {
                RetainedFormPage { key: "form-{visit_key}", path: visit, Outlet::<Route> {} }
            }
        }
    } else {
        // Cancel old resources and remount every nested private/read-only view
        // together with its profile links. Profile-only updates do not reset it.
        rsx! { ReadOnlySessionScope { identity, Outlet::<Route> {} } }
    }
}

#[component]
fn ReadOnlySessionScope(identity: Option<uuid::Uuid>, children: Element) -> Element {
    // Dioxus keys reset scopes in a keyed fragment, not on a single static
    // component slot. The one-item list is intentional lifecycle ownership.
    rsx! {
        for account in [identity] {
            AccountReadContents { key: "account-{account:?}", children: children.clone() }
        }
    }
}

#[component]
fn AccountReadContents(children: Element) -> Element {
    rsx! { {children} }
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
        // Dioxus's client interpreter does not remove inert=false. Omit the
        // attribute completely when enabled, including after account recovery.
        div { id: "session-retained-page", hidden: blocked, inert: blocked.then_some(true), tabindex: "-1", {children} }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::context::{AuthContext, UserProfile};
    use crate::components::AuthRequired;
    use dioxus::router::RouterContext;
    use dioxus_core::{current_scope_id, AttributeValue, ElementId};
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

    #[component]
    fn ReadOwnerProbe(mounts: Rc<Cell<usize>>) -> Element {
        let auth = use_auth();
        use_hook(move || mounts.set(mounts.get() + 1));
        let data_owner = use_signal(move || auth.current_user_id);
        rsx! { p { "data-owner-{data_owner():?}" } }
    }

    #[component]
    fn ReadTransitionRoot(
        control: Rc<RefCell<Option<Signal<AuthContext>>>>,
        mounts: Rc<Cell<usize>>,
    ) -> Element {
        let auth = use_signal(|| identity(1));
        use_hook(move || *control.borrow_mut() = Some(auth));
        use_context_provider(|| auth);
        rsx! { ReadOnlySessionScope { identity: auth().current_user_id, ReadOwnerProbe { mounts } } }
    }

    type RouteControl = Rc<RefCell<Option<(RouterContext, ScopeId)>>>;

    #[derive(Clone)]
    struct ObservedRoute(Route);

    impl std::str::FromStr for ObservedRoute {
        type Err = <Route as std::str::FromStr>::Err;

        fn from_str(path: &str) -> Result<Self, Self::Err> {
            path.parse().map(Self)
        }
    }

    impl std::fmt::Display for ObservedRoute {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.0.fmt(formatter)
        }
    }

    impl Routable for ObservedRoute {
        const SITE_MAP: &'static [dioxus::router::routable::SiteMapSegment] = Route::SITE_MAP;

        fn render(&self, level: usize) -> Element {
            let contents = self.0.render(level);
            rsx! { RouteObserver {} {contents} }
        }
    }

    #[component]
    fn RouteObserver() -> Element {
        let control = use_context::<RouteControl>();
        let current = router();
        let scope = current_scope_id();
        use_hook(move || *control.borrow_mut() = Some((current, scope)));
        rsx! {}
    }

    #[derive(Props, Clone)]
    struct RoutedSessionRootProps {
        initial_path: String,
        control: RouteControl,
    }

    impl PartialEq for RoutedSessionRootProps {
        fn eq(&self, other: &Self) -> bool {
            self.initial_path == other.initial_path && Rc::ptr_eq(&self.control, &other.control)
        }
    }

    #[allow(non_snake_case)]
    fn RoutedSessionRoot(props: RoutedSessionRootProps) -> Element {
        let RoutedSessionRootProps {
            initial_path,
            control,
        } = props;
        use_context_provider(|| Signal::new(identity(1)));
        use_context_provider(|| control);
        rsx! { dioxus::router::components::HistoryProvider {
            history: move |_| Rc::new(dioxus::history::MemoryHistory::with_initial_path(&initial_path)) as Rc<dyn History>,
            Router::<ObservedRoute> {}
        } }
    }

    fn collection_name_input_assignment(edits: &[dioxus_core::Mutation]) -> Option<ElementId> {
        // The name attribute is static template data; the dynamic default
        // value identifies this input in actual renderer mutations.
        edits.iter().find_map(|edit| match edit {
            dioxus_core::Mutation::SetAttribute {
                name: "value",
                value: AttributeValue::Text(value),
                id,
                ..
            } if value == "Untitled Collection" => Some(*id),
            _ => None,
        })
    }

    #[test]
    fn session_startup_empty_query_keeps_the_actual_collection_input_node() {
        let control = Rc::new(RefCell::new(None));
        let mut dom = VirtualDom::new_with_props(
            RoutedSessionRoot,
            RoutedSessionRootProps {
                initial_path: "/collections/new?".into(),
                control: control.clone(),
            },
        );
        let initial = dom.rebuild_to_vec();
        let original_input =
            collection_name_input_assignment(&initial.edits).expect("collection input");
        let (current, scope) = control.borrow().unwrap();
        dom.in_scope(scope, || current.replace("/collections/new"));
        let mut edits = dioxus_core::Mutations::default();
        dom.render_immediate(&mut edits);
        assert_eq!(
            collection_name_input_assignment(&edits.edits),
            None,
            "Startup replaced the original input {original_input:?}"
        );
        assert!(dioxus_ssr::render(&dom).contains("name=\"collection_name\""));

        // A real parent change is a new editor, not an empty-query spelling change.
        dom.in_scope(scope, || {
            current.replace("/collections/new?parent=550e8400-e29b-41d4-a716-446655440000")
        });
        let mut edits = dioxus_core::Mutations::default();
        dom.render_immediate(&mut edits);
        let replacement_input =
            collection_name_input_assignment(&edits.edits).expect("new parent form");
        assert_ne!(original_input, replacement_input);
    }

    #[test]
    fn section_navigation_keeps_the_actual_form_input_until_the_query_changes() {
        use dioxus::html::{SerializedFormData, SerializedHtmlEventConverter};
        set_event_converter(Box::new(SerializedHtmlEventConverter));
        let control = Rc::new(RefCell::new(None));
        let mut dom = VirtualDom::new_with_props(
            RoutedSessionRoot,
            RoutedSessionRootProps {
                initial_path: "/collections/new".into(),
                control: control.clone(),
            },
        );
        let initial = dom.rebuild_to_vec();
        let original_input = collection_name_input_assignment(&initial.edits).unwrap();
        dom.runtime().handle_event(
            "input",
            Event::new(
                Rc::new(PlatformEventData::new(Box::new(SerializedFormData::new(
                    "Local section navigation test".into(),
                    Vec::new(),
                )))) as Rc<dyn std::any::Any>,
                true,
            ),
            original_input,
        );
        dom.render_immediate(&mut dioxus_core::NoOpMutations);
        assert!(dioxus_ssr::render(&dom).contains("Local section navigation test"));
        for path in [
            "/collections/new#cover",
            "/collections/new#notes",
            "/collections/new",
        ] {
            let (current, scope) = control.borrow().unwrap();
            dom.in_scope(scope, || current.push(path));
            let mut edits = dioxus_core::Mutations::default();
            dom.render_immediate(&mut edits);
            assert_eq!(
                collection_name_input_assignment(&edits.edits),
                None,
                "Section navigation to {path} replaced the original input {original_input:?}"
            );
            assert!(dioxus_ssr::render(&dom).contains("Local section navigation test"));
        }

        for forward in [false, true] {
            let (current, scope) = control.borrow().unwrap();
            dom.in_scope(scope, || {
                if forward {
                    current.go_forward();
                } else {
                    current.go_back();
                }
            });
            let mut edits = dioxus_core::Mutations::default();
            dom.render_immediate(&mut edits);
            assert_eq!(collection_name_input_assignment(&edits.edits), None);
            assert!(dioxus_ssr::render(&dom).contains("Local section navigation test"));
        }

        let (current, scope) = control.borrow().unwrap();
        dom.in_scope(scope, || {
            current.push("/collections/new?parent=550e8400-e29b-41d4-a716-446655440000#cover")
        });
        let mut edits = dioxus_core::Mutations::default();
        dom.render_immediate(&mut edits);
        assert!(
            collection_name_input_assignment(&edits.edits).is_some(),
            "A different parent query must still create a new form"
        );
        assert!(!dioxus_ssr::render(&dom).contains("Local section navigation test"));
    }

    #[test]
    fn active_editor_inert_mutations_are_absent_not_false() {
        let control = Rc::new(RefCell::new(None));
        let mounts = Rc::new(Cell::new(0));
        let mut dom = VirtualDom::new_with_props(
            TransitionRoot,
            TransitionRootProps {
                control: control.clone(),
                mounts: mounts.clone(),
            },
        );
        let initial = dom.rebuild_to_vec();
        let assert_presence_only = |edits: &[dioxus_core::Mutation]| {
            for edit in edits {
                if let dioxus_core::Mutation::SetAttribute {
                    name: "inert",
                    value,
                    ..
                } = edit
                {
                    assert!(
                        matches!(value, AttributeValue::None | AttributeValue::Bool(true)),
                        "The browser treats inert=false as inert: {edit:?}"
                    );
                }
            }
        };
        assert_presence_only(&initial.edits);
        let mut auth = control.borrow().unwrap();
        for current in [AuthContext::default(), identity(2), identity(1)] {
            dom.in_scope(ScopeId::ROOT, || auth.set(current));
            let mut edits = dioxus_core::Mutations::default();
            dom.render_immediate(&mut edits);
            assert_presence_only(&edits.edits);
        }
        assert_eq!(mounts.get(), 1);
    }

    #[test]
    fn retained_form_keys_ignore_sections_but_preserve_nonempty_queries() {
        for path in [
            "/collections/new",
            "/collections/new?",
            "/collections/new#",
            "/collections/new?#",
        ] {
            assert_eq!(retained_form_visit_key(path), "/collections/new");
        }
        assert_eq!(
            retained_form_visit_key("/collections/new?#cover"),
            "/collections/new"
        );
        for path in [
            "/collections/new?parent=550e8400-e29b-41d4-a716-446655440000",
            "/collections/new?note=what?",
            "/collections/new?note=%3F%23&tag=湯麵",
            "/settings/accounts?link_error=denied",
        ] {
            assert_eq!(retained_form_visit_key(path), path);
        }
        for (path, expected) in [
            ("/recipes/new#recipe-photos", "/recipes/new"),
            ("/recipes/new#recipe-notes", "/recipes/new"),
            ("/recipes/new#", "/recipes/new"),
            ("/recipes/new?#recipe-photos", "/recipes/new"),
            ("/settings/profile#avatar", "/settings/profile"),
            (
                "/collections/new?note=%3F%23&tag=湯麵#cover",
                "/collections/new?note=%3F%23&tag=湯麵",
            ),
            (
                "/collections/new?parent=one#cover",
                "/collections/new?parent=one",
            ),
            (
                "/collections/new?parent=two#cover",
                "/collections/new?parent=two",
            ),
        ] {
            assert_eq!(retained_form_visit_key(path), expected);
        }
        assert_ne!(
            retained_form_visit_key("/recipes/new#recipe-photos"),
            retained_form_visit_key("/collections/new#cover")
        );
    }

    #[test]
    fn session_read_scope_really_remounts_cached_data_on_identity_change() {
        let control = Rc::new(RefCell::new(None));
        let mounts = Rc::new(Cell::new(0));
        let mut dom = VirtualDom::new_with_props(
            ReadTransitionRoot,
            ReadTransitionRootProps {
                control: control.clone(),
                mounts: mounts.clone(),
            },
        );
        dom.rebuild_in_place();
        let mut auth = control.borrow().unwrap();
        for (expected_mounts, current) in [
            (2, identity(2)),
            (3, AuthContext::default()),
            (4, identity(1)),
        ] {
            let expected_owner = current.current_user_id;
            dom.in_scope(ScopeId::ROOT, || auth.set(current));
            dom.render_immediate(&mut dioxus_core::NoOpMutations);
            assert_eq!(
                mounts.get(),
                expected_mounts,
                "Changing a single component key is not enough to reset cached read resources"
            );
            assert!(dioxus_ssr::render(&dom).contains(&format!("data-owner-{expected_owner:?}")));
        }
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
        assert!(source.contains("inert: blocked.then_some(true)"));
        assert!(source.contains("target: \"_blank\""));
        assert!(!source.contains("location().reload"));
        let css = include_str!("../../assets/main.css");
        assert!(css.contains("#session-retained-page[hidden] dialog"));
        assert!(css.contains("#session-retained-page[hidden] [popover]"));
    }
}
