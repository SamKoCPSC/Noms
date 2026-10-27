//! Private library recipe checks. Stream names only, then hydrate one bounded card page.
use super::pantry::PantryError;
use crate::pantry::{feasibility::PantryMatcher, PantryRecipePage};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Datelike, Utc};
use futures_util::TryStreamExt;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use uuid::Uuid;

const PAGE_SIZE: usize = crate::security::MAX_COLLECTION_RECIPE_CARDS_PER_PAGE;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    pantry: Uuid,
    revision: i64,
    updated: DateTime<Utc>,
    recipe: Uuid,
}

fn decode_cursor(
    value: Option<&str>,
    pantry: Uuid,
    revision: i64,
) -> Result<Option<Cursor>, PantryError> {
    let invalid = || PantryError::Invalid("Invalid recipe page. Start from the first page.".into());
    let Some(value) = value else { return Ok(None) };
    if value.len() > 512 {
        return Err(invalid());
    }
    let bytes = URL_SAFE_NO_PAD.decode(value).map_err(|_| invalid())?;
    let cursor: Cursor = serde_json::from_slice(&bytes).map_err(|_| invalid())?;
    if cursor.pantry != pantry
        || cursor.revision != revision
        || cursor.recipe.is_nil()
        || !(1..=9999).contains(&cursor.updated.year())
    {
        return Err(invalid());
    }
    Ok(Some(cursor))
}

// UNION deduplicates before scanning, including memberships in nested collections.
// A saved unlisted recipe is readable; another user's private recipe never is.
// Ingredient names stay in Rust so Unicode folding is identical to recipe detail checks.
pub(super) const CANDIDATES: &str = "
WITH library AS (
    SELECT id FROM recipes WHERE user_id = $1
    UNION
    SELECT cr.recipe_id FROM collection_recipes cr
    JOIN collections c ON c.id = cr.collection_id
    WHERE c.owner_id = $1
)
SELECT r.id, r.updated_at,
       ARRAY(SELECT CASE WHEN jsonb_typeof(i->'name') = 'string' THEN i->>'name' END
             FROM jsonb_array_elements(CASE WHEN jsonb_typeof(r.ingredients) = 'array'
                  THEN r.ingredients ELSE '[]'::jsonb END) AS i) AS names
FROM library l JOIN recipes r ON r.id = l.id
WHERE (r.user_id = $1 OR r.visibility IN ('public', 'unlisted'))
  AND jsonb_typeof(r.ingredients) = 'array'
  AND octet_length(r.ingredients::text) <= $4
  AND ($2::timestamptz IS NULL OR (r.updated_at, r.id) < ($2, $3::uuid))
ORDER BY r.updated_at DESC, r.id DESC";

pub async fn find(
    pool: &PgPool,
    owner: Uuid,
    pantry_id: Uuid,
    revision: i64,
    cursor: Option<&str>,
) -> Result<PantryRecipePage, PantryError> {
    if pantry_id.is_nil() || revision < 0 {
        return Err(PantryError::Invalid("Invalid pantry reference.".into()));
    }
    let cursor = decode_cursor(cursor, pantry_id, revision)?;
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await?;
    let pantry = super::pantry::load(&mut tx, owner, pantry_id).await?;
    if pantry.revision != revision {
        return Err(PantryError::Conflict);
    }
    let matcher = PantryMatcher::new(&pantry.entries);
    let mut matches = Vec::new();
    {
        let mut candidates = sqlx::query(CANDIDATES)
            .bind(owner)
            .bind(cursor.as_ref().map(|c| c.updated))
            .bind(cursor.as_ref().map(|c| c.recipe))
            .bind(crate::security::MAX_RECIPE_SNAPSHOT_BYTES as i64)
            .fetch(&mut *tx);
        while let Some(row) = candidates.try_next().await? {
            let names: Vec<Option<String>> = row.try_get("names")?;
            if names
                .iter()
                .any(|n| n.as_ref().is_none_or(|n| n.trim().is_empty()))
            {
                continue;
            }
            if matcher.matches_names(names.iter().filter_map(|n| n.as_deref())) {
                matches.push((
                    row.try_get::<Uuid, _>("id")?,
                    row.try_get::<DateTime<Utc>, _>("updated_at")?,
                ));
                if matches.len() > PAGE_SIZE {
                    break;
                }
            }
        }
    }
    let next_cursor = if matches.len() > PAGE_SIZE {
        matches.truncate(PAGE_SIZE);
        let (recipe, updated) = *matches.last().expect("nonempty full page");
        Some(
            URL_SAFE_NO_PAD.encode(
                serde_json::to_vec(&Cursor {
                    pantry: pantry_id,
                    revision,
                    updated,
                    recipe,
                })
                .map_err(|_| PantryError::Limit)?,
            ),
        )
    } else {
        None
    };
    let ids: Vec<_> = matches.into_iter().map(|(id, _)| id).collect();
    let recipes = if ids.is_empty() {
        Vec::new()
    } else {
        let query = format!(
            "SELECT {} FROM recipes r {} WHERE r.id = ANY($1) \
             AND (r.user_id=$2 OR r.visibility IN ('public','unlisted')) \
             ORDER BY r.updated_at DESC,r.id DESC",
            super::recipe_card_read_columns("r", "u"),
            super::recipe_card_read_joins("r", "u"),
        );
        let rows = sqlx::query(&query)
            .bind(&ids)
            .bind(owner)
            .fetch_all(&mut *tx)
            .await?;
        rows.iter()
            .map(super::recipe_card_from_row)
            .collect::<Result<Vec<_>, _>>()?
    };
    crate::security::validate_collection_recipe_card_page(&recipes)
        .map_err(|_| PantryError::Limit)?;
    tx.commit().await?;
    Ok(PantryRecipePage {
        pantry_id,
        pantry_revision: revision,
        recipes,
        next_cursor,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pantry_recipes_cursor_is_bound_to_pantry_and_revision() {
        let pantry = Uuid::new_v4();
        let cursor = URL_SAFE_NO_PAD.encode(
            serde_json::to_vec(&Cursor {
                pantry,
                revision: 3,
                updated: Utc::now(),
                recipe: Uuid::new_v4(),
            })
            .unwrap(),
        );
        assert!(decode_cursor(Some(&cursor), pantry, 3).is_ok());
        assert!(decode_cursor(Some(&cursor), Uuid::new_v4(), 3).is_err());
        assert!(decode_cursor(Some(&cursor), pantry, 4).is_err());
    }
}
