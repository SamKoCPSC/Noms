//! Bounded collection trees containing canonical recipe snapshots, never database authority.
pub const MAX_JSON_BYTES: usize = 8 * 1024 * 1024;
#[cfg(feature = "server")]
pub const MAX_COLLECTIONS: usize = 100;
#[cfg(feature = "server")]
pub const MAX_RECIPES: usize = 100;
#[cfg(feature = "server")]
pub const MAX_MEMBERSHIPS: usize = 1_000;
#[cfg(feature = "server")]
pub const MAX_IMAGES: usize = 128;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CollectionImportPreview {
    pub name: String,
    pub collection_count: usize,
    pub recipe_count: usize,
    pub external_image_urls: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CollectionImageReview {
    pub recipe_id: uuid::Uuid,
    pub draft_id: uuid::Uuid,
    pub title: String,
    pub image_count: usize,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CollectionImportResult {
    pub owner_id: uuid::Uuid,
    pub collection_id: uuid::Uuid,
    pub collection_count: usize,
    pub recipe_count: usize,
    pub warnings: Vec<String>,
    pub image_reviews: Vec<CollectionImageReview>,
}

#[cfg(feature = "server")]
pub use server::*;

#[cfg(all(test, feature = "server"))]
#[path = "collection_json_tests.rs"]
mod tests;

#[cfg(feature = "server")]
mod server {
    use super::*;
    use crate::{
        recipe_json::RecipeFile,
        types::{RecipeEditorState, RecipeImage},
    };
    use serde::{Deserialize, Serialize};
    use serde_json::Value;
    use std::collections::{HashMap, HashSet};
    use uuid::Uuid;

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct CollectionFile {
        pub format: String,
        pub version: u32,
        pub snapshot_schema: String,
        pub generator: crate::recipe_json::Generator,
        pub exported_at: chrono::DateTime<chrono::Utc>,
        pub scope: String,
        pub snapshot_digest: String,
        pub snapshot: Value,
    }

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct CollectionSnapshot {
        pub root_collection: String,
        /// Parents precede children; vector order records sibling order.
        pub collections: Vec<PortableCollection>,
        pub recipes: Vec<RecipeFile>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct PortableCollection {
        pub portable_id: String,
        pub parent_collection: Option<String>,
        pub name: String,
        pub description: Option<String>,
        pub image: Option<RecipeImage>,
        pub recipes: Vec<String>,
    }

    #[derive(Debug)]
    pub struct ParsedCollection {
        pub snapshot: CollectionSnapshot,
        pub recipes: Vec<(RecipeFile, RecipeEditorState)>,
    }

    pub fn portable_collection_id(id: Uuid) -> String {
        format!("urn:noms:collection:{id}")
    }

    fn validate(snapshot: CollectionSnapshot) -> Result<ParsedCollection, String> {
        if snapshot.collections.is_empty()
            || snapshot.collections.len() > MAX_COLLECTIONS
            || snapshot.recipes.len() > MAX_RECIPES
        {
            return Err(
                "A collection file supports 1–100 collections and up to 100 recipes.".into(),
            );
        }
        let mut depths = HashMap::new();
        let mut siblings = HashSet::new();
        let mut child_counts = HashMap::<&str, usize>::new();
        let mut references = HashSet::new();
        let mut memberships = 0;
        let mut images = 0;
        for (index, collection) in snapshot.collections.iter().enumerate() {
            let valid_id = collection
                .portable_id
                .strip_prefix("urn:noms:collection:")
                .and_then(|id| Uuid::parse_str(id).ok())
                .is_some_and(|id| portable_collection_id(id) == collection.portable_id);
            if !valid_id
                || depths.contains_key(collection.portable_id.as_str())
                || collection.name.trim().is_empty()
                || collection.name.chars().count() > crate::security::MAX_COLLECTION_NAME_CHARS
                || collection.description.as_ref().is_some_and(|d| {
                    d.chars().count() > crate::security::MAX_COLLECTION_DESCRIPTION_CHARS
                })
                || !siblings.insert((
                    collection.parent_collection.as_deref(),
                    collection.name.to_lowercase(),
                ))
            {
                return Err(
                    "Collection identities, names or descriptions are invalid or duplicated."
                        .into(),
                );
            }
            let depth = match collection.parent_collection.as_deref() {
                None if index == 0 && collection.portable_id == snapshot.root_collection => 1,
                Some(parent) => {
                    let depth = depths.get(parent).ok_or(
                        "Collection parents must precede their children in one connected tree.",
                    )?;
                    let count = child_counts.entry(parent).or_default();
                    *count += 1;
                    if *count > crate::security::MAX_COLLECTION_CHILDREN {
                        return Err("Too many child collections.".into());
                    }
                    depth + 1
                }
                _ => {
                    return Err(
                        "The file must contain one root collection and its descendants.".into(),
                    )
                }
            };
            if depth > crate::security::MAX_COLLECTION_DEPTH {
                return Err("The collection hierarchy is too deep.".into());
            }
            depths.insert(collection.portable_id.as_str(), depth);
            if let Some(image) = &collection.image {
                crate::security::validate_image_metadata(image)
                    .map_err(|_| "Invalid collection cover metadata.")?;
                images += 1;
            }
            let mut members = HashSet::new();
            for recipe in &collection.recipes {
                if !members.insert(recipe) {
                    return Err("A recipe is duplicated within a collection.".into());
                }
                references.insert(recipe.as_str());
            }
            memberships += collection.recipes.len();
        }
        if memberships > MAX_MEMBERSHIPS {
            return Err("A collection file supports up to 1,000 recipe memberships.".into());
        }
        let mut ids = HashSet::new();
        let mut recipes = vec![];
        for recipe in &snapshot.recipes {
            let text = serde_json::to_string(recipe).map_err(|_| "Cannot read recipe snapshot.")?;
            let (file, editor) = crate::recipe_json::parse_recipe(&text)?;
            let source = crate::recipe_json::source_recipe_id(&file)?;
            if file.portable_id != format!("urn:noms:recipe:{source}") {
                return Err("Collection recipe identities must use canonical UUIDs.".into());
            }
            crate::security::validate_recipe_editor(
                &editor,
                crate::security::RecipeValidationMode::Publish,
            )
            .map_err(|_| "Collection recipes must contain valid saved-recipe content.")?;
            if file.version != 3 || !ids.insert(file.portable_id.clone()) {
                return Err(
                    "A collection must contain unique canonical v3 recipe snapshots.".into(),
                );
            }
            let mut steps = editor.instructions.iter().collect::<Vec<_>>();
            images += editor.images.len();
            while let Some(step) = steps.pop() {
                images += step.photos.len();
                steps.extend(&step.sub_steps);
            }
            recipes.push((file, editor));
        }
        if images > MAX_IMAGES {
            return Err("A collection file supports up to 128 image placements. Export a smaller collection.".into());
        }
        if references.len() != ids.len() || references.iter().any(|id| !ids.contains(*id)) {
            return Err("Every collection recipe must have exactly one snapshot, and every snapshot must be used.".into());
        }
        Ok(ParsedCollection { snapshot, recipes })
    }

    pub fn export(snapshot: CollectionSnapshot) -> Result<String, String> {
        let parsed = validate(snapshot)?;
        let snapshot =
            serde_json::to_value(parsed.snapshot).map_err(|_| "Cannot serialize collection.")?;
        let file = CollectionFile {
            format: "noms.collection.json".into(),
            version: 1,
            snapshot_schema: "noms.collection.snapshot.v1".into(),
            generator: crate::recipe_json::Generator {
                name: "Noms".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
            exported_at: chrono::Utc::now(),
            scope: "collection_tree".into(),
            snapshot_digest: crate::recipe_json::digest(&snapshot)?,
            snapshot,
        };
        let text =
            serde_json::to_string_pretty(&file).map_err(|_| "Cannot serialize collection JSON.")?;
        if text.len() > MAX_JSON_BYTES {
            return Err("Collection JSON exceeds 8 MiB. Export a smaller collection.".into());
        }
        Ok(text)
    }

    pub fn parse(text: &str) -> Result<ParsedCollection, String> {
        if text.is_empty() || text.len() > MAX_JSON_BYTES {
            return Err("Choose a nonempty Noms collection JSON file up to 8 MiB.".into());
        }
        let file: CollectionFile = serde_json::from_str(text)
            .map_err(|_| "This is not a valid Noms collection JSON file.")?;
        if file.format != "noms.collection.json"
            || file.version != 1
            || file.snapshot_schema != "noms.collection.snapshot.v1"
            || file.scope != "collection_tree"
        {
            return Err("This collection format, version or scope is not supported.".into());
        }
        if file.snapshot_digest != crate::recipe_json::digest(&file.snapshot)? {
            return Err("The collection checksum does not match. The file may have been changed or damaged.".into());
        }
        let snapshot: CollectionSnapshot = serde_json::from_value(file.snapshot.clone())
            .map_err(|_| "Invalid collection snapshot fields.")?;
        if serde_json::to_value(&snapshot).map_err(|_| "Invalid collection snapshot.")?
            != file.snapshot
        {
            return Err("The collection contains missing or unsupported fields.".into());
        }
        validate(snapshot)
    }
}
