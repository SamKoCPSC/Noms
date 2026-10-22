//! Consistent, owner-authorized reads of a bounded collection subtree.
use super::*;
use crate::collection_json::{self, CollectionSnapshot, PortableCollection};
use std::collections::{HashMap, HashSet};

pub(crate) async fn insert_collection_import_node(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    user: Uuid,
    parent: Option<Uuid>,
    name: &str,
    node: &PortableCollection,
    order: i32,
    image: Option<&RecipeImage>,
) -> Result<Uuid, DbError> {
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO collections (owner_id, parent_collection_id, name, description, sort_order, visibility, image)
         VALUES ($1,$2,$3,$4,$5,'private',$6) RETURNING id"
    ).bind(user).bind(parent).bind(name).bind(&node.description).bind(order)
        .bind(image.map(serde_json::to_value).transpose().map_err(DbError::SerdeJson)?)
        .fetch_one(&mut **tx).await.map_err(collection_query_error)?;
    media_assets::sync_recipe_media_references(
        tx,
        MediaHolder::Collection(id),
        user,
        MediaPurpose::CollectionCover,
        image.map(std::slice::from_ref).unwrap_or_default(),
        &[],
        MediaAttachAuthority::UploaderOnly,
    )
    .await?;
    Ok(id)
}

pub(crate) struct CollectionExport {
    pub collections: Vec<PortableCollection>,
    pub recipes: Vec<(Recipe, Vec<String>)>,
}

pub async fn export_collection_json(
    pool: &PgPool,
    user: Uuid,
    root: Uuid,
) -> Result<String, String> {
    let data = read_collection_export(pool, user, root).await?;
    let mut snapshots = vec![];
    for (recipe, tags) in data.recipes {
        let json = crate::recipe_json::export_recipe(&recipe, tags)?;
        snapshots.push(serde_json::from_str(&json).map_err(|_| "Cannot encode recipe snapshot.")?);
    }
    collection_json::export(CollectionSnapshot {
        root_collection: collection_json::portable_collection_id(root),
        collections: data.collections,
        recipes: snapshots,
    })
}

pub(crate) async fn read_collection_export(
    pool: &PgPool,
    user: Uuid,
    root: Uuid,
) -> Result<CollectionExport, String> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|_| "Cannot read this collection.")?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await
        .map_err(|_| "Cannot read this collection.")?;
    let rows = sqlx::query(&format!(
        "SELECT c.id, c.owner_id, c.parent_collection_id, c.name, c.description, c.image,
         {} AS media_asset_urls, c.visibility, c.sort_order, c.created_at, c.updated_at
         FROM collections c WHERE c.owner_id = $1
         ORDER BY c.sort_order, lower(c.name), c.id LIMIT $2",
        collection_media_asset_urls_select("c")
    ))
    .bind(user)
    .bind((MAX_COLLECTIONS_PER_USER + 1) as i64)
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| "Cannot read this collection.")?;
    if rows.len() > MAX_COLLECTIONS_PER_USER {
        return Err("Your collections exceed the supported limit.".into());
    }
    let collections = rows
        .iter()
        .map(collection_from_row)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "Cannot read collection metadata.")?;
    let root_index = collections
        .iter()
        .position(|c| c.id == root)
        .ok_or("Collection not found or not owned by you.")?;
    let mut stack = vec![root_index];
    let mut selected = vec![];
    let mut visited = HashSet::new();
    while let Some(index) = stack.pop() {
        let collection = &collections[index];
        if !visited.insert(collection.id) || selected.len() == collection_json::MAX_COLLECTIONS {
            return Err("Export a smaller collection tree (up to 100 collections).".into());
        }
        selected.push(collection);
        for (index, _) in collections
            .iter()
            .enumerate()
            .rev()
            .filter(|(_, c)| c.parent_collection_id == Some(collection.id))
        {
            stack.push(index);
        }
    }
    let ids = selected.iter().map(|c| c.id).collect::<Vec<_>>();
    let members = sqlx::query_as::<_, (Uuid, Uuid)>(
        "SELECT collection_id, recipe_id FROM collection_recipes WHERE collection_id = ANY($1)
         ORDER BY collection_id, sort_order, created_at, recipe_id LIMIT $2",
    )
    .bind(&ids)
    .bind((collection_json::MAX_MEMBERSHIPS + 1) as i64)
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| "Cannot read collection membership.")?;
    if members.len() > collection_json::MAX_MEMBERSHIPS {
        return Err("Export a smaller tree (up to 1,000 recipe memberships).".into());
    }
    let mut recipe_ids = vec![];
    let mut seen = HashSet::new();
    for collection in &selected {
        for (_, id) in members.iter().filter(|(c, _)| *c == collection.id) {
            if seen.insert(*id) {
                recipe_ids.push(*id);
            }
        }
    }
    if recipe_ids.len() > collection_json::MAX_RECIPES {
        return Err("Export a smaller collection (up to 100 recipes).".into());
    }
    // Bound decoded snapshot work before loading recipe JSONB across the batch.
    let (count, bytes): (i64, i64) = sqlx::query_as(
        "SELECT COUNT(*), COALESCE(SUM(octet_length(to_jsonb(r)::text)),0)::bigint
         FROM recipes r WHERE id = ANY($1) AND (user_id = $2 OR visibility IN ('public','unlisted'))"
    ).bind(&recipe_ids).bind(user).fetch_one(&mut *tx).await.map_err(|_| "Cannot read recipe sizes.")?;
    if count as usize != recipe_ids.len() {
        return Err("Some collection recipes are no longer accessible. Remove them or restore access before exporting.".into());
    }
    if bytes as usize > collection_json::MAX_JSON_BYTES {
        return Err("Collection recipes exceed 8 MiB. Export a smaller collection.".into());
    }
    let recipes = sqlx::query_as::<_, Recipe>(&format!(
        "SELECT {} FROM recipes r JOIN users u ON u.id = r.user_id
         WHERE r.id = ANY($1) AND (r.user_id = $2 OR r.visibility IN ('public','unlisted'))",
        recipe_read_columns("r", "u")
    ))
    .bind(&recipe_ids)
    .bind(user)
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| "Cannot read recipe snapshots.")?;
    let mut recipes = recipes
        .into_iter()
        .map(|r| (r.id, r))
        .collect::<HashMap<_, _>>();
    let mut tags = HashMap::<Uuid, Vec<String>>::new();
    for (id, tag) in sqlx::query_as::<_, (Uuid, String)>(
        "SELECT recipe_id, tag FROM recipe_tags WHERE recipe_id = ANY($1) ORDER BY tag",
    )
    .bind(&recipe_ids)
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| "Cannot read recipe tags.")?
    {
        tags.entry(id).or_default().push(tag);
    }
    let mut ordered_recipes = vec![];
    for id in recipe_ids {
        let recipe = recipes
            .remove(&id)
            .ok_or("A collection recipe is unavailable.")?;
        ordered_recipes.push((recipe, tags.remove(&id).unwrap_or_default()));
    }
    let nodes = selected
        .iter()
        .map(|c| PortableCollection {
            portable_id: collection_json::portable_collection_id(c.id),
            parent_collection: if c.id == root {
                None
            } else {
                c.parent_collection_id
                    .map(collection_json::portable_collection_id)
            },
            name: c.name.clone(),
            description: c.description.clone(),
            image: c.image.clone(),
            recipes: members
                .iter()
                .filter(|(id, _)| *id == c.id)
                .map(|(_, recipe)| format!("urn:noms:recipe:{recipe}"))
                .collect(),
        })
        .collect();
    tx.commit()
        .await
        .map_err(|_| "Cannot finish collection export.")?;
    Ok(CollectionExport {
        collections: nodes,
        recipes: ordered_recipes,
    })
}
