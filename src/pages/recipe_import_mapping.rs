use crate::types::{RecipeEditorState, RecipeEquipment, RecipeIngredient, RecipeStep};
use serde_json::Value;

/// Converts preview data into the existing private draft contract without fetching media.
pub(super) fn map_import_candidate(
    candidate: &Value,
    keep_context: bool,
) -> Result<RecipeEditorState, String> {
    if candidate["schemaVersion"] != 1
        || !matches!(candidate["status"].as_str(), Some("ready" | "needs_review"))
    {
        return Err("This clipboard does not contain a supported recipe candidate.".into());
    }
    let fields = &candidate["fields"];
    let mut editor = RecipeEditorState {
        title: required_text(&fields["title"]["value"])?,
        ..Default::default()
    };
    editor.description = fields["description"]["value"].as_str().map(str::to_owned);
    editor.prep_time_minutes = optional_number(&fields["prepTime"]["value"])?;
    editor.cook_time_minutes = optional_number(&fields["cookTime"]["value"])?;
    editor.servings = optional_number(&fields["yield"]["value"]["servings"])?;
    if editor.servings == Some(0) {
        return Err("Servings must be greater than zero.".into());
    }
    let mut context = Vec::new();
    for (key, label) in [
        ("suppliedUrl", "Source"),
        ("author", "Author"),
        ("attribution", "Attribution"),
    ] {
        if let Some(text) = candidate["source"][key].as_str() {
            context.push(format!("{label}: {text}"));
        }
    }
    for (key, label) in [("totalTime", "Total time"), ("yield", "Yield")] {
        if let Some(text) = fields[key]["raw"].as_str() {
            context.push(format!("{label}: {text}"));
        }
    }
    for section in fields["ingredientSections"]
        .as_array()
        .ok_or("Missing ingredients.")?
    {
        if let Some(name) = section["name"].as_str() {
            context.push(format!("Ingredient group: {name}"));
        }
        for item in section["items"].as_array().ok_or("Invalid ingredients.")? {
            editor.ingredients.push(RecipeIngredient {
                name: required_text(&item["name"])?,
                amount: item["amount"].as_str().unwrap_or_default().into(),
                unit: item["unit"].as_str().unwrap_or_default().into(),
            });
            if let Some(raw) = item["raw"].as_str() {
                context.push(format!("Ingredient: {raw}"));
            }
        }
    }
    for section in fields["instructionSections"]
        .as_array()
        .ok_or("Missing instructions.")?
    {
        if let Some(name) = section["name"].as_str() {
            context.push(format!("Instruction group: {name}"));
        }
        for step in section["steps"].as_array().ok_or("Invalid instructions.")? {
            editor.instructions.push(map_step(step, 0)?);
            if let Some(text) = step["text"].as_str() {
                context.push(format!("Step {}: {text}", editor.instructions.len()));
            }
        }
    }
    if editor.ingredients.is_empty() || editor.instructions.is_empty() {
        return Err("A recipe needs ingredients and instructions before creating a draft.".into());
    }
    if let Some(items) = fields["equipment"].as_array() {
        for item in items {
            editor.equipment.push(RecipeEquipment {
                name: required_text(&item["value"])?,
            });
        }
    }
    if let Some(notes) = fields["notes"].as_array() {
        for note in notes {
            if let Some(text) = note["value"].as_str() {
                context.push(text.into());
            }
        }
    }
    if keep_context && !context.is_empty() {
        editor.commentary = Some(context.join("\n"));
    }
    Ok(editor)
}

/// Without an intermediate review screen, actionable warnings and unmapped
/// text belong in editable Notes so they survive autosave and reload.
pub(super) fn map_direct_import_candidate(candidate: &Value) -> Result<RecipeEditorState, String> {
    let mut editor = map_import_candidate(candidate, true)?;
    if let Some(provider) = candidate["source"]["provider"].as_str() {
        let original = editor.commentary.take().unwrap_or_default();
        editor.commentary = Some(format!("Recipe website: {provider}\n{original}"));
    }
    let mut review = Vec::new();
    if let Some(warnings) = candidate["warnings"].as_array() {
        for warning in warnings {
            if matches!(warning["severity"].as_str(), Some("warning" | "error")) {
                if let Some(message) = warning["message"].as_str() {
                    review.push(message.to_owned());
                }
            }
        }
    }
    if let Some(lines) = candidate["unclassifiedLines"].as_array() {
        for line in lines {
            if let Some(raw) = line["raw"].as_str() {
                review.push(format!("Additional recipe text: {raw}"));
            }
        }
    }
    if let Some(alternatives) = candidate["competingCandidates"].as_array() {
        for alternative in alternatives {
            if let Some(title) = alternative["title"].as_str() {
                let reason = alternative["reason"]
                    .as_str()
                    .unwrap_or("Other recipe region");
                review.push(format!("Other recipe region: {title} — {reason}"));
            }
        }
    }
    if !review.is_empty() {
        let notes = editor.commentary.get_or_insert_with(String::new);
        if !notes.is_empty() {
            notes.push_str("\n\n");
        }
        notes.push_str("Import review — check these details before publishing:\n");
        notes.push_str(&review.join("\n"));
    }
    Ok(editor)
}

/// Reject malformed core text rather than silently losing content during mapping.
fn required_text(value: &Value) -> Result<String, String> {
    value
        .as_str()
        .filter(|text| !text.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| "A required recipe field is missing.".into())
}

/// The editor stores signed 32-bit whole minutes and servings.
fn optional_number(value: &Value) -> Result<Option<i32>, String> {
    if value.is_null() {
        return Ok(None);
    }
    let number = value
        .as_i64()
        .and_then(|number| i32::try_from(number).ok())
        .filter(|number| *number >= 0)
        .ok_or("A recipe number is outside the supported range.")?;
    Ok(Some(number))
}

/// Preserve nesting up to the editor limit; never truncate deeper instructions.
fn map_step(value: &Value, depth: usize) -> Result<RecipeStep, String> {
    if depth > 3 {
        return Err("Instructions exceed the editor's three nesting levels.".into());
    }
    let children = value["substeps"]
        .as_array()
        .ok_or("Invalid instruction nesting.")?;
    Ok(RecipeStep {
        text: required_text(&value["text"])?,
        sub_steps: children
            .iter()
            .map(|child| map_step(child, depth + 1))
            .collect::<Result<_, _>>()?,
        photos: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::map_import_candidate;
    use serde_json::{json, Value};

    fn candidate() -> Value {
        serde_json::from_str(include_str!(
            "../../tests/fixtures/recipe-import/v1/basic-visible-recipe/expected.json"
        ))
        .unwrap()
    }

    #[test]
    fn import_maps_recipe_fields_to_a_private_editor_without_remote_images() {
        let c = candidate();
        let editor = map_import_candidate(&c, true).unwrap();
        assert_eq!(
            editor.title,
            c["fields"]["title"]["value"].as_str().unwrap()
        );
        assert_eq!(editor.visibility, "private");
        assert!(editor.images.is_empty());
        assert!(editor.tags.is_empty());
        assert!(!editor.ingredients.is_empty());
        assert!(!editor.instructions.is_empty());
        assert_eq!(
            editor.ingredients[0].amount,
            c["fields"]["ingredientSections"][0]["items"][0]["amount"]
                .as_str()
                .unwrap()
        );
        let encoded = serde_json::to_string(&editor).unwrap();
        assert_eq!(
            serde_json::from_str::<crate::types::RecipeEditorState>(&encoded).unwrap(),
            editor
        );
    }

    #[test]
    fn import_requires_supported_candidate_with_complete_core_fields() {
        for status in ["unsupported", "unknown"] {
            let mut c = candidate();
            c["status"] = json!(status);
            assert!(map_import_candidate(&c, true).is_err());
        }
        for field in ["title", "ingredientSections", "instructionSections"] {
            let mut c = candidate();
            c["fields"][field] = Value::Null;
            assert!(map_import_candidate(&c, true).is_err());
        }
        let mut c = candidate();
        c["schemaVersion"] = json!(2);
        assert!(map_import_candidate(&c, true).is_err());
    }

    #[test]
    fn import_preserves_unmapped_source_notes_and_group_context_only_with_consent() {
        let mut c = candidate();
        c["source"]["suppliedUrl"] = json!("https://example.invalid/recipe");
        c["source"]["author"] = json!("Aster Vale");
        c["fields"]["ingredientSections"][0]["name"] = json!("Filling");
        c["fields"]["notes"] = json!([{"raw":"Chill overnight.","value":"Chill overnight."}]);
        c["fields"]["totalTime"] = json!({"raw":"2 hours","value":120});
        c["fields"]["yield"] =
            json!({"raw":"two bowls","value":{"display":"two bowls","servings":null}});
        c["fields"]["images"] = json!([{"sourceUrl":"https://example.invalid/never-fetch.jpg"}]);
        let editor = map_import_candidate(&c, true).unwrap();
        let commentary = editor.commentary.unwrap();
        for text in [
            "https://example.invalid/recipe",
            "Aster Vale",
            "Filling",
            "Chill overnight.",
            "2 hours",
            "two bowls",
        ] {
            assert!(
                commentary.contains(text),
                "Missing {text} from {commentary}"
            );
        }
        assert!(editor.images.is_empty());
        assert!(map_import_candidate(&c, false)
            .unwrap()
            .commentary
            .is_none());
    }

    #[test]
    fn import_preserves_compound_amounts_and_nested_steps() {
        let mut c = candidate();
        c["fields"]["ingredientSections"][0]["items"][0]["amount"] =
            json!("1 cup plus 2 tablespoons");
        c["fields"]["ingredientSections"][0]["items"][0]["unit"] = Value::Null;
        c["fields"]["instructionSections"][0]["steps"][0]["substeps"] =
            json!([{"text":"Stir gently.","raw":"Stir gently.","substeps":[]}]);
        let editor = map_import_candidate(&c, true).unwrap();
        assert_eq!(editor.ingredients[0].amount, "1 cup plus 2 tablespoons");
        assert_eq!(editor.ingredients[0].unit, "");
        assert_eq!(editor.instructions[0].sub_steps[0].text, "Stir gently.");
    }

    #[test]
    fn import_rejects_overflow_and_incomplete_items_instead_of_dropping_them() {
        let mut c = candidate();
        c["fields"]["prepTime"]["value"] = json!(i64::MAX);
        assert!(map_import_candidate(&c, true).is_err());
        let mut c = candidate();
        c["fields"]["ingredientSections"][0]["items"][0]["name"] = json!("");
        assert!(map_import_candidate(&c, true).is_err());
        let mut c = candidate();
        c["fields"]["cookTime"] = json!({"raw":"minus one", "value":-1});
        assert!(map_import_candidate(&c, true).is_err());
        let mut c = candidate();
        c["fields"]["yield"] = json!({"raw":"zero", "value":{"display":"zero", "servings":0}});
        assert!(map_import_candidate(&c, true).is_err());
    }

    #[test]
    fn import_maps_equipment_and_keeps_untrusted_text_literal() {
        let mut c = candidate();
        c["fields"]["title"]["value"] = json!("<img src=x onerror=alert(1)> Soup");
        c["fields"]["equipment"] = json!([{"raw":"Large pot","value":"Large pot"}]);
        let editor = map_import_candidate(&c, true).unwrap();
        assert_eq!(editor.title, "<img src=x onerror=alert(1)> Soup");
        assert_eq!(editor.equipment[0].name, "Large pot");
    }

    #[test]
    fn import_rejects_steps_deeper_than_the_existing_editor_can_represent() {
        let mut c = candidate();
        let mut step = json!({"text":"Deep step","raw":"Deep step","substeps":[]});
        for _ in 0..5 {
            step = json!({"text":"Parent","raw":"Parent","substeps":[step]});
        }
        c["fields"]["instructionSections"][0]["steps"] = json!([step]);
        assert!(map_import_candidate(&c, true).is_err());
    }

    #[test]
    fn import_accepts_the_editors_full_zero_through_three_depth_range() {
        let mut c = candidate();
        let mut step = json!({"text":"Deep step","raw":"Deep step","substeps":[]});
        for _ in 0..3 {
            step = json!({"text":"Parent","raw":"Parent","substeps":[step]});
        }
        c["fields"]["instructionSections"][0]["steps"] = json!([step]);
        assert!(map_import_candidate(&c, true).is_ok());
    }

    #[test]
    fn direct_import_preserves_review_warnings_and_unclassified_recipe_text_in_notes() {
        let mut c = candidate();
        c["warnings"] = json!([
            {"severity":"warning","message":"Check the alternative quantities."},
            {"severity":"info","message":"Active content was discarded."}
        ]);
        c["unclassifiedLines"] = json!([{"raw":"Dough rising: 2 hours"}]);
        let editor = super::map_direct_import_candidate(&c).unwrap();
        let notes = editor.commentary.unwrap();
        assert!(notes.contains("Import review"));
        assert!(notes.contains("Check the alternative quantities."));
        assert!(notes.contains("Dough rising: 2 hours"));
        assert!(!notes.contains("Active content was discarded."));
        assert_eq!(editor.visibility, "private");
        assert!(editor.images.is_empty());
    }

    #[test]
    fn direct_import_refuses_invalid_candidates_and_retains_original_wording() {
        let c = candidate();
        let mapped = super::map_direct_import_candidate(&c).unwrap();
        assert!(mapped.commentary.unwrap().contains("Ingredient:"));
        let mut invalid = c;
        invalid["status"] = json!("unsupported");
        assert!(super::map_direct_import_candidate(&invalid).is_err());
    }

    #[test]
    fn direct_import_retains_provider_and_alternative_region_context_without_inventing_a_link() {
        let mut c = candidate();
        c["source"]["provider"] = json!("RecipeTinEats");
        c["source"]["suppliedUrl"] = Value::Null;
        c["competingCandidates"] =
            json!([{"title":"Other soup","reason":"Earlier editorial region"}]);
        let notes = super::map_direct_import_candidate(&c)
            .unwrap()
            .commentary
            .unwrap();
        assert!(notes.contains("Recipe website: RecipeTinEats"));
        assert!(notes.contains("Other soup — Earlier editorial region"));
        assert!(!notes.contains("https://www.recipetineats.com"));
    }
}
