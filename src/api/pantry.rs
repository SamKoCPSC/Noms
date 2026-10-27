//! Session-bound pantry inventory endpoints.
use crate::pantry::*;
use dioxus::prelude::*;
use uuid::Uuid;
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
    .map_err(|_| ServerFnError::new("Too many pantry requests. Wait a moment and retry."))?;
    Ok(id)
}
#[cfg(feature = "server")]
async fn budget<T>(
    future: impl std::future::Future<Output = Result<T, crate::db::pantry::PantryError>>,
) -> Result<T, ServerFnError> {
    crate::security::run_with_database_budget(crate::security::DATABASE_EXECUTION_BUDGET, future)
        .await
        .map_err(|_| {
            ServerFnError::new("This pantry request timed out. Retry to confirm its result.")
        })?
        .map_err(|e| ServerFnError::new(e.to_string()))
}
#[server]
pub async fn list_kitchens(
    expected_account_id: Option<Uuid>,
) -> Result<Vec<PantrySummary>, ServerFnError> {
    let owner = actor(expected_account_id, false).await?;
    budget(crate::db::pantry::summaries(&crate::db::get_pool(), owner)).await
}
#[server]
pub async fn get_kitchen(
    expected_account_id: Option<Uuid>,
    id: Uuid,
) -> Result<PantryKitchen, ServerFnError> {
    let owner = actor(expected_account_id, false).await?;
    budget(crate::db::pantry::get(&crate::db::get_pool(), owner, id)).await
}

#[server]
pub async fn find_pantry_recipes(
    expected_account_id: Option<Uuid>,
    pantry_id: Uuid,
    pantry_revision: i64,
    cursor: Option<String>,
) -> Result<PantryRecipePage, ServerFnError> {
    let owner = actor(expected_account_id, false).await?;
    crate::security::run_with_expensive_read_work_budget(budget(crate::db::pantry_recipes::find(
        &crate::db::get_pool(),
        owner,
        pantry_id,
        pantry_revision,
        cursor.as_deref(),
    )))
    .await
    .map_err(|_| ServerFnError::new("Recipe checks are busy. Please try again."))?
}
#[server]
pub async fn create_kitchen(
    expected_account_id: Option<Uuid>,
    request: PantryCreate,
) -> Result<PantryKitchen, ServerFnError> {
    let owner = actor(expected_account_id, true).await?;
    budget(crate::db::pantry::create(
        &crate::db::get_pool(),
        owner,
        request,
    ))
    .await
}
#[server]
pub async fn add_pantry_entry(
    expected_account_id: Option<Uuid>,
    profile_id: Uuid,
    kind: PantryKind,
    request_id: Uuid,
    stock: PantryStock,
) -> Result<(), ServerFnError> {
    let owner = actor(expected_account_id, true).await?;
    budget(crate::db::pantry::add(
        &crate::db::get_pool(),
        owner,
        profile_id,
        kind,
        request_id,
        stock,
    ))
    .await
}
#[server]
pub async fn update_pantry_entry(
    expected_account_id: Option<Uuid>,
    profile_id: Uuid,
    kind: PantryKind,
    item_id: Uuid,
    expected_revision: i64,
    change: PantryChange,
) -> Result<(), ServerFnError> {
    let owner = actor(expected_account_id, true).await?;
    budget(crate::db::pantry::change(
        &crate::db::get_pool(),
        owner,
        profile_id,
        kind,
        item_id,
        expected_revision,
        change,
    ))
    .await
}
#[server]
pub async fn remove_pantry_entry(
    expected_account_id: Option<Uuid>,
    profile_id: Uuid,
    kind: PantryKind,
    item_id: Uuid,
    expected_revision: i64,
) -> Result<(), ServerFnError> {
    let owner = actor(expected_account_id, true).await?;
    budget(crate::db::pantry::remove(
        &crate::db::get_pool(),
        owner,
        profile_id,
        kind,
        item_id,
        expected_revision,
    ))
    .await
}
#[server]
pub async fn edit_kitchen(
    expected_account_id: Option<Uuid>,
    profile_id: Uuid,
    expected_revision: i64,
    name: String,
    note: String,
) -> Result<(), ServerFnError> {
    let owner = actor(expected_account_id, true).await?;
    budget(crate::db::pantry::edit_profile(
        &crate::db::get_pool(),
        owner,
        profile_id,
        expected_revision,
        name,
        note,
    ))
    .await
}
#[server]
pub async fn delete_kitchen(
    expected_account_id: Option<Uuid>,
    profile_id: Uuid,
    expected_revision: i64,
) -> Result<(), ServerFnError> {
    let owner = actor(expected_account_id, true).await?;
    budget(crate::db::pantry::delete(
        &crate::db::get_pool(),
        owner,
        profile_id,
        expected_revision,
    ))
    .await
}
