//! Bounded HTML export of the current authorized saved recipe.
use crate::{
    recipe_html::{render, HtmlExport},
    storage::{ExportImageError, MediaObjectKind, MediaUrl, StorageClient},
    types::{Recipe, RecipeStep},
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::{collections::HashMap, time::Duration};
use uuid::Uuid;
const MAX_HTML_BYTES: usize = 32 * 1024 * 1024;
const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;

fn image_references(recipe: &Recipe) -> Vec<(String, String, bool)> {
    fn steps(items: &[RecipeStep], prefix: &str, refs: &mut Vec<(String, String, bool)>) {
        for (i, step) in items.iter().enumerate() {
            let path = format!("{prefix}{}", i + 1);
            for (j, image) in step.photos.iter().enumerate() {
                refs.push((
                    format!("Step {path}, image {}", j + 1),
                    image.url.clone(),
                    image.asset_id.is_some(),
                ));
            }
            steps(&step.sub_steps, &format!("{path}."), refs);
        }
    }
    let mut refs = recipe
        .images
        .iter()
        .enumerate()
        .map(|(i, image)| {
            (
                format!("Recipe image {}", i + 1),
                image.url.clone(),
                image.asset_id.is_some(),
            )
        })
        .collect::<Vec<_>>();
    steps(&recipe.instructions, "", &mut refs);
    refs
}
async fn embed(
    storage: &StorageClient,
    url: &str,
    managed: bool,
    deadline: tokio::time::Instant,
) -> Result<String, String> {
    let object = match storage.classify_media_url(url) {
        Ok(MediaUrl::Internal(object)) if managed && object.kind() == MediaObjectKind::RecipeFull => object,
        Ok(MediaUrl::ExternalHttps) => return Err("externally linked images cannot be embedded; upload a saved copy in Noms to include it.".into()),
        _ => return Err("this image is not a supported saved recipe image.".into()),
    };
    if tokio::time::Instant::now() >= deadline {
        return Err("the image could not be checked in time. Try again.".into());
    }
    let end = deadline.min(tokio::time::Instant::now() + Duration::from_secs(3));
    let bytes = match tokio::time::timeout_at(
        end,
        storage.read_export_image(object.key(), MAX_IMAGE_BYTES),
    )
    .await
    {
        Ok(Ok(bytes)) => bytes,
        Ok(Err(ExportImageError::TooLarge)) => {
            return Err("this image exceeds the 10 MiB download limit.".into())
        }
        _ => {
            return Err(
                "this image could not be accessed. It may be temporarily unavailable.".into(),
            )
        }
    };
    if !super::upload::validate_webp_magic(&bytes) {
        return Err("this saved file is not a supported WebP image.".into());
    }
    Ok(format!("data:image/webp;base64,{}", STANDARD.encode(bytes)))
}
pub(crate) async fn prepare(
    pool: &sqlx::PgPool,
    storage: &StorageClient,
    id: Uuid,
    viewer: Option<Uuid>,
    include_images: bool,
) -> Result<HtmlExport, String> {
    let (recipe, json) = crate::security::run_with_database_budget(
        crate::security::DATABASE_EXECUTION_BUDGET,
        super::recipe::read_recipe_export(pool, id, viewer),
    )
    .await
    .map_err(|_| "Recipe export timed out. Try again.".to_string())?
    .map_err(|_| {
        "This recipe is unavailable or you do not have access to export it.".to_string()
    })?;
    let file: crate::recipe_json::RecipeFile =
        serde_json::from_str(&json).map_err(|_| "Could not read saved recipe content.")?;
    let tags = file.snapshot["tags"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str().map(str::to_owned))
        .collect::<Vec<_>>();
    let mut embedded = HashMap::new();
    let mut warnings = vec![];
    let base_size = render::document(&recipe, &tags, &embedded, &warnings, include_images).len();
    if include_images {
        let references = image_references(&recipe);
        let mut counts = HashMap::<String, usize>::new();
        for (_, url, _) in &references {
            *counts.entry(url.clone()).or_default() += 1;
        }
        let mut cache = HashMap::<String, Result<(), String>>::new();
        let mut remaining = MAX_HTML_BYTES.saturating_sub(base_size + 64 * 1024);
        let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
        for (label, url, managed) in references {
            if !cache.contains_key(&url) {
                let result = match embed(storage, &url, managed, deadline).await {
                    Ok(uri) => {
                        let cost = uri.len().saturating_mul(counts[&url]);
                        if cost > remaining {
                            Err("including this image would exceed the 32 MiB HTML limit. Try fewer or smaller images.".into())
                        } else {
                            remaining -= cost;
                            embedded.insert(url.clone(), uri);
                            Ok(())
                        }
                    }
                    Err(reason) => Err(reason),
                };
                cache.insert(url.clone(), result);
            }
            if let Err(reason) = &cache[&url] {
                warnings.push(format!("{label}: {reason}"));
            }
        }
    }
    let html = render::document(&recipe, &tags, &embedded, &warnings, include_images);
    if html.len() > MAX_HTML_BYTES {
        return Err("HTML exceeds 32 MiB. Try a text-only HTML export.".into());
    }
    Ok(HtmlExport { html, warnings })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{RecipeEquipment, RecipeImage, RecipeImageEdit, RecipeIngredient};
    use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn html_export_checks_access_embeds_unique_saved_reads_and_preserves_nested_content() {
        let (_db, pool) = crate::test_utils::setup_test_db().await;
        let mock = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(
                ResponseTemplate::new(200).set_body_bytes(b"RIFF0000WEBPsynthetic".as_slice()),
            )
            .mount(&mock)
            .await;
        let storage = StorageClient::for_test_with_endpoint_and_public_url(
            &mock.uri(),
            "https://saved.example/media",
        );
        let (recipe, owner) = super::super::recipe_archive::tests::source(&pool, &storage).await;
        assert!(prepare(&pool, &storage, recipe.id, None, true)
            .await
            .is_err());
        assert!(
            prepare(&pool, &storage, recipe.id, Some(Uuid::new_v4()), true)
                .await
                .is_err()
        );
        assert!(mock.received_requests().await.unwrap().is_empty());
        let text = prepare(&pool, &storage, recipe.id, Some(owner), false)
            .await
            .unwrap();
        assert!(text.warnings.is_empty());
        assert!(!text.html.contains("<img"));
        assert!(text.html.contains("Text-only export"));
        assert!(mock.received_requests().await.unwrap().is_empty());
        for visibility in ["private", "public", "unlisted"] {
            crate::db::update_recipe_visibility(&pool, recipe.id, owner, visibility)
                .await
                .unwrap();
            let copy = prepare(
                &pool,
                &storage,
                recipe.id,
                (visibility == "private").then_some(owner),
                true,
            )
            .await
            .unwrap();
            assert_eq!(copy.warnings.len(), 1);
            assert!(copy.warnings[0].contains("externally linked"));
            assert_eq!(
                copy.html.matches("src=\"data:image/webp;base64,").count(),
                3
            );
            assert!(copy.html.contains("Step 1.1"));
            assert!(
                !copy.html.contains("saved.example") && !copy.html.contains("external.example")
            );
            assert!(copy.html.contains("default-src 'none'") && !copy.html.contains("<script"));
        }
        assert_eq!(mock.received_requests().await.unwrap().len(), 6);
    }

    #[tokio::test]
    async fn unavailable_and_oversized_images_warn_without_blocking_recipe_text() {
        let (_db, pool) = crate::test_utils::setup_test_db().await;
        let mock = MockServer::start().await;
        let storage = StorageClient::for_test_with_endpoint_and_public_url(
            &mock.uri(),
            "https://saved.example/media",
        );
        let (recipe, owner) = super::super::recipe_archive::tests::source(&pool, &storage).await;
        let missing = prepare(&pool, &storage, recipe.id, Some(owner), true)
            .await
            .unwrap();
        assert_eq!(missing.warnings.len(), 4);
        assert!(missing.html.contains("Archive fixture"));
        assert!(!missing.html.contains("<img"));
        let mut body = vec![0; MAX_IMAGE_BYTES];
        body[..12].copy_from_slice(b"RIFF0000WEBP");
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(body))
            .mount(&mock)
            .await;
        let limited = prepare(&pool, &storage, recipe.id, Some(owner), true)
            .await
            .unwrap();
        assert!(limited.html.len() <= MAX_HTML_BYTES);
        assert!(limited.warnings.iter().any(|w| w.contains("32 MiB")));
        assert!(limited.html.contains("Cook") && limited.html.contains("Stir"));
    }

    pub(crate) async fn demo_recipe(
        pool: &sqlx::PgPool,
        storage: &StorageClient,
    ) -> (Recipe, Uuid) {
        let (recipe, owner) = super::super::recipe_archive::tests::source(pool, storage).await;
        let mut editor = crate::recipe_json::parse_recipe(
            &crate::recipe_json::export_recipe(&recipe, vec![]).unwrap(),
        )
        .unwrap()
        .1;
        editor.title = "Roasted tomato & white bean soup".into();
        editor.description = Some("A cozy, weeknight bowl with sweet roasted tomatoes, creamy beans and a little lemon. Best shared with warm, crusty bread.".into());
        editor.commentary = Some("Make it yours\nAdd a pinch of chili flakes for heat, or finish with a spoonful of yogurt.\n\nKeeps in the fridge for up to 3 days. Reheat gently and add a splash of water if needed.".into());
        editor.prep_time_minutes = Some(15);
        editor.cook_time_minutes = Some(35);
        editor.servings = Some(4);
        editor.images.truncate(1);
        editor.ingredients = [
            ("800", "g", "ripe tomatoes, halved"),
            ("2", "tbsp", "olive oil"),
            ("1", "", "yellow onion, roughly chopped"),
            ("3", "", "garlic cloves, peeled"),
            ("2", "cans", "white beans, drained"),
            ("500", "ml", "vegetable stock"),
            ("1", "tsp", "smoked paprika"),
            ("½", "", "lemon, juiced"),
            ("1", "handful", "fresh basil"),
            ("", "", "Sea salt and black pepper"),
        ]
        .into_iter()
        .map(|(amount, unit, name)| RecipeIngredient {
            amount: amount.into(),
            unit: unit.into(),
            name: name.into(),
        })
        .collect();
        editor.instructions = vec![
            RecipeStep { text: "Roast the vegetables".into(), photos: vec![], sub_steps: vec![
                RecipeStep { text: "Heat the oven to 220°C / 425°F. Arrange the tomatoes, onion and garlic on a baking tray.".into(), photos: vec![], sub_steps: vec![] },
                RecipeStep { text: "Drizzle with olive oil, season, and roast for 25 minutes until the edges are golden.".into(), photos: vec![], sub_steps: vec![] },
            ]},
            RecipeStep { text: "Build the soup. Tip the roasted vegetables into a large pot. Add the white beans, stock and smoked paprika. Simmer gently for 10 minutes.".into(), photos: vec![], sub_steps: vec![] },
            RecipeStep { text: "Blend to your liking, leaving a few beans whole for texture. Stir in the lemon juice and taste for seasoning.".into(), photos: vec![], sub_steps: vec![] },
            RecipeStep { text: "Ladle into warm bowls, scatter over the basil and finish with a drizzle of olive oil. Serve with crusty bread.".into(), photos: vec![], sub_steps: vec![] },
        ];
        editor.equipment = vec![
            RecipeEquipment {
                name: "Large baking tray".into(),
            },
            RecipeEquipment {
                name: "Soup pot".into(),
            },
            RecipeEquipment {
                name: "Hand blender".into(),
            },
        ];
        editor.tags = vec![
            "Vegetarian".into(),
            "Weeknight".into(),
            "Comfort food".into(),
        ];
        editor.visibility = "public".into();
        let copy = crate::db::create_published_recipe(
            pool,
            owner,
            &super::super::recipe::editor_to_draft_payload(editor),
            None,
        )
        .await
        .unwrap();
        (copy, owner)
    }

    /// Keeps an isolated temporary database alive for the real app's browser checks.
    #[tokio::test]
    #[ignore = "Manual browser fixture; requires NOMS_HTML_BROWSER_FIXTURE_DIR"]
    async fn html_browser_fixture() {
        let dir = std::path::PathBuf::from(
            std::env::var("NOMS_HTML_BROWSER_FIXTURE_DIR").expect("fixture output directory"),
        );
        let (database, pool) = crate::test_utils::setup_test_db().await;
        let storage = StorageClient::for_test_with_endpoint_and_public_url(
            "http://127.0.0.1:39091",
            "http://127.0.0.1:39091/noms-media",
        );
        let (recipe, owner) = demo_recipe(&pool, &storage).await;
        let mut editor = crate::recipe_json::parse_recipe(
            &crate::recipe_json::export_recipe(&recipe, vec![]).unwrap(),
        )
        .unwrap()
        .1;
        editor.title = "Soup with an unavailable image".into();
        editor.visibility = "public".into();
        editor.images.push(RecipeImage::new(
            "https://external.example/never-fetch.webp".into(),
            None,
            RecipeImageEdit::default_rect_16x9(),
        ));
        let warning = crate::db::create_published_recipe(
            &pool,
            owner,
            &super::super::recipe::editor_to_draft_payload(editor),
            None,
        )
        .await
        .unwrap();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("fixture.json"), serde_json::to_vec(&serde_json::json!({
            "database_url": database.connection_uri(), "recipe_id": recipe.id, "warning_recipe_id": warning.id,
        })).unwrap()).unwrap();
        for _ in 0..1800 {
            if dir.join("stop").exists() {
                break;
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        drop(database);
    }
}
