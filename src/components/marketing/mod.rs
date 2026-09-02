use dioxus::prelude::*;

use crate::Route;

#[derive(Props, Clone, PartialEq)]
pub struct MarketingPageMetaProps {
    pub title: &'static str,
    pub description: &'static str,
    pub path: &'static str,
}

/// Route-specific metadata kept next to semantic page content so SSR responses
/// remain useful before hydration.
#[component]
pub fn MarketingPageMeta(props: MarketingPageMetaProps) -> Element {
    let canonical_url = canonical_url(props.path);

    rsx! {
        document::Title { "{props.title}" }
        document::Meta { name: "description", content: props.description }
        document::Meta { property: "og:title", content: props.title }
        document::Meta { property: "og:description", content: props.description }
        document::Meta { property: "og:type", content: "website" }
        document::Meta { property: "og:url", content: canonical_url.clone() }
        document::Link { rel: "canonical", href: canonical_url }
    }
}

fn canonical_url(path: &str) -> String {
    #[cfg(feature = "server")]
    {
        let base_url =
            std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
        return format!("{}{}", base_url.trim_end_matches('/'), path);
    }

    #[cfg(not(feature = "server"))]
    {
        path.to_string()
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct MarketingCtaPanelProps {
    pub title: &'static str,
    pub body: &'static str,
    pub primary_label: &'static str,
    pub primary_route: Route,
    #[props(default)]
    pub primary_href: Option<&'static str>,
    pub secondary_label: &'static str,
    pub secondary_route: Route,
}

#[component]
pub fn MarketingCtaPanel(props: MarketingCtaPanelProps) -> Element {
    rsx! {
        section { class: "marketing-cta-panel container", aria_labelledby: "marketing-final-cta",
            div { class: "marketing-cta-panel__inner",
                div {
                    h2 { id: "marketing-final-cta", "{props.title}" }
                    p { "{props.body}" }
                }
                div { class: "marketing-actions",
                    if let Some(primary_href) = props.primary_href {
                        a {
                            href: primary_href,
                            class: "btn btn-primary touch-target",
                            "{props.primary_label}"
                        }
                    } else {
                        Link {
                            to: props.primary_route,
                            class: "btn btn-primary touch-target",
                            "{props.primary_label}"
                        }
                    }
                    Link {
                        to: props.secondary_route,
                        class: "btn btn-secondary touch-target",
                        "{props.secondary_label}"
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::canonical_url;

    #[test]
    fn canonical_path_uses_configured_base_or_local_default() {
        let url = canonical_url("/about");
        assert!(url.ends_with("/about"));
        assert!(url.starts_with("http://") || url.starts_with("https://"));
    }
}
