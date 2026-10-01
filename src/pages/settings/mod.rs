mod account_delete_flow;
mod mutation_recovery;
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
        #[route("/accounts")]
        SettingsAccounts {},
    }

    #[component]
    fn TestRoot(authenticated: bool, accounts: bool) -> Element {
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
                history: move |_| std::rc::Rc::new(dioxus::history::MemoryHistory::with_initial_path(if accounts { "/accounts" } else { "/" })) as std::rc::Rc<dyn History>,
                Router::<TestRoute> {}
            }
        }
    }

    fn render(authenticated: bool) -> String {
        let mut dom = VirtualDom::new_with_props(
            TestRoot,
            TestRootProps {
                authenticated,
                accounts: false,
            },
        );
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
        assert!(html.contains("value=\"Test Cook\""), "{html}");
        assert!(html.contains("value=\"test-cook\""), "{html}");
        assert_eq!(html.matches("readonly=true").count(), 3, "{html}");
    }

    #[test]
    fn signed_out_settings_do_not_render_private_fields_or_actions() {
        let html = render(false);
        assert!(html.contains("Sign in to continue"), "{html}");
        assert!(!html.contains("private@example.test"));
        assert!(!html.contains("profile-display-name"));
        assert!(!html.contains("Delete Account"));
    }

    #[test]
    fn linked_accounts_loading_has_named_status_and_guest_hides_private_actions() {
        for authenticated in [true, false] {
            let mut dom = VirtualDom::new_with_props(
                TestRoot,
                TestRootProps {
                    authenticated,
                    accounts: true,
                },
            );
            dom.rebuild_in_place();
            let html = dioxus_ssr::render(&dom);
            if authenticated {
                assert!(html.contains("Loading linked accounts…"), "{html}");
                assert!(html.contains("role=\"status\""), "{html}");
                assert!(!html.contains("Connect Google"), "{html}");
                assert!(!html.contains("Connect GitHub"), "{html}");
            } else {
                assert!(html.contains("Sign in to continue"), "{html}");
                assert!(!html.contains("Connected accounts are private"), "{html}");
                assert!(!html.contains("Connect GitHub"), "{html}");
            }
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn linked_accounts_failed_resource_renders_safe_retry_not_connect_options() {
        // No FullstackContext/session: the real read fails before touching DB.
        let mut dom = VirtualDom::new_with_props(
            TestRoot,
            TestRootProps {
                authenticated: true,
                accounts: true,
            },
        );
        dom.rebuild_in_place();
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                dom.wait_for_work().await;
                dom.render_immediate(&mut NoOpMutations);
                let html = dioxus_ssr::render(&dom);
                if html.contains("Retry linked accounts") {
                    assert!(
                        html.contains("Your linked accounts couldn’t be loaded. Please try again."),
                        "{html}"
                    );
                    assert!(html.contains("role=\"alert\""), "{html}");
                    assert!(html.contains("aria-disabled=false"), "{html}");
                    assert!(!html.contains("Not authenticated"), "{html}");
                    assert!(!html.contains("Connect Google"), "{html}");
                    assert!(!html.contains("Connect GitHub"), "{html}");
                    break;
                }
            }
        })
        .await
        .expect("failed account resource must settle into safe recovery");
    }
}
