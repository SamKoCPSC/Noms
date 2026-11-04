//! Private library checks. Stream names/optional flags, then hydrate bounded cards.
use super::pantry::PantryError;
use crate::pantry::{
    feasibility::PantryMatcher, PantryRecipeMode, PantryRecipePage, MAX_ALMOST_COOKABLE_MISSING,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Datelike, Utc};
use futures_util::TryStreamExt;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::{cmp::Reverse, collections::BTreeMap};
use uuid::Uuid;

const PAGE_SIZE: usize = crate::security::MAX_COLLECTION_RECIPE_CARDS_PER_PAGE;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    pantry: Uuid,
    revision: i64,
    updated: DateTime<Utc>,
    recipe: Uuid,
    #[serde(default)]
    mode: PantryRecipeMode,
    #[serde(default)]
    missing_required: usize,
}

fn decode_cursor(
    value: Option<&str>,
    pantry: Uuid,
    revision: i64,
    mode: PantryRecipeMode,
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
        || cursor.mode != mode
        || match mode {
            PantryRecipeMode::Cookable => cursor.missing_required != 0,
            PantryRecipeMode::AlmostCookable => {
                !(1..=MAX_ALMOST_COOKABLE_MISSING).contains(&cursor.missing_required)
            }
        }
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
                  THEN r.ingredients ELSE '[]'::jsonb END) AS i) AS names,
       ARRAY(SELECT CASE WHEN NOT (i ? 'optional') THEN false
                    WHEN jsonb_typeof(i->'optional') = 'boolean' THEN i->'optional' = 'true'::jsonb END
             FROM jsonb_array_elements(CASE WHEN jsonb_typeof(r.ingredients) = 'array'
                  THEN r.ingredients ELSE '[]'::jsonb END) AS i) AS optional_flags
FROM library l JOIN recipes r ON r.id = l.id
WHERE (r.user_id = $1 OR r.visibility IN ('public', 'unlisted'))
  AND jsonb_typeof(r.ingredients) = 'array'
  AND octet_length(r.ingredients::text) <= $4
  AND ($2::timestamptz IS NULL OR (r.updated_at, r.id) < ($2, $3::uuid))
ORDER BY r.updated_at DESC, r.id DESC";

/// Compatibility entry point for callers that only need cookable recipes.
#[cfg(test)]
pub async fn find(
    pool: &PgPool,
    owner: Uuid,
    pantry_id: Uuid,
    revision: i64,
    cursor: Option<&str>,
) -> Result<PantryRecipePage, PantryError> {
    find_in_mode(
        pool,
        owner,
        pantry_id,
        revision,
        cursor,
        PantryRecipeMode::Cookable,
    )
    .await
}

type Rank = (Reverse<usize>, DateTime<Utc>, Uuid);

struct Match {
    missing: Vec<String>,
    optional: usize,
}

/// Both views share authorization, the presence matcher and one read snapshot.
pub async fn find_in_mode(
    pool: &PgPool,
    owner: Uuid,
    pantry_id: Uuid,
    revision: i64,
    cursor: Option<&str>,
    mode: PantryRecipeMode,
) -> Result<PantryRecipePage, PantryError> {
    if pantry_id.is_nil() || revision < 0 {
        return Err(PantryError::Invalid("Invalid pantry reference.".into()));
    }
    let cursor = decode_cursor(cursor, pantry_id, revision, mode)?;
    let cursor_rank = cursor
        .as_ref()
        .map(|c| (Reverse(c.missing_required), c.updated, c.recipe));
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await?;
    let pantry = super::pantry::load(&mut tx, owner, pantry_id).await?;
    if pantry.revision != revision {
        return Err(PantryError::Conflict);
    }
    let matcher = PantryMatcher::new(&pantry.entries);
    // Keep just one page and a lookahead. Almost-cookable ranking must inspect
    // the whole authorized library: a later recipe can need fewer ingredients.
    let mut matches: BTreeMap<Rank, Match> = BTreeMap::new();
    {
        let ordered = mode == PantryRecipeMode::Cookable;
        let mut candidates = sqlx::query(CANDIDATES)
            .bind(owner)
            .bind(cursor.as_ref().filter(|_| ordered).map(|c| c.updated))
            .bind(cursor.as_ref().filter(|_| ordered).map(|c| c.recipe))
            .bind(crate::security::MAX_RECIPE_SNAPSHOT_BYTES as i64)
            .fetch(&mut *tx);
        while let Some(row) = candidates.try_next().await? {
            let names: Vec<Option<String>> = row.try_get("names")?;
            let optional: Vec<Option<bool>> = row.try_get("optional_flags")?;
            if names.is_empty()
                || names.len() > crate::security::MAX_RECIPE_INGREDIENTS
                || names.iter().any(|n| {
                    n.as_ref()
                        .is_none_or(|n| n.trim().is_empty() || n.chars().count() > 500)
                })
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
            let missing = check.missing_required_names();
            let accepted = match mode {
                PantryRecipeMode::Cookable => missing.is_empty(),
                PantryRecipeMode::AlmostCookable => {
                    (1..=MAX_ALMOST_COOKABLE_MISSING).contains(&missing.len())
                }
            };
            if !accepted {
                continue;
            }
            let rank = (
                Reverse(missing.len()),
                row.try_get::<DateTime<Utc>, _>("updated_at")?,
                row.try_get::<Uuid, _>("id")?,
            );
            if cursor_rank.is_some_and(|cursor| rank >= cursor) {
                continue;
            }
            matches.insert(
                rank,
                Match {
                    missing,
                    optional: check.missing_optional_count(),
                },
            );
            if matches.len() > PAGE_SIZE {
                if ordered {
                    break;
                }
                if matches.len() > PAGE_SIZE + 1 {
                    matches.pop_first();
                }
            }
        }
    }
    let has_next = matches.len() > PAGE_SIZE;
    if has_next {
        matches.pop_first();
    }
    let next_cursor = if has_next {
        let (&(Reverse(missing_required), updated, recipe), _) =
            matches.first_key_value().expect("nonempty full page");
        Some(
            URL_SAFE_NO_PAD.encode(
                serde_json::to_vec(&Cursor {
                    pantry: pantry_id,
                    revision,
                    updated,
                    recipe,
                    mode,
                    missing_required,
                })
                .map_err(|_| PantryError::Limit)?,
            ),
        )
    } else {
        None
    };
    let optional_ingredients_missing = matches
        .iter()
        .filter(|(_, row)| row.optional > 0)
        .map(|(key, row)| (key.2, row.optional))
        .collect();
    let required_ingredients_missing = matches
        .iter()
        .filter(|(_, row)| !row.missing.is_empty())
        .map(|(key, row)| (key.2, row.missing.clone()))
        .collect();
    let ids: Vec<_> = matches.keys().rev().map(|key| key.2).collect();
    let recipes = if ids.is_empty() {
        Vec::new()
    } else {
        let query = format!(
            "SELECT {} FROM recipes r {} WHERE r.id = ANY($1) \
             AND (r.user_id=$2 OR r.visibility IN ('public','unlisted')) \
             ORDER BY array_position($1, r.id)",
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
    let page = PantryRecipePage {
        pantry_id,
        pantry_revision: revision,
        mode,
        recipes,
        optional_ingredients_missing,
        required_ingredients_missing,
        next_cursor,
    };
    if serde_json::to_vec(&page)
        .map_err(|_| PantryError::Limit)?
        .len()
        > crate::security::MAX_COLLECTION_RECIPE_CARD_PAGE_BYTES
    {
        return Err(PantryError::Limit);
    }
    tx.commit().await?;
    Ok(page)
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
                mode: PantryRecipeMode::Cookable,
                missing_required: 0,
            })
            .unwrap(),
        );
        assert!(decode_cursor(Some(&cursor), pantry, 3, PantryRecipeMode::Cookable).is_ok());
        assert!(decode_cursor(Some(&cursor), pantry, 3, PantryRecipeMode::AlmostCookable).is_err());
        let legacy = URL_SAFE_NO_PAD.encode(
            serde_json::to_vec(&serde_json::json!({
                "pantry": pantry, "revision": 3, "updated": Utc::now(), "recipe": Uuid::new_v4(),
            }))
            .unwrap(),
        );
        assert!(decode_cursor(Some(&legacy), pantry, 3, PantryRecipeMode::Cookable).is_ok());
        assert!(decode_cursor(Some(&legacy), pantry, 3, PantryRecipeMode::AlmostCookable).is_err());
        for missing in [0, MAX_ALMOST_COOKABLE_MISSING + 1] {
            let invalid = URL_SAFE_NO_PAD.encode(
                serde_json::to_vec(&Cursor {
                    pantry,
                    revision: 3,
                    updated: Utc::now(),
                    recipe: Uuid::new_v4(),
                    mode: PantryRecipeMode::AlmostCookable,
                    missing_required: missing,
                })
                .unwrap(),
            );
            assert!(
                decode_cursor(Some(&invalid), pantry, 3, PantryRecipeMode::AlmostCookable).is_err()
            );
        }
        assert!(
            decode_cursor(Some(&cursor), Uuid::new_v4(), 3, PantryRecipeMode::Cookable).is_err()
        );
        assert!(decode_cursor(Some(&cursor), pantry, 4, PantryRecipeMode::Cookable).is_err());
    }
}
