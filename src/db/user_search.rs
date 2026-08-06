//! Fixed user-search queries.
//!
//! Only the public user projection participates in this operation. Search expressions are chosen
//! from the closed set below, and all caller-provided values are bound through `QueryBuilder`.

use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use super::DbError;
use crate::search::validate_user_search_request;
use crate::types::{
    AdvancedUserSearchCriteria, TextCriterion, TextMatchMode, UserSearchRequest,
    UserSearchResponse, UserSearchResult, UserSearchSort, UserSearchSummary,
};

const USERNAME_WORDS: &str = "(u.username::pdb.simple('alias=user_username_words'))";
const USERNAME_PARTIAL: &str = "(u.username::pdb.ngram(2, 20, 'alias=user_username_partial'))";
const DISPLAY_NAME_WORDS: &str = "(u.display_name::pdb.simple('alias=user_display_name_words'))";
const DISPLAY_NAME_PARTIAL: &str =
    "(u.display_name::pdb.ngram(2, 20, 'alias=user_display_name_partial'))";
const BIO_WORDS: &str = "(u.bio::pdb.simple('alias=user_bio_words'))";

#[derive(Clone, Copy)]
enum UserTextField {
    Username,
    DisplayName,
    Bio,
}

impl UserTextField {
    fn word_expression(self) -> &'static str {
        match self {
            Self::Username => USERNAME_WORDS,
            Self::DisplayName => DISPLAY_NAME_WORDS,
            Self::Bio => BIO_WORDS,
        }
    }

    fn partial_expression(self) -> Option<&'static str> {
        match self {
            Self::Username => Some(USERNAME_PARTIAL),
            Self::DisplayName => Some(DISPLAY_NAME_PARTIAL),
            Self::Bio => None,
        }
    }

    fn weight(self) -> i32 {
        match self {
            Self::Username => 3,
            Self::DisplayName => 2,
            Self::Bio => 1,
        }
    }
}

#[derive(sqlx::FromRow)]
struct SearchUserRow {
    id: Uuid,
    username: String,
    display_name: String,
    avatar_url: Option<String>,
    bio: Option<String>,
    public_recipe_count: i64,
    relevance: Option<f32>,
}

impl SearchUserRow {
    fn into_result(self) -> UserSearchResult {
        UserSearchResult {
            user: UserSearchSummary {
                id: self.id,
                username: self.username,
                display_name: self.display_name,
                avatar_url: self.avatar_url,
                bio: self.bio,
                public_recipe_count: self.public_recipe_count,
            },
            relevance: self.relevance,
        }
    }
}

/// Search users through the application's fixed public-user projection.
pub async fn search_users(
    pool: &PgPool,
    request: &UserSearchRequest,
) -> Result<UserSearchResponse, DbError> {
    validate_user_search_request(request)
        .map_err(|error| DbError::InvalidSearchRequest(error.to_string()))?;

    let positive_text = has_positive_text(request);
    let results = fetch_results(pool, request, positive_text).await?;
    let total_count = fetch_count(pool, request).await?;

    Ok(UserSearchResponse {
        results,
        total_count,
    })
}

/// Fetch the lightweight public-user section used by grouped search.
pub(super) async fn search_user_section(
    pool: &PgPool,
    text: &str,
    limit: u32,
) -> Result<(Vec<UserSearchResult>, i64), DbError> {
    let request = UserSearchRequest {
        text: Some(text.to_owned()),
        page_size: limit,
        ..UserSearchRequest::default()
    };
    validate_user_search_request(&request)
        .map_err(|error| DbError::InvalidSearchRequest(error.to_string()))?;

    let positive_text = has_positive_text(&request);
    let results = fetch_results(pool, &request, positive_text).await?;
    let total_count = fetch_count(pool, &request).await?;
    Ok((results, total_count))
}

async fn fetch_results(
    pool: &PgPool,
    request: &UserSearchRequest,
    positive_text: bool,
) -> Result<Vec<UserSearchResult>, DbError> {
    let mut query = QueryBuilder::<Postgres>::new(
        "SELECT u.id, u.username, u.display_name, u.avatar_url, u.bio, \
         (SELECT COUNT(*) FROM recipes public_recipe \
          WHERE public_recipe.user_id = u.id AND public_recipe.visibility = 'public')::bigint \
          AS public_recipe_count, ",
    );
    if positive_text {
        query.push("pdb.score(u.id)::real AS relevance ");
    } else {
        query.push("NULL::real AS relevance ");
    }
    query.push("FROM users u WHERE TRUE");
    push_criteria(&mut query, request);
    push_sort(&mut query, request.sort, positive_text);
    let offset = (request.page - 1) * request.page_size;
    query
        .push(" LIMIT ")
        .push_bind(i64::from(request.page_size))
        .push(" OFFSET ")
        .push_bind(i64::from(offset));

    Ok(query
        .build_query_as::<SearchUserRow>()
        .fetch_all(pool)
        .await
        .map_err(DbError::Query)?
        .into_iter()
        .map(SearchUserRow::into_result)
        .collect())
}

async fn fetch_count(pool: &PgPool, request: &UserSearchRequest) -> Result<i64, DbError> {
    let mut query = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM users u WHERE TRUE");
    push_criteria(&mut query, request);
    query
        .build_query_scalar::<i64>()
        .fetch_one(pool)
        .await
        .map_err(DbError::Query)
}

fn push_criteria(query: &mut QueryBuilder<'_, Postgres>, request: &UserSearchRequest) {
    if let Some(text) = request.text.as_deref() {
        for token in text.split_whitespace() {
            query.push(" AND (");
            push_simple_token(query, token);
            query.push(")");
        }
    }

    if let Some(advanced) = &request.advanced {
        push_advanced_text(query, advanced);
    }
}

fn push_simple_token(query: &mut QueryBuilder<'_, Postgres>, token: &str) {
    for (index, field) in [
        UserTextField::Username,
        UserTextField::DisplayName,
        UserTextField::Bio,
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
    advanced: &AdvancedUserSearchCriteria,
) {
    for (field, criteria) in [
        (UserTextField::Username, &advanced.username),
        (UserTextField::DisplayName, &advanced.display_name),
        (UserTextField::Bio, &advanced.bio),
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
    field: UserTextField,
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
    field: UserTextField,
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

fn push_sort(
    query: &mut QueryBuilder<'_, Postgres>,
    requested_sort: UserSearchSort,
    positive_text: bool,
) {
    let sort = if requested_sort == UserSearchSort::Relevance && !positive_text {
        UserSearchSort::UsernameAscending
    } else {
        requested_sort
    };
    match sort {
        UserSearchSort::Relevance => query.push(" ORDER BY relevance DESC, u.id ASC"),
        UserSearchSort::UsernameAscending => {
            query.push(" ORDER BY lower(u.username) ASC, u.id ASC")
        }
        UserSearchSort::DisplayNameAscending => {
            query.push(" ORDER BY lower(u.display_name) ASC, u.id ASC")
        }
    };
}

fn has_positive_text(request: &UserSearchRequest) -> bool {
    request
        .text
        .as_deref()
        .is_some_and(|text| !text.trim().is_empty())
        || request.advanced.as_ref().is_some_and(|advanced| {
            advanced.username.include.is_some()
                || advanced.display_name.include.is_some()
                || advanced.bio.include.is_some()
        })
}
