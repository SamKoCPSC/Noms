//! Shared validation for user-controlled post-authentication redirects.

use percent_encoding::percent_decode_str;
use url::{Position, Url};

pub const REDIRECT_URI_MAX_LEN: usize = 2048;
const VALIDATION_ORIGIN: &str = "https://noms.invalid/";

/// Return a normalized same-origin redirect target.
///
/// Special-scheme URL parsers treat backslashes as path separators. Validation
/// therefore checks every percent-decoded layer before resolving against a
/// fixed HTTPS origin, preventing encoded separators from becoming authority.
pub fn normalize_same_origin_redirect(uri: &str) -> Option<String> {
    if uri.is_empty()
        || uri.len() > REDIRECT_URI_MAX_LEN
        || uri.trim() != uri
        || uri.chars().any(char::is_control)
    {
        return None;
    }

    validate_decoded_layers(uri)?;

    let base = Url::parse(VALIDATION_ORIGIN).expect("static validation origin is valid");
    let resolved = base.join(uri).ok()?;
    if resolved.origin() != base.origin()
        || !resolved.username().is_empty()
        || resolved.password().is_some()
    {
        return None;
    }

    // URL resolution removes dot segments. Validate the serialized reference
    // again because a path such as `/.//evil.example` normalizes to
    // `//evil.example`, which a Location header consumer treats as an authority.
    let normalized = resolved[Position::BeforePath..].to_string();
    validate_decoded_layers(&normalized)?;
    Some(normalized)
}

fn has_unsafe_path_form(uri: &str) -> bool {
    let path = uri.split(['?', '#']).next().unwrap_or_default();
    !path.starts_with('/') || path.starts_with("//") || path.contains('\\')
}

fn validate_decoded_layers(uri: &str) -> Option<()> {
    if !has_valid_percent_encoding(uri) {
        return None;
    }

    let mut decoded_layer = uri.to_string();
    loop {
        if decoded_layer.chars().any(char::is_control)
            || decoded_layer.contains('\u{fffd}')
            || has_unsafe_path_form(&decoded_layer)
        {
            return None;
        }

        let next_layer = percent_decode_str(&decoded_layer)
            .decode_utf8()
            .ok()?
            .into_owned();
        if next_layer == decoded_layer {
            return Some(());
        }
        decoded_layer = next_layer;
    }
}

fn has_valid_percent_encoding(value: &str) -> bool {
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len()
                || !bytes[index + 1].is_ascii_hexdigit()
                || !bytes[index + 2].is_ascii_hexdigit()
            {
                return false;
            }
            index += 3;
        } else {
            index += 1;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::normalize_same_origin_redirect;

    #[test]
    fn rejects_paths_that_normalize_to_protocol_relative_references() {
        for uri in ["/.//evil.example", "/%2e%2e//evil.example"] {
            assert_eq!(
                normalize_same_origin_redirect(uri),
                None,
                "unsafe redirect accepted: {uri}"
            );
        }
    }

    #[test]
    fn rejects_malformed_percent_encoding() {
        for uri in ["/%", "/%zz", "/%FF"] {
            assert_eq!(
                normalize_same_origin_redirect(uri),
                None,
                "malformed redirect accepted: {uri}"
            );
        }
    }

    #[test]
    fn preserves_valid_encoded_percent_signs() {
        for uri in ["/search?q=100%25", "/search?q=%25zz"] {
            assert_eq!(
                normalize_same_origin_redirect(uri),
                Some(uri.to_string()),
                "valid encoded percent was rejected: {uri}"
            );
        }
    }
}
