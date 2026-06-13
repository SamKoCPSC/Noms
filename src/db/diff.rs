//! Reverse-diff library for recipe versioning.
//!
//! Uses JSON Patch (RFC 6902) to store minimal diffs between recipe versions.
//! The latest version stores a full snapshot; historical versions store a
//! `reverse_diff` patch that, when applied to the next version, reconstructs
//! this version.
//!
//! Example chain:
//!   v3 (latest, full snapshot) --reverse_diff--> v2 --reverse_diff--> v1
//!
//! To reconstruct v1: apply v2's reverse_diff to v2, then apply v1's reverse_diff to result.

use serde_json::Value;

#[cfg(feature = "server")]
use crate::db::DbError;

/// Serializable snapshot of a recipe's editable fields.
///
/// Lightweight version of the full Recipe struct, containing only
/// the fields that participate in versioning.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RecipeSnapshot {
    pub title: String,
    pub description: Option<String>,
    pub prep_time_min: Option<i32>,
    pub cook_time_min: Option<i32>,
    pub total_time_min: Option<i32>,
    pub servings: Option<i32>,
    pub ingredients: Vec<Value>,
    pub steps: Vec<Value>,
}

/// Convert a `RecipeVersion` row into a JSON Value for diff computation.
#[cfg(feature = "server")]
pub fn recipe_to_json(version: &crate::db::RecipeVersion) -> Result<Value, DbError> {
    let snapshot = RecipeSnapshot {
        title: version
            .title
            .clone()
            .ok_or_else(|| DbError::DiffError("version title is NULL".to_string()))?,
        description: version.description.clone(),
        prep_time_min: version.prep_time_min,
        cook_time_min: version.cook_time_min,
        total_time_min: version.total_time_min,
        servings: version.servings,
        ingredients: version
            .ingredients
            .as_ref()
            .and_then(|v| v.as_array())
            .map(|arr| arr.to_vec())
            .unwrap_or_default(),
        steps: version
            .steps
            .as_ref()
            .and_then(|v| v.as_array())
            .map(|arr| arr.to_vec())
            .unwrap_or_default(),
    };
    serde_json::to_value(snapshot).map_err(|e| DbError::DiffError(e.to_string()))
}

/// Convert a JSON Value back into a `RecipeSnapshot`.
#[cfg(feature = "server")]
pub fn json_to_recipe(value: &Value) -> Result<RecipeSnapshot, DbError> {
    serde_json::from_value(value.clone()).map_err(|e| DbError::DiffError(e.to_string()))
}

/// Compute a JSON Patch (RFC 6902) from `old_doc` to `new_doc`.
#[cfg(feature = "server")]
pub fn compute_diff(old_doc: &Value, new_doc: &Value) -> Result<json_patch::Patch, DbError> {
    Ok(json_patch::diff(old_doc, new_doc))
}

/// Reverse a JSON Patch by inverting each operation.
///
/// Given patch A -> B, produces patch B -> A.
/// Requires `old_doc` to extract values for reversed `remove` and `replace` operations.
///
/// Algorithm:
/// - `add` -> `remove` (same path)
/// - `remove` -> `add` (same path, value from old_doc)
/// - `replace` -> `replace` (same path, old value from old_doc)
/// - `move` -> `move` (swap from and path)
/// - `copy` -> no-op
/// - `test` -> no-op
#[cfg(feature = "server")]
pub fn reverse_patch(
    patch: &json_patch::Patch,
    old_doc: &Value,
) -> Result<json_patch::Patch, DbError> {
    use json_patch::{AddOperation, MoveOperation, PatchOperation, RemoveOperation, ReplaceOperation};

    let mut reversed = Vec::new();

    for op in patch.iter() {
        match op {
            PatchOperation::Add(AddOperation { path, .. }) => {
                reversed.push(PatchOperation::Remove(RemoveOperation { path: path.clone() }));
            }
            PatchOperation::Remove(RemoveOperation { path }) => {
                let old_value = path.resolve(old_doc).cloned().unwrap_or(Value::Null);
                reversed.push(PatchOperation::Add(AddOperation {
                    path: path.clone(),
                    value: old_value,
                }));
            }
            PatchOperation::Replace(ReplaceOperation { path, .. }) => {
                let old_value = path.resolve(old_doc).cloned().unwrap_or(Value::Null);
                reversed.push(PatchOperation::Replace(ReplaceOperation {
                    path: path.clone(),
                    value: old_value,
                }));
            }
            PatchOperation::Move(MoveOperation { from, path }) => {
                reversed.push(PatchOperation::Move(MoveOperation {
                    from: path.clone(),
                    path: from.clone(),
                }));
            }
            PatchOperation::Copy { .. } | PatchOperation::Test { .. } => {
                // No-op for reverse
            }
        }
    }

    Ok(json_patch::Patch(reversed))
}

/// Reconstruct an older version from a chain of reverse diffs.
///
/// Starting from `latest_json`, applies reverse_diff patches in order
/// (newest to oldest) to reconstruct the target version.
#[cfg(feature = "server")]
pub fn reconstruct_from_chain(
    latest_json: &Value,
    reverse_diffs: &[Value],
) -> Result<Value, DbError> {
    let mut current = latest_json.clone();

    for diff_value in reverse_diffs {
        let patch: json_patch::Patch = serde_json::from_value(diff_value.clone())
            .map_err(|e| DbError::DiffError(format!("invalid reverse_diff: {e}")))?;
        json_patch::patch(&mut current, &patch)
            .map_err(|e| DbError::DiffError(format!("failed to apply reverse_diff: {e}")))?;
    }

    Ok(current)
}

/// Serialize recipe fields from an incoming request into a JSON snapshot.
///
/// Used by `update_recipe_versioned()` to create the "new" JSON representation
/// before computing diffs. Takes the same field types as the API request body.
#[cfg(feature = "server")]
#[allow(clippy::too_many_arguments)]
pub fn recipe_to_json_from_fields(
    title: &str,
    description: Option<&str>,
    prep_time_min: Option<i32>,
    cook_time_min: Option<i32>,
    total_time_min: Option<i32>,
    servings: Option<i32>,
    ingredients: &Option<serde_json::Value>,
    steps: &Option<serde_json::Value>,
) -> Result<Value, DbError> {
    let mut obj = serde_json::Map::new();
    obj.insert("title".into(), Value::String(title.to_string()));
    obj.insert(
        "description".into(),
        description.map(|s| Value::String(s.to_string())).unwrap_or(Value::Null),
    );
    obj.insert(
        "prep_time_min".into(),
        prep_time_min.map(|v| Value::Number(v.into())).unwrap_or(Value::Null),
    );
    obj.insert(
        "cook_time_min".into(),
        cook_time_min.map(|v| Value::Number(v.into())).unwrap_or(Value::Null),
    );
    obj.insert(
        "total_time_min".into(),
        total_time_min.map(|v| Value::Number(v.into())).unwrap_or(Value::Null),
    );
    obj.insert(
        "servings".into(),
        servings.map(|v| Value::Number(v.into())).unwrap_or(Value::Null),
    );
    obj.insert(
        "ingredients".into(),
        ingredients.clone().unwrap_or(Value::Null),
    );
    obj.insert("steps".into(), steps.clone().unwrap_or(Value::Null));
    Ok(Value::Object(obj))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_recipe_snapshot_roundtrip() {
        let snapshot = RecipeSnapshot {
            title: "Test Recipe".to_string(),
            description: Some("A test".to_string()),
            prep_time_min: Some(10),
            cook_time_min: Some(20),
            total_time_min: Some(30),
            servings: Some(4),
            ingredients: vec![json!({"name": "flour", "amount": "2 cups"})],
            steps: vec![json!({"step": 1, "instruction": "Mix"})],
        };
        let json = serde_json::to_value(&snapshot).unwrap();
        let restored = serde_json::from_value::<RecipeSnapshot>(json).unwrap();
        assert_eq!(restored.title, snapshot.title);
    }

    #[test]
    fn test_compute_and_apply_diff() {
        let old = json!({"title": "Old", "servings": 2});
        let new = json!({"title": "New", "servings": 4, "description": "Added"});
        let patch = json_patch::diff(&old, &new);
        let mut result = old.clone();
        json_patch::patch(&mut result, &patch).unwrap();
        assert_eq!(result, new);
    }

    #[test]
    fn test_reverse_patch_roundtrip() {
        let old = json!({"title": "Original", "servings": 2, "ingredients": [1, 2, 3]});
        let new = json!({"title": "Modified", "servings": 4, "ingredients": [1, 2, 3, 4], "extra": true});
        let forward = json_patch::diff(&old, &new);
        let reverse = super::reverse_patch(&forward, &old).unwrap();
        let mut doc = old.clone();
        json_patch::patch(&mut doc, &forward).unwrap();
        assert_eq!(doc, new);
        json_patch::patch(&mut doc, &reverse).unwrap();
        assert_eq!(doc, old);
    }

    #[test]
    fn test_reverse_patch_remove_operation() {
        let old = json!({"title": "Keep", "description": "Will be removed"});
        let new = json!({"title": "Keep"});
        let forward = json_patch::diff(&old, &new);
        let reverse = super::reverse_patch(&forward, &old).unwrap();
        let mut doc = new.clone();
        json_patch::patch(&mut doc, &reverse).unwrap();
        assert_eq!(doc, old);
    }

    #[test]
    fn test_reconstruct_from_chain() {
        let v1 = json!({"title": "V1", "servings": 2});
        let v2 = json!({"title": "V2", "servings": 4});
        let v3 = json!({"title": "V3", "servings": 6});
        let v2_reverse = json_patch::diff(&v3, &v2);
        let v1_reverse = json_patch::diff(&v2, &v1);
        let diffs = vec![
            serde_json::to_value(&v2_reverse).unwrap(),
            serde_json::to_value(&v1_reverse).unwrap(),
        ];
        let reconstructed = super::reconstruct_from_chain(&v3, &diffs).unwrap();
        assert_eq!(reconstructed, v1);
    }

    #[test]
    fn test_reconstruct_single_step() {
        let v1 = json!({"title": "Original"});
        let v2 = json!({"title": "Updated"});
        let reverse = json_patch::diff(&v2, &v1);
        let diffs = vec![serde_json::to_value(&reverse).unwrap()];
        let result = super::reconstruct_from_chain(&v2, &diffs).unwrap();
        assert_eq!(result, v1);
    }

    #[test]
    fn test_empty_diff() {
        let doc = json!({"title": "Same"});
        let patch = json_patch::diff(&doc, &doc);
        assert!(patch.is_empty());
    }

    #[test]
    fn test_reverse_empty_patch() {
        let doc = json!({"title": "Same"});
        let patch = json_patch::Patch::default();
        let reverse = super::reverse_patch(&patch, &doc).unwrap();
        assert!(reverse.is_empty());
    }

    #[test]
    fn test_recipe_to_json_from_fields_includes_null_fields() {
        // When fields are None, the JSON should still include the keys (as null),
        // not omit them entirely. This matches recipe_to_json's behavior.
        let result = super::recipe_to_json_from_fields(
            "Test",
            None,
            None,
            None,
            None,
            None,
            &None,
            &None,
        )
        .unwrap();

        // All keys must be present
        assert!(result.get("title").is_some());
        assert!(result.get("description").is_some());
        assert!(result.get("prep_time_min").is_some());
        assert!(result.get("cook_time_min").is_some());
        assert!(result.get("total_time_min").is_some());
        assert!(result.get("servings").is_some());
        assert!(result.get("ingredients").is_some());
        assert!(result.get("steps").is_some());

        // None fields should be null, not omitted
        assert_eq!(result.get("description").unwrap(), &json!(null));
        assert_eq!(result.get("prep_time_min").unwrap(), &json!(null));
        assert_eq!(result.get("cook_time_min").unwrap(), &json!(null));
        assert_eq!(result.get("total_time_min").unwrap(), &json!(null));
        assert_eq!(result.get("servings").unwrap(), &json!(null));
        assert_eq!(result.get("ingredients").unwrap(), &json!(null));
        assert_eq!(result.get("steps").unwrap(), &json!(null));
    }

    #[test]
    fn test_recipe_to_json_from_fields_no_spurious_diff_with_nulls() {
        // When both old and new have the same None fields, the diff should be empty
        // (no spurious add/remove operations for fields that are null on both sides).
        let old = super::recipe_to_json_from_fields(
            "Title", None, None, None, None, None, &None, &None,
        )
        .unwrap();
        let new = super::recipe_to_json_from_fields(
            "Title", None, None, None, None, None, &None, &None,
        )
        .unwrap();
        let diff = super::compute_diff(&old, &new).unwrap();
        assert!(diff.is_empty(), "diff should be empty when fields are identical");
    }
}
