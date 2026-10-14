#![allow(dead_code)]

use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};

use percent_encoding::{percent_decode_str, utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
use serde::{Deserialize, Serialize};

use crate::search::{
    normalize_search_tag, validate_advanced_url_size, validate_all_search_request,
    validate_collection_search_request, validate_recipe_search_request,
    validate_user_search_request,
};
use crate::types::{
    AdvancedCollectionSearchCriteria, AdvancedRecipeSearchCriteria, AdvancedUserSearchCriteria,
    AllSearchRequest, CollectionSearchRequest, CollectionSearchSort, IntegerRange,
    RecipeSearchRequest, RecipeSearchSort, RecipeSearchVisibility, SearchScope, UserSearchRequest,
    UserSearchSort,
};

const QUERY_VALUE_ENCODE_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

type ParsedParameters = (HashMap<String, Vec<String>>, HashMap<String, usize>);

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SearchRouteQuery {
    raw: String,
}

impl SearchRouteQuery {
    pub fn new(raw: impl Into<String>) -> Self {
        Self { raw: raw.into() }
    }

    pub fn as_str(&self) -> &str {
        &self.raw
    }
}

impl From<&str> for SearchRouteQuery {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl Display for SearchRouteQuery {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.raw)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActiveAdvancedCriteria {
    Recipes(AdvancedRecipeSearchCriteria),
    Collections(AdvancedCollectionSearchCriteria),
    Users(AdvancedUserSearchCriteria),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActiveSearchRequest {
    All(AllSearchRequest),
    Recipes(Box<RecipeSearchRequest>),
    Collections(CollectionSearchRequest),
    Users(UserSearchRequest),
}

impl ActiveSearchRequest {
    pub fn scope(&self) -> SearchScope {
        match self {
            Self::All(_) => SearchScope::All,
            Self::Recipes(_) => SearchScope::Recipes,
            Self::Collections(_) => SearchScope::Collections,
            Self::Users(_) => SearchScope::Users,
        }
    }

    pub fn text(&self) -> &str {
        match self {
            Self::All(request) => &request.text,
            Self::Recipes(request) => request.text.as_deref().unwrap_or_default(),
            Self::Collections(request) => request.text.as_deref().unwrap_or_default(),
            Self::Users(request) => request.text.as_deref().unwrap_or_default(),
        }
    }

    pub fn page(&self) -> u32 {
        match self {
            Self::All(_) => 1,
            Self::Recipes(request) => request.page,
            Self::Collections(request) => request.page,
            Self::Users(request) => request.page,
        }
    }

    pub fn advanced(&self) -> Option<ActiveAdvancedCriteria> {
        match self {
            Self::All(_) => None,
            Self::Recipes(request) => request
                .advanced
                .clone()
                .map(ActiveAdvancedCriteria::Recipes),
            Self::Collections(request) => request
                .advanced
                .clone()
                .map(ActiveAdvancedCriteria::Collections),
            Self::Users(request) => request.advanced.clone().map(ActiveAdvancedCriteria::Users),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedSearchQuery {
    pub request: ActiveSearchRequest,
    pub canonical_query: String,
}

impl ParsedSearchQuery {
    pub fn switch_scope(&self, scope: SearchScope) -> Result<Self, SearchUrlError> {
        let text = self.request.text().to_string();
        let request = match scope {
            SearchScope::All => ActiveSearchRequest::All(AllSearchRequest {
                text,
                ..AllSearchRequest::default()
            }),
            SearchScope::Recipes => ActiveSearchRequest::Recipes(Box::new(RecipeSearchRequest {
                text: nonblank_text(text),
                ..RecipeSearchRequest::default()
            })),
            SearchScope::Collections => ActiveSearchRequest::Collections(CollectionSearchRequest {
                text: nonblank_text(text),
                ..CollectionSearchRequest::default()
            }),
            SearchScope::Users => ActiveSearchRequest::Users(UserSearchRequest {
                text: nonblank_text(text),
                ..UserSearchRequest::default()
            }),
        };
        parsed_from_request(request)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchUrlError(pub String);

impl Display for SearchUrlError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for SearchUrlError {}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AdvancedEnvelope {
    version: u8,
    scope: String,
    criteria: serde_json::Value,
}

pub fn parse_search_query(raw: &str) -> Result<ParsedSearchQuery, SearchUrlError> {
    let (parameters, encoded_lengths) = parse_parameters(raw)?;
    let scope = match optional_parameter(&parameters, "scope")? {
        None | Some("all") => SearchScope::All,
        Some("recipes") => SearchScope::Recipes,
        Some("collections") => SearchScope::Collections,
        Some("users") => SearchScope::Users,
        Some(value) => return Err(invalid("scope", value)),
    };
    let text = optional_parameter(&parameters, "q")?
        .unwrap_or_default()
        .trim()
        .to_string();
    let page = parse_page(optional_parameter(&parameters, "page")?)?;
    let advanced = parse_advanced(
        scope,
        optional_parameter(&parameters, "advanced")?,
        encoded_lengths.get("advanced").copied().unwrap_or_default(),
    )?;

    let request = match scope {
        SearchScope::All => {
            if advanced.is_some() {
                return Err(SearchUrlError(
                    "advanced criteria are not available for All search".to_string(),
                ));
            }
            ActiveSearchRequest::All(AllSearchRequest {
                text,
                ..AllSearchRequest::default()
            })
        }
        SearchScope::Recipes => {
            let recipe_advanced = match advanced {
                Some(ActiveAdvancedCriteria::Recipes(criteria)) => Some(criteria),
                None => None,
                _ => return Err(SearchUrlError("advanced scope mismatch".to_string())),
            };
            let tags_all = parse_tags(optional_parameter(&parameters, "tags")?)?;
            let total_time = parse_time(optional_parameter(&parameters, "time")?)?;
            let servings = parse_servings(optional_parameter(&parameters, "servings")?)?;
            let visibility = parse_visibility(optional_parameter(&parameters, "vis")?)?;
            let mut sort = parse_recipe_sort(optional_parameter(&parameters, "sort")?)?;
            if sort == RecipeSearchSort::Relevance
                && !has_positive_recipe_text(&text, recipe_advanced.as_ref())
            {
                sort = RecipeSearchSort::UpdatedNewest;
            }
            ActiveSearchRequest::Recipes(Box::new(RecipeSearchRequest {
                text: nonblank_text(text),
                advanced: recipe_advanced,
                visibility,
                tags_all,
                total_time,
                servings,
                sort,
                page,
                ..RecipeSearchRequest::default()
            }))
        }
        SearchScope::Collections => {
            let collection_advanced = match advanced {
                Some(ActiveAdvancedCriteria::Collections(criteria)) => Some(criteria),
                None => None,
                _ => return Err(SearchUrlError("advanced scope mismatch".to_string())),
            };
            let mut sort = parse_collection_sort(optional_parameter(&parameters, "sort")?)?;
            if sort == CollectionSearchSort::Relevance
                && !has_positive_collection_text(&text, collection_advanced.as_ref())
            {
                sort = CollectionSearchSort::UpdatedNewest;
            }
            ActiveSearchRequest::Collections(CollectionSearchRequest {
                text: nonblank_text(text),
                advanced: collection_advanced,
                sort,
                page,
                ..CollectionSearchRequest::default()
            })
        }
        SearchScope::Users => {
            let user_advanced = match advanced {
                Some(ActiveAdvancedCriteria::Users(criteria)) => Some(criteria),
                None => None,
                _ => return Err(SearchUrlError("advanced scope mismatch".to_string())),
            };
            let mut sort = parse_user_sort(optional_parameter(&parameters, "sort")?)?;
            if sort == UserSearchSort::Relevance
                && !has_positive_user_text(&text, user_advanced.as_ref())
            {
                sort = UserSearchSort::UsernameAscending;
            }
            ActiveSearchRequest::Users(UserSearchRequest {
                text: nonblank_text(text),
                advanced: user_advanced,
                sort,
                page,
                ..UserSearchRequest::default()
            })
        }
    };
    parsed_from_request(request)
}

pub fn parsed_from_request(
    mut request: ActiveSearchRequest,
) -> Result<ParsedSearchQuery, SearchUrlError> {
    normalize_request(&mut request);
    validate_request(&request)?;
    let canonical_query = canonical_query(&request)?;
    Ok(ParsedSearchQuery {
        request,
        canonical_query,
    })
}

fn normalize_request(request: &mut ActiveSearchRequest) {
    match request {
        ActiveSearchRequest::All(request) => request.text = request.text.trim().to_string(),
        ActiveSearchRequest::Recipes(request) => {
            request.text = request.text.take().and_then(nonblank_text);
            request.tags_all = normalized_tags(std::mem::take(&mut request.tags_all));
            if request.sort == RecipeSearchSort::Relevance
                && !has_positive_recipe_text(
                    request.text.as_deref().unwrap_or_default(),
                    request.advanced.as_ref(),
                )
            {
                request.sort = RecipeSearchSort::UpdatedNewest;
            }
        }
        ActiveSearchRequest::Collections(request) => {
            request.text = request.text.take().and_then(nonblank_text);
            if request.sort == CollectionSearchSort::Relevance
                && !has_positive_collection_text(
                    request.text.as_deref().unwrap_or_default(),
                    request.advanced.as_ref(),
                )
            {
                request.sort = CollectionSearchSort::UpdatedNewest;
            }
        }
        ActiveSearchRequest::Users(request) => {
            request.text = request.text.take().and_then(nonblank_text);
            if request.sort == UserSearchSort::Relevance
                && !has_positive_user_text(
                    request.text.as_deref().unwrap_or_default(),
                    request.advanced.as_ref(),
                )
            {
                request.sort = UserSearchSort::UsernameAscending;
            }
        }
    }
}

fn validate_request(request: &ActiveSearchRequest) -> Result<(), SearchUrlError> {
    let result = match request {
        ActiveSearchRequest::All(request) => validate_all_search_request(request),
        ActiveSearchRequest::Recipes(request) => validate_recipe_search_request(request),
        ActiveSearchRequest::Collections(request) => validate_collection_search_request(request),
        ActiveSearchRequest::Users(request) => validate_user_search_request(request),
    };
    result.map_err(|error| SearchUrlError(error.to_string()))
}

fn canonical_query(request: &ActiveSearchRequest) -> Result<String, SearchUrlError> {
    let mut pairs = Vec::<(&str, String)>::new();
    match request {
        ActiveSearchRequest::All(request) => {
            if !request.text.is_empty() {
                pairs.push(("q", request.text.clone()));
            }
        }
        ActiveSearchRequest::Recipes(request) => {
            pairs.push(("scope", "recipes".to_string()));
            push_text(&mut pairs, request.text.as_deref());
            if request.sort != RecipeSearchSort::Relevance {
                pairs.push(("sort", recipe_sort_name(request.sort).to_string()));
            }
            if !request.tags_all.is_empty() {
                pairs.push(("tags", request.tags_all.join(",")));
            }
            if let Some(range) = request.total_time {
                pairs.push(("time", time_name(range)?.to_string()));
            }
            if let Some(range) = request.servings {
                pairs.push(("servings", servings_name(range)?.to_string()));
            }
            if request.visibility != RecipeSearchVisibility::AllVisible {
                pairs.push(("vis", visibility_name(request.visibility).to_string()));
            }
            if let Some(advanced) = &request.advanced {
                pairs.push((
                    "advanced",
                    encode_advanced(
                        SearchScope::Recipes,
                        &ActiveAdvancedCriteria::Recipes(advanced.clone()),
                    )?,
                ));
            }
            push_page(&mut pairs, request.page);
        }
        ActiveSearchRequest::Collections(request) => {
            pairs.push(("scope", "collections".to_string()));
            push_text(&mut pairs, request.text.as_deref());
            if request.sort != CollectionSearchSort::Relevance {
                pairs.push(("sort", collection_sort_name(request.sort).to_string()));
            }
            if let Some(advanced) = &request.advanced {
                pairs.push((
                    "advanced",
                    encode_advanced(
                        SearchScope::Collections,
                        &ActiveAdvancedCriteria::Collections(advanced.clone()),
                    )?,
                ));
            }
            push_page(&mut pairs, request.page);
        }
        ActiveSearchRequest::Users(request) => {
            pairs.push(("scope", "users".to_string()));
            push_text(&mut pairs, request.text.as_deref());
            if request.sort != UserSearchSort::Relevance {
                pairs.push(("sort", user_sort_name(request.sort).to_string()));
            }
            if let Some(advanced) = &request.advanced {
                pairs.push((
                    "advanced",
                    encode_advanced(
                        SearchScope::Users,
                        &ActiveAdvancedCriteria::Users(advanced.clone()),
                    )?,
                ));
            }
            push_page(&mut pairs, request.page);
        }
    }
    Ok(pairs
        .into_iter()
        .map(|(key, value)| {
            if key == "advanced" {
                format!("{key}={value}")
            } else {
                format!("{key}={}", percent_encode(&value))
            }
        })
        .collect::<Vec<_>>()
        .join("&"))
}

pub fn encode_advanced(
    scope: SearchScope,
    advanced: &ActiveAdvancedCriteria,
) -> Result<String, SearchUrlError> {
    let (criteria_scope, criteria) = match advanced {
        ActiveAdvancedCriteria::Recipes(value) => (
            SearchScope::Recipes,
            serde_json::to_value(value).map_err(json_error)?,
        ),
        ActiveAdvancedCriteria::Collections(value) => (
            SearchScope::Collections,
            serde_json::to_value(value).map_err(json_error)?,
        ),
        ActiveAdvancedCriteria::Users(value) => (
            SearchScope::Users,
            serde_json::to_value(value).map_err(json_error)?,
        ),
    };
    if scope != criteria_scope {
        return Err(SearchUrlError("advanced scope mismatch".to_string()));
    }
    let json = serde_json::to_string(&AdvancedEnvelope {
        version: 1,
        scope: scope_name(scope).to_string(),
        criteria,
    })
    .map_err(json_error)?;
    let encoded = percent_encode(&json);
    validate_advanced_url_size(json.len(), encoded.len())
        .map_err(|error| SearchUrlError(error.to_string()))?;
    Ok(encoded)
}

fn parse_advanced(
    scope: SearchScope,
    encoded_json: Option<&str>,
    encoded_length: usize,
) -> Result<Option<ActiveAdvancedCriteria>, SearchUrlError> {
    let Some(json) = encoded_json else {
        return Ok(None);
    };
    validate_advanced_url_size(json.len(), encoded_length)
        .map_err(|error| SearchUrlError(error.to_string()))?;
    let envelope: AdvancedEnvelope = serde_json::from_str(json).map_err(json_error)?;
    if envelope.version != 1 {
        return Err(SearchUrlError(
            "unsupported advanced search version".to_string(),
        ));
    }
    if envelope.scope != scope_name(scope) {
        return Err(SearchUrlError("advanced scope mismatch".to_string()));
    }
    let advanced = match scope {
        SearchScope::Recipes => ActiveAdvancedCriteria::Recipes(
            serde_json::from_value(envelope.criteria).map_err(json_error)?,
        ),
        SearchScope::Collections => ActiveAdvancedCriteria::Collections(
            serde_json::from_value(envelope.criteria).map_err(json_error)?,
        ),
        SearchScope::Users => ActiveAdvancedCriteria::Users(
            serde_json::from_value(envelope.criteria).map_err(json_error)?,
        ),
        SearchScope::All => {
            return Err(SearchUrlError(
                "advanced criteria are not available for All search".to_string(),
            ));
        }
    };
    Ok(Some(advanced))
}

fn parse_parameters(raw: &str) -> Result<ParsedParameters, SearchUrlError> {
    let mut parameters: HashMap<String, Vec<String>> = HashMap::new();
    let mut encoded_lengths = HashMap::new();
    for pair in raw
        .trim_start_matches('?')
        .split('&')
        .filter(|pair| !pair.is_empty())
    {
        let (encoded_key, encoded_value) = pair.split_once('=').unwrap_or((pair, ""));
        let key = decode(encoded_key)?;
        let value = decode(encoded_value)?;
        encoded_lengths.insert(key.clone(), encoded_value.len());
        parameters.entry(key).or_default().push(value);
    }
    Ok((parameters, encoded_lengths))
}

fn optional_parameter<'a>(
    parameters: &'a HashMap<String, Vec<String>>,
    key: &str,
) -> Result<Option<&'a str>, SearchUrlError> {
    match parameters.get(key) {
        None => Ok(None),
        Some(values) if values.len() == 1 => Ok(values.first().map(String::as_str)),
        Some(_) => Err(SearchUrlError(format!("duplicate {key} parameter"))),
    }
}

fn decode(value: &str) -> Result<String, SearchUrlError> {
    let bytes = value.as_bytes();
    for (index, byte) in bytes.iter().enumerate() {
        if *byte == b'%'
            && (index + 2 >= bytes.len()
                || !bytes[index + 1].is_ascii_hexdigit()
                || !bytes[index + 2].is_ascii_hexdigit())
        {
            return Err(SearchUrlError(
                "query contains malformed percent encoding".to_string(),
            ));
        }
    }
    percent_decode_str(&value.replace('+', " "))
        .decode_utf8()
        .map(|value| value.into_owned())
        .map_err(|_| SearchUrlError("query contains invalid UTF-8".to_string()))
}

pub fn percent_encode(value: &str) -> String {
    utf8_percent_encode(value, QUERY_VALUE_ENCODE_SET).to_string()
}

pub fn scope_name(scope: SearchScope) -> &'static str {
    match scope {
        SearchScope::All => "all",
        SearchScope::Recipes => "recipes",
        SearchScope::Collections => "collections",
        SearchScope::Users => "users",
    }
}

fn nonblank_text(text: String) -> Option<String> {
    let trimmed = text.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn normalized_tags(tags: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut tags = tags
        .into_iter()
        .map(|tag| normalize_search_tag(&tag))
        .filter(|tag| seen.insert(tag.clone()))
        .collect::<Vec<_>>();
    tags.sort_by_key(|tag| tag.to_lowercase());
    tags
}

fn parse_tags(value: Option<&str>) -> Result<Vec<String>, SearchUrlError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let tags = normalized_tags(value.split(',').map(str::to_string).collect());
    if tags.iter().any(String::is_empty) {
        return Err(SearchUrlError("tags must not be blank".to_string()));
    }
    Ok(tags)
}

fn parse_page(value: Option<&str>) -> Result<u32, SearchUrlError> {
    value
        .map(|value| value.parse().map_err(|_| invalid("page", value)))
        .transpose()
        .map(|value| value.unwrap_or(1))
}

fn parse_time(value: Option<&str>) -> Result<Option<IntegerRange>, SearchUrlError> {
    value
        .map(|value| {
            Ok(match value {
                "0-30" => IntegerRange {
                    lower_inclusive: Some(0),
                    upper_exclusive: Some(30),
                },
                "30-60" => IntegerRange {
                    lower_inclusive: Some(30),
                    upper_exclusive: Some(60),
                },
                "60-120" => IntegerRange {
                    lower_inclusive: Some(60),
                    upper_exclusive: Some(120),
                },
                "120+" => IntegerRange {
                    lower_inclusive: Some(120),
                    upper_exclusive: None,
                },
                _ => return Err(invalid("time", value)),
            })
        })
        .transpose()
}

fn parse_servings(value: Option<&str>) -> Result<Option<IntegerRange>, SearchUrlError> {
    value
        .map(|value| {
            Ok(match value {
                "1-2" => IntegerRange {
                    lower_inclusive: Some(1),
                    upper_exclusive: Some(3),
                },
                "3-4" => IntegerRange {
                    lower_inclusive: Some(3),
                    upper_exclusive: Some(5),
                },
                "5-8" => IntegerRange {
                    lower_inclusive: Some(5),
                    upper_exclusive: Some(9),
                },
                "9+" => IntegerRange {
                    lower_inclusive: Some(9),
                    upper_exclusive: None,
                },
                _ => return Err(invalid("servings", value)),
            })
        })
        .transpose()
}

fn parse_visibility(value: Option<&str>) -> Result<RecipeSearchVisibility, SearchUrlError> {
    Ok(match value {
        None | Some("all") => RecipeSearchVisibility::AllVisible,
        Some("mine") => RecipeSearchVisibility::MineOnly,
        Some("public") => RecipeSearchVisibility::PublicOnly,
        Some(value) => return Err(invalid("vis", value)),
    })
}

fn parse_recipe_sort(value: Option<&str>) -> Result<RecipeSearchSort, SearchUrlError> {
    Ok(match value {
        None | Some("relevance") => RecipeSearchSort::Relevance,
        Some("updated_newest") => RecipeSearchSort::UpdatedNewest,
        Some("updated_oldest") => RecipeSearchSort::UpdatedOldest,
        Some("title_ascending") => RecipeSearchSort::TitleAscending,
        Some("title_descending") => RecipeSearchSort::TitleDescending,
        Some("quickest") => RecipeSearchSort::Quickest,
        Some("longest") => RecipeSearchSort::Longest,
        Some(value) => return Err(invalid("sort", value)),
    })
}

fn parse_collection_sort(value: Option<&str>) -> Result<CollectionSearchSort, SearchUrlError> {
    Ok(match value {
        None | Some("relevance") => CollectionSearchSort::Relevance,
        Some("updated_newest") => CollectionSearchSort::UpdatedNewest,
        Some("updated_oldest") => CollectionSearchSort::UpdatedOldest,
        Some("name_ascending") => CollectionSearchSort::NameAscending,
        Some("name_descending") => CollectionSearchSort::NameDescending,
        Some(value) => return Err(invalid("sort", value)),
    })
}

fn parse_user_sort(value: Option<&str>) -> Result<UserSearchSort, SearchUrlError> {
    Ok(match value {
        None | Some("relevance") => UserSearchSort::Relevance,
        Some("username_ascending") => UserSearchSort::UsernameAscending,
        Some("display_name_ascending") => UserSearchSort::DisplayNameAscending,
        Some(value) => return Err(invalid("sort", value)),
    })
}

fn recipe_sort_name(value: RecipeSearchSort) -> &'static str {
    match value {
        RecipeSearchSort::Relevance => "relevance",
        RecipeSearchSort::UpdatedNewest => "updated_newest",
        RecipeSearchSort::UpdatedOldest => "updated_oldest",
        RecipeSearchSort::TitleAscending => "title_ascending",
        RecipeSearchSort::TitleDescending => "title_descending",
        RecipeSearchSort::Quickest => "quickest",
        RecipeSearchSort::Longest => "longest",
    }
}

fn collection_sort_name(value: CollectionSearchSort) -> &'static str {
    match value {
        CollectionSearchSort::Relevance => "relevance",
        CollectionSearchSort::UpdatedNewest => "updated_newest",
        CollectionSearchSort::UpdatedOldest => "updated_oldest",
        CollectionSearchSort::NameAscending => "name_ascending",
        CollectionSearchSort::NameDescending => "name_descending",
    }
}

fn user_sort_name(value: UserSearchSort) -> &'static str {
    match value {
        UserSearchSort::Relevance => "relevance",
        UserSearchSort::UsernameAscending => "username_ascending",
        UserSearchSort::DisplayNameAscending => "display_name_ascending",
    }
}

fn time_name(range: IntegerRange) -> Result<&'static str, SearchUrlError> {
    match (range.lower_inclusive, range.upper_exclusive) {
        (Some(0), Some(30)) => Ok("0-30"),
        (Some(30), Some(60)) => Ok("30-60"),
        (Some(60), Some(120)) => Ok("60-120"),
        (Some(120), None) => Ok("120+"),
        _ => Err(SearchUrlError("unsupported total-time range".to_string())),
    }
}

fn servings_name(range: IntegerRange) -> Result<&'static str, SearchUrlError> {
    match (range.lower_inclusive, range.upper_exclusive) {
        (Some(1), Some(3)) => Ok("1-2"),
        (Some(3), Some(5)) => Ok("3-4"),
        (Some(5), Some(9)) => Ok("5-8"),
        (Some(9), None) => Ok("9+"),
        _ => Err(SearchUrlError("unsupported servings range".to_string())),
    }
}

fn visibility_name(value: RecipeSearchVisibility) -> &'static str {
    match value {
        RecipeSearchVisibility::AllVisible => "all",
        RecipeSearchVisibility::MineOnly => "mine",
        RecipeSearchVisibility::PublicOnly => "public",
    }
}

fn has_positive_recipe_text(text: &str, advanced: Option<&AdvancedRecipeSearchCriteria>) -> bool {
    !text.trim().is_empty()
        || advanced.is_some_and(|advanced| {
            advanced.title.include.is_some()
                || advanced.description.include.is_some()
                || advanced.ingredients.include.is_some()
                || advanced.instructions.include.is_some()
        })
}

fn has_positive_collection_text(
    text: &str,
    advanced: Option<&AdvancedCollectionSearchCriteria>,
) -> bool {
    !text.trim().is_empty()
        || advanced.is_some_and(|advanced| {
            advanced.name.include.is_some() || advanced.description.include.is_some()
        })
}

fn has_positive_user_text(text: &str, advanced: Option<&AdvancedUserSearchCriteria>) -> bool {
    !text.trim().is_empty()
        || advanced.is_some_and(|advanced| {
            advanced.username.include.is_some()
                || advanced.display_name.include.is_some()
                || advanced.bio.include.is_some()
        })
}

fn push_text(pairs: &mut Vec<(&'static str, String)>, text: Option<&str>) {
    if let Some(text) = text.filter(|text| !text.is_empty()) {
        pairs.push(("q", text.to_string()));
    }
}

fn push_page(pairs: &mut Vec<(&'static str, String)>, page: u32) {
    if page > 1 {
        pairs.push(("page", page.to_string()));
    }
}

fn invalid(field: &str, value: &str) -> SearchUrlError {
    SearchUrlError(format!("invalid {field} value: {value}"))
}

fn json_error(error: serde_json::Error) -> SearchUrlError {
    SearchUrlError(format!("invalid advanced search state: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::{MAX_ADVANCED_URL_DECODED_BYTES, MAX_SEARCH_TEXT_CHARS};
    use crate::types::{
        AdvancedCollectionSearchCriteria, AdvancedRecipeSearchCriteria, AdvancedTextField,
        AdvancedUserSearchCriteria, CollectionSearchSort, IntegerRange, RecipeSearchSort,
        RecipeSearchVisibility, TextCriterion, TextMatchMode, UserSearchSort,
    };

    fn criterion(text: &str) -> TextCriterion {
        TextCriterion {
            text: text.to_string(),
            mode: TextMatchMode::ExactPhrase,
        }
    }

    #[test]
    fn default_and_blank_queries_canonicalize_to_deterministic_browse_states() {
        let all = parse_search_query("").expect("default query should parse");
        assert!(matches!(all.request, ActiveSearchRequest::All(_)));
        assert_eq!(all.canonical_query, "");

        let recipes = parse_search_query("scope=recipes&q=%20%20&sort=relevance")
            .expect("blank recipe query should parse");
        let ActiveSearchRequest::Recipes(request) = &recipes.request else {
            panic!("expected recipe request");
        };
        assert_eq!(request.text, None);
        assert_eq!(request.sort, RecipeSearchSort::UpdatedNewest);
        assert_eq!(recipes.canonical_query, "scope=recipes&sort=updated_newest");

        let collections = parse_search_query("scope=collections&sort=relevance")
            .expect("blank collection query should parse");
        let ActiveSearchRequest::Collections(request) = &collections.request else {
            panic!("expected collection request");
        };
        assert_eq!(request.sort, CollectionSearchSort::UpdatedNewest);

        let users = parse_search_query("scope=users&sort=relevance")
            .expect("blank user query should parse");
        let ActiveSearchRequest::Users(request) = &users.request else {
            panic!("expected user request");
        };
        assert_eq!(request.sort, UserSearchSort::UsernameAscending);
    }

    #[test]
    fn recipe_query_round_trips_filters_sort_and_pagination_canonically() {
        let parsed = parse_search_query(
            "scope=recipes&q=weeknight%20pasta&sort=quickest&tags=vegan%2Cfamily&time=30-60&servings=3-4&vis=mine&page=2",
        )
        .expect("recipe query should parse");
        let ActiveSearchRequest::Recipes(request) = &parsed.request else {
            panic!("expected recipe request");
        };
        assert_eq!(request.text.as_deref(), Some("weeknight pasta"));
        assert_eq!(request.tags_all, vec!["family", "vegan"]);
        assert_eq!(
            request.total_time,
            Some(IntegerRange {
                lower_inclusive: Some(30),
                upper_exclusive: Some(60)
            })
        );
        assert_eq!(
            request.servings,
            Some(IntegerRange {
                lower_inclusive: Some(3),
                upper_exclusive: Some(5)
            })
        );
        assert_eq!(request.visibility, RecipeSearchVisibility::MineOnly);
        assert_eq!(request.sort, RecipeSearchSort::Quickest);
        assert_eq!(request.page, 2);

        let reparsed = parse_search_query(&parsed.canonical_query)
            .expect("canonical recipe query should round trip");
        assert_eq!(reparsed.request, parsed.request);
        assert_eq!(reparsed.canonical_query, parsed.canonical_query);
    }

    #[test]
    fn scope_switching_removes_inapplicable_state_and_resets_page() {
        let parsed = parse_search_query(
            "scope=users&q=chef&tags=vegan&time=0-30&servings=1-2&vis=public&sort=username_ascending&page=4",
        )
        .expect("user query should ignore recipe-only state");
        assert!(!parsed.canonical_query.contains("tags="));
        assert!(!parsed.canonical_query.contains("time="));
        assert!(!parsed.canonical_query.contains("servings="));
        assert!(!parsed.canonical_query.contains("vis="));

        let switched = parsed
            .switch_scope(crate::types::SearchScope::Collections)
            .expect("scope switch should remain valid");
        assert_eq!(
            switched.request.scope(),
            crate::types::SearchScope::Collections
        );
        assert_eq!(switched.request.page(), 1);
        assert_eq!(switched.request.text(), "chef");
        assert!(!switched.canonical_query.contains("page="));
    }

    #[test]
    fn advanced_payloads_round_trip_for_every_bounded_scope() {
        let recipe = AdvancedRecipeSearchCriteria {
            title: AdvancedTextField {
                include: Some(criterion("silky noodles")),
                exclude: vec![criterion("instant")],
            },
            ..AdvancedRecipeSearchCriteria::default()
        };
        let collection = AdvancedCollectionSearchCriteria {
            name: AdvancedTextField {
                include: Some(criterion("weeknight favorites")),
                exclude: Vec::new(),
            },
            ..AdvancedCollectionSearchCriteria::default()
        };
        let user = AdvancedUserSearchCriteria {
            username: AdvancedTextField {
                include: Some(criterion("chef sam")),
                exclude: Vec::new(),
            },
            ..AdvancedUserSearchCriteria::default()
        };

        for (scope, advanced) in [
            (
                crate::types::SearchScope::Recipes,
                ActiveAdvancedCriteria::Recipes(recipe.clone()),
            ),
            (
                crate::types::SearchScope::Collections,
                ActiveAdvancedCriteria::Collections(collection.clone()),
            ),
            (
                crate::types::SearchScope::Users,
                ActiveAdvancedCriteria::Users(user.clone()),
            ),
        ] {
            let encoded = encode_advanced(scope, &advanced).expect("advanced encoding failed");
            let parsed = parse_search_query(&format!(
                "scope={}&q=preserved&advanced={encoded}",
                scope_name(scope)
            ))
            .expect("advanced query should parse");
            assert_eq!(parsed.request.advanced(), Some(advanced.clone()));
            assert_eq!(parsed.request.text(), "preserved");
            assert_eq!(
                parse_search_query(&parsed.canonical_query)
                    .expect("advanced canonical query should round trip")
                    .request,
                parsed.request
            );
        }
    }

    #[test]
    fn malformed_explicit_state_never_builds_a_backend_request() {
        for query in [
            "scope=unknown",
            "scope=recipes&sort=username_ascending",
            "scope=recipes&time=bad",
            "scope=recipes&servings=0-0",
            "scope=recipes&vis=friends",
            "scope=users&page=0",
            "scope=recipes&q=a&q=b",
            "scope=all&advanced=%7B%7D",
            "scope=users&q=%ZZ",
        ] {
            assert!(
                parse_search_query(query).is_err(),
                "query should fail: {query}"
            );
        }
        assert!(
            parse_search_query(&format!("q={}", "x".repeat(MAX_SEARCH_TEXT_CHARS + 1))).is_err()
        );
    }

    #[test]
    fn advanced_version_scope_and_size_are_enforced() {
        let wrong_version = percent_encode(r#"{"version":2,"scope":"users","criteria":{}}"#);
        assert!(parse_search_query(&format!("scope=users&advanced={wrong_version}")).is_err());

        let wrong_scope = percent_encode(r#"{"version":1,"scope":"recipes","criteria":{}}"#);
        assert!(parse_search_query(&format!("scope=users&advanced={wrong_scope}")).is_err());

        let oversized_criterion = "x".repeat(MAX_ADVANCED_URL_DECODED_BYTES);
        let oversized = AdvancedUserSearchCriteria {
            bio: AdvancedTextField {
                include: Some(criterion(&oversized_criterion)),
                exclude: Vec::new(),
            },
            ..AdvancedUserSearchCriteria::default()
        };
        assert!(encode_advanced(
            crate::types::SearchScope::Users,
            &ActiveAdvancedCriteria::Users(oversized)
        )
        .is_err());
    }
}
