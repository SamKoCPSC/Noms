//! Session-owned grocery functions. Client account IDs are consistency guards only.
use crate::grocery::*;
use dioxus::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GroceryRecipeOption {
    pub id: Uuid,
    pub title: String,
    pub servings: Option<i32>,
    pub owned: bool,
}

#[cfg(feature = "server")]
async fn actor(expected: Option<Uuid>, mutation: bool) -> Result<Uuid, ServerFnError> {
    let id = crate::auth::account_binding::require_form_account(expected).await?;
    crate::security::enforce_user_request_budget(
        id,
        if mutation {
            crate::security::RequestClass::Mutation
        } else {
            crate::security::RequestClass::CollectionRead
        },
    )
    .map_err(|_| ServerFnError::new("Too many grocery requests. Wait a moment and retry."))?;
    Ok(id)
}
#[cfg(feature = "server")]
async fn budget<T>(
    future: impl std::future::Future<Output = Result<T, crate::db::grocery::GroceryError>>,
) -> Result<T, ServerFnError> {
    crate::security::run_with_database_budget(crate::security::DATABASE_EXECUTION_BUDGET, future)
        .await
        .map_err(|_| {
            ServerFnError::new("This grocery request timed out. Retry to confirm its result.")
        })?
        .map_err(|error| ServerFnError::new(error.to_string()))
}

#[server]
pub async fn list_grocery_lists(
    expected_account_id: Option<Uuid>,
) -> Result<Vec<GroceryListSummary>, ServerFnError> {
    let owner = actor(expected_account_id, false).await?;
    budget(crate::db::grocery::summaries(&crate::db::get_pool(), owner)).await
}
#[server]
pub async fn get_grocery_list(
    expected_account_id: Option<Uuid>,
    list_id: Uuid,
) -> Result<GroceryList, ServerFnError> {
    let owner = actor(expected_account_id, false).await?;
    budget(crate::db::grocery::get(
        &crate::db::get_pool(),
        owner,
        list_id,
    ))
    .await
}
#[server]
pub async fn create_grocery_list(
    expected_account_id: Option<Uuid>,
    request: GroceryCreate,
) -> Result<GroceryList, ServerFnError> {
    let owner = actor(expected_account_id, true).await?;
    budget(crate::db::grocery::create(
        &crate::db::get_pool(),
        owner,
        request,
    ))
    .await
}
#[server]
pub async fn preview_grocery_recipe(
    expected_account_id: Option<Uuid>,
    recipe_id: Uuid,
) -> Result<GroceryRecipePreview, ServerFnError> {
    let owner = actor(expected_account_id, false).await?;
    let pool = crate::db::get_pool();
    budget(async {
        let mut conn = pool.acquire().await?;
        crate::db::grocery::preview_source(&mut conn, owner, recipe_id).await
    })
    .await
}

/// Public discovery plus the user's own private/unlisted recipes. Other unlisted
/// recipes may be selected by their existing share link; never enumerate them.
#[server]
pub async fn find_grocery_recipes(
    expected_account_id: Option<Uuid>,
    query: String,
    offset: i64,
) -> Result<Vec<GroceryRecipeOption>, ServerFnError> {
    let owner = actor(expected_account_id, false).await?;
    if query.chars().count() > 256 || !(0..=10_000).contains(&offset) {
        return Err(ServerFnError::new("Invalid recipe search."));
    }
    let pool = crate::db::get_pool();
    budget(async {
        use sqlx::Row;
        let rows=sqlx::query("SELECT id,title,servings,user_id=$1 AS owned FROM recipes WHERE (user_id=$1 OR visibility='public') AND strpos(lower(title),lower($2)) > 0 ORDER BY (user_id=$1) DESC,updated_at DESC,id DESC LIMIT 25 OFFSET $3")
            .bind(owner).bind(query.trim()).bind(offset).fetch_all(&pool).await?;
        rows.into_iter().map(|r|Ok(GroceryRecipeOption{id:r.try_get("id")?,title:r.try_get("title")?,servings:r.try_get("servings")?,owned:r.try_get("owned")?})).collect()
    }).await
}
#[server]
pub async fn add_grocery_item(
    expected_account_id: Option<Uuid>,
    list_id: Uuid,
    request_id: Uuid,
    fields: ItemFields,
) -> Result<(), ServerFnError> {
    let owner = actor(expected_account_id, true).await?;
    budget(crate::db::grocery::add_item(
        &crate::db::get_pool(),
        owner,
        list_id,
        request_id,
        fields,
    ))
    .await
}
#[server]
pub async fn update_grocery_item(
    expected_account_id: Option<Uuid>,
    list_id: Uuid,
    item_id: Uuid,
    expected_revision: i64,
    change: GroceryItemChange,
) -> Result<(), ServerFnError> {
    let owner = actor(expected_account_id, true).await?;
    budget(crate::db::grocery::change_item(
        &crate::db::get_pool(),
        owner,
        list_id,
        item_id,
        expected_revision,
        change,
    ))
    .await
}
#[server]
pub async fn remove_grocery_item(
    expected_account_id: Option<Uuid>,
    list_id: Uuid,
    item_id: Uuid,
    expected_revision: i64,
) -> Result<(), ServerFnError> {
    let owner = actor(expected_account_id, true).await?;
    budget(crate::db::grocery::remove_item(
        &crate::db::get_pool(),
        owner,
        list_id,
        item_id,
        expected_revision,
    ))
    .await
}
#[server]
pub async fn rename_grocery_list(
    expected_account_id: Option<Uuid>,
    list_id: Uuid,
    expected_revision: i64,
    name: String,
) -> Result<(), ServerFnError> {
    let owner = actor(expected_account_id, true).await?;
    budget(crate::db::grocery::rename(
        &crate::db::get_pool(),
        owner,
        list_id,
        expected_revision,
        name,
    ))
    .await
}
#[server]
pub async fn delete_grocery_list(
    expected_account_id: Option<Uuid>,
    list_id: Uuid,
    expected_revision: i64,
) -> Result<(), ServerFnError> {
    let owner = actor(expected_account_id, true).await?;
    budget(crate::db::grocery::delete(
        &crate::db::get_pool(),
        owner,
        list_id,
        expected_revision,
    ))
    .await
}
