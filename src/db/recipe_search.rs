//! Fixed recipe-search queries.
//!
//! This module deliberately exposes no generic query language. Every SQL fragment is selected
//! from a closed set below and every request value is passed through `QueryBuilder::push_bind`.

use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use super::DbError;
use crate::search::{normalize_search_tag, validate_recipe_search_request};
use crate::types::{
    AdvancedRecipeSearchCriteria, IntegerRange, IntegerRangeFacetBucket, Recipe, RecipeEquipment,
    RecipeImage, RecipeIngredient, RecipeSearchFacets, RecipeSearchRequest, RecipeSearchResponse,
    RecipeSearchSort, RecipeSearchVisibility, RecipeStep, RecipeVisibilityFacetValue, SearchResult,
    TagFacetBucket, TextCriterion, TextMatchMode, VisibilityFacetBucket,
};

const RESULT_COLUMNS: &str = "r.id, r.user_id, r.title, r.description, r.commentary, \
    r.prep_time_minutes, r.cook_time_minutes, r.servings, r.ingredients, r.instructions, \
    r.equipment, r.images, r.visibility, r.created_at, r.updated_at, \
    u.username AS author_username, u.avatar_url AS author_avatar_url";

const TITLE_WORDS: &str = "(r.title::pdb.simple('alias=recipe_title_words'))";
const TITLE_PARTIAL: &str = "(r.title::pdb.ngram(2, 15, 'alias=recipe_title_partial'))";
const DESCRIPTION_WORDS: &str = "(r.description::pdb.simple('alias=recipe_description_words'))";
const DESCRIPTION_PARTIAL: &str =
    "(r.description::pdb.ngram(2, 15, 'alias=recipe_description_partial'))";
const INGREDIENTS_WORDS: &str = "(jsonb_path_query_array(r.ingredients, \
    'strict $.** ? (@.type() == \"string\")')::pdb.simple('alias=recipe_ingredients_words'))";
const INSTRUCTIONS_WORDS: &str = "(jsonb_path_query_array(r.instructions, \
    'strict $.**.text')::pdb.simple('alias=recipe_instructions_words'))";

#[derive(Clone, Copy)]
enum RecipeTextField {
    Title,
    Description,
    Ingredients,
    Instructions,
}

impl RecipeTextField {
    fn word_expression(self) -> &'static str {
        match self {
            Self::Title => TITLE_WORDS,
            Self::Description => DESCRIPTION_WORDS,
            Self::Ingredients => INGREDIENTS_WORDS,
            Self::Instructions => INSTRUCTIONS_WORDS,
        }
    }

    fn partial_expression(self) -> Option<&'static str> {
        match self {
            Self::Title => Some(TITLE_PARTIAL),
            Self::Description => Some(DESCRIPTION_PARTIAL),
            Self::Ingredients | Self::Instructions => None,
        }
    }

    fn weight(self) -> i32 {
        match self {
            Self::Title => 3,
            Self::Description => 2,
            Self::Ingredients | Self::Instructions => 1,
        }
    }
}

#[derive(Clone, Copy, Default)]
struct OmittedFilter {
    visibility: bool,
    total_time: bool,
    servings: bool,
}

#[derive(sqlx::FromRow)]
struct SearchRecipeRow {
    id: Uuid,
    user_id: Uuid,
    title: String,
    description: Option<String>,
    commentary: Option<String>,
    prep_time_minutes: Option<i32>,
    cook_time_minutes: Option<i32>,
    servings: Option<i32>,
    ingredients: serde_json::Value,
    instructions: serde_json::Value,
    equipment: serde_json::Value,
    images: serde_json::Value,
    visibility: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    author_username: String,
    author_avatar_url: Option<String>,
    relevance: Option<f32>,
}

impl SearchRecipeRow {
    fn into_result(self) -> Result<SearchResult, DbError> {
        Ok(SearchResult {
            recipe: Recipe {
                id: self.id,
                user_id: self.user_id,
                title: self.title,
                description: self.description,
                commentary: self.commentary,
                prep_time_minutes: self.prep_time_minutes,
                cook_time_minutes: self.cook_time_minutes,
                servings: self.servings,
                ingredients: serde_json::from_value::<Vec<RecipeIngredient>>(self.ingredients)
                    .map_err(DbError::SerdeJson)?,
                instructions: serde_json::from_value::<Vec<RecipeStep>>(self.instructions)
                    .map_err(DbError::SerdeJson)?,
                equipment: serde_json::from_value::<Vec<RecipeEquipment>>(self.equipment)
                    .map_err(DbError::SerdeJson)?,
                images: serde_json::from_value::<Vec<RecipeImage>>(self.images)
                    .map_err(DbError::SerdeJson)?,
                visibility: self.visibility,
                created_at: self.created_at,
                updated_at: self.updated_at,
                author_username: self.author_username,
                author_avatar_url: self.author_avatar_url,
            },
            relevance: self.relevance,
        })
    }
}

/// Search the recipes visible to `resolved_user_id` using the fixed recipe contract.
pub async fn search_recipes(
    pool: &PgPool,
    resolved_user_id: Uuid,
    request: &RecipeSearchRequest,
) -> Result<RecipeSearchResponse, DbError> {
    // Server functions validate before entering the DB layer. Keeping this check here protects
    // internal callers as well and prevents invalid pagination arithmetic or unbounded requests.
    validate_recipe_search_request(request)
        .map_err(|error| DbError::InvalidSearchRequest(error.to_string()))?;

    let positive_text = has_positive_text(request);
    let results = fetch_results(pool, resolved_user_id, request, positive_text).await?;
    let total_count = fetch_count(pool, resolved_user_id, request).await?;
    let tags = fetch_tag_facets(pool, resolved_user_id, request).await?;
    let visibility = fetch_visibility_facets(pool, resolved_user_id, request).await?;
    let total_time =
        fetch_integer_facets(pool, resolved_user_id, request, IntegerFacet::TotalTime).await?;
    let servings =
        fetch_integer_facets(pool, resolved_user_id, request, IntegerFacet::Servings).await?;

    Ok(RecipeSearchResponse {
        results,
        total_count,
        facets: RecipeSearchFacets {
            tags,
            visibility,
            total_time,
            servings,
        },
    })
}

/// Fetch the lightweight recipe section used by grouped search.
///
/// This deliberately skips recipe facets, which are not part of the grouped response.
pub(super) async fn search_recipe_section(
    pool: &PgPool,
    resolved_user_id: Uuid,
    text: &str,
    limit: u32,
) -> Result<(Vec<SearchResult>, i64), DbError> {
    let request = RecipeSearchRequest {
        text: Some(text.to_owned()),
        page_size: limit,
        ..RecipeSearchRequest::default()
    };
    validate_recipe_search_request(&request)
        .map_err(|error| DbError::InvalidSearchRequest(error.to_string()))?;

    let positive_text = has_positive_text(&request);
    let results = fetch_results(pool, resolved_user_id, &request, positive_text).await?;
    let total_count = fetch_count(pool, resolved_user_id, &request).await?;
    Ok((results, total_count))
}

async fn fetch_results(
    pool: &PgPool,
    user_id: Uuid,
    request: &RecipeSearchRequest,
    positive_text: bool,
) -> Result<Vec<SearchResult>, DbError> {
    let mut query = QueryBuilder::<Postgres>::new("SELECT ");
    query.push(RESULT_COLUMNS).push(", ");
    if positive_text {
        query.push("pdb.score(r.id)::real AS relevance ");
    } else {
        query.push("NULL::real AS relevance ");
    }
    query.push("FROM recipes r JOIN users u ON u.id = r.user_id WHERE ");
    push_criteria(&mut query, user_id, request, OmittedFilter::default());
    push_sort(&mut query, request.sort, positive_text);
    let offset = (request.page - 1) * request.page_size;
    query
        .push(" LIMIT ")
        .push_bind(i64::from(request.page_size))
        .push(" OFFSET ")
        .push_bind(i64::from(offset));

    query
        .build_query_as::<SearchRecipeRow>()
        .fetch_all(pool)
        .await
        .map_err(DbError::Query)?
        .into_iter()
        .map(SearchRecipeRow::into_result)
        .collect()
}

async fn fetch_count(
    pool: &PgPool,
    user_id: Uuid,
    request: &RecipeSearchRequest,
) -> Result<i64, DbError> {
    let mut query = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM recipes r WHERE ");
    push_criteria(&mut query, user_id, request, OmittedFilter::default());
    query
        .build_query_scalar::<i64>()
        .fetch_one(pool)
        .await
        .map_err(DbError::Query)
}

async fn fetch_tag_facets(
    pool: &PgPool,
    user_id: Uuid,
    request: &RecipeSearchRequest,
) -> Result<Vec<TagFacetBucket>, DbError> {
    let mut query = QueryBuilder::<Postgres>::new(
        "SELECT rt.tag, COUNT(*)::bigint AS count FROM recipe_tags rt \
         JOIN recipes r ON r.id = rt.recipe_id WHERE ",
    );
    push_criteria(&mut query, user_id, request, OmittedFilter::default());
    query.push(" GROUP BY rt.tag ORDER BY count DESC, rt.tag ASC LIMIT 50");
    let rows = query
        .build_query_as::<(String, i64)>()
        .fetch_all(pool)
        .await
        .map_err(DbError::Query)?;
    Ok(rows
        .into_iter()
        .map(|(tag, count)| TagFacetBucket { tag, count })
        .collect())
}

async fn fetch_visibility_facets(
    pool: &PgPool,
    user_id: Uuid,
    request: &RecipeSearchRequest,
) -> Result<Vec<VisibilityFacetBucket>, DbError> {
    let mut query = QueryBuilder::<Postgres>::new(
        "SELECT r.visibility, COUNT(*)::bigint FROM recipes r WHERE ",
    );
    push_criteria(
        &mut query,
        user_id,
        request,
        OmittedFilter {
            visibility: true,
            ..OmittedFilter::default()
        },
    );
    query.push(" GROUP BY r.visibility");
    let rows = query
        .build_query_as::<(String, i64)>()
        .fetch_all(pool)
        .await
        .map_err(DbError::Query)?;

    Ok(["private", "unlisted", "public"]
        .into_iter()
        .map(|value| VisibilityFacetBucket {
            visibility: match value {
                "private" => RecipeVisibilityFacetValue::Private,
                "unlisted" => RecipeVisibilityFacetValue::Unlisted,
                _ => RecipeVisibilityFacetValue::Public,
            },
            count: rows
                .iter()
                .find(|(visibility, _)| visibility == value)
                .map_or(0, |(_, count)| *count),
        })
        .collect())
}

#[derive(Clone, Copy)]
enum IntegerFacet {
    TotalTime,
    Servings,
}

impl IntegerFacet {
    fn buckets(self) -> &'static [(i32, Option<i32>)] {
        match self {
            Self::TotalTime => &[(0, Some(30)), (30, Some(60)), (60, Some(120)), (120, None)],
            Self::Servings => &[(1, Some(3)), (3, Some(5)), (5, Some(9)), (9, None)],
        }
    }
}

async fn fetch_integer_facets(
    pool: &PgPool,
    user_id: Uuid,
    request: &RecipeSearchRequest,
    facet: IntegerFacet,
) -> Result<Vec<IntegerRangeFacetBucket>, DbError> {
    let omitted = match facet {
        IntegerFacet::TotalTime => OmittedFilter {
            total_time: true,
            ..OmittedFilter::default()
        },
        IntegerFacet::Servings => OmittedFilter {
            servings: true,
            ..OmittedFilter::default()
        },
    };

    let mut counts = Vec::with_capacity(4);
    for &(lower, upper) in facet.buckets() {
        let mut query = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM recipes r WHERE ");
        push_criteria(&mut query, user_id, request, omitted);
        query.push(" AND ");
        match facet {
            IntegerFacet::TotalTime => {
                query
                    .push(
                        "(r.prep_time_minutes IS NOT NULL OR r.cook_time_minutes IS NOT NULL) AND \
                    (COALESCE(r.prep_time_minutes, 0) + COALESCE(r.cook_time_minutes, 0)) >= ",
                    )
                    .push_bind(lower);
                if let Some(upper) = upper {
                    query
                        .push(" AND (COALESCE(r.prep_time_minutes, 0) + COALESCE(r.cook_time_minutes, 0)) < ")
                        .push_bind(upper);
                }
            }
            IntegerFacet::Servings => {
                query
                    .push("r.servings IS NOT NULL AND r.servings >= ")
                    .push_bind(lower);
                if let Some(upper) = upper {
                    query.push(" AND r.servings < ").push_bind(upper);
                }
            }
        }
        counts.push(
            query
                .build_query_scalar::<i64>()
                .fetch_one(pool)
                .await
                .map_err(DbError::Query)?,
        );
    }

    Ok(facet
        .buckets()
        .iter()
        .zip(counts)
        .map(|(&(lower, upper), count)| IntegerRangeFacetBucket {
            lower_inclusive: Some(lower),
            upper_exclusive: upper,
            count,
        })
        .collect())
}

fn push_criteria(
    query: &mut QueryBuilder<'_, Postgres>,
    user_id: Uuid,
    request: &RecipeSearchRequest,
    omitted: OmittedFilter,
) {
    // Authorization is invariant and is applied before every result, count, and facet variant.
    query
        .push("(r.user_id = ")
        .push_bind(user_id)
        .push(" OR r.visibility = 'public')");

    if !omitted.visibility {
        match request.visibility {
            RecipeSearchVisibility::AllVisible => {}
            RecipeSearchVisibility::MineOnly => {
                query.push(" AND r.user_id = ").push_bind(user_id);
            }
            RecipeSearchVisibility::PublicOnly => {
                query.push(" AND r.visibility = 'public'");
            }
        }
    }

    if let Some(text) = request.text.as_deref() {
        for token in text.split_whitespace() {
            query.push(" AND (");
            push_simple_token(query, token);
            query.push(")");
        }
    }

    if let Some(advanced) = &request.advanced {
        push_advanced_text(query, advanced);
        push_tag_any(query, &advanced.tags_any);
        for tag in normalized_tags(&advanced.tags_excluded) {
            query
                .push(
                    " AND NOT EXISTS (SELECT 1 FROM recipe_tags excluded_tag WHERE \
                    excluded_tag.recipe_id = r.id AND excluded_tag.tag = ",
                )
                .push_bind(tag)
                .push(")");
        }
        push_timestamp_range(query, "r.created_at", advanced.created_at);
        push_timestamp_range(query, "r.updated_at", advanced.updated_at);
    }

    for tag in normalized_tags(&request.tags_all) {
        query
            .push(
                " AND EXISTS (SELECT 1 FROM recipe_tags required_tag WHERE \
                required_tag.recipe_id = r.id AND required_tag.tag = ",
            )
            .push_bind(tag)
            .push(")");
    }

    if !omitted.total_time {
        if let Some(range) = request.total_time {
            query.push(" AND (r.prep_time_minutes IS NOT NULL OR r.cook_time_minutes IS NOT NULL)");
            push_integer_range(
                query,
                "(COALESCE(r.prep_time_minutes, 0) + COALESCE(r.cook_time_minutes, 0))",
                range,
            );
        }
    }
    if !omitted.servings {
        if let Some(range) = request.servings {
            query.push(" AND r.servings IS NOT NULL");
            push_integer_range(query, "r.servings", range);
        }
    }
}

fn push_simple_token(query: &mut QueryBuilder<'_, Postgres>, token: &str) {
    for (index, field) in [
        RecipeTextField::Title,
        RecipeTextField::Description,
        RecipeTextField::Ingredients,
        RecipeTextField::Instructions,
    ]
    .into_iter()
    .enumerate()
    {
        if index > 0 {
            query.push(" OR ");
        }
        push_field_term(query, field, token, true);
    }
}

fn push_advanced_text(
    query: &mut QueryBuilder<'_, Postgres>,
    advanced: &AdvancedRecipeSearchCriteria,
) {
    for (field, criteria) in [
        (RecipeTextField::Title, &advanced.title),
        (RecipeTextField::Description, &advanced.description),
        (RecipeTextField::Ingredients, &advanced.ingredients),
        (RecipeTextField::Instructions, &advanced.instructions),
    ] {
        if let Some(include) = &criteria.include {
            query.push(" AND (");
            push_text_criterion(query, field, include, true);
            query.push(")");
        }
        for exclusion in &criteria.exclude {
            query.push(" AND NOT (");
            push_text_criterion(query, field, exclusion, false);
            query.push(")");
        }
    }
}

fn push_text_criterion(
    query: &mut QueryBuilder<'_, Postgres>,
    field: RecipeTextField,
    criterion: &TextCriterion,
    score: bool,
) {
    match criterion.mode {
        TextMatchMode::ExactPhrase => {
            query
                .push(field.word_expression())
                .push(" @@@ pdb.phrase(")
                .push_bind(criterion.text.trim().to_owned())
                .push(")");
            if score {
                query.push("::pdb.boost(").push(field.weight()).push(")");
            }
        }
        TextMatchMode::AllTerms => {
            for (index, term) in criterion.text.split_whitespace().enumerate() {
                if index > 0 {
                    query.push(" AND ");
                }
                query.push("(");
                push_field_term(query, field, term, score);
                query.push(")");
            }
        }
        TextMatchMode::AnyTerm => {
            for (index, term) in criterion.text.split_whitespace().enumerate() {
                if index > 0 {
                    query.push(" OR ");
                }
                query.push("(");
                push_field_term(query, field, term, score);
                query.push(")");
            }
        }
    }
}

fn push_field_term(
    query: &mut QueryBuilder<'_, Postgres>,
    field: RecipeTextField,
    term: &str,
    score: bool,
) {
    query
        .push(field.word_expression())
        .push(" @@@ pdb.match_conjunction(")
        .push_bind(term.to_owned())
        .push(")");
    if score {
        query.push("::pdb.boost(").push(field.weight()).push(")");
    }
    if let Some(partial) = field.partial_expression() {
        query
            .push(" OR ")
            .push(partial)
            .push(" @@@ pdb.match_conjunction(")
            .push_bind(term.to_owned())
            .push(")");
        if score {
            query.push("::pdb.boost(").push(field.weight()).push(")");
        }
    }
}

fn push_tag_any(query: &mut QueryBuilder<'_, Postgres>, tags: &[String]) {
    let tags = normalized_tags(tags);
    if tags.is_empty() {
        return;
    }
    query
        .push(
            " AND EXISTS (SELECT 1 FROM recipe_tags any_tag WHERE \
            any_tag.recipe_id = r.id AND any_tag.tag = ANY(",
        )
        .push_bind(tags)
        .push("))");
}

fn normalized_tags(tags: &[String]) -> Vec<String> {
    let mut normalized: Vec<_> = tags.iter().map(|tag| normalize_search_tag(tag)).collect();
    normalized.sort();
    normalized.dedup();
    normalized
}

fn push_integer_range(
    query: &mut QueryBuilder<'_, Postgres>,
    expression: &'static str,
    range: IntegerRange,
) {
    if let Some(lower) = range.lower_inclusive {
        query
            .push(" AND ")
            .push(expression)
            .push(" >= ")
            .push_bind(lower);
    }
    if let Some(upper) = range.upper_exclusive {
        query
            .push(" AND ")
            .push(expression)
            .push(" < ")
            .push_bind(upper);
    }
}

fn push_timestamp_range(
    query: &mut QueryBuilder<'_, Postgres>,
    expression: &'static str,
    range: Option<crate::types::TimestampRange>,
) {
    if let Some(range) = range {
        if let Some(lower) = range.lower_inclusive {
            query
                .push(" AND ")
                .push(expression)
                .push(" >= ")
                .push_bind(lower);
        }
        if let Some(upper) = range.upper_exclusive {
            query
                .push(" AND ")
                .push(expression)
                .push(" < ")
                .push_bind(upper);
        }
    }
}

fn push_sort(
    query: &mut QueryBuilder<'_, Postgres>,
    requested_sort: RecipeSearchSort,
    positive_text: bool,
) {
    let sort = if requested_sort == RecipeSearchSort::Relevance && !positive_text {
        RecipeSearchSort::UpdatedNewest
    } else {
        requested_sort
    };
    match sort {
        RecipeSearchSort::Relevance => query.push(" ORDER BY relevance DESC, r.id ASC"),
        RecipeSearchSort::UpdatedNewest => query.push(" ORDER BY r.updated_at DESC, r.id ASC"),
        RecipeSearchSort::UpdatedOldest => query.push(" ORDER BY r.updated_at ASC, r.id ASC"),
        RecipeSearchSort::TitleAscending => query.push(" ORDER BY lower(r.title) ASC, r.id ASC"),
        RecipeSearchSort::TitleDescending => query.push(" ORDER BY lower(r.title) DESC, r.id ASC"),
        RecipeSearchSort::Quickest => query.push(
            " ORDER BY (r.prep_time_minutes IS NULL AND r.cook_time_minutes IS NULL) ASC, \
             (COALESCE(r.prep_time_minutes, 0) + COALESCE(r.cook_time_minutes, 0)) ASC, r.id ASC",
        ),
        RecipeSearchSort::Longest => query.push(
            " ORDER BY (r.prep_time_minutes IS NULL AND r.cook_time_minutes IS NULL) ASC, \
             (COALESCE(r.prep_time_minutes, 0) + COALESCE(r.cook_time_minutes, 0)) DESC, r.id ASC",
        ),
    };
}

fn has_positive_text(request: &RecipeSearchRequest) -> bool {
    request
        .text
        .as_deref()
        .is_some_and(|text| !text.trim().is_empty())
        || request.advanced.as_ref().is_some_and(|advanced| {
            advanced.title.include.is_some()
                || advanced.description.include.is_some()
                || advanced.ingredients.include.is_some()
                || advanced.instructions.include.is_some()
        })
}
