use super::*;
use serde_json::{json, Value};
use uuid::Uuid;

fn snapshot() -> CollectionSnapshot {
    let recipe = crate::types::Recipe {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        title: "Soup & bread 🥣".into(),
        description: None,
        commentary: Some(String::new()),
        prep_time_minutes: Some(0),
        cook_time_minutes: Some(25),
        servings: Some(2),
        ingredients: vec![crate::types::RecipeIngredient {
            vocabulary_id: None,
            amount: "½".into(),
            unit: "cup".into(),
            name: "beans".into(),
            ..Default::default()
        }],
        instructions: vec![crate::types::RecipeStep {
            text: "Cook".into(),
            photos: vec![],
            sub_steps: vec![crate::types::RecipeStep {
                text: "Stir".into(),
                photos: vec![],
                sub_steps: vec![],
            }],
        }],
        equipment: vec![],
        images: vec![],
        visibility: "public".into(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        author_username: "cook".into(),
        author_avatar_url: None,
    };
    let file: crate::recipe_json::RecipeFile = serde_json::from_str(
        &crate::recipe_json::export_recipe(&recipe, vec!["Soup".into(), "Quick".into()]).unwrap(),
    )
    .unwrap();
    let root = portable_collection_id(Uuid::new_v4());
    let child = portable_collection_id(Uuid::new_v4());
    CollectionSnapshot {
        root_collection: root.clone(),
        collections: vec![
            PortableCollection {
                portable_id: root.clone(),
                parent_collection: None,
                name: "Family".into(),
                description: Some("Keep this order".into()),
                image: None,
                recipes: vec![file.portable_id.clone()],
            },
            PortableCollection {
                portable_id: child,
                parent_collection: Some(root),
                name: "Soup".into(),
                description: None,
                image: None,
                recipes: vec![file.portable_id.clone()],
            },
        ],
        recipes: vec![file],
    }
}

fn encode(value: Value) -> String {
    let mut file: CollectionFile = serde_json::from_str(&export(snapshot()).unwrap()).unwrap();
    file.snapshot = value;
    file.snapshot_digest = crate::recipe_json::digest(&file.snapshot).unwrap();
    serde_json::to_string(&file).unwrap()
}

#[test]
fn collection_json_roundtrip_preserves_tree_shared_membership_and_canonical_recipe() {
    let original = snapshot();
    let expected = serde_json::to_value(&original).unwrap();
    let text = export(original).unwrap();
    let parsed = parse(&text).unwrap();
    assert_eq!(serde_json::to_value(&parsed.snapshot).unwrap(), expected);
    assert_eq!(parsed.recipes.len(), 1);
    assert_eq!(parsed.recipes[0].1.visibility, "private");
    assert_eq!(parsed.recipes[0].1.ingredients[0].amount, "½");
    assert_eq!(
        parsed.recipes[0].1.instructions[0].sub_steps[0].text,
        "Stir"
    );
    assert_eq!(
        parsed.snapshot.collections[0].recipes,
        parsed.snapshot.collections[1].recipes
    );
}

#[test]
fn collection_json_checksums_cover_metadata_membership_and_nested_recipe_content() {
    let text = export(snapshot()).unwrap();
    let mut file: CollectionFile = serde_json::from_str(&text).unwrap();
    file.snapshot["collections"][0]["name"] = json!("Changed");
    assert!(parse(&serde_json::to_string(&file).unwrap())
        .unwrap_err()
        .contains("checksum"));
    let mut value = serde_json::to_value(snapshot()).unwrap();
    value["recipes"][0]["snapshot"]["title"] = json!("Changed recipe");
    assert!(parse(&encode(value)).unwrap_err().contains("checksum"));
    let mut file: CollectionFile = serde_json::from_str(&text).unwrap();
    file.version = 99;
    assert!(parse(&serde_json::to_string(&file).unwrap()).is_err());
}

#[test]
fn collection_json_rejects_disconnected_duplicate_missing_and_unknown_fields() {
    let base = serde_json::to_value(snapshot()).unwrap();
    for kind in 0..14 {
        let mut value = base.clone();
        match kind {
            0 => value["collections"][1]["parent_collection"] = Value::Null,
            1 => {
                value["collections"][1]["parent_collection"] =
                    json!(portable_collection_id(Uuid::new_v4()))
            }
            2 => {
                value["collections"][1]["portable_id"] =
                    value["collections"][0]["portable_id"].clone()
            }
            3 => {
                let duplicate = value["collections"][0]["recipes"][0].clone();
                value["collections"][0]["recipes"]
                    .as_array_mut()
                    .unwrap()
                    .push(duplicate);
            }
            4 => {
                value["collections"][0]["recipes"] =
                    json!([format!("urn:noms:recipe:{}", Uuid::new_v4())])
            }
            5 => {
                value["collections"][0]["recipes"] = json!([]);
                value["collections"][1]["recipes"] = json!([]);
            }
            6 => {
                let duplicate = value["recipes"][0].clone();
                value["recipes"].as_array_mut().unwrap().push(duplicate);
            }
            7 => {
                value["collections"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("description");
            }
            8 => {
                value["collections"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("image");
            }
            9 => value["collections"][0]["visibility"] = json!("public"),
            10 => value["collections"][0]["name"] = json!(" "),
            11 => value["root_collection"] = value["collections"][1]["portable_id"].clone(),
            12 => {
                value["recipes"][0]["snapshot"]["ingredients"] = json!([]);
                value["recipes"][0]["snapshot_digest"] =
                    json!(crate::recipe_json::digest(&value["recipes"][0]["snapshot"]).unwrap());
            }
            13 => {
                let id = "urn:noms:recipe:AAAAAAAA-AAAA-4AAA-8AAA-AAAAAAAAAAAA";
                value["recipes"][0]["portable_id"] = json!(id);
                value["collections"][0]["recipes"] = json!([id]);
                value["collections"][1]["recipes"] = json!([id]);
            }
            _ => unreachable!(),
        }
        assert!(
            parse(&encode(value)).is_err(),
            "Invalid shape {kind} was accepted"
        );
    }
}

#[test]
fn collection_json_empty_collections_are_valid_and_limits_are_enforced() {
    let mut empty = snapshot();
    empty.collections.truncate(1);
    empty.collections[0].recipes.clear();
    empty.recipes.clear();
    assert!(parse(&export(empty).unwrap()).is_ok());
    assert!(parse(&" ".repeat(MAX_JSON_BYTES + 1)).is_err());
    let mut value = serde_json::to_value(snapshot()).unwrap();
    value["collections"][0]["name"] =
        json!("x".repeat(crate::security::MAX_COLLECTION_NAME_CHARS + 1));
    assert!(parse(&encode(value)).is_err());
    let mut tree = snapshot();
    tree.collections.truncate(1);
    for index in 1..=crate::security::MAX_COLLECTION_DEPTH {
        let parent = tree.collections.last().unwrap().portable_id.clone();
        tree.collections.push(PortableCollection {
            portable_id: portable_collection_id(Uuid::new_v4()),
            parent_collection: Some(parent),
            name: format!("Depth {index}"),
            description: None,
            image: None,
            recipes: vec![],
        });
    }
    assert!(export(tree).is_err());
}
