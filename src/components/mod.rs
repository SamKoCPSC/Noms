pub use app_layout::AppLayout;
pub use auth_required::AuthRequired;
pub use error_fallback::ErrorFallback;
pub use footer::Footer;
pub use kitchen::KitchenLayout;
pub use navbar::Navbar;

pub mod base;
pub mod informational;

mod app_layout;
mod auth_required;
mod error_fallback;
mod footer;
pub(crate) mod kitchen;
pub mod mobile;
mod navbar;

#[cfg(test)]
mod design_foundation_tests;

#[cfg(test)]
mod modal_behavior_tests;

#[cfg(test)]
mod feedback_tests;

#[cfg(test)]
mod informational_module_tests {
    use super::informational::{CallToActionPanel, PageMeta};

    #[test]
    fn shared_authored_page_components_use_the_informational_namespace() {
        assert!(std::any::type_name_of_val(&PageMeta).contains("informational"));
        assert!(std::any::type_name_of_val(&CallToActionPanel).contains("informational"));
    }
}

pub mod equipment_autocomplete;
pub mod ingredient_autocomplete;
pub(crate) mod ingredient_vocabulary;
pub(crate) mod recipe_title_autocomplete;
pub mod unit_autocomplete;
