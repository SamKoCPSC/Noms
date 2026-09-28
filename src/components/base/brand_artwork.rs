use dioxus::prelude::*;
use manganis::{ImageAssetOptions, ImageSize};

const BRAND_STYLES: Asset = asset!("/assets/brand/ui.css");
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

/// All placements share the painted book; only the display size varies.
#[component]
pub fn BrandMark(#[props(default)] large: bool) -> Element {
    rsx! {
        document::Stylesheet { href: BRAND_STYLES }
        img {
            class: if large { "brand-mark brand-mark--large" } else { "brand-mark" },
            src: PAINTED_BOOK,
            alt: "",
            aria_hidden: "true",
            width: if large { "96" } else { "32" },
            height: if large { "120" } else { "40" },
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
        let small = dioxus_ssr::render_element(rsx! { BrandMark {} });
        assert!(
            small.contains("noms-logo-painted-s3-v1") && !small.contains(".svg"),
            "{small}"
        );
        assert!(
            small.contains("width=\"32\"") && small.contains("height=\"40\""),
            "{small}"
        );
        let painted = dioxus_ssr::render_element(rsx! { BrandMark { large: true } });
        assert!(painted.contains("noms-logo-painted-s3-v1"), "{painted}");
        let small_source = small
            .split("src=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap();
        let large_source = painted
            .split("src=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap();
        assert_eq!(
            small_source, large_source,
            "All placements must share the same painted book asset"
        );
        for html in [&small, &painted] {
            assert!(
                html.contains("alt=\"\"") && html.contains("aria-hidden=\"true\""),
                "{html}"
            );
        }
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
