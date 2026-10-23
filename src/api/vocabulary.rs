//! Maintained ingredient suggestions for authenticated recipe authors.
use dioxus::prelude::*;

#[server]
pub async fn ingredient_suggestions(
    text: String,
) -> Result<Vec<crate::vocabulary::Suggestion>, ServerFnError> {
    if text.chars().count() > crate::vocabulary::MAX_TEXT {
        return Err(ServerFnError::new("Ingredient query is too long"));
    }
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    crate::security::enforce_user_request_budget(
        user_id,
        crate::security::RequestClass::Autocomplete,
    )
    .map_err(|_| ServerFnError::new("Request rate limit exceeded"))?;
    crate::security::run_with_search_work_budget(async {
        crate::security::run_with_database_budget(
            crate::security::DATABASE_EXECUTION_BUDGET,
            crate::vocabulary::store::autocomplete(&crate::db::get_pool(), &text),
        )
        .await
        .map_err(|_| ServerFnError::new("Suggestions timed out"))?
        .map_err(|error| {
            tracing::error!(%error, "Ingredient suggestions failed");
            ServerFnError::new("Suggestions unavailable")
        })
    })
    .await
    .map_err(|_| ServerFnError::new("Suggestions are busy"))?
}
