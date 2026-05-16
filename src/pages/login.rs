use dioxus::prelude::*;

use crate::components::base::{Button, ButtonVariant, Card, Input};
use crate::Route;

/// Sign-in page with OAuth provider buttons (not wired yet).
#[component]
pub fn Login() -> Element {
    rsx! {
        div { class: "container",
            div {
                display: "flex",
                flex_direction: "column",
                align_items: "center",
                justify_content: "center",
                min_height: "60vh",
                text_align: "center",
                h1 {
                    font_size: "32px",
                    color: "var(--text-primary)",
                    margin_bottom: "var(--space-md)",
                    "Welcome back"
                }
                p {
                    color: "var(--text-secondary)",
                    margin_bottom: "var(--space-xl)",
                    "Sign in to access your recipe library."
                }
                div {
                    max_width: "400px",
                    width: "100%",
                    Card {
                        div {
                            display: "flex",
                            flex_direction: "column",
                            gap: "var(--space-md)",
                            padding: "var(--space-xl)",
                            div {
                                display: "flex",
                                flex_direction: "column",
                                gap: "var(--space-sm)",
                                label {
                                    text_align: "left",
                                    font_size: "14px",
                                    font_weight: "600",
                                    color: "var(--text-secondary)",
                                    "Email"
                                }
                                Input {
                                    placeholder: "you@example.com",
                                    input_type: "email",
                                    oninput: move |_| {},
                                }
                            }
                            div {
                                display: "flex",
                                flex_direction: "column",
                                gap: "var(--space-sm)",
                                label {
                                    text_align: "left",
                                    font_size: "14px",
                                    font_weight: "600",
                                    color: "var(--text-secondary)",
                                    "Password"
                                }
                                Input {
                                    placeholder: "••••••••",
                                    input_type: "password",
                                    oninput: move |_| {},
                                }
                            }
                            Button {
                                variant: ButtonVariant::Primary,
                                onclick: move |_| {},
                                "Sign In"
                            }
                            div {
                                display: "flex",
                                align_items: "center",
                                gap: "var(--space-md)",
                                margin_top: "var(--space-sm)",
                                div {
                                    flex: "1",
                                    height: "1px",
                                    background: "var(--text-tertiary)",
                                }
                                span {
                                    font_size: "13px",
                                    color: "var(--text-tertiary)",
                                    "or"
                                }
                                div {
                                    flex: "1",
                                    height: "1px",
                                    background: "var(--text-tertiary)",
                                }
                            }
                            Button {
                                variant: ButtonVariant::Secondary,
                                onclick: move |_| {},
                                "Continue with Google"
                            }
                            Button {
                                variant: ButtonVariant::Secondary,
                                onclick: move |_| {},
                                "Continue with GitHub"
                            }
                        }
                    }
                }
                p {
                    margin_top: "var(--space-lg)",
                    font_size: "14px",
                    color: "var(--text-secondary)",
                    Link {
                        to: Route::Home {},
                        "← Back to home"
                    }
                }
            }
        }
    }
}
