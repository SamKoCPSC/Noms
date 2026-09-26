mod account_delete_flow;
mod settings_accounts;
mod settings_profile;

pub use settings_accounts::SettingsAccounts;
pub use settings_profile::SettingsProfile;

#[cfg(test)]
mod presentation_tests {
    use super::*;
    use crate::auth::context::{AuthContext, UserProfile};
    use dioxus::prelude::*;
    use dioxus::router::components::HistoryProvider;
    use dioxus_core::NoOpMutations;

    #[derive(Clone, PartialEq, Routable)]
    enum TestRoute {
        #[route("/")]
        SettingsProfile {},
    }

    #[component]
    fn TestRoot(authenticated: bool) -> Element {
        let context = use_signal(move || AuthContext {
            is_authenticated: authenticated,
            current_user: Some(UserProfile {
                id: uuid::Uuid::nil(),
                username: "test-cook".into(),
                display_name: "Test Cook".into(),
                email: "private@example.test".into(),
                avatar_url: None,
                bio: None,
            }),
            ..Default::default()
        });
        use_context_provider(|| context);
        rsx! {
            HistoryProvider {
                history: |_| std::rc::Rc::new(dioxus::history::MemoryHistory::with_initial_path("/")) as std::rc::Rc<dyn History>,
                Router::<TestRoute> {}
            }
        }
    }

    fn render(authenticated: bool) -> String {
        let mut dom = VirtualDom::new_with_props(TestRoot, TestRootProps { authenticated });
        dom.rebuild(&mut NoOpMutations);
        dioxus_ssr::render(&dom)
    }

    #[test]
    fn profile_fields_have_labels_and_deletion_is_closed_by_default() {
        let html = render(true);
        for id in ["profile-display-name", "profile-username", "profile-bio"] {
            assert!(html.contains(&format!("for=\"{id}\"")), "{html}");
            assert!(html.contains(&format!("id=\"{id}\"")), "{html}");
        }
        assert!(html.contains("aria-current=\"page\""), "{html}");
        assert!(html.contains("href=\"/settings/accounts\""), "{html}");
        assert!(html.contains("href=\"/u/test-cook\""), "{html}");
        let deletion = html.split("user-settings__danger").nth(1).unwrap();
        assert!(!deletion.split('>').next().unwrap().contains("open"));
        assert!(html.contains("Not shown on your public profile."));
    }

    #[test]
    fn signed_out_settings_do_not_render_private_fields_or_actions() {
        let html = render(false);
        assert!(html.contains("Sign in to continue"), "{html}");
        assert!(!html.contains("private@example.test"));
        assert!(!html.contains("profile-display-name"));
        assert!(!html.contains("Delete Account"));
    }
}
