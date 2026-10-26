//! Owner-scoped, atomic grocery persistence. Recipe references are copied, never FKs.
use crate::grocery::*;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, PgPool, Row};
use uuid::Uuid;

#[derive(Debug)]
pub enum GroceryError {
    Invalid(String),
    Missing,
    Conflict,
    SourceChanged,
    RetryMismatch,
    Limit,
    Database(sqlx::Error),
}
impl std::fmt::Display for GroceryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(message)=>f.write_str(message),
            Self::Missing=>f.write_str("Grocery list or item not found."),
            Self::Conflict=>f.write_str("This list changed in another session. Refresh it before retrying."),
            Self::SourceChanged=>f.write_str("A selected recipe changed. Review its updated ingredients before retrying."),
            Self::RetryMismatch=>f.write_str("This request was already used for different contents. Review the saved list before starting a new request."),
            Self::Limit=>f.write_str("This list exceeds the supported size or account limit."),
            Self::Database(error)=>{tracing::warn!(error=%error,"Grocery database operation failed");f.write_str("Your grocery list could not be saved. Please retry.")},
        }
    }
}
impl From<sqlx::Error> for GroceryError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}
impl From<String> for GroceryError {
    fn from(error: String) -> Self {
        Self::Invalid(error)
    }
}
fn json<T: serde::Serialize>(value: &T) -> Result<serde_json::Value, GroceryError> {
    serde_json::to_value(value).map_err(|_| GroceryError::Invalid("Invalid grocery data.".into()))
}
fn decode<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> Result<T, GroceryError> {
    serde_json::from_value(value)
        .map_err(|_| GroceryError::Invalid("Saved grocery data could not be read.".into()))
}
fn fingerprint<T: serde::Serialize>(value: &T) -> Result<String, GroceryError> {
    Ok(format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(value)
                .map_err(|_| GroceryError::Invalid("Invalid grocery data.".into()))?
        )
    ))
}
fn bounded<T: serde::Serialize>(value: T) -> Result<T, GroceryError> {
    if serde_json::to_vec(&value)
        .map_err(|_| GroceryError::Limit)?
        .len()
        > MAX_PAYLOAD
    {
        Err(GroceryError::Limit)
    } else {
        Ok(value)
    }
}

pub async fn preview_source(
    conn: &mut PgConnection,
    owner: Uuid,
    id: Uuid,
) -> Result<GroceryRecipePreview, GroceryError> {
    let row=sqlx::query("SELECT id,title,servings,ingredients,updated_at FROM recipes WHERE id=$1 AND (user_id=$2 OR visibility IN ('public','unlisted')) FOR SHARE")
        .bind(id).bind(owner).fetch_optional(conn).await?.ok_or_else(||GroceryError::Invalid("Recipe not accessible. Remove the selection or choose another recipe.".into()))?;
    let title: String = row.try_get("title")?;
    let servings: Option<i32> = row.try_get("servings")?;
    let ingredients: serde_json::Value = row.try_get("ingredients")?;
    let updated: DateTime<Utc> = row.try_get("updated_at")?;
    let revision = fingerprint(&(&title, servings, &ingredients, updated))?;
    bounded(GroceryRecipePreview {
        source: GrocerySource {
            recipe_id: id,
            title,
            base_servings: servings,
            target_servings: None,
            revision,
        },
        ingredients: decode(ingredients)?,
    })
}

pub async fn summaries(
    pool: &PgPool,
    owner: Uuid,
) -> Result<Vec<GroceryListSummary>, GroceryError> {
    let rows=sqlx::query("SELECT l.id,l.name,jsonb_array_length(l.sources)::bigint AS recipes,COUNT(i.id) AS items,COUNT(i.id) FILTER (WHERE i.checked) AS done FROM grocery_lists l LEFT JOIN grocery_list_items i ON i.list_id=l.id WHERE l.owner_id=$1 GROUP BY l.id ORDER BY l.created_at DESC,l.id DESC LIMIT 100")
        .bind(owner).fetch_all(pool).await?;
    rows.into_iter()
        .map(|r| {
            Ok(GroceryListSummary {
                id: r.try_get("id")?,
                name: r.try_get("name")?,
                item_count: r.try_get("items")?,
                checked_count: r.try_get("done")?,
                recipe_count: r.try_get::<i64, _>("recipes")? as usize,
            })
        })
        .collect()
}

pub async fn get(pool: &PgPool, owner: Uuid, id: Uuid) -> Result<GroceryList, GroceryError> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await?;
    let result = load_owned(&mut tx, owner, id).await?;
    tx.commit().await?;
    Ok(result)
}
async fn load_owned(
    conn: &mut PgConnection,
    owner: Uuid,
    id: Uuid,
) -> Result<GroceryList, GroceryError> {
    let r=sqlx::query("SELECT id,owner_id,name,note,sources,revision,created_at FROM grocery_lists WHERE id=$1 AND owner_id=$2").bind(id).bind(owner).fetch_optional(&mut *conn).await?.ok_or(GroceryError::Missing)?;
    let rows=sqlx::query("SELECT id,name,amount,unit,original,contributions,checked,position,revision FROM grocery_list_items WHERE list_id=$1 ORDER BY position,id LIMIT 1001").bind(id).fetch_all(&mut *conn).await?;
    if rows.len() > MAX_ITEMS {
        return Err(GroceryError::Limit);
    }
    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        items.push(GroceryItem {
            id: row.try_get("id")?,
            fields: ItemFields {
                name: row.try_get("name")?,
                amount: row.try_get("amount")?,
                unit: row.try_get("unit")?,
            },
            original: row
                .try_get::<Option<serde_json::Value>, _>("original")?
                .map(decode)
                .transpose()?,
            contributions: decode(row.try_get("contributions")?)?,
            checked: row.try_get("checked")?,
            position: row.try_get("position")?,
            revision: row.try_get("revision")?,
        });
    }
    let result = GroceryList {
        id: r.try_get("id")?,
        owner_id: r.try_get("owner_id")?,
        name: r.try_get("name")?,
        note: r.try_get("note")?,
        sources: decode(r.try_get("sources")?)?,
        items,
        revision: r.try_get("revision")?,
        created_at: r.try_get("created_at")?,
    };
    bounded(result)
}
async fn lock_list(conn: &mut PgConnection, owner: Uuid, list: Uuid) -> Result<i64, GroceryError> {
    sqlx::query_scalar("SELECT revision FROM grocery_lists WHERE id=$1 AND owner_id=$2 FOR UPDATE")
        .bind(list)
        .bind(owner)
        .fetch_optional(conn)
        .await?
        .ok_or(GroceryError::Missing)
}
async fn bump(conn: &mut PgConnection, id: Uuid) -> Result<(), GroceryError> {
    sqlx::query("UPDATE grocery_lists SET revision=revision+1,updated_at=NOW() WHERE id=$1")
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}
async fn insert_item(
    conn: &mut PgConnection,
    list: Uuid,
    item: &GeneratedItem,
    position: i32,
    original: bool,
    key: Option<Uuid>,
    hash: Option<String>,
) -> Result<(), GroceryError> {
    sqlx::query("INSERT INTO grocery_list_items (list_id,name,amount,unit,original,contributions,position,add_key,add_hash) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9)")
        .bind(list).bind(&item.fields.name).bind(&item.fields.amount).bind(&item.fields.unit).bind(if original{Some(json(&item.fields)?)}else{None}).bind(json(&item.contributions)?).bind(position).bind(key).bind(hash).execute(conn).await?;
    Ok(())
}

pub async fn create(
    pool: &PgPool,
    owner: Uuid,
    request: GroceryCreate,
) -> Result<GroceryList, GroceryError> {
    request.validate()?;
    let hash = fingerprint(&request)?;
    let mut tx = pool.begin().await?;
    // Serializes quota checks and retries for this account, without trusting a submitted owner.
    let user: Option<Uuid> = sqlx::query_scalar("SELECT id FROM users WHERE id=$1 FOR UPDATE")
        .bind(owner)
        .fetch_optional(&mut *tx)
        .await?;
    if user.is_none() {
        return Err(GroceryError::Missing);
    }
    if let Some(row) =
        sqlx::query("SELECT id,request_hash FROM grocery_lists WHERE owner_id=$1 AND request_id=$2")
            .bind(owner)
            .bind(request.request_id)
            .fetch_optional(&mut *tx)
            .await?
    {
        if row.try_get::<String, _>("request_hash")? != hash {
            return Err(GroceryError::RetryMismatch);
        }
        let id = row.try_get("id")?;
        tx.commit().await?;
        return get(pool, owner, id).await;
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM grocery_lists WHERE owner_id=$1")
        .bind(owner)
        .fetch_one(&mut *tx)
        .await?;
    if count >= MAX_LISTS as i64 {
        return Err(GroceryError::Limit);
    }
    // Consistent source-lock ordering avoids deadlocks for overlapping selections.
    let mut ordered = request.recipes.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|r| r.recipe_id);
    let mut fetched = std::collections::HashMap::new();
    for selection in ordered {
        let mut preview = preview_source(&mut tx, owner, selection.recipe_id).await?;
        if preview.source.revision != selection.expected_revision {
            return Err(GroceryError::SourceChanged);
        }
        preview.source.target_servings = selection.target_servings;
        fetched.insert(selection.recipe_id, preview);
    }
    let previews = request
        .recipes
        .iter()
        .map(|r| {
            fetched
                .remove(&r.recipe_id)
                .expect("validated unique source")
        })
        .collect::<Vec<_>>();
    let generated = build_requirements(&previews)?;
    if generated.len() + request.manual_items.len() > MAX_ITEMS {
        return Err(GroceryError::Limit);
    }
    bounded((&generated, &request.manual_items))?;
    let sources = previews.into_iter().map(|p| p.source).collect::<Vec<_>>();
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO grocery_lists (id,owner_id,name,note,request_id,request_hash,sources) VALUES($1,$2,$3,$4,$5,$6,$7)")
        .bind(id).bind(owner).bind(request.name.trim()).bind(&request.note).bind(request.request_id).bind(hash).bind(json(&sources)?).execute(&mut *tx).await?;
    for (index, item) in generated.iter().enumerate() {
        insert_item(&mut tx, id, item, index as i32, true, None, None).await?;
    }
    for (index, fields) in request.manual_items.into_iter().enumerate() {
        insert_item(
            &mut tx,
            id,
            &GeneratedItem {
                fields,
                contributions: vec![],
            },
            (generated.len() + index) as i32,
            false,
            None,
            None,
        )
        .await?;
    }
    let saved = load_owned(&mut tx, owner, id).await?;
    tx.commit().await?;
    Ok(saved)
}

pub async fn add_item(
    pool: &PgPool,
    owner: Uuid,
    list: Uuid,
    key: Uuid,
    fields: ItemFields,
) -> Result<(), GroceryError> {
    fields.validate()?;
    let hash = fingerprint(&fields)?;
    let mut tx = pool.begin().await?;
    lock_list(&mut tx, owner, list).await?;
    if let Some(existing) = sqlx::query_scalar::<_, String>(
        "SELECT add_hash FROM grocery_list_items WHERE list_id=$1 AND add_key=$2",
    )
    .bind(list)
    .bind(key)
    .fetch_optional(&mut *tx)
    .await?
    {
        if existing != hash {
            return Err(GroceryError::RetryMismatch);
        }
        return Ok(());
    }
    let (count, position): (i64, i32) = sqlx::query_as(
        "SELECT count(*),COALESCE(MAX(position),-1)+1 FROM grocery_list_items WHERE list_id=$1",
    )
    .bind(list)
    .fetch_one(&mut *tx)
    .await?;
    if count >= MAX_ITEMS as i64 {
        return Err(GroceryError::Limit);
    }
    insert_item(
        &mut tx,
        list,
        &GeneratedItem {
            fields,
            contributions: vec![],
        },
        position,
        false,
        Some(key),
        Some(hash),
    )
    .await?;
    bump(&mut tx, list).await?;
    load_owned(&mut tx, owner, list).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn change_item(
    pool: &PgPool,
    owner: Uuid,
    list: Uuid,
    item: Uuid,
    expected: i64,
    change: GroceryItemChange,
) -> Result<(), GroceryError> {
    if let GroceryItemChange::Fields(fields) = &change {
        fields.validate()?;
    }
    let mut tx = pool.begin().await?;
    lock_list(&mut tx, owner, list).await?;
    let row=sqlx::query("SELECT revision,name,amount,unit,checked FROM grocery_list_items WHERE id=$1 AND list_id=$2 FOR UPDATE").bind(item).bind(list).fetch_optional(&mut *tx).await?.ok_or(GroceryError::Missing)?;
    let revision: i64 = row.try_get("revision")?;
    let matches = match &change {
        GroceryItemChange::Checked(checked) => *checked == row.try_get::<bool, _>("checked")?,
        GroceryItemChange::Fields(fields) => {
            fields.name == row.try_get::<String, _>("name")?
                && fields.amount == row.try_get::<String, _>("amount")?
                && fields.unit == row.try_get::<String, _>("unit")?
        }
    };
    if matches {
        return Ok(());
    }
    if revision != expected {
        return Err(GroceryError::Conflict);
    }
    match change {
        GroceryItemChange::Checked(checked) => {
            sqlx::query("UPDATE grocery_list_items SET checked=$1,revision=revision+1,updated_at=NOW() WHERE id=$2 AND list_id=$3").bind(checked).bind(item).bind(list).execute(&mut *tx).await?;
        }
        GroceryItemChange::Fields(fields) => {
            sqlx::query("UPDATE grocery_list_items SET name=$1,amount=$2,unit=$3,revision=revision+1,updated_at=NOW() WHERE id=$4 AND list_id=$5").bind(fields.name).bind(fields.amount).bind(fields.unit).bind(item).bind(list).execute(&mut *tx).await?;
        }
    }
    bump(&mut tx, list).await?;
    load_owned(&mut tx, owner, list).await?;
    tx.commit().await?;
    Ok(())
}
pub async fn remove_item(
    pool: &PgPool,
    owner: Uuid,
    list: Uuid,
    item: Uuid,
    expected: i64,
) -> Result<(), GroceryError> {
    let mut tx = pool.begin().await?;
    lock_list(&mut tx, owner, list).await?;
    if sqlx::query("DELETE FROM grocery_list_items WHERE id=$1 AND list_id=$2 AND revision=$3")
        .bind(item)
        .bind(list)
        .bind(expected)
        .execute(&mut *tx)
        .await?
        .rows_affected()
        != 1
    {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM grocery_list_items WHERE id=$1 AND list_id=$2)",
        )
        .bind(item)
        .bind(list)
        .fetch_one(&mut *tx)
        .await?;
        return Err(if exists {
            GroceryError::Conflict
        } else {
            GroceryError::Missing
        });
    }
    bump(&mut tx, list).await?;
    load_owned(&mut tx, owner, list).await?;
    tx.commit().await?;
    Ok(())
}
pub async fn rename(
    pool: &PgPool,
    owner: Uuid,
    list: Uuid,
    expected: i64,
    name: String,
) -> Result<(), GroceryError> {
    validate_list_name(&name)?;
    let mut tx = pool.begin().await?;
    if lock_list(&mut tx, owner, list).await? != expected {
        return Err(GroceryError::Conflict);
    }
    sqlx::query("UPDATE grocery_lists SET name=$1,revision=revision+1,updated_at=NOW() WHERE id=$2 AND owner_id=$3").bind(name.trim()).bind(list).bind(owner).execute(&mut *tx).await?;
    load_owned(&mut tx, owner, list).await?;
    tx.commit().await?;
    Ok(())
}
pub async fn delete(
    pool: &PgPool,
    owner: Uuid,
    list: Uuid,
    expected: i64,
) -> Result<(), GroceryError> {
    let mut tx = pool.begin().await?;
    if lock_list(&mut tx, owner, list).await? != expected {
        return Err(GroceryError::Conflict);
    }
    sqlx::query("DELETE FROM grocery_lists WHERE id=$1 AND owner_id=$2")
        .bind(list)
        .bind(owner)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}
