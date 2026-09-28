use dioxus::prelude::*;
use manganis::{ImageAssetOptions, ImageSize};

const BRAND_STYLES: Asset = asset!("/assets/brand/ui.css");
const BOOK_LOGO: Asset = asset!("/assets/brand/noms-logo.svg");
const PAINTED_BOOK: Asset = asset!(
    "/assets/brand/noms-logo-painted-s3-v1.png",
    ImageAssetOptions::new()
        .with_webp()
        .with_size(ImageSize::Manual {
            width: 192,
            height: 240
        })
);

/// Painted header identities, not replacements for small interactive icons.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PageIconKind {
    Dashboard,
    Explore,
    Search,
    Collections,
    Editor,
    History,
    Profile,
    Settings,
}

impl PageIconKind {
    fn asset(self) -> Asset {
        // Preserve the source paintings; ship appropriately sized transparent
        // derivatives through the normal hashed Dioxus asset pipeline.
        match self {
            Self::Dashboard => asset!(
                "/assets/brand/page-icons/dashboard-v1.png",
                ImageAssetOptions::new()
                    .with_webp()
                    .with_size(ImageSize::Manual {
                        width: 192,
                        height: 192
                    })
            ),
            Self::Explore => asset!(
                "/assets/brand/page-icons/explore-v1.png",
                ImageAssetOptions::new()
                    .with_webp()
                    .with_size(ImageSize::Manual {
                        width: 192,
                        height: 192
                    })
            ),
            Self::Search => asset!(
                "/assets/brand/page-icons/search-v1.png",
                ImageAssetOptions::new()
                    .with_webp()
                    .with_size(ImageSize::Manual {
                        width: 192,
                        height: 192
                    })
            ),
            Self::Collections => asset!(
                "/assets/brand/page-icons/collections-v1.png",
                ImageAssetOptions::new()
                    .with_webp()
                    .with_size(ImageSize::Manual {
                        width: 192,
                        height: 192
                    })
            ),
            Self::Editor => asset!(
                "/assets/brand/page-icons/editor-v1.png",
                ImageAssetOptions::new()
                    .with_webp()
                    .with_size(ImageSize::Manual {
                        width: 192,
                        height: 192
                    })
            ),
            Self::History => asset!(
                "/assets/brand/page-icons/history-v1.png",
                ImageAssetOptions::new()
                    .with_webp()
                    .with_size(ImageSize::Manual {
                        width: 192,
                        height: 192
                    })
            ),
            Self::Profile => asset!(
                "/assets/brand/page-icons/profile-v1.png",
                ImageAssetOptions::new()
                    .with_webp()
                    .with_size(ImageSize::Manual {
                        width: 192,
                        height: 192
                    })
            ),
            Self::Settings => asset!(
                "/assets/brand/page-icons/settings-v1.png",
                ImageAssetOptions::new()
                    .with_webp()
                    .with_size(ImageSize::Manual {
                        width: 192,
                        height: 192
                    })
            ),
        }
    }
}

/// Decorative book beside an existing wordmark or labelled sign-in heading.
#[component]
pub fn BrandMark(#[props(default)] painted: bool) -> Element {
    rsx! {
        document::Stylesheet { href: BRAND_STYLES }
        img {
            class: if painted { "brand-mark brand-mark--painted" } else { "brand-mark" },
            src: if painted { PAINTED_BOOK } else { BOOK_LOGO },
            alt: "",
            aria_hidden: "true",
            width: if painted { "96" } else { "32" },
            height: if painted { "120" } else { "40" },
            decoding: "async",
        }
    }
}

/// Adjacent text supplies the accessible name; this art adds no focus target.
#[component]
pub fn PageIcon(kind: PageIconKind, #[props(default)] compact: bool) -> Element {
    rsx! {
        document::Stylesheet { href: BRAND_STYLES }
        img {
            class: if compact { "page-identity-icon page-identity-icon--compact" } else { "page-identity-icon" },
            src: kind.asset(),
            alt: "",
            aria_hidden: "true",
            width: if compact { "48" } else { "72" },
            height: if compact { "48" } else { "72" },
            decoding: "async",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_page_identities_render_distinct_decorative_sized_assets() {
        let icons = [
            (PageIconKind::Dashboard, "dashboard-v1"),
            (PageIconKind::Explore, "explore-v1"),
            (PageIconKind::Search, "search-v1"),
            (PageIconKind::Collections, "collections-v1"),
            (PageIconKind::Editor, "editor-v1"),
            (PageIconKind::History, "history-v1"),
            (PageIconKind::Profile, "profile-v1"),
            (PageIconKind::Settings, "settings-v1"),
        ];
        for (kind, name) in icons {
            let html = dioxus_ssr::render_element(rsx! { PageIcon { kind } });
            assert!(html.contains(name), "{html}");
            assert!(html.contains("alt=\"\""), "{html}");
            assert!(html.contains("aria-hidden=\"true\""), "{html}");
            assert!(
                html.contains("width=\"72\"") && html.contains("height=\"72\""),
                "{html}"
            );
            assert!(!html.contains("tabindex"), "{html}");
        }
    }

    #[test]
    fn brand_marks_and_compact_artwork_reserve_correct_aspect_ratios() {
        let vector = dioxus_ssr::render_element(rsx! { BrandMark {} });
        assert!(
            vector.contains("noms-logo") && vector.contains(".svg"),
            "{vector}"
        );
        assert!(
            vector.contains("width=\"32\"") && vector.contains("height=\"40\""),
            "{vector}"
        );
        let painted = dioxus_ssr::render_element(rsx! { BrandMark { painted: true } });
        assert!(painted.contains("noms-logo-painted-s3-v1"), "{painted}");
        assert!(
            painted.contains("width=\"96\"") && painted.contains("height=\"120\""),
            "{painted}"
        );
        let compact = dioxus_ssr::render_element(
            rsx! { PageIcon { kind: PageIconKind::History, compact: true } },
        );
        assert!(
            compact.contains("width=\"48\"") && compact.contains("height=\"48\""),
            "{compact}"
        );
    }
}
