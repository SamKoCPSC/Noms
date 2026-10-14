use dioxus::prelude::*;

use crate::Route;

/// Which settings sub-page is currently active.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum SettingsTab {
    #[default]
    Profile,
    Accounts,
}

/// Horizontal neumorphic tab bar for navigating between settings pages.
#[component]
pub fn SettingsTabs(active: SettingsTab) -> Element {
    rsx! {
        nav { class: "settings-tabs", aria_label: "Account settings",
            Link {
                to: Route::SettingsProfile {},
                class: if active == SettingsTab::Profile { "settings-tab settings-tab-active" } else { "settings-tab" },
                aria_current: if active == SettingsTab::Profile { Some("page") } else { None },
                "Profile"
            }
            Link {
                to: Route::SettingsAccounts {},
                class: if active == SettingsTab::Accounts { "settings-tab settings-tab-active" } else { "settings-tab" },
                aria_current: if active == SettingsTab::Accounts { Some("page") } else { None },
                "Linked accounts"
            }
        }
    }
}
