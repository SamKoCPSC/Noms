//! All inventory writes serialize through an owner-scoped profile lock.
use crate::pantry::*;
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, PgPool, Row};
use uuid::Uuid;
#[derive(Debug)]
pub enum PantryError {
    Invalid(String),
    Missing,
    Conflict,
    RetryMismatch,
    Limit,
    Database(sqlx::Error),
}
impl std::fmt::Display for PantryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self{
Self::Invalid(s)=>f.write_str(s),Self::Missing=>f.write_str("Kitchen or inventory item not found."),Self::Conflict=>f.write_str("This kitchen changed in another session. Refresh saved data before retrying."),Self::RetryMismatch=>f.write_str("This request was already used for different contents. Refresh the kitchen before starting a new request."),Self::Limit=>f.write_str("This kitchen exceeds the supported size or account limit."),Self::Database(e)=>{tracing::warn!(error=%e,"Pantry database operation failed");f.write_str("Your kitchen could not be saved. Please retry.")}}
    }
}
impl From<sqlx::Error> for PantryError {
    fn from(e: sqlx::Error) -> Self {
        Self::Database(e)
    }
}
impl From<String> for PantryError {
    fn from(e: String) -> Self {
        Self::Invalid(e)
    }
}
fn fingerprint<T: serde::Serialize>(value: &T) -> Result<String, PantryError> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(value).map_err(|_| PantryError::Limit)?)
    ))
}
fn bounded<T: serde::Serialize>(value: T) -> Result<T, PantryError> {
    if serde_json::to_vec(&value)
        .map_err(|_| PantryError::Limit)?
        .len()
        > MAX_PAYLOAD
    {
        Err(PantryError::Limit)
    } else {
        Ok(value)
    }
}
// SQL identifiers come only from this exhaustive enum, never request strings.
fn table(kind: PantryKind) -> &'static str {
    match kind {
        PantryKind::Ingredient => "pantry_ingredients",
        PantryKind::Equipment => "pantry_equipment",
    }
}
pub async fn summaries(pool: &PgPool, owner: Uuid) -> Result<Vec<PantrySummary>, PantryError> {
    let rows=sqlx::query("WITH p AS (SELECT id,name,created_at FROM kitchen_profiles WHERE owner_id=$1 ORDER BY created_at DESC,id DESC LIMIT 100), i AS (SELECT profile_id,count(*) AS total,count(*) FILTER (WHERE available AND NOT quantity_zero) AS stocked FROM pantry_ingredients JOIN p ON p.id=profile_id GROUP BY profile_id), e AS (SELECT profile_id,count(*) AS total,count(*) FILTER (WHERE available) AS ready FROM pantry_equipment JOIN p ON p.id=profile_id GROUP BY profile_id) SELECT p.id,p.name,COALESCE(i.total,0) AS ingredients,COALESCE(i.stocked,0) AS stocked,COALESCE(e.total,0) AS equipment,COALESCE(e.ready,0) AS ready FROM p LEFT JOIN i ON i.profile_id=p.id LEFT JOIN e ON e.profile_id=p.id ORDER BY p.created_at DESC,p.id DESC").bind(owner).fetch_all(pool).await?;
    rows.into_iter()
        .map(|r| {
            Ok(PantrySummary {
                id: r.try_get("id")?,
                name: r.try_get("name")?,
                ingredient_count: r.try_get("ingredients")?,
                equipment_count: r.try_get("equipment")?,
                in_stock_count: r.try_get("stocked")?,
                available_equipment_count: r.try_get("ready")?,
            })
        })
        .collect()
}
async fn load(
    conn: &mut PgConnection,
    owner: Uuid,
    id: Uuid,
) -> Result<PantryKitchen, PantryError> {
    let r=sqlx::query("SELECT id,owner_id,name,note,revision,created_at FROM kitchen_profiles WHERE id=$1 AND owner_id=$2").bind(id).bind(owner).fetch_optional(&mut *conn).await?.ok_or(PantryError::Missing)?;
    let mut entries = Vec::new();
    for kind in [PantryKind::Ingredient, PantryKind::Equipment] {
        let cols = if kind == PantryKind::Ingredient {
            "amount,unit,quantity_zero"
        } else {
            "''::text AS amount,''::text AS unit,FALSE AS quantity_zero"
        };
        let rows=sqlx::query(&format!("SELECT id,name,{cols},available,position,revision FROM {} WHERE profile_id=$1 ORDER BY position,id LIMIT $2",table(kind))).bind(id).bind((kind.limit()+1) as i64).fetch_all(&mut *conn).await?;
        if rows.len() > kind.limit() {
            return Err(PantryError::Limit);
        }
        for r in rows {
            entries.push(PantryEntry {
                id: r.try_get("id")?,
                kind,
                stock: PantryStock {
                    fields: PantryFields {
                        name: r.try_get("name")?,
                        amount: r.try_get("amount")?,
                        unit: r.try_get("unit")?,
                    },
                    available: r.try_get("available")?,
                },
                quantity_zero: r.try_get("quantity_zero")?,
                position: r.try_get("position")?,
                revision: r.try_get("revision")?,
            });
        }
    }
    bounded(PantryKitchen {
        id: r.try_get("id")?,
        owner_id: r.try_get("owner_id")?,
        name: r.try_get("name")?,
        note: r.try_get("note")?,
        entries,
        revision: r.try_get("revision")?,
        created_at: r.try_get("created_at")?,
    })
}
pub async fn get(pool: &PgPool, owner: Uuid, id: Uuid) -> Result<PantryKitchen, PantryError> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await?;
    let result = load(&mut tx, owner, id).await?;
    tx.commit().await?;
    Ok(result)
}
async fn lock(conn: &mut PgConnection, owner: Uuid, id: Uuid) -> Result<i64, PantryError> {
    sqlx::query_scalar(
        "SELECT revision FROM kitchen_profiles WHERE id=$1 AND owner_id=$2 FOR UPDATE",
    )
    .bind(id)
    .bind(owner)
    .fetch_optional(conn)
    .await?
    .ok_or(PantryError::Missing)
}
async fn bump(conn: &mut PgConnection, id: Uuid) -> Result<(), PantryError> {
    sqlx::query("UPDATE kitchen_profiles SET revision=revision+1,updated_at=NOW() WHERE id=$1")
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}
async fn insert(
    conn: &mut PgConnection,
    profile: Uuid,
    kind: PantryKind,
    stock: &PantryStock,
    position: i32,
    key: Option<Uuid>,
    hash: Option<String>,
) -> Result<(), PantryError> {
    stock.fields.validate(kind)?;
    if kind == PantryKind::Ingredient {
        sqlx::query("INSERT INTO pantry_ingredients(profile_id,name,amount,unit,quantity_zero,available,position,add_key,add_hash) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9)").bind(profile).bind(&stock.fields.name).bind(&stock.fields.amount).bind(&stock.fields.unit).bind(quantity_zero(&stock.fields.amount)?.unwrap_or(false)).bind(stock.available).bind(position).bind(key).bind(hash).execute(conn).await?;
    } else {
        sqlx::query("INSERT INTO pantry_equipment(profile_id,name,available,position,add_key,add_hash) VALUES($1,$2,$3,$4,$5,$6)").bind(profile).bind(&stock.fields.name).bind(stock.available).bind(position).bind(key).bind(hash).execute(conn).await?;
    }
    Ok(())
}
pub async fn create(
    pool: &PgPool,
    owner: Uuid,
    request: PantryCreate,
) -> Result<PantryKitchen, PantryError> {
    request.validate()?;
    let hash = fingerprint(&request)?;
    let mut tx = pool.begin().await?;
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE id=$1 FOR UPDATE")
        .bind(owner)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(PantryError::Missing)?;
    if let Some(r) = sqlx::query(
        "SELECT id,request_hash FROM kitchen_profiles WHERE owner_id=$1 AND request_id=$2",
    )
    .bind(owner)
    .bind(request.request_id)
    .fetch_optional(&mut *tx)
    .await?
    {
        if r.try_get::<String, _>("request_hash")? != hash {
            return Err(PantryError::RetryMismatch);
        }
        let saved = load(&mut tx, owner, r.try_get("id")?).await?;
        tx.commit().await?;
        return Ok(saved);
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM kitchen_profiles WHERE owner_id=$1")
        .bind(owner)
        .fetch_one(&mut *tx)
        .await?;
    if count >= MAX_PROFILES as i64 {
        return Err(PantryError::Limit);
    }
    let (ingredients, equipment) = if let Some(copy) = request.copy {
        let revision: i64 = sqlx::query_scalar(
            "SELECT revision FROM kitchen_profiles WHERE id=$1 AND owner_id=$2 FOR SHARE",
        )
        .bind(copy.id)
        .bind(owner)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(PantryError::Missing)?;
        if revision != copy.expected_revision {
            return Err(PantryError::Conflict);
        }
        let source = load(&mut tx, owner, copy.id).await?;
        let (i, e): (Vec<_>, Vec<_>) = source
            .entries
            .into_iter()
            .partition(|x| x.kind == PantryKind::Ingredient);
        (
            i.into_iter().map(|x| x.stock).collect(),
            e.into_iter().map(|x| x.stock).collect(),
        )
    } else {
        (request.ingredients, request.equipment)
    };
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO kitchen_profiles(id,owner_id,name,note,request_id,request_hash) VALUES($1,$2,$3,$4,$5,$6)").bind(id).bind(owner).bind(&request.name).bind(&request.note).bind(request.request_id).bind(hash).execute(&mut *tx).await?;
    for (kind, stocks) in [
        (PantryKind::Ingredient, ingredients),
        (PantryKind::Equipment, equipment),
    ] {
        for (i, stock) in stocks.iter().enumerate() {
            insert(&mut tx, id, kind, stock, i as i32, None, None).await?
        }
    }
    let saved = load(&mut tx, owner, id).await?;
    tx.commit().await?;
    Ok(saved)
}
pub async fn add(
    pool: &PgPool,
    owner: Uuid,
    profile: Uuid,
    kind: PantryKind,
    key: Uuid,
    stock: PantryStock,
) -> Result<(), PantryError> {
    stock.fields.validate(kind)?;
    let hash = fingerprint(&stock)?;
    let mut tx = pool.begin().await?;
    lock(&mut tx, owner, profile).await?;
    if let Some(previous) = sqlx::query_scalar::<_, String>(&format!(
        "SELECT add_hash FROM {} WHERE profile_id=$1 AND add_key=$2",
        table(kind)
    ))
    .bind(profile)
    .bind(key)
    .fetch_optional(&mut *tx)
    .await?
    {
        return if previous == hash {
            Ok(())
        } else {
            Err(PantryError::RetryMismatch)
        };
    }
    let (count, position): (i64, i32) = sqlx::query_as(&format!(
        "SELECT count(*),COALESCE(MAX(position),-1)+1 FROM {} WHERE profile_id=$1",
        table(kind)
    ))
    .bind(profile)
    .fetch_one(&mut *tx)
    .await?;
    if count >= kind.limit() as i64 {
        return Err(PantryError::Limit);
    }
    insert(
        &mut tx,
        profile,
        kind,
        &stock,
        position,
        Some(key),
        Some(hash),
    )
    .await?;
    bump(&mut tx, profile).await?;
    load(&mut tx, owner, profile).await?;
    tx.commit().await?;
    Ok(())
}
pub async fn change(
    pool: &PgPool,
    owner: Uuid,
    profile: Uuid,
    kind: PantryKind,
    id: Uuid,
    expected: i64,
    change: PantryChange,
) -> Result<(), PantryError> {
    if let PantryChange::Fields(f) = &change {
        f.validate(kind)?
    }
    let mut tx = pool.begin().await?;
    lock(&mut tx, owner, profile).await?;
    let cols = if kind == PantryKind::Ingredient {
        "amount,unit"
    } else {
        "''::text AS amount,''::text AS unit"
    };
    let r = sqlx::query(&format!(
        "SELECT name,{cols},available,revision FROM {} WHERE id=$1 AND profile_id=$2 FOR UPDATE",
        table(kind)
    ))
    .bind(id)
    .bind(profile)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(PantryError::Missing)?;
    let matches = match &change {
        PantryChange::Availability(v) => *v == r.try_get::<bool, _>("available")?,
        PantryChange::Fields(f) => {
            f.name == r.try_get::<String, _>("name")?
                && f.amount == r.try_get::<String, _>("amount")?
                && f.unit == r.try_get::<String, _>("unit")?
        }
    };
    if matches {
        return Ok(());
    }
    if expected != r.try_get::<i64, _>("revision")? {
        return Err(PantryError::Conflict);
    }
    match change {
        PantryChange::Availability(value) => {
            sqlx::query(&format!("UPDATE {} SET available=$1,revision=revision+1,updated_at=NOW() WHERE id=$2 AND profile_id=$3",table(kind))).bind(value).bind(id).bind(profile).execute(&mut *tx).await?;
        }
        PantryChange::Fields(f) => {
            if kind == PantryKind::Ingredient {
                let zero = quantity_zero(&f.amount)?.unwrap_or(false);
                sqlx::query("UPDATE pantry_ingredients SET name=$1,amount=$2,unit=$3,quantity_zero=$4,revision=revision+1,updated_at=NOW() WHERE id=$5 AND profile_id=$6").bind(f.name).bind(f.amount).bind(f.unit).bind(zero).bind(id).bind(profile).execute(&mut *tx).await?;
            } else {
                sqlx::query("UPDATE pantry_equipment SET name=$1,revision=revision+1,updated_at=NOW() WHERE id=$2 AND profile_id=$3").bind(f.name).bind(id).bind(profile).execute(&mut *tx).await?;
            }
        }
    }
    bump(&mut tx, profile).await?;
    load(&mut tx, owner, profile).await?;
    tx.commit().await?;
    Ok(())
}
pub async fn remove(
    pool: &PgPool,
    owner: Uuid,
    profile: Uuid,
    kind: PantryKind,
    id: Uuid,
    expected: i64,
) -> Result<(), PantryError> {
    let mut tx = pool.begin().await?;
    lock(&mut tx, owner, profile).await?;
    let affected = sqlx::query(&format!(
        "DELETE FROM {} WHERE id=$1 AND profile_id=$2 AND revision=$3",
        table(kind)
    ))
    .bind(id)
    .bind(profile)
    .bind(expected)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if affected != 1 {
        let exists: bool = sqlx::query_scalar(&format!(
            "SELECT EXISTS(SELECT 1 FROM {} WHERE id=$1 AND profile_id=$2)",
            table(kind)
        ))
        .bind(id)
        .bind(profile)
        .fetch_one(&mut *tx)
        .await?;
        return Err(if exists {
            PantryError::Conflict
        } else {
            PantryError::Missing
        });
    }
    bump(&mut tx, profile).await?;
    tx.commit().await?;
    Ok(())
}
pub async fn edit_profile(
    pool: &PgPool,
    owner: Uuid,
    id: Uuid,
    expected: i64,
    name: String,
    note: String,
) -> Result<(), PantryError> {
    validate_profile(&name, &note)?;
    let mut tx = pool.begin().await?;
    let rev = lock(&mut tx, owner, id).await?;
    let current = load(&mut tx, owner, id).await?;
    if current.name == name && current.note == note {
        return Ok(());
    }
    if expected != rev {
        return Err(PantryError::Conflict);
    }
    sqlx::query("UPDATE kitchen_profiles SET name=$1,note=$2,revision=revision+1,updated_at=NOW() WHERE id=$3 AND owner_id=$4").bind(name).bind(note).bind(id).bind(owner).execute(&mut *tx).await?;
    load(&mut tx, owner, id).await?;
    tx.commit().await?;
    Ok(())
}
pub async fn delete(
    pool: &PgPool,
    owner: Uuid,
    id: Uuid,
    expected: i64,
) -> Result<(), PantryError> {
    let mut tx = pool.begin().await?;
    if lock(&mut tx, owner, id).await? != expected {
        return Err(PantryError::Conflict);
    }
    sqlx::query("DELETE FROM kitchen_profiles WHERE id=$1 AND owner_id=$2")
        .bind(id)
        .bind(owner)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}
