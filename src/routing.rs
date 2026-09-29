//! Application route boundary around Dioxus's generated URL handling.

/// Keep one route inventory while retaining Dioxus's generated display, layout,
/// site map and path matching. Only Search owns an encoded whole-query parser:
/// Dioxus 0.7.9 decodes that query before `FromQuery`, losing the distinction
/// between parameter separators and encoded characters inside values.
macro_rules! define_app_routes {
    ($(#[$derive:meta])* enum $generated:ident {
        $( $(#[$attribute:meta])* $variant:ident { $( $field:ident: $field_type:ty ),* $(,)? }, )*
    }) => {
        #[derive(Debug, Clone, PartialEq)]
        pub enum Route {
            $( $variant { $( $field: $field_type ),* }, )*
        }

        #[doc(hidden)]
        $(#[$derive])*
        pub enum $generated {
            $( $(#[$attribute])* $variant { $( $field: $field_type ),* }, )*
        }

        impl From<$generated> for Route {
            fn from(route: $generated) -> Self {
                match route {
                    $( $generated::$variant { $( $field ),* } => Self::$variant { $( $field ),* }, )*
                }
            }
        }

        impl From<Route> for $generated {
            fn from(route: Route) -> Self {
                match route {
                    $( Route::$variant { $( $field ),* } => Self::$variant { $( $field ),* }, )*
                }
            }
        }

        impl std::str::FromStr for Route {
            type Err = <$generated as std::str::FromStr>::Err;

            fn from_str(url: &str) -> Result<Self, Self::Err> {
                let mut route = Self::from(url.parse::<$generated>()?);
                if let Self::Search { query } = &mut route {
                    // Match the path with Dioxus, but pass the untouched query
                    // to Search so its parser decodes each value exactly once.
                    let without_fragment = url.split_once('#').map_or(url, |(url, _)| url);
                    let raw_query = without_fragment.split_once('?').map_or("", |(_, query)| query);
                    *query = crate::search_url::SearchRouteQuery::new(raw_query);
                }
                Ok(route)
            }
        }

        impl TryFrom<&str> for Route {
            type Error = <Self as std::str::FromStr>::Err;

            fn try_from(url: &str) -> Result<Self, Self::Error> {
                url.parse()
            }
        }

        impl std::fmt::Display for Route {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Display::fmt(&$generated::from(self.clone()), formatter)
            }
        }

        impl dioxus::prelude::Routable for Route {
            const SITE_MAP: &'static [dioxus::router::routable::SiteMapSegment] =
                <$generated as dioxus::prelude::Routable>::SITE_MAP;

            fn render(&self, level: usize) -> dioxus::prelude::Element {
                dioxus::prelude::Routable::render(&$generated::from(self.clone()), level)
            }
        }
    };
}

pub(crate) use define_app_routes;
