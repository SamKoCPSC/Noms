use serde::{Deserialize, Serialize};

use crate::db::DbError;
use crate::types::{Recipe, RecipeEquipment, RecipeImage, RecipeIngredient, RecipeStep};

/// Canonical immutable recipe content. Access policy lives on the `recipes` row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeSnapshot {
    pub title: String,
    pub description: Option<String>,
    pub commentary: Option<String>,
    pub prep_time_minutes: Option<i32>,
    pub cook_time_minutes: Option<i32>,
    pub servings: Option<i32>,
    pub ingredients: Vec<RecipeIngredient>,
    pub instructions: Vec<RecipeStep>,
    pub equipment: Vec<RecipeEquipment>,
    pub images: Vec<RecipeImage>,
    pub tags: Vec<String>,
}

/// Mutable draft content plus the visibility used only when first published.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeDraftPayload {
    #[serde(flatten)]
    pub content: RecipeSnapshot,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_visibility: Option<String>,
}

impl std::ops::Deref for RecipeDraftPayload {
    type Target = RecipeSnapshot;

    fn deref(&self) -> &Self::Target {
        &self.content
    }
}

impl std::ops::DerefMut for RecipeDraftPayload {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.content
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapshotChange {
    pub op: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_value: Option<serde_json::Value>,
}

pub fn recipe_to_snapshot(recipe: &Recipe, tags: Vec<String>) -> RecipeSnapshot {
    RecipeSnapshot {
        title: recipe.title.clone(),
        description: recipe.description.clone(),
        commentary: recipe.commentary.clone(),
        prep_time_minutes: recipe.prep_time_minutes,
        cook_time_minutes: recipe.cook_time_minutes,
        servings: recipe.servings,
        ingredients: recipe.ingredients.clone(),
        instructions: recipe.instructions.clone(),
        equipment: recipe.equipment.clone(),
        images: recipe.images.clone(),
        tags,
    }
}

pub fn snapshot_to_json(snapshot: &RecipeSnapshot) -> Result<serde_json::Value, DbError> {
    serde_json::to_value(snapshot).map_err(DbError::SerdeJson)
}

pub fn json_to_snapshot(value: &serde_json::Value) -> Result<RecipeSnapshot, DbError> {
    serde_json::from_value(value.clone()).map_err(DbError::SerdeJson)
}

#[cfg(test)]
mod draft_payload_tests {
    use super::{RecipeDraftPayload, RecipeSnapshot};

    fn content() -> RecipeSnapshot {
        RecipeSnapshot {
            title: "Test recipe".to_string(),
            description: None,
            commentary: None,
            prep_time_minutes: None,
            cook_time_minutes: None,
            servings: None,
            ingredients: Vec::new(),
            instructions: Vec::new(),
            equipment: Vec::new(),
            images: Vec::new(),
            tags: Vec::new(),
        }
    }

    #[test]
    fn existing_recipe_draft_payload_omits_visibility() {
        let serialized = serde_json::to_value(RecipeDraftPayload {
            content: content(),
            initial_visibility: None,
        })
        .unwrap();

        assert!(serialized.get("visibility").is_none());
        assert!(serialized.get("initial_visibility").is_none());
    }

    #[test]
    fn new_recipe_draft_payload_records_initial_visibility() {
        let serialized = serde_json::to_value(RecipeDraftPayload {
            content: content(),
            initial_visibility: Some("unlisted".to_string()),
        })
        .unwrap();

        assert_eq!(serialized["initial_visibility"], "unlisted");
    }
}

pub fn compute_diff(
    old_snapshot: &RecipeSnapshot,
    new_snapshot: &RecipeSnapshot,
) -> Result<Vec<SnapshotChange>, DbError> {
    let old_value = snapshot_to_json(old_snapshot)?;
    let new_value = snapshot_to_json(new_snapshot)?;
    let mut changes = Vec::new();
    diff_values("", &old_value, &new_value, &mut changes);
    Ok(changes)
}

fn diff_values(
    path: &str,
    old_value: &serde_json::Value,
    new_value: &serde_json::Value,
    changes: &mut Vec<SnapshotChange>,
) {
    match (old_value, new_value) {
        (serde_json::Value::Object(old), serde_json::Value::Object(new)) => {
            let mut keys = old.keys().chain(new.keys()).collect::<Vec<_>>();
            keys.sort();
            keys.dedup();
            for key in keys {
                let child_path = format!("{path}/{}", escape_json_pointer_token(key));
                match (old.get(key), new.get(key)) {
                    (Some(old), Some(new)) => diff_values(&child_path, old, new, changes),
                    (Some(old), None) => changes.push(SnapshotChange {
                        op: "remove".to_string(),
                        path: child_path,
                        value: None,
                        old_value: Some(old.clone()),
                    }),
                    (None, Some(new)) => changes.push(SnapshotChange {
                        op: "add".to_string(),
                        path: child_path,
                        value: Some(new.clone()),
                        old_value: None,
                    }),
                    (None, None) => {}
                }
            }
        }
        (serde_json::Value::Array(old), serde_json::Value::Array(new)) => {
            let shared_length = old.len().min(new.len());
            for index in 0..shared_length {
                diff_values(
                    &format!("{path}/{index}"),
                    &old[index],
                    &new[index],
                    changes,
                );
            }
            for index in (new.len()..old.len()).rev() {
                changes.push(SnapshotChange {
                    op: "remove".to_string(),
                    path: format!("{path}/{index}"),
                    value: None,
                    old_value: Some(old[index].clone()),
                });
            }
            for (index, value) in new.iter().enumerate().skip(old.len()) {
                changes.push(SnapshotChange {
                    op: "add".to_string(),
                    path: format!("{path}/{index}"),
                    value: Some(value.clone()),
                    old_value: None,
                });
            }
        }
        _ if old_value != new_value => changes.push(SnapshotChange {
            op: "replace".to_string(),
            path: path.to_string(),
            value: Some(new_value.clone()),
            old_value: Some(old_value.clone()),
        }),
        _ => {}
    }
}

pub fn escape_json_pointer_token(token: &str) -> String {
    token.replace('~', "~0").replace('/', "~1")
}

pub fn change_summary(diff: &[SnapshotChange]) -> String {
    if diff.is_empty() {
        return "No content changes".to_string();
    }
    let mut fields = diff
        .iter()
        .filter_map(|change| change.path.split('/').nth(1))
        .map(|field| field.replace("~1", "/").replace("~0", "~"))
        .collect::<Vec<_>>();
    fields.sort();
    fields.dedup();
    match fields.as_slice() {
        [] => "Updated recipe".to_string(),
        [field] => format!("Updated {field}"),
        [first, second] => format!("Updated {first} and {second}"),
        _ => {
            let last = fields.pop().expect("non-empty fields checked above");
            format!("Updated {}, and {last}", fields.join(", "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        Recipe, RecipeEquipment, RecipeImage, RecipeImageEdit, RecipeIngredient, RecipeStep,
    };
    use chrono::Utc;
    use uuid::Uuid;

    fn complete_recipe() -> Recipe {
        let image = RecipeImage::new(
            "https://example.test/full.webp".to_string(),
            Some("https://example.test/thumb.webp".to_string()),
            RecipeImageEdit::default_rect_16x9(),
        )
        .with_file_metadata("dinner.webp".to_string(), 42);

        Recipe {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            title: "Dinner / special~edition".to_string(),
            description: Some("A complete snapshot".to_string()),
            commentary: None,
            prep_time_minutes: Some(10),
            cook_time_minutes: Some(30),
            servings: Some(4),
            ingredients: vec![RecipeIngredient {
                vocabulary_id: None,
                amount: "2".to_string(),
                unit: "cups".to_string(),
                name: "beans".to_string(),
                ..Default::default()
            }],
            instructions: vec![RecipeStep {
                text: "Cook".to_string(),
                sub_steps: vec![RecipeStep {
                    text: "Stir".to_string(),
                    sub_steps: vec![],
                    photos: vec![image.clone()],
                }],
                photos: vec![],
            }],
            equipment: vec![RecipeEquipment {
                name: "Pot".to_string(),
            }],
            images: vec![image],
            visibility: "public".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            author_username: "owner".to_string(),
            author_avatar_url: None,
        }
    }

    #[test]
    fn snapshot_round_trip_preserves_all_canonical_fields() {
        let recipe = complete_recipe();
        let snapshot = recipe_to_snapshot(&recipe, vec!["quick".into(), "vegan".into()]);
        let json = snapshot_to_json(&snapshot).unwrap();
        let decoded = json_to_snapshot(&json).unwrap();

        assert_eq!(decoded, snapshot);
        assert_eq!(decoded.instructions[0].sub_steps[0].photos.len(), 1);
        assert_eq!(decoded.images[0].file_name.as_deref(), Some("dinner.webp"));
        assert_eq!(decoded.tags, vec!["quick", "vegan"]);
    }

    #[test]
    fn diff_reports_scalars_arrays_nested_steps_and_escaped_pointer_paths() {
        let recipe = complete_recipe();
        let old = recipe_to_snapshot(&recipe, vec!["quick".into()]);
        let mut new = old.clone();
        new.title = "Dinner revised".to_string();
        new.description = None;
        new.ingredients.push(RecipeIngredient {
            vocabulary_id: None,
            amount: "1".to_string(),
            unit: "tsp".to_string(),
            name: "salt".to_string(),
            ..Default::default()
        });
        new.instructions[0].sub_steps[0].text = "Stir gently".to_string();
        new.tags = vec!["weeknight".into()];

        let diff = compute_diff(&old, &new).unwrap();
        let encoded = serde_json::to_string(&diff).unwrap();
        assert!(encoded.contains("/title"));
        assert!(encoded.contains("/description"));
        assert!(encoded.contains("/ingredients/1"));
        assert!(encoded.contains("/instructions/0/sub_steps/0/text"));
        assert!(encoded.contains("/tags"));
        assert!(!change_summary(&diff).is_empty());

        let escaped = escape_json_pointer_token("Dinner / special~edition");
        assert_eq!(escaped, "Dinner ~1 special~0edition");
    }

    #[test]
    fn reordered_arrays_and_empty_or_null_fields_round_trip() {
        let recipe = complete_recipe();
        let mut old = recipe_to_snapshot(&recipe, vec![]);
        old.description = None;
        old.ingredients.push(RecipeIngredient {
            vocabulary_id: None,
            amount: "1".into(),
            unit: "cup".into(),
            name: "rice".into(),
            ..Default::default()
        });
        let mut new = old.clone();
        new.ingredients.reverse();
        new.instructions.clear();
        new.equipment.clear();
        new.images.clear();

        let diff = compute_diff(&old, &new).unwrap();
        assert!(!diff.is_empty());
        assert_eq!(
            json_to_snapshot(&snapshot_to_json(&new).unwrap()).unwrap(),
            new
        );
    }

    #[test]
    fn change_summary_lists_every_changed_field() {
        let diff = vec![
            SnapshotChange {
                op: "replace".into(),
                path: "/title".into(),
                value: Some(serde_json::json!("New title")),
                old_value: Some(serde_json::json!("Old title")),
            },
            SnapshotChange {
                op: "replace".into(),
                path: "/description".into(),
                value: Some(serde_json::json!("New description")),
                old_value: Some(serde_json::json!("Old description")),
            },
            SnapshotChange {
                op: "add".into(),
                path: "/tags/1".into(),
                value: Some(serde_json::json!("quick")),
                old_value: None,
            },
        ];

        assert_eq!(
            change_summary(&diff),
            "Updated description, tags, and title"
        );
    }

    #[test]
    fn change_summary_lists_a_changed_field_only_once() {
        let diff = vec![
            SnapshotChange {
                op: "replace".into(),
                path: "/ingredients/0/name".into(),
                value: Some(serde_json::json!("sea salt")),
                old_value: Some(serde_json::json!("salt")),
            },
            SnapshotChange {
                op: "replace".into(),
                path: "/ingredients/0/amount".into(),
                value: Some(serde_json::json!("2")),
                old_value: Some(serde_json::json!("1")),
            },
        ];

        assert_eq!(change_summary(&diff), "Updated ingredients");
    }

    #[test]
    fn change_summary_joins_exactly_two_changed_fields() {
        let diff = vec![
            SnapshotChange {
                op: "replace".into(),
                path: "/title".into(),
                value: Some(serde_json::json!("New title")),
                old_value: Some(serde_json::json!("Old title")),
            },
            SnapshotChange {
                op: "replace".into(),
                path: "/description".into(),
                value: Some(serde_json::json!("New description")),
                old_value: Some(serde_json::json!("Old description")),
            },
        ];

        assert_eq!(change_summary(&diff), "Updated description and title");
    }

    #[test]
    fn change_summary_preserves_the_empty_diff_message() {
        assert_eq!(change_summary(&[]), "No content changes");
    }
}
