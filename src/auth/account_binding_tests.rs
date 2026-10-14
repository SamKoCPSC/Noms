use super::*;

#[test]
fn account_binding_accepts_only_the_verified_initiating_account() {
    let original = Uuid::new_v4();
    let replacement = Uuid::new_v4();
    assert_eq!(
        check_account_binding(Some(original), Some(original)).unwrap(),
        original
    );
    for (expected, verified) in [
        (Some(original), Some(replacement)),
        (None, Some(replacement)),
    ] {
        assert!(check_account_binding(expected, verified)
            .unwrap_err()
            .to_string()
            .contains("Account changed"));
    }
    assert!(check_account_binding(Some(original), None)
        .unwrap_err()
        .to_string()
        .contains("Not authenticated"));
}

#[test]
fn form_binding_latches_once_and_never_rebinds_local_entries() {
    let original = Uuid::new_v4();
    let replacement = Uuid::new_v4();
    let mut binding = None;
    latch_form_account(&mut binding, None);
    assert_eq!(binding, None);
    latch_form_account(&mut binding, Some(original));
    latch_form_account(&mut binding, None);
    latch_form_account(&mut binding, Some(replacement));
    assert_eq!(binding, Some(original));
}

#[test]
fn stale_profile_completion_cannot_replace_a_different_auth_context() {
    let original = Uuid::new_v4();
    let replacement = Uuid::new_v4();
    assert!(account_completion_matches(
        Some(original),
        Some(original),
        original
    ));
    for (current, returned) in [
        (Some(replacement), original),
        (None, original),
        (Some(original), replacement),
    ] {
        assert!(!account_completion_matches(
            Some(original),
            current,
            returned
        ));
    }
    assert!(!account_completion_matches(None, Some(original), original));
}

#[test]
fn every_server_mutation_checks_binding_before_side_effects() {
    // Wiring regression: pure comparison tests alone would not protect an
    // endpoint that accidentally reverted to cookie-only account selection.
    for (source, names) in [
        (
            include_str!("../api/recipe.rs"),
            vec![
                "create_recipe",
                "update_recipe",
                "update_recipe_visibility",
                "create_recipe_draft",
                "create_edit_draft",
                "save_recipe_draft",
                "discard_recipe_draft",
                "publish_recipe_draft",
                "restore_version",
                "fork_recipe",
                "delete_recipe",
            ],
        ),
        (
            include_str!("../api/collection.rs"),
            vec![
                "create_collection",
                "move_collection",
                "reorder_child_collections",
                "update_collection",
                "update_collection_visibility",
                "delete_collection",
                "add_recipe_to_collection",
                "remove_recipe_from_collection",
                "reorder_collection_recipes",
            ],
        ),
        (
            include_str!("../api/upload.rs"),
            vec!["upload_image", "discard_upload"],
        ),
        (
            include_str!("../pages/settings/settings_profile.rs"),
            vec![
                "save_profile",
                "upload_avatar",
                "remove_avatar",
                "delete_account",
            ],
        ),
        (
            include_str!("../pages/settings/settings_accounts.rs"),
            vec!["unlink_account"],
        ),
    ] {
        for name in names {
            let marker = format!("pub async fn {name}(");
            let body = source
                .split(&marker)
                .nth(1)
                .unwrap()
                .split("#[server]")
                .next()
                .unwrap();
            let first_statement = body
                .split("let user_id =")
                .nth(1)
                .unwrap()
                .split(';')
                .next()
                .unwrap();
            assert!(
                body.split(") ->")
                    .next()
                    .unwrap()
                    .contains("expected_account_id"),
                "{name}"
            );
            assert!(
                first_statement.contains("require_form_account(expected_account_id).await?"),
                "{name}"
            );
        }
    }
}

#[tokio::test]
async fn requests_without_verified_session_stop_before_profile_or_storage_access() {
    let expected = Some(Uuid::new_v4());
    let error = crate::api::upload::upload_image(
        expected,
        vec![],
        vec![],
        "test.webp".into(),
        crate::api::upload::UploadPurpose::Recipe,
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("Not authenticated"));
    let error = crate::api::recipe::create_recipe_draft(
        expected,
        crate::types::RecipeEditorState::default(),
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("Not authenticated"));
}
