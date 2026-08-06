//! Fixed owner-scoped collection-search queries.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use super::DbError;
use crate::search::validate_collection_search_request;
use crate::types::{
    AdvancedCollectionSearchCriteria, CollectionSearchRequest, CollectionSearchResponse,
    CollectionSearchResult, CollectionSearchSort, CollectionSearchSummary, TextCriterion,
    TextMatchMode,
};

const NAME_WORDS: &str = "(c.name::pdb.simple('alias=collection_name_words'))";
const NAME_PARTIAL: &str = "(c.name::pdb.ngram(2, 15, 'alias=collection_name_partial'))";
const DESCRIPTION_WORDS: &str = "(c.description::pdb.simple('alias=collection_description_words'))";
const DESCRIPTION_PARTIAL: &str =
    "(c.description::pdb.ngram(2, 15, 'alias=collection_description_partial'))";

#[derive(Clone, Copy)]
enum CollectionTextField {
    Name,
    Description,
}

impl CollectionTextField {
    fn word_expression(self) -> &'static str {
        match self {
            Self::Name => NAME_WORDS,
            Self::Description => DESCRIPTION_WORDS,
        }
    }

    fn partial_expression(self) -> &'static str {
        match self {
            Self::Name => NAME_PARTIAL,
            Self::Description => DESCRIPTION_PARTIAL,
        }
    }

    fn weight(self) -> i32 {
        match self {
            Self::Name => 3,
            Self::Description => 1,
        }
    }
}

#[derive(sqlx::FromRow)]
struct SearchCollectionRow {
    id: Uuid,
    name: String,
    description: Option<String>,
    updated_at: DateTime<Utc>,
    relevance: Option<f32>,
}

/// Search only collections owned by `resolved_user_id`.
pub async fn search_collections(
    pool: &PgPool,
    resolved_user_id: Uuid,
    request: &CollectionSearchRequest,
) -> Result<CollectionSearchResponse, DbError> {
    validate_collection_search_request(request)
        .map_err(|error| DbError::InvalidSearchRequest(error.to_string()))?;

    let positive_text = has_positive_text(request);
    let results = fetch_results(pool, resolved_user_id, request, positive_text).await?;
    let total_count = fetch_count(pool, resolved_user_id, request).await?;
    Ok(CollectionSearchResponse {
        results,
        total_count,
    })
}

pub(super) async fn search_collection_section(
    pool: &PgPool,
    resolved_user_id: Uuid,
    text: &str,
    limit: u32,
) -> Result<(Vec<CollectionSearchResult>, i64), DbError> {
    let request = CollectionSearchRequest {
        text: Some(text.to_owned()),
        page_size: limit,
        ..CollectionSearchRequest::default()
    };
    validate_collection_search_request(&request)
        .map_err(|error| DbError::InvalidSearchRequest(error.to_string()))?;
    let positive_text = has_positive_text(&request);
    let results = fetch_results(pool, resolved_user_id, &request, positive_text).await?;
    let total_count = fetch_count(pool, resolved_user_id, &request).await?;
    Ok((results, total_count))
}

async fn fetch_results(
    pool: &PgPool,
    user_id: Uuid,
    request: &CollectionSearchRequest,
    positive_text: bool,
) -> Result<Vec<CollectionSearchResult>, DbError> {
    let mut query =
        QueryBuilder::<Postgres>::new("SELECT c.id, c.name, c.description, c.updated_at, ");
    if positive_text {
        query.push("pdb.score(c.id)::real AS relevance ");
    } else {
        query.push("NULL::real AS relevance ");
    }
    query
        .push("FROM collections c WHERE c.owner_id = ")
        .push_bind(user_id);
    push_criteria(&mut query, request);
    push_sort(&mut query, request.sort, positive_text);
    let offset = (request.page - 1) * request.page_size;
    query
        .push(" LIMIT ")
        .push_bind(i64::from(request.page_size))
        .push(" OFFSET ")
        .push_bind(i64::from(offset));

    let rows = query
        .build_query_as::<SearchCollectionRow>()
        .fetch_all(pool)
        .await
        .map_err(DbError::Query)?;
    attach_breadcrumbs(pool, user_id, rows).await
}

async fn attach_breadcrumbs(
    pool: &PgPool,
    user_id: Uuid,
    rows: Vec<SearchCollectionRow>,
) -> Result<Vec<CollectionSearchResult>, DbError> {
    if rows.is_empty() {
        return Ok(Vec::new());
    }
    let collection_ids: Vec<_> = rows.iter().map(|row| row.id).collect();
    let breadcrumb_rows = sqlx::query_as::<_, (Uuid, String, i32)>(
        "WITH RECURSIVE ancestors AS (
             SELECT c.id AS collection_id, c.id AS ancestor_id, c.parent_collection_id,
                    c.name, 0 AS depth, ARRAY[c.id] AS path
             FROM collections c
             WHERE c.owner_id = $1 AND c.id = ANY($2)
             UNION ALL
             SELECT ancestors.collection_id, parent.id, parent.parent_collection_id,
                    parent.name, ancestors.depth + 1, ancestors.path || parent.id
             FROM ancestors
             INNER JOIN collections parent ON parent.id = ancestors.parent_collection_id
             WHERE parent.owner_id = $1 AND NOT parent.id = ANY(ancestors.path)
         )
         SELECT collection_id, name, depth
         FROM ancestors
         ORDER BY collection_id, depth DESC",
    )
    .bind(user_id)
    .bind(collection_ids)
    .fetch_all(pool)
    .await
    .map_err(DbError::Query)?;

    let mut breadcrumbs: HashMap<Uuid, Vec<String>> = HashMap::new();
    for (collection_id, name, _) in breadcrumb_rows {
        breadcrumbs.entry(collection_id).or_default().push(name);
    }

    Ok(rows
        .into_iter()
        .map(|row| CollectionSearchResult {
            breadcrumb: breadcrumbs.remove(&row.id).unwrap_or_default(),
            collection: CollectionSearchSummary {
                id: row.id,
                name: row.name,
                description: row.description,
                updated_at: row.updated_at,
            },
            relevance: row.relevance,
        })
        .collect())
}

async fn fetch_count(
    pool: &PgPool,
    user_id: Uuid,
    request: &CollectionSearchRequest,
) -> Result<i64, DbError> {
    let mut query =
        QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM collections c WHERE c.owner_id = ");
    query.push_bind(user_id);
    push_criteria(&mut query, request);
    query
        .build_query_scalar::<i64>()
        .fetch_one(pool)
        .await
        .map_err(DbError::Query)
}

fn push_criteria(query: &mut QueryBuilder<'_, Postgres>, request: &CollectionSearchRequest) {
    if let Some(text) = request.text.as_deref() {
        for token in text.split_whitespace() {
            query.push(" AND (");
            push_simple_token(query, token);
            query.push(")");
        }
    }
    if let Some(advanced) = &request.advanced {
        push_advanced_text(query, advanced);
        if let Some(range) = advanced.updated_at {
            if let Some(lower) = range.lower_inclusive {
                query.push(" AND c.updated_at >= ").push_bind(lower);
            }
            if let Some(upper) = range.upper_exclusive {
                query.push(" AND c.updated_at < ").push_bind(upper);
            }
        }
    }
}

fn push_simple_token(query: &mut QueryBuilder<'_, Postgres>, token: &str) {
    push_field_term(query, CollectionTextField::Name, token, true);
    query.push(" OR ");
    push_field_term(query, CollectionTextField::Description, token, true);
}

fn push_advanced_text(
    query: &mut QueryBuilder<'_, Postgres>,
    advanced: &AdvancedCollectionSearchCriteria,
) {
    for (field, criteria) in [
        (CollectionTextField::Name, &advanced.name),
        (CollectionTextField::Description, &advanced.description),
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
    field: CollectionTextField,
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
    field: CollectionTextField,
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
    query
        .push(" OR ")
        .push(field.partial_expression())
        .push(" @@@ pdb.match_conjunction(")
        .push_bind(term.to_owned())
        .push(")");
    if score {
        query.push("::pdb.boost(").push(field.weight()).push(")");
    }
}

fn push_sort(
    query: &mut QueryBuilder<'_, Postgres>,
    requested_sort: CollectionSearchSort,
    positive_text: bool,
) {
    let sort = if requested_sort == CollectionSearchSort::Relevance && !positive_text {
        CollectionSearchSort::UpdatedNewest
    } else {
        requested_sort
    };
    match sort {
        CollectionSearchSort::Relevance => query.push(" ORDER BY relevance DESC, c.id ASC"),
        CollectionSearchSort::UpdatedNewest => query.push(" ORDER BY c.updated_at DESC, c.id ASC"),
        CollectionSearchSort::UpdatedOldest => query.push(" ORDER BY c.updated_at ASC, c.id ASC"),
        CollectionSearchSort::NameAscending => query.push(" ORDER BY lower(c.name) ASC, c.id ASC"),
        CollectionSearchSort::NameDescending => {
            query.push(" ORDER BY lower(c.name) DESC, c.id ASC")
        }
    };
}

fn has_positive_text(request: &CollectionSearchRequest) -> bool {
    request
        .text
        .as_deref()
        .is_some_and(|text| !text.trim().is_empty())
        || request.advanced.as_ref().is_some_and(|advanced| {
            advanced.name.include.is_some() || advanced.description.include.is_some()
        })
}
