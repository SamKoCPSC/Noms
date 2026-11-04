//! Presence-based put-away: never add quantities to stale pantry amounts.
use super::grocery::GroceryError;
use sqlx::{PgPool, Row};
use std::collections::HashSet;
use uuid::Uuid;

pub async fn transfer(
    pool: &PgPool,
    owner: Uuid,
    list_id: Uuid,
    revision: i64,
    pantry_id: Uuid,
    ids: Vec<Uuid>,
) -> Result<(), GroceryError> {
    let selected: HashSet<_> = ids.iter().copied().collect();
    if ids.is_empty() || ids.len() > crate::grocery::MAX_ITEMS || ids.len() != selected.len() {
        return Err(GroceryError::Invalid(
            "Select purchased ingredients to add to your pantry.".into(),
        ));
    }
    let mut tx = pool.begin().await?;
    sqlx::query_scalar::<_, i64>(
        "SELECT revision FROM kitchen_profiles WHERE id=$1 AND owner_id=$2 FOR UPDATE",
    )
    .bind(pantry_id)
    .bind(owner)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(GroceryError::PantryMissing)?;
    let current: i64 = sqlx::query_scalar(
        "SELECT revision FROM grocery_lists WHERE id=$1 AND owner_id=$2 FOR SHARE",
    )
    .bind(list_id)
    .bind(owner)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(GroceryError::Missing)?;
    if current != revision {
        return Err(GroceryError::Conflict);
    }
    let list = super::grocery::load_owned(&mut tx, owner, list_id).await?;
    let items: Vec<_> = list
        .items
        .iter()
        .filter(|i| selected.contains(&i.id))
        .collect();
    if items.len() != ids.len() || items.iter().any(|i| !i.checked) {
        return Err(GroceryError::Invalid("Only items marked purchased in this list can be added. Refresh the list and review your selection.".into()));
    }
    let existing = sqlx::query(
        "SELECT id,name,position FROM pantry_ingredients WHERE profile_id=$1 ORDER BY position,id",
    )
    .bind(pantry_id)
    .fetch_all(&mut *tx)
    .await?;
    let names: HashSet<String> = existing
        .iter()
        .map(|r| r.get::<String, _>("name").to_lowercase())
        .collect();
    let new_count = items
        .iter()
        .filter(|i| !names.contains(&i.fields.name.to_lowercase()))
        .count();
    if existing.len() + new_count > crate::pantry::MAX_INGREDIENTS {
        return Err(GroceryError::Limit);
    }
    let mut position = existing
        .iter()
        .map(|r| r.get::<i32, _>("position"))
        .max()
        .unwrap_or(-1)
        .checked_add(1)
        .ok_or(GroceryError::Limit)?;
    let purchased_names: HashSet<_> = items.iter().map(|i| i.fields.name.to_lowercase()).collect();
    let restock_ids: Vec<Uuid> = existing
        .iter()
        .filter(|r| purchased_names.contains(&r.get::<String, _>("name").to_lowercase()))
        .map(|r| r.get("id"))
        .collect();
    let updated = sqlx::query("UPDATE pantry_ingredients SET available=TRUE,revision=revision+1,updated_at=NOW() WHERE profile_id=$1 AND id=ANY($2) AND NOT available")
        .bind(pantry_id).bind(restock_ids).execute(&mut *tx).await?;
    let mut changed = updated.rows_affected() > 0;
    // Keep distinct purchased rows/units for a new name. Subsequent submissions
    // see that name as present, so retries cannot insert another copy.
    for item in items {
        if !names.contains(&item.fields.name.to_lowercase()) {
            item.fields.validate()?;
            sqlx::query("INSERT INTO pantry_ingredients(profile_id,name,amount,unit,quantity_zero,available,position) VALUES($1,$2,$3,$4,$5,TRUE,$6)")
                .bind(pantry_id).bind(&item.fields.name).bind(&item.fields.amount).bind(&item.fields.unit)
                .bind(crate::grocery::quantity_is_zero(&item.fields.amount)==Some(true)).bind(position).execute(&mut *tx).await?;
            position = position.checked_add(1).ok_or(GroceryError::Limit)?;
            changed = true;
        }
    }
    if changed {
        sqlx::query("UPDATE kitchen_profiles SET revision=revision+1,updated_at=NOW() WHERE id=$1")
            .bind(pantry_id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}
