//! Shared search request validation.
#![allow(dead_code)]

use std::collections::HashSet;

use unicode_normalization::UnicodeNormalization;

use crate::types::{
    AdvancedCollectionSearchCriteria, AdvancedRecipeSearchCriteria, AdvancedTextField,
    AllSearchRequest, CollectionSearchRequest, IntegerRange, RecipeSearchRequest, TextCriterion,
    TimestampRange, UserSearchRequest,
};

pub const MAX_SEARCH_TEXT_CHARS: usize = 256;
pub const MAX_SEARCH_EXCLUSIONS_PER_FIELD: usize = 5;
pub const MAX_SEARCH_TAG_CHARS: usize = 100;
pub const MAX_SEARCH_TAGS: usize = 25;
pub const DEFAULT_SEARCH_PAGE_SIZE: u32 = 20;
pub const MAX_SEARCH_PAGE_SIZE: u32 = 100;
pub const MAX_RECIPE_SEARCH_PAGE_SIZE: u32 = 20;
pub const MAX_SEARCH_TERMS: usize = 64;
pub const MAX_SEARCH_RESPONSE_BYTES: usize = 512 * 1024;
pub const MAX_SEARCH_OFFSET: u32 = 10_000;
pub const DEFAULT_ALL_SECTION_LIMIT: u32 = 5;
pub const MAX_ALL_SECTION_LIMIT: u32 = 10;
pub const MAX_ADVANCED_URL_DECODED_BYTES: usize = 4_096;
pub const MAX_ADVANCED_URL_ENCODED_BYTES: usize = 8_192;
pub const MIN_AUTOCOMPLETE_CHARS: usize = 2;
pub const MAX_AUTOCOMPLETE_SUGGESTIONS: usize = 8;
pub const ALL_AUTOCOMPLETE_RECIPE_LIMIT: usize = 4;
pub const ALL_AUTOCOMPLETE_COLLECTION_LIMIT: usize = 2;
pub const ALL_AUTOCOMPLETE_USER_LIMIT: usize = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchValidationError {
    EmptyText { field: &'static str },
    TextTooLong { field: &'static str, max: usize },
    TooManyExclusions { field: &'static str, max: usize },
    EmptyTag,
    TagTooLong { max: usize },
    TooManyTags { max: usize },
    ConflictingTag { tag: String },
    InvalidRange { field: &'static str },
    InvalidPageSize { max: u32 },
    InvalidPage,
    PageOffsetTooLarge { max: u32 },
    InvalidSectionLimit { max: u32 },
    AutocompleteTooShort { min: usize },
    AdvancedUrlTooLarge { encoded: bool, max: usize },
    TooManyTerms { max: usize },
    ResponseTooLarge,
}

impl std::fmt::Display for SearchValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManyTerms { max } => {
                write!(formatter, "search supports at most {max} total terms")
            }
            Self::ResponseTooLarge => {
                write!(formatter, "search response exceeds the output budget")
            }
            Self::EmptyText { field } => write!(formatter, "{field} must not be blank"),
            Self::TextTooLong { field, max } => {
                write!(formatter, "{field} must be {max} characters or fewer")
            }
            Self::TooManyExclusions { field, max } => {
                write!(formatter, "{field} supports at most {max} exclusions")
            }
            Self::EmptyTag => write!(formatter, "tags must not be blank"),
            Self::TagTooLong { max } => write!(formatter, "tags must be {max} characters or fewer"),
            Self::TooManyTags { max } => {
                write!(formatter, "at most {max} distinct tags are allowed")
            }
            Self::ConflictingTag { tag } => {
                write!(
                    formatter,
                    "tag '{tag}' cannot be both required and excluded"
                )
            }
            Self::InvalidRange { field } => write!(formatter, "{field} range is invalid"),
            Self::InvalidPageSize { max } => {
                write!(formatter, "page size must be between 1 and {max}")
            }
            Self::InvalidPage => write!(formatter, "page must be at least 1"),
            Self::PageOffsetTooLarge { max } => {
                write!(formatter, "calculated page offset must not exceed {max}")
            }
            Self::InvalidSectionLimit { max } => {
                write!(formatter, "section limit must be between 1 and {max}")
            }
            Self::AutocompleteTooShort { min } => {
                write!(
                    formatter,
                    "autocomplete text must contain at least {min} characters"
                )
            }
            Self::AdvancedUrlTooLarge { encoded, max } => {
                let form = if *encoded { "encoded" } else { "decoded" };
                write!(
                    formatter,
                    "{form} advanced URL state must not exceed {max} bytes"
                )
            }
        }
    }
}

impl std::error::Error for SearchValidationError {}

pub fn validate_recipe_search_request(
    request: &RecipeSearchRequest,
) -> Result<(), SearchValidationError> {
    validate_optional_simple_text(request.text.as_deref())?;
    if request.page_size > MAX_RECIPE_SEARCH_PAGE_SIZE {
        return Err(SearchValidationError::InvalidPageSize {
            max: MAX_RECIPE_SEARCH_PAGE_SIZE,
        });
    }
    validate_pagination(request.page, request.page_size)?;

    let mut term_count = request.text.as_deref().map_or(0, search_term_count);
    if let Some(advanced) = &request.advanced {
        for field in [
            &advanced.title,
            &advanced.description,
            &advanced.ingredients,
            &advanced.instructions,
        ] {
            for criterion in field.include.iter().chain(field.exclude.iter()) {
                term_count += search_term_count(&criterion.text);
            }
        }
    }
    validate_term_count(term_count)?;

    if let Some(range) = request.total_time {
        validate_integer_range("total_time", range, 0)?;
    }
    if let Some(range) = request.servings {
        validate_integer_range("servings", range, 1)?;
    }

    if let Some(advanced) = &request.advanced {
        validate_advanced_recipe(advanced)?;
    }
    validate_recipe_tags(request)
}

pub fn validate_collection_search_request(
    request: &CollectionSearchRequest,
) -> Result<(), SearchValidationError> {
    validate_optional_simple_text(request.text.as_deref())?;
    validate_pagination(request.page, request.page_size)?;
    if let Some(advanced) = &request.advanced {
        validate_advanced_collection(advanced)?;
    }
    Ok(())
}

pub fn validate_user_search_request(
    request: &UserSearchRequest,
) -> Result<(), SearchValidationError> {
    validate_optional_simple_text(request.text.as_deref())?;
    validate_pagination(request.page, request.page_size)?;
    if let Some(advanced) = &request.advanced {
        validate_text_field("username", &advanced.username)?;
        validate_text_field("display_name", &advanced.display_name)?;
        validate_text_field("bio", &advanced.bio)?;
    }
    Ok(())
}

pub fn validate_all_search_request(
    request: &AllSearchRequest,
) -> Result<(), SearchValidationError> {
    validate_optional_simple_text(Some(&request.text))?;
    validate_term_count(search_term_count(&request.text))?;
    if request.section_limit == 0 || request.section_limit > MAX_ALL_SECTION_LIMIT {
        return Err(SearchValidationError::InvalidSectionLimit {
            max: MAX_ALL_SECTION_LIMIT,
        });
    }
    Ok(())
}

pub fn validate_autocomplete_text(text: &str) -> Result<(), SearchValidationError> {
    validate_term_count(search_term_count(text))?;
    let length = text.trim().chars().count();
    if length < MIN_AUTOCOMPLETE_CHARS {
        return Err(SearchValidationError::AutocompleteTooShort {
            min: MIN_AUTOCOMPLETE_CHARS,
        });
    }
    validate_text_length("autocomplete text", text.trim())
}

pub fn validate_advanced_url_size(
    decoded_bytes: usize,
    encoded_bytes: usize,
) -> Result<(), SearchValidationError> {
    if decoded_bytes > MAX_ADVANCED_URL_DECODED_BYTES {
        return Err(SearchValidationError::AdvancedUrlTooLarge {
            encoded: false,
            max: MAX_ADVANCED_URL_DECODED_BYTES,
        });
    }
    if encoded_bytes > MAX_ADVANCED_URL_ENCODED_BYTES {
        return Err(SearchValidationError::AdvancedUrlTooLarge {
            encoded: true,
            max: MAX_ADVANCED_URL_ENCODED_BYTES,
        });
    }
    Ok(())
}

pub fn normalize_search_tag(tag: &str) -> String {
    tag.trim().nfc().collect()
}

// Each occurrence generates work even when two terms normalize to the same text.
// NFC preserves whitespace boundaries, matching the SQL builder's tokenization.
fn search_term_count(text: &str) -> usize {
    text.nfc().collect::<String>().split_whitespace().count()
}

fn validate_term_count(count: usize) -> Result<(), SearchValidationError> {
    if count > MAX_SEARCH_TERMS {
        return Err(SearchValidationError::TooManyTerms {
            max: MAX_SEARCH_TERMS,
        });
    }
    Ok(())
}

/// Count serialized output without allocating another response-sized buffer.
pub fn validate_search_response<T: serde::Serialize + ?Sized>(
    response: &T,
) -> Result<(), SearchValidationError> {
    struct OutputBudget(usize);
    impl std::io::Write for OutputBudget {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > self.0 {
                return Err(std::io::Error::other("search output budget exceeded"));
            }
            self.0 -= bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(OutputBudget(MAX_SEARCH_RESPONSE_BYTES), response)
        .map_err(|_| SearchValidationError::ResponseTooLarge)
}

fn validate_optional_simple_text(text: Option<&str>) -> Result<(), SearchValidationError> {
    if let Some(text) = text {
        validate_text_length("text", text.trim())?;
    }
    Ok(())
}

fn validate_text_length(field: &'static str, text: &str) -> Result<(), SearchValidationError> {
    if text.chars().count() > MAX_SEARCH_TEXT_CHARS {
        return Err(SearchValidationError::TextTooLong {
            field,
            max: MAX_SEARCH_TEXT_CHARS,
        });
    }
    Ok(())
}

fn validate_criterion(
    field: &'static str,
    criterion: &TextCriterion,
) -> Result<(), SearchValidationError> {
    let trimmed = criterion.text.trim();
    if trimmed.is_empty() {
        return Err(SearchValidationError::EmptyText { field });
    }
    validate_text_length(field, trimmed)
}

fn validate_text_field(
    field: &'static str,
    value: &AdvancedTextField,
) -> Result<(), SearchValidationError> {
    if value.exclude.len() > MAX_SEARCH_EXCLUSIONS_PER_FIELD {
        return Err(SearchValidationError::TooManyExclusions {
            field,
            max: MAX_SEARCH_EXCLUSIONS_PER_FIELD,
        });
    }
    if let Some(include) = &value.include {
        validate_criterion(field, include)?;
    }
    for exclusion in &value.exclude {
        validate_criterion(field, exclusion)?;
    }
    Ok(())
}

fn validate_advanced_recipe(
    advanced: &AdvancedRecipeSearchCriteria,
) -> Result<(), SearchValidationError> {
    validate_text_field("title", &advanced.title)?;
    validate_text_field("description", &advanced.description)?;
    validate_text_field("ingredients", &advanced.ingredients)?;
    validate_text_field("instructions", &advanced.instructions)?;
    if let Some(range) = advanced.created_at {
        validate_timestamp_range("created_at", range)?;
    }
    if let Some(range) = advanced.updated_at {
        validate_timestamp_range("updated_at", range)?;
    }
    Ok(())
}

fn validate_advanced_collection(
    advanced: &AdvancedCollectionSearchCriteria,
) -> Result<(), SearchValidationError> {
    validate_text_field("name", &advanced.name)?;
    validate_text_field("description", &advanced.description)?;
    if let Some(range) = advanced.updated_at {
        validate_timestamp_range("updated_at", range)?;
    }
    Ok(())
}

fn validate_integer_range(
    field: &'static str,
    range: IntegerRange,
    minimum: i32,
) -> Result<(), SearchValidationError> {
    if range.lower_inclusive.is_none() && range.upper_exclusive.is_none() {
        return Err(SearchValidationError::InvalidRange { field });
    }
    if range.lower_inclusive.is_some_and(|value| value < minimum)
        || range.upper_exclusive.is_some_and(|value| value <= minimum)
        || matches!(
            (range.lower_inclusive, range.upper_exclusive),
            (Some(lower), Some(upper)) if lower >= upper
        )
    {
        return Err(SearchValidationError::InvalidRange { field });
    }
    Ok(())
}

fn validate_timestamp_range(
    field: &'static str,
    range: TimestampRange,
) -> Result<(), SearchValidationError> {
    if range.lower_inclusive.is_none() && range.upper_exclusive.is_none() {
        return Err(SearchValidationError::InvalidRange { field });
    }
    if matches!(
        (range.lower_inclusive, range.upper_exclusive),
        (Some(lower), Some(upper)) if lower >= upper
    ) {
        return Err(SearchValidationError::InvalidRange { field });
    }
    Ok(())
}

fn validate_pagination(page: u32, page_size: u32) -> Result<(), SearchValidationError> {
    if page == 0 {
        return Err(SearchValidationError::InvalidPage);
    }
    if page_size == 0 || page_size > MAX_SEARCH_PAGE_SIZE {
        return Err(SearchValidationError::InvalidPageSize {
            max: MAX_SEARCH_PAGE_SIZE,
        });
    }
    let offset = page
        .checked_sub(1)
        .and_then(|value| value.checked_mul(page_size))
        .ok_or(SearchValidationError::PageOffsetTooLarge {
            max: MAX_SEARCH_OFFSET,
        })?;
    if offset > MAX_SEARCH_OFFSET {
        return Err(SearchValidationError::PageOffsetTooLarge {
            max: MAX_SEARCH_OFFSET,
        });
    }
    Ok(())
}

fn validate_recipe_tags(request: &RecipeSearchRequest) -> Result<(), SearchValidationError> {
    let positive_tags = request.tags_all.iter().chain(
        request
            .advanced
            .iter()
            .flat_map(|advanced| advanced.tags_any.iter()),
    );
    let excluded_tags = request
        .advanced
        .iter()
        .flat_map(|advanced| advanced.tags_excluded.iter());

    let mut positive = HashSet::new();
    let mut excluded = HashSet::new();
    for tag in positive_tags {
        positive.insert(validate_and_normalize_tag(tag)?);
    }
    for tag in excluded_tags {
        excluded.insert(validate_and_normalize_tag(tag)?);
    }

    let distinct_count = positive.union(&excluded).count();
    if distinct_count > MAX_SEARCH_TAGS {
        return Err(SearchValidationError::TooManyTags {
            max: MAX_SEARCH_TAGS,
        });
    }
    if let Some(tag) = positive.intersection(&excluded).next() {
        return Err(SearchValidationError::ConflictingTag { tag: tag.clone() });
    }
    Ok(())
}

fn validate_and_normalize_tag(tag: &str) -> Result<String, SearchValidationError> {
    let normalized = normalize_search_tag(tag);
    if normalized.is_empty() {
        return Err(SearchValidationError::EmptyTag);
    }
    if normalized.chars().count() > MAX_SEARCH_TAG_CHARS {
        return Err(SearchValidationError::TagTooLong {
            max: MAX_SEARCH_TAG_CHARS,
        });
    }
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use super::*;
    use crate::types::{
        AdvancedCollectionSearchCriteria, AdvancedRecipeSearchCriteria, AdvancedTextField,
        AdvancedUserSearchCriteria, AllSearchRequest, CollectionSearchRequest, IntegerRange,
        RecipeSearchRequest, TextCriterion, TextMatchMode, UserSearchRequest,
    };

    fn valid_recipe_request() -> RecipeSearchRequest {
        RecipeSearchRequest::default()
    }

    #[test]
    fn search_serialization_budget_counts_the_entire_response() {
        let mut response = crate::types::RecipeSearchResponse {
            results: vec![],
            total_count: 0,
            facets: crate::types::RecipeSearchFacets {
                tags: vec![],
                visibility: vec![],
                total_time: vec![],
                servings: vec![],
            },
        };
        assert!(validate_search_response(&response).is_ok());
        response.facets.tags.push(crate::types::TagFacetBucket {
            tag: "x".repeat(512 * 1024),
            count: 1,
        });
        assert!(validate_search_response(&response).is_err());
        let exactly_at_limit = "x".repeat(512 * 1024 - 2);
        assert!(validate_search_response(&exactly_at_limit).is_ok());
        assert!(validate_search_response(&(exactly_at_limit + "x")).is_err());
    }

    #[test]
    fn recipe_search_rejects_aggregate_clause_amplification() {
        let criterion = TextCriterion {
            text: vec!["x"; 128].join(" "),
            mode: TextMatchMode::AllTerms,
        };
        let field = AdvancedTextField {
            include: Some(criterion.clone()),
            exclude: vec![criterion; 5],
        };
        let request = RecipeSearchRequest {
            advanced: Some(AdvancedRecipeSearchCriteria {
                title: field.clone(),
                description: field.clone(),
                ingredients: field.clone(),
                instructions: field,
                ..Default::default()
            }),
            ..Default::default()
        };
        assert!(validate_recipe_search_request(&request).is_err());
    }

    #[test]
    fn recipe_search_counts_terms_across_fields_and_exclusions() {
        let criterion = |count| TextCriterion {
            text: vec!["é"; count].join(" "),
            mode: TextMatchMode::AnyTerm,
        };
        let mut request = RecipeSearchRequest {
            advanced: Some(AdvancedRecipeSearchCriteria {
                title: AdvancedTextField {
                    include: Some(criterion(16)),
                    exclude: vec![],
                },
                description: AdvancedTextField {
                    include: Some(criterion(16)),
                    exclude: vec![],
                },
                ingredients: AdvancedTextField {
                    include: Some(criterion(16)),
                    exclude: vec![],
                },
                instructions: AdvancedTextField {
                    include: None,
                    exclude: vec![criterion(16)],
                },
                ..Default::default()
            }),
            ..Default::default()
        };
        assert!(validate_recipe_search_request(&request).is_ok());
        request.advanced.as_mut().unwrap().instructions.exclude[0]
            .text
            .push_str(" e\u{301}");
        assert!(validate_recipe_search_request(&request).is_err());
    }

    #[test]
    fn recipe_search_page_ceiling_is_twenty() {
        let mut request = RecipeSearchRequest {
            page_size: 20,
            ..Default::default()
        };
        assert!(validate_recipe_search_request(&request).is_ok());
        request.page_size = 21;
        assert!(validate_recipe_search_request(&request).is_err());
        request.page_size = 100;
        assert!(validate_recipe_search_request(&request).is_err());
    }

    #[test]
    fn accepts_boundary_sized_recipe_request() {
        let mut request = valid_recipe_request();
        request.text = Some("a".repeat(MAX_SEARCH_TEXT_CHARS));
        request.page_size = MAX_RECIPE_SEARCH_PAGE_SIZE;
        request.page = MAX_SEARCH_OFFSET / MAX_RECIPE_SEARCH_PAGE_SIZE + 1;

        assert_eq!(validate_recipe_search_request(&request), Ok(()));
    }

    #[test]
    fn counts_unicode_scalars_instead_of_utf16_units() {
        let mut request = valid_recipe_request();
        request.text = Some("🍝".repeat(200));

        assert_eq!(validate_recipe_search_request(&request), Ok(()));
    }

    #[test]
    fn rejects_oversized_text_and_exclusion_lists() {
        let mut request = valid_recipe_request();
        request.text = Some("a".repeat(MAX_SEARCH_TEXT_CHARS + 1));
        assert!(matches!(
            validate_recipe_search_request(&request),
            Err(SearchValidationError::TextTooLong { .. })
        ));

        request.text = None;
        request.advanced = Some(AdvancedRecipeSearchCriteria {
            title: AdvancedTextField {
                include: None,
                exclude: (0..=MAX_SEARCH_EXCLUSIONS_PER_FIELD)
                    .map(|_| TextCriterion {
                        text: "exclude".to_string(),
                        mode: TextMatchMode::AllTerms,
                    })
                    .collect(),
            },
            ..Default::default()
        });
        assert!(matches!(
            validate_recipe_search_request(&request),
            Err(SearchValidationError::TooManyExclusions { .. })
        ));
    }

    #[test]
    fn normalizes_tags_before_counting_and_detecting_conflicts() {
        let mut request = valid_recipe_request();
        request.tags_all = vec!["  Cafe\u{301}  ".to_string(), "quick".to_string()];
        request.advanced = Some(AdvancedRecipeSearchCriteria {
            tags_any: vec!["Café".to_string()],
            ..Default::default()
        });
        assert_eq!(validate_recipe_search_request(&request), Ok(()));

        request
            .advanced
            .as_mut()
            .unwrap()
            .tags_excluded
            .push(" Café ".to_string());
        assert!(matches!(
            validate_recipe_search_request(&request),
            Err(SearchValidationError::ConflictingTag { tag }) if tag == "Café"
        ));
    }

    #[test]
    fn rejects_tag_and_pagination_limits() {
        let mut request = valid_recipe_request();
        request.tags_all = vec!["   ".to_string()];
        assert_eq!(
            validate_recipe_search_request(&request),
            Err(SearchValidationError::EmptyTag)
        );

        request.tags_all = vec!["a".repeat(MAX_SEARCH_TAG_CHARS + 1)];
        assert!(matches!(
            validate_recipe_search_request(&request),
            Err(SearchValidationError::TagTooLong { .. })
        ));

        request.tags_all = (0..=MAX_SEARCH_TAGS)
            .map(|index| format!("tag-{index}"))
            .collect();
        assert!(matches!(
            validate_recipe_search_request(&request),
            Err(SearchValidationError::TooManyTags { .. })
        ));

        request.tags_all.clear();
        request.page = 0;
        assert_eq!(
            validate_recipe_search_request(&request),
            Err(SearchValidationError::InvalidPage)
        );

        request.page = 1;
        request.page_size = MAX_SEARCH_PAGE_SIZE + 1;
        assert!(matches!(
            validate_recipe_search_request(&request),
            Err(SearchValidationError::InvalidPageSize { .. })
        ));

        request.page_size = MAX_RECIPE_SEARCH_PAGE_SIZE;
        request.page = MAX_SEARCH_OFFSET / MAX_RECIPE_SEARCH_PAGE_SIZE + 2;
        assert!(matches!(
            validate_recipe_search_request(&request),
            Err(SearchValidationError::PageOffsetTooLarge { .. })
        ));

        request.page = u32::MAX;
        assert!(matches!(
            validate_recipe_search_request(&request),
            Err(SearchValidationError::PageOffsetTooLarge { .. })
        ));
    }

    #[test]
    fn rejects_empty_and_inverted_ranges() {
        let mut request = valid_recipe_request();
        request.total_time = Some(IntegerRange::default());
        assert!(matches!(
            validate_recipe_search_request(&request),
            Err(SearchValidationError::InvalidRange {
                field: "total_time"
            })
        ));

        request.total_time = Some(IntegerRange {
            lower_inclusive: Some(60),
            upper_exclusive: Some(30),
        });
        assert!(matches!(
            validate_recipe_search_request(&request),
            Err(SearchValidationError::InvalidRange {
                field: "total_time"
            })
        ));

        let now = Utc::now();
        request.total_time = None;
        request.advanced = Some(AdvancedRecipeSearchCriteria {
            created_at: Some(crate::types::TimestampRange {
                lower_inclusive: Some(now),
                upper_exclusive: Some(now - Duration::seconds(1)),
            }),
            ..Default::default()
        });
        assert!(matches!(
            validate_recipe_search_request(&request),
            Err(SearchValidationError::InvalidRange {
                field: "created_at"
            })
        ));
    }

    #[test]
    fn validates_all_autocomplete_and_advanced_url_limits() {
        assert_eq!(
            validate_all_search_request(&AllSearchRequest::default()),
            Ok(())
        );
        assert!(matches!(
            validate_all_search_request(&AllSearchRequest {
                section_limit: MAX_ALL_SECTION_LIMIT + 1,
                ..Default::default()
            }),
            Err(SearchValidationError::InvalidSectionLimit { .. })
        ));

        assert!(matches!(
            validate_autocomplete_text(" 🍝 "),
            Err(SearchValidationError::AutocompleteTooShort { .. })
        ));
        assert_eq!(validate_autocomplete_text(" 🍝a "), Ok(()));

        assert_eq!(
            validate_advanced_url_size(
                MAX_ADVANCED_URL_DECODED_BYTES,
                MAX_ADVANCED_URL_ENCODED_BYTES
            ),
            Ok(())
        );
        assert!(matches!(
            validate_advanced_url_size(MAX_ADVANCED_URL_DECODED_BYTES + 1, 1),
            Err(SearchValidationError::AdvancedUrlTooLarge { encoded: false, .. })
        ));
        assert!(matches!(
            validate_advanced_url_size(1, MAX_ADVANCED_URL_ENCODED_BYTES + 1),
            Err(SearchValidationError::AdvancedUrlTooLarge { encoded: true, .. })
        ));

        assert_eq!(ALL_AUTOCOMPLETE_RECIPE_LIMIT, 4);
        assert_eq!(ALL_AUTOCOMPLETE_COLLECTION_LIMIT, 2);
        assert_eq!(ALL_AUTOCOMPLETE_USER_LIMIT, 2);
        assert_eq!(
            ALL_AUTOCOMPLETE_RECIPE_LIMIT
                + ALL_AUTOCOMPLETE_COLLECTION_LIMIT
                + ALL_AUTOCOMPLETE_USER_LIMIT,
            MAX_AUTOCOMPLETE_SUGGESTIONS
        );
    }

    #[test]
    fn validates_collection_and_user_advanced_fields() {
        let collection_request = CollectionSearchRequest {
            advanced: Some(AdvancedCollectionSearchCriteria {
                name: AdvancedTextField {
                    include: Some(TextCriterion {
                        text: "   ".to_string(),
                        mode: TextMatchMode::ExactPhrase,
                    }),
                    exclude: Vec::new(),
                },
                ..Default::default()
            }),
            ..Default::default()
        };
        assert!(matches!(
            validate_collection_search_request(&collection_request),
            Err(SearchValidationError::EmptyText { field: "name" })
        ));

        let user_request = UserSearchRequest {
            advanced: Some(AdvancedUserSearchCriteria {
                bio: AdvancedTextField {
                    include: None,
                    exclude: vec![TextCriterion {
                        text: "a".repeat(MAX_SEARCH_TEXT_CHARS + 1),
                        mode: TextMatchMode::AllTerms,
                    }],
                },
                ..Default::default()
            }),
            ..Default::default()
        };
        assert!(matches!(
            validate_user_search_request(&user_request),
            Err(SearchValidationError::TextTooLong { field: "bio", .. })
        ));
    }

    #[test]
    fn request_deserialization_rejects_unknown_capabilities() {
        let value = serde_json::json!({
            "text": "pasta",
            "advanced": null,
            "visibility": "all_visible",
            "tags_all": [],
            "total_time": null,
            "servings": null,
            "sort": "relevance",
            "page": 1,
            "page_size": 20,
            "sql": "DROP TABLE recipes"
        });

        assert!(serde_json::from_value::<RecipeSearchRequest>(value).is_err());

        let scope_inapplicable_sort = serde_json::json!({
            "sort": "username_ascending"
        });
        assert!(serde_json::from_value::<RecipeSearchRequest>(scope_inapplicable_sort).is_err());
    }
}
