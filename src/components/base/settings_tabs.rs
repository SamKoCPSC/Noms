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
        div { class: "settings-tabs",
            Link {
                to: Route::SettingsProfile {},
                class: if active == SettingsTab::Profile { "settings-tab settings-tab-active" } else { "settings-tab" },
                "Profile"
            }
            Link {
                to: Route::SettingsAccounts {},
                class: if active == SettingsTab::Accounts { "settings-tab settings-tab-active" } else { "settings-tab" },
                "Accounts"
            }
        }
    }
}
