//! Versioned single-recipe interchange. Database identities are never import authority.
pub const MAX_JSON_BYTES: usize = 2 * 1024 * 1024;

#[cfg(feature = "server")]
pub use server::*;

#[cfg(feature = "server")]
mod server {
    use super::MAX_JSON_BYTES;
    use crate::types::RecipeEditorState;
    use serde::{Deserialize, Serialize};
    use serde_json::{json, Value};
    use sha2::{Digest, Sha256};
    use uuid::Uuid;

    const FORMAT: &str = "noms.recipe.json";
    const SCHEMA: &str = "noms.recipe.snapshot.v3";

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct RecipeFile {
        pub format: String,
        pub version: u32,
        pub snapshot_schema: String,
        pub generator: Generator,
        pub exported_at: chrono::DateTime<chrono::Utc>,
        pub scope: String,
        pub portable_id: String,
        pub source_attribution: SourceAttribution,
        pub image_mode: String,
        pub missing_fields: Vec<String>,
        pub snapshot_digest: String,
        pub snapshot: Value,
    }

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Generator {
        pub name: String,
        pub version: String,
    }

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct SourceAttribution {
        pub author_username: String,
    }

    fn digest(snapshot: &Value) -> Result<String, String> {
        let canonical =
            serde_jcs::to_vec(snapshot).map_err(|_| "Cannot canonicalize recipe JSON.")?;
        Ok(format!("sha256:{:x}", Sha256::digest(canonical)))
    }

    /// Explicit nulls and empty photo arrays are part of the portable contract,
    /// even where the internal types omit these for application transport.
    fn explicit_fields(value: &mut Value) {
        match value {
            Value::Object(object) => {
                if object.contains_key("url") && object.contains_key("edits") {
                    for key in ["asset_id", "thumb_url", "file_name", "file_size"] {
                        object.entry(key).or_insert(Value::Null);
                    }
                }
                if object.contains_key("text") && object.contains_key("sub_steps") {
                    object.entry("photos").or_insert(json!([]));
                }
                for child in object.values_mut() {
                    explicit_fields(child);
                }
            }
            Value::Array(items) => items.iter_mut().for_each(explicit_fields),
            _ => {}
        }
    }

    fn snapshot_value(editor: &RecipeEditorState) -> Result<Value, String> {
        let mut value = serde_json::to_value(editor).map_err(|_| "Cannot serialize recipe.")?;
        value.as_object_mut().unwrap().remove("visibility");
        explicit_fields(&mut value);
        Ok(value)
    }

    pub fn export_recipe(
        recipe: &crate::types::Recipe,
        tags: Vec<String>,
    ) -> Result<String, String> {
        let editor = RecipeEditorState {
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
            visibility: "private".into(),
        };
        crate::security::validate_recipe_editor(
            &editor,
            crate::security::RecipeValidationMode::Draft,
        )
        .map_err(|e| e.to_string())?;
        let mut snapshot = snapshot_value(&editor)?;
        // Tags have set semantics; all other arrays retain authored order.
        let tags = snapshot["tags"].as_array_mut().unwrap();
        tags.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
        tags.dedup();
        let file = RecipeFile {
            format: FORMAT.into(),
            version: 3,
            snapshot_schema: SCHEMA.into(),
            generator: Generator {
                name: "Noms".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
            exported_at: chrono::Utc::now(),
            scope: "recipe".into(),
            portable_id: format!("urn:noms:recipe:{}", recipe.id),
            source_attribution: SourceAttribution {
                author_username: recipe.author_username.clone(),
            },
            image_mode: "references".into(),
            missing_fields: vec![],
            snapshot_digest: digest(&snapshot)?,
            snapshot,
        };
        let json =
            serde_json::to_string_pretty(&file).map_err(|_| "Cannot serialize recipe JSON.")?;
        if json.len() > MAX_JSON_BYTES {
            return Err("Recipe JSON exceeds the 2 MiB limit.".into());
        }
        Ok(json)
    }

    pub fn parse_recipe(json: &str) -> Result<(RecipeFile, RecipeEditorState), String> {
        if json.len() > MAX_JSON_BYTES {
            return Err("Choose a Noms JSON file smaller than 2 MiB.".into());
        }
        let file: RecipeFile =
            serde_json::from_str(json).map_err(|_| "This is not a valid Noms recipe JSON file.")?;
        if file.format != FORMAT
            || !matches!(
                (
                    file.version,
                    file.snapshot_schema.as_str(),
                    file.image_mode.as_str()
                ),
                (3, SCHEMA, "references")
                    | (2, "noms.recipe.snapshot.v2", "none")
                    | (1, "noms.recipe.snapshot.v1", "references")
            )
            || file.scope != "recipe"
            || !file.missing_fields.is_empty()
        {
            return Err(
                "This recipe format, version, scope or image mode is not supported.".into(),
            );
        }
        source_recipe_id(&file)?;
        if file.snapshot_digest != digest(&file.snapshot)? {
            return Err(
                "The recipe checksum does not match. The file may have been changed or damaged."
                    .into(),
            );
        }
        let mut value = file.snapshot.clone();
        let object = value
            .as_object_mut()
            .ok_or("Recipe snapshot must be an object.")?;
        if object.contains_key("visibility") {
            return Err("Portable snapshots cannot set visibility.".into());
        }
        object.insert("visibility".into(), json!("private"));
        let editor: RecipeEditorState =
            serde_json::from_value(value).map_err(|_| "Invalid recipe fields.")?;
        // A round-trip shape check rejects missing, unknown or defaulted fields
        // throughout nested steps and image edits instead of silently losing them.
        if snapshot_value(&editor)? != file.snapshot {
            return Err("The recipe contains missing or unsupported fields.".into());
        }
        crate::security::validate_recipe_editor(
            &editor,
            crate::security::RecipeValidationMode::Draft,
        )
        .map_err(|_| {
            "Recipe fields exceed supported limits or contain invalid values.".to_string()
        })?;
        if editor.prep_time_minutes.is_some_and(|value| value < 0)
            || editor.cook_time_minutes.is_some_and(|value| value < 0)
            || editor.servings.is_some_and(|value| value <= 0)
        {
            return Err("Recipe times and servings contain invalid values.".into());
        }
        if file.version == 2 && has_images(&editor) {
            return Err(
                "This JSON format must not contain images. Add images manually after importing."
                    .into(),
            );
        }
        Ok((file, editor))
    }

    fn has_images(editor: &RecipeEditorState) -> bool {
        let mut steps: Vec<_> = editor.instructions.iter().collect();
        if !editor.images.is_empty() {
            return true;
        }
        while let Some(step) = steps.pop() {
            if !step.photos.is_empty() {
                return true;
            }
            steps.extend(step.sub_steps.iter());
        }
        false
    }

    pub fn source_recipe_id(file: &RecipeFile) -> Result<Uuid, String> {
        file.portable_id
            .strip_prefix("urn:noms:recipe:")
            .and_then(|id| Uuid::parse_str(id).ok())
            .ok_or_else(|| "Invalid portable recipe identity.".into())
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::types::{
            Recipe, RecipeEquipment, RecipeImage, RecipeImageEdit, RecipeIngredient, RecipeStep,
        };

        fn recipe() -> Recipe {
            Recipe {
                id: Uuid::new_v4(),
                user_id: Uuid::new_v4(),
                title: "祖母’s soup 🍲".into(),
                description: None,
                commentary: Some(String::new()),
                prep_time_minutes: Some(0),
                cook_time_minutes: Some(45),
                servings: Some(2),
                ingredients: vec![RecipeIngredient {
                    amount: "1 ½".into(),
                    unit: "cups".into(),
                    name: "beans".into(),
                }],
                instructions: vec![RecipeStep {
                    text: "Cook".into(),
                    photos: vec![],
                    sub_steps: vec![RecipeStep {
                        text: "Stir".into(),
                        sub_steps: vec![],
                        photos: vec![RecipeImage::new(
                            "https://images.example.test/stir.webp".into(),
                            None,
                            RecipeImageEdit::default_rect_16x9(),
                        )],
                    }],
                }],
                equipment: vec![RecipeEquipment { name: "Pot".into() }],
                images: vec![],
                visibility: "public".into(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                author_username: "cook".into(),
                author_avatar_url: None,
            }
        }

        fn file() -> RecipeFile {
            serde_json::from_str(
                &export_recipe(&recipe(), vec!["soup".into(), "beans".into()]).unwrap(),
            )
            .unwrap()
        }

        fn encode(file: &RecipeFile) -> String {
            serde_json::to_string(file).unwrap()
        }

        fn resign(file: &mut RecipeFile) {
            file.snapshot_digest = digest(&file.snapshot).unwrap();
        }

        #[test]
        fn json_round_trip_preserves_text_images_nested_photos_and_edits() {
            let original = recipe();
            let output = export_recipe(&original, vec!["soup".into(), "beans".into()]).unwrap();
            let (file, editor) = parse_recipe(&output).unwrap();
            assert_eq!(editor.title, original.title);
            assert_eq!(editor.description, None);
            assert_eq!(editor.commentary, Some(String::new()));
            assert_eq!(editor.prep_time_minutes, Some(0));
            assert_eq!(editor.cook_time_minutes, Some(45));
            assert_eq!(editor.servings, Some(2));
            assert_eq!(editor.ingredients, original.ingredients);
            assert_eq!(editor.instructions, original.instructions);
            assert_eq!(editor.equipment, original.equipment);
            assert_eq!(editor.images, original.images);
            assert_eq!(editor.tags, vec!["beans", "soup"]);
            assert_eq!(editor.visibility, "private");
            assert_eq!(source_recipe_id(&file).unwrap(), original.id);
            assert!(file.snapshot.get("description").unwrap().is_null());
            assert_eq!(file.snapshot["instructions"][0]["photos"], json!([]));
            assert!(output.contains("images.example.test"));
            assert!(
                file.snapshot["instructions"][0]["sub_steps"][0]["photos"][0]["thumb_url"]
                    .is_null()
            );
            assert_eq!(file.image_mode, "references");
            for forbidden in ["user_id", "visibility", "created_at", "drafts", "versions"] {
                assert!(file.snapshot.get(forbidden).is_none());
            }
        }

        #[test]
        fn json_digest_is_independent_of_envelope_whitespace_keys_and_tag_order() {
            let original = recipe();
            let a: RecipeFile = serde_json::from_str(
                &export_recipe(&original, vec!["z".into(), "a".into(), "z".into()]).unwrap(),
            )
            .unwrap();
            let mut b: RecipeFile = serde_json::from_str(
                &export_recipe(&original, vec!["a".into(), "z".into()]).unwrap(),
            )
            .unwrap();
            assert_eq!(a.snapshot_digest, b.snapshot_digest);
            b.generator.version = "another version".into();
            assert!(parse_recipe(&encode(&b)).is_ok());
            assert_eq!(
                digest(&json!({"b":1,"a":1.0})).unwrap(),
                digest(&json!({"a":1,"b":1})).unwrap()
            );
        }

        #[test]
        fn legacy_references_are_preserved_and_image_free_v2_remains_supported() {
            let mut file = file();
            for version in [1, 3] {
                file.version = version;
                file.snapshot_schema = format!("noms.recipe.snapshot.v{version}");
                assert!(
                    !parse_recipe(&encode(&file)).unwrap().1.instructions[0].sub_steps[0]
                        .photos
                        .is_empty()
                );
            }
            file.version = 2;
            file.snapshot_schema = "noms.recipe.snapshot.v2".into();
            file.image_mode = "none".into();
            assert!(parse_recipe(&encode(&file)).is_err());
            file.snapshot["instructions"][0]["sub_steps"][0]["photos"] = json!([]);
            resign(&mut file);
            assert!(parse_recipe(&encode(&file)).is_ok());
        }

        #[test]
        fn json_preserves_explicit_empty_recipe_lists_in_an_editable_draft() {
            let mut file = file();
            for field in ["ingredients", "instructions", "equipment", "images", "tags"] {
                file.snapshot[field] = json!([]);
            }
            resign(&mut file);
            let (_, editor) = parse_recipe(&encode(&file)).unwrap();
            assert!(editor.ingredients.is_empty());
            assert!(editor.instructions.is_empty());
            assert_eq!(editor.visibility, "private");
        }

        #[test]
        fn json_rejects_changed_content_and_unknown_versions_before_import() {
            let mut file = file();
            file.snapshot["title"] = json!("Changed");
            assert!(parse_recipe(&encode(&file))
                .unwrap_err()
                .contains("checksum"));
            resign(&mut file);
            file.version = 99;
            assert!(parse_recipe(&encode(&file)).is_err());
            assert!(parse_recipe("not json").is_err());
            assert!(parse_recipe(&" ".repeat(MAX_JSON_BYTES + 1)).is_err());
        }

        #[test]
        fn json_rejects_missing_and_unknown_fields_instead_of_silently_defaulting() {
            for mutation in [
                "missing_null",
                "extra",
                "missing_photos",
                "nested_extra",
                "public",
            ] {
                let mut file = file();
                match mutation {
                    "missing_null" => {
                        file.snapshot.as_object_mut().unwrap().remove("description");
                    }
                    "extra" => {
                        file.snapshot["unsupported_metadata"] = json!("do not lose me");
                    }
                    "missing_photos" => {
                        file.snapshot["instructions"][0]
                            .as_object_mut()
                            .unwrap()
                            .remove("photos");
                    }
                    "nested_extra" => {
                        file.snapshot["ingredients"][0]["unknown"] = json!(true);
                    }
                    "public" => {
                        file.snapshot["visibility"] = json!("public");
                    }
                    _ => unreachable!(),
                }
                resign(&mut file);
                assert!(parse_recipe(&encode(&file)).is_err(), "{mutation}");
            }
        }

        #[test]
        fn json_rejects_invalid_scalar_bounds_and_deep_structures() {
            for (key, value) in [
                ("servings", json!(0)),
                ("cook_time_minutes", json!(-1)),
                ("title", json!("x".repeat(1000))),
            ] {
                let mut file = file();
                file.snapshot[key] = value;
                resign(&mut file);
                assert!(parse_recipe(&encode(&file)).is_err(), "{key}");
            }
            let mut file = file();
            let mut step = json!({"text":"Cook","photos":[],"sub_steps":[]});
            for _ in 0..10 {
                step = json!({"text":"Cook","photos":[],"sub_steps":[step]});
            }
            file.snapshot["instructions"] = json!([step]);
            resign(&mut file);
            assert!(parse_recipe(&encode(&file)).is_err());
        }
    }
}
