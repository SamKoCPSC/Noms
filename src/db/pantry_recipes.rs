//! Private library checks. Stream names/optional flags, then hydrate bounded cards.
use super::pantry::PantryError;
use crate::pantry::{
    feasibility::{IngredientStatus, PantryMatcher},
    PantryRecipePage, RecipeShortage,
};
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
    #[serde(default)]
    rank: usize,
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
SELECT r.id, r.updated_at, r.ingredients AS authored,
       ARRAY(SELECT CASE WHEN jsonb_typeof(i->'name') = 'string' THEN i->>'name' END
             FROM jsonb_array_elements(CASE WHEN jsonb_typeof(r.ingredients) = 'array'
                  THEN r.ingredients ELSE '[]'::jsonb END) AS i) AS names,
       ARRAY(SELECT CASE WHEN NOT (i ? 'optional') THEN false
                    WHEN jsonb_typeof(i->'optional') = 'boolean' THEN i->'optional' = 'true'::jsonb END
             FROM jsonb_array_elements(CASE WHEN jsonb_typeof(r.ingredients) = 'array'
                  THEN r.ingredients ELSE '[]'::jsonb END) AS i) AS optional_flags
FROM library l JOIN recipes r ON r.id = l.id
WHERE (r.user_id = $1 OR r.visibility IN ('public', 'unlisted'))
  AND jsonb_typeof(r.ingredients) = 'array'
  AND octet_length(r.ingredients::text) <= $2
ORDER BY r.updated_at DESC, r.id DESC";

pub async fn find(
    pool: &PgPool,
    owner: Uuid,
    pantry_id: Uuid,
    revision: i64,
    cursor: Option<&str>,
    almost: bool,
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
    if cursor
        .as_ref()
        .is_some_and(|c| c.rank > 2 || (c.rank > 0) != almost)
    {
        return Err(PantryError::Invalid("Invalid recipe page.".into()));
    }
    let mut totals = [0; 3];
    // Keep at most a page plus one per rank while counting the entire library.
    let mut ranked = [Vec::new(), Vec::new(), Vec::new()];
    {
        let mut candidates = sqlx::query(CANDIDATES)
            .bind(owner)
            .bind(crate::security::MAX_RECIPE_SNAPSHOT_BYTES as i64)
            .fetch(&mut *tx);
        while let Some(row) = candidates.try_next().await? {
            let names: Vec<Option<String>> = row.try_get("names")?;
            let optional: Vec<Option<bool>> = row.try_get("optional_flags")?;
            if names.is_empty()
                || names
                    .iter()
                    .any(|n| n.as_ref().is_none_or(|n| n.trim().is_empty()))
                || optional.len() != names.len()
                || optional.iter().any(Option::is_none)
            {
                continue;
            }
            let check =
                matcher.check_requirements(names.iter().zip(&optional).map(|(name, optional)| {
                    (
                        name.as_deref().expect("validated name"),
                        optional.expect("validated optional flag"),
                    )
                }));
            let missing: Vec<_> = check
                .ingredients
                .iter()
                .filter(|i| i.status == IngredientStatus::Missing)
                .collect();
            let rank = missing.len();
            if rank > 2 {
                continue;
            }
            let authored = if rank > 0 {
                let authored: serde_json::Value = row.try_get("authored")?;
                let Ok(items) =
                    serde_json::from_value::<Vec<crate::types::RecipeIngredient>>(authored)
                else {
                    continue;
                };
                items
            } else {
                Vec::new()
            };

            totals[rank] += 1;
            if (rank > 0) != almost || ranked[rank].len() > PAGE_SIZE {
                continue;
            }
            let id: Uuid = row.try_get("id")?;
            let updated: DateTime<Utc> = row.try_get("updated_at")?;
            if cursor.as_ref().is_some_and(|c| {
                rank < c.rank || (rank == c.rank && (updated, id) >= (c.updated, c.recipe))
            }) {
                continue;
            }
            let shortage = almost.then(|| RecipeShortage {
                missing: missing.into_iter().map(|i| i.name.clone()).collect(),
                substitutions_available: authored.iter().any(|i| {
                    !i.optional
                        && !matcher.is_available(&i.name)
                        && i.substitutes.iter().any(|s| matcher.is_available(&s.name))
                }),
            });
            ranked[rank].push((id, updated, check.missing_optional_count(), rank, shortage));
        }
    }
    let mut matches: Vec<_> = ranked.into_iter().flatten().take(PAGE_SIZE + 1).collect();
    let next_cursor = if matches.len() > PAGE_SIZE {
        matches.truncate(PAGE_SIZE);
        let &(recipe, updated, _, rank, _) = matches.last().expect("nonempty full page");
        Some(
            URL_SAFE_NO_PAD.encode(
                serde_json::to_vec(&Cursor {
                    pantry: pantry_id,
                    revision,
                    rank,
                    updated,
                    recipe,
                })
                .map_err(|_| PantryError::Limit)?,
            ),
        )
    } else {
        None
    };
    let optional_ingredients_missing = matches
        .iter()
        .filter(|(_, _, count, _, _)| *count > 0)
        .map(|(id, _, count, _, _)| (*id, *count))
        .collect();
    let ids: Vec<_> = matches.iter().map(|(id, ..)| *id).collect();
    let recipes = if ids.is_empty() {
        Vec::new()
    } else {
        let query = format!(
            "SELECT {} FROM recipes r {} WHERE r.id = ANY($1) \
             AND (r.user_id=$2 OR r.visibility IN ('public','unlisted')) \
             ORDER BY array_position($1::uuid[], r.id)",
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
        cookable_count: totals[0],
        almost_cookable_count: totals[1] + totals[2],
        optional_ingredients_missing,
        shortages: matches
            .into_iter()
            .filter_map(|(id, _, _, _, shortage)| shortage.map(|shortage| (id, shortage)))
            .collect(),
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
                rank: 0,
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
