use crate::components::base::EditedRecipeImage;
use crate::types::{RecipeImage, RecipeImageEdit, RecipeStep};
use dioxus::prelude::*;

#[component]
pub(super) fn PublicationReview(
    title: String,
    ingredient_count: usize,
    step_count: usize,
    visibility: String,
    preserve_visibility: bool,
    oncancel: EventHandler<()>,
    onconfirm: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        crate::components::base::SharedModal {
            aria_labelledby: "publication-review-title", focus_selector: "#publication-review-back", on_dismiss: oncancel,
            div { class: "modal-card mobile-action-sheet", onclick: move |event| event.stop_propagation(),
                div { class: "mobile-sheet-heading", h2 { id: "publication-review-title", "Review your recipe" } button { id: "publication-review-back", r#type: "button", class: "btn btn-ghost touch-target", aria_label: "Back to editing", onclick: move |_| oncancel.call(()), "✕" } }
                p { class: "recipe-editor__hint", "Your working draft is private. Publishing creates a snapshot in your recipe history." }
                h3 { class: "mobile-review-recipe-title", if title.trim().is_empty() { "Untitled recipe" } else { "{title}" } }
                p { "{ingredient_count} " if ingredient_count == 1 { "ingredient" } else { "ingredients" } " · {step_count} " if step_count == 1 { "step" } else { "steps" } }
                div { class: "mobile-review-visibility neumo-inset",
                    if preserve_visibility { strong { "Sharing stays the same" } p { "Publishing keeps this recipe’s current visibility. Manage sharing from the published recipe." } }
                    else { strong { "Who can see the published recipe?" } p {
                        match visibility.as_str() {
                            "public" => "Public — appears in Explore and search.",
                            "unlisted" => "Unlisted — anyone with the link can view.",
                            _ => "Private — only you can see this recipe.",
                        }
                    } }
                }
                p { class: "recipe-editor__hint", "To change recipe details, return to editing. Nothing is published until you confirm." }
                div { class: "mobile-review-actions",
                    button { r#type: "button", class: "btn btn-secondary touch-target", onclick: move |_| oncancel.call(()), "Keep editing" }
                    button { r#type: "button", class: "btn btn-primary touch-target", onclick: move |event| onconfirm.call(event), "Publish recipe" }
                }
            }
        }
    }
}

/// Editor fields need stable accessible names, including repeated ingredient
/// rows whose visible labels would otherwise be only placeholders.
#[component]
pub(super) fn RecipeEditorInput(
    accessible_name: String,
    value: String,
    placeholder: String,
    oninput: EventHandler<String>,
    #[props(default = "text".to_string())] input_type: String,
) -> Element {
    rsx! {
        input {
            class: "neumo-inset input input--shared",
            aria_label: accessible_name,
            r#type: input_type,
            value,
            placeholder,
            oninput: move |event| oninput.call(event.value()),
        }
    }
}
/// Time and serving fields share one aligned row and the parent editor's signals.
#[component]
pub(super) fn RecipeTimingFields(
    mut prep_time: Signal<String>,
    mut cook_time: Signal<String>,
    mut servings: Signal<String>,
) -> Element {
    rsx! {
        div { class: "recipe-editor__timing",
            label {
                span { "Prep time (min)" }
                RecipeEditorInput {
                    accessible_name: "Prep time in minutes", value: prep_time(),
                    placeholder: "0", input_type: "number",
                    oninput: move |value| prep_time.set(value),
                }
            }
            label {
                span { "Cook time (min)" }
                RecipeEditorInput {
                    accessible_name: "Cook time in minutes", value: cook_time(),
                    placeholder: "0", input_type: "number",
                    oninput: move |value| cook_time.set(value),
                }
            }
            label {
                span { "Servings" }
                RecipeEditorInput {
                    accessible_name: "Servings", value: servings(),
                    placeholder: "4", input_type: "number",
                    oninput: move |value| servings.set(value),
                }
            }
        }
    }
}

/// A contents list, not a completion checklist: empty optional sections never
/// prevent saving or publishing. Counts include only content the user entered.
#[component]
pub(super) fn RecipeEditorContents(
    mut active_section: Signal<&'static str>,
    title_present: bool,
    photo_count: usize,
    ingredient_count: usize,
    step_count: usize,
    equipment_count: usize,
    notes_present: bool,
) -> Element {
    let counted = |count: usize, singular: &str, plural: &str| {
        format!("{count} {}", if count == 1 { singular } else { plural })
    };
    let sections = [
        (
            "basics",
            "Basics",
            if title_present {
                "Title added"
            } else {
                "Add a title"
            }
            .to_string(),
        ),
        (
            "photos",
            "Photos",
            if photo_count == 0 {
                "Optional".to_string()
            } else {
                format!("Optional · {}", counted(photo_count, "photo", "photos"))
            },
        ),
        (
            "ingredients",
            "Ingredients",
            if ingredient_count == 0 {
                "No ingredients yet".to_string()
            } else {
                format!(
                    "{} added",
                    counted(ingredient_count, "ingredient", "ingredients"),
                )
            },
        ),
        (
            "method",
            "Method",
            if step_count == 0 {
                "No steps yet".to_string()
            } else {
                format!("{} added", counted(step_count, "step", "steps"))
            },
        ),
        (
            "equipment",
            "Equipment",
            if equipment_count == 0 {
                "Optional".to_string()
            } else {
                format!("Optional · {}", counted(equipment_count, "item", "items"))
            },
        ),
        (
            "notes",
            "Notes",
            if notes_present {
                "Optional · Added"
            } else {
                "Optional"
            }
            .to_string(),
        ),
    ];
    rsx! {
        document::Stylesheet { href: asset!("/assets/recipe-editor.css") }
        nav {
            class: "recipe-editor__contents neumo-raised",
            aria_label: "Recipe editor sections",
            h2 { "Recipe editor" }
            ol {
                for (id, label, status) in sections {
                    li { key: "{id}",
                        a {
                            href: "#recipe-{id}",
                            class: if active_section() == id { "recipe-editor__section-link--active" } else { "" },
                            aria_current: if active_section() == id { Some("location") } else { None },
                            onclick: move | _ |
                                    active_section.set(id),
                            span {
                                class: "recipe-editor__timeline-node",
                                aria_hidden: "true",
                            }
                            span {
                                strong { "{label}" }
                                small { "{status}" }
                            }
                        }
                    }
                }
            }
            p { "Jump to any section" }
        }
    }
}
/// A recipe step row in the editor's flat representation.
///
/// `depth` controls nesting (zero is a root step), while photos retain the
/// complete persisted image record so both editor routes display and save the
/// same metadata.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct StepForm {
    pub(super) text: String,
    pub(super) depth: usize,
    pub(super) photos: Vec<RecipeImage>,
}

/// Reorder whole image records, without transferring photos between steps.
/// Missing/out-of-range positions are harmless; both routes share this boundary.
pub(super) fn reorder_step_photo_within_step(
    steps: &mut [StepForm],
    source: (usize, usize),
    target: (usize, usize),
) -> bool {
    if source.0 != target.0 {
        return false;
    }
    let Some(step) = steps.get_mut(source.0) else {
        return false;
    };
    crate::utils::reorder::move_item(&mut step.photos, source.1, target.1)
}

const MAX_STEP_DEPTH: usize = 3;

pub(super) fn can_indent_step(depth: usize) -> bool {
    depth < MAX_STEP_DEPTH
}

/// Flatten a tree of recipe steps into the editor's preorder representation.
pub(super) fn flatten_steps(steps: &[RecipeStep]) -> Vec<StepForm> {
    fn append_steps(steps: &[RecipeStep], depth: usize, flat: &mut Vec<StepForm>) {
        for step in steps {
            flat.push(StepForm {
                text: step.text.clone(),
                depth,
                photos: step.photos.clone(),
            });
            append_steps(&step.sub_steps, depth + 1, flat);
        }
    }

    let mut flat = Vec::new();
    append_steps(steps, 0, &mut flat);
    flat
}

/// Build a recipe-step tree from the editor's flat preorder representation.
pub(super) fn build_step_tree(flat: &[StepForm]) -> Vec<RecipeStep> {
    struct StepNode {
        step: RecipeStep,
        child_indices: Vec<usize>,
    }

    fn materialize_step(nodes: &[StepNode], node_index: usize) -> RecipeStep {
        let node = &nodes[node_index];
        let mut step = node.step.clone();
        step.sub_steps = node
            .child_indices
            .iter()
            .map(|child_index| materialize_step(nodes, *child_index))
            .collect();
        step
    }

    let mut nodes = Vec::<StepNode>::with_capacity(flat.len());
    let mut root_indices = Vec::<usize>::new();
    let mut ancestors = Vec::<(usize, usize)>::new();

    for form in flat {
        while ancestors
            .last()
            .is_some_and(|(ancestor_depth, _)| *ancestor_depth >= form.depth)
        {
            ancestors.pop();
        }

        let node_index = nodes.len();
        nodes.push(StepNode {
            step: RecipeStep {
                text: form.text.clone(),
                sub_steps: Vec::new(),
                photos: form.photos.clone(),
            },
            child_indices: Vec::new(),
        });

        if let Some((_, parent_index)) = ancestors.last() {
            nodes[*parent_index].child_indices.push(node_index);
        } else {
            root_indices.push(node_index);
        }

        ancestors.push((form.depth, node_index));
    }

    root_indices
        .into_iter()
        .map(|root_index| materialize_step(&nodes, root_index))
        .collect()
}

pub(super) fn step_photo_display_name(photo: &RecipeImage) -> String {
    photo.file_name.clone().unwrap_or_else(|| {
        photo
            .url
            .split('?')
            .next()
            .and_then(|url| url.rsplit('/').next())
            .filter(|name| !name.is_empty())
            .unwrap_or("Step photo")
            .to_string()
    })
}

/// Method photos use the same named rectangular view as the recipe gallery.
pub(super) fn step_photo_editor_edit(photo: &RecipeImage) -> RecipeImageEdit {
    photo
        .edits
        .iter()
        .find(|edit| edit.name == "rectangle:16:9")
        .cloned()
        .unwrap_or_else(RecipeImageEdit::default_rect_16x9)
}

/// An existing step photo changes metadata only, not its asset or other views.
pub(super) fn apply_step_photo_edit(photo: &mut RecipeImage, edit: RecipeImageEdit) {
    if let Some(previous) = photo
        .edits
        .iter_mut()
        .find(|previous| previous.name == edit.name)
    {
        *previous = edit;
    } else {
        photo.edits.push(edit);
    }
}

#[component]
pub(super) fn StepPhotoEditButton(
    image: RecipeImage,
    disabled: bool,
    onedit: EventHandler<RecipeImage>,
) -> Element {
    let name = step_photo_display_name(&image);
    let selected_image = image.clone();
    let unavailable = disabled || image.url.is_empty();
    rsx! {
        button {
            r#type: "button",
            class: "image-upload__preview-button touch-target",
            aria_label: "Edit step photo: {name}",
            disabled: unavailable,
            onclick: move |_| onedit.call(selected_image.clone()),
            EditedRecipeImage {
                image,
                class: "image-upload__thumb step-photo-preview",
                alt: "Step photo".to_string(),
                prefer_thumbnail: true,
                on_error: move |_| {},
            }
        }
    }
}

pub(super) fn format_step_photo_file_size(file_size: Option<u64>) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = KIB * 1024;

    match file_size {
        Some(bytes) if bytes >= MIB => format!("{:.1} MB", bytes as f64 / MIB as f64),
        Some(bytes) if bytes >= KIB => format!("{} KB", bytes / KIB),
        Some(bytes) => format!("{bytes} B"),
        None => "Size unavailable".to_string(),
    }
}

/// A signal owns each local preview until modal close/failure or component
/// teardown. Upload tasks keep only the URL string, never another owning copy.
pub(super) struct StepPhotoPreview(pub String);

impl Drop for StepPhotoPreview {
    fn drop(&mut self) {
        // Keep the native SSR build free of browser side effects.
        let _ = &self.0;
        #[cfg(target_arch = "wasm32")]
        let _ = web_sys::Url::revoke_object_url(&self.0);
    }
}

#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn step_photo_processing_failure(filename: &str) -> String {
    let filename: String = filename.chars().take(120).collect();
    format!("Could not process \"{filename}\". Choose a valid JPEG, PNG, or WebP image.")
}

#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn step_photo_upload_error_message(filename: &str, error: &ServerFnError) -> String {
    let filename: String = filename.chars().take(120).collect();
    let feedback = crate::components::base::image_upload_error_message(error, false);
    format!("\"{filename}\": {feedback} Choose the photo again to retry; adjustments must be reapplied. Other photos are unchanged.")
}

#[component]
pub(super) fn StepPhotoUploadFeedback(uploading: bool, errors: Vec<String>) -> Element {
    rsx! {
        p { role: "status", aria_live: "polite",
            if uploading { "Uploading step photos… Keep this editor open until uploading finishes." }
            else { "Photos are added in selection order. Open a photo to adjust it." }
        }
        if !errors.is_empty() {
            div { role: "alert", overflow_wrap: "anywhere", color: "var(--text-primary)",
                for message in errors { p { "{message}" } }
            }
        }
    }
}

/// Clear the native picker even after invalid selections so the same file can
/// be selected again. `accept` remains only a hint; uploads validate originals.
pub(super) fn reset_step_photo_input(_step_index: usize) {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        if let Some(document) = web_sys::window().and_then(|window| window.document()) {
            if let Some(input) = document.get_element_by_id(&format!("step-photo-{_step_index}")) {
                if let Some(input) = input.dyn_ref::<web_sys::HtmlInputElement>() {
                    input.set_value("");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn both_editors_use_the_shared_step_photo_drop_boundary_and_live_lock() {
        for source in [
            include_str!("recipe_new.rs"),
            include_str!("recipe_edit.rs"),
        ] {
            let drop = source
                .split("ondrop: move |event| {")
                .nth(1)
                .unwrap()
                .split("StepPhotoEditButton {")
                .next()
                .unwrap();
            assert!(drop.contains("if step_photo_controls_locked() { return; }"));
            assert!(drop.contains("reorder_step_photo_within_step("));
            assert!(drop.contains("(source_step_idx, source_photo_idx)"));
            assert!(drop.contains("(idx, photo_idx)"));
            assert!(drop.contains("dragged_step_photo.set(None)"));
            assert!(!drop.contains("photos.remove("));
        }
    }

    #[test]
    fn step_photo_drag_preserves_metadata_and_other_steps() {
        let mut edited = photo("second.webp", "second.png", 2048);
        edited.asset_id = Some(uuid::Uuid::from_u128(43));
        edited
            .edits
            .push(crate::types::RecipeImageEdit::default_ellipse_1x1());
        let mut steps = vec![
            super::StepForm {
                text: "First step".into(),
                depth: 0,
                photos: vec![photo("first.webp", "first.png", 1024), edited.clone()],
            },
            super::StepForm {
                text: "Second step".into(),
                depth: 1,
                photos: vec![photo("third.webp", "third.png", 3072)],
            },
        ];
        let original = steps.clone();
        assert!(super::reorder_step_photo_within_step(
            &mut steps,
            (0, 1),
            (0, 0)
        ));
        assert_eq!(steps[0].photos, vec![edited, original[0].photos[0].clone()]);
        assert_eq!(steps[0].text, original[0].text);
        assert_eq!(steps[0].depth, original[0].depth);
        assert_eq!(steps[1], original[1]);
        let persisted = super::build_step_tree(&steps);
        assert_eq!(persisted[0].photos, steps[0].photos);
        assert_eq!(persisted[0].sub_steps[0].photos, original[1].photos);
    }

    #[test]
    fn cross_step_stale_and_self_photo_drops_do_not_lose_or_transfer_images() {
        let mut steps = vec![
            super::StepForm {
                text: "First step".into(),
                depth: 0,
                photos: vec![
                    photo("first.webp", "first.png", 1024),
                    photo("second.webp", "second.png", 2048),
                ],
            },
            super::StepForm {
                text: "Second step".into(),
                depth: 0,
                photos: vec![photo("third.webp", "third.png", 3072)],
            },
        ];
        let original = steps.clone();
        for (source, target) in [
            ((0, 0), (1, 0)),
            ((1, 0), (0, 1)),
            ((9, 0), (9, 0)),
            ((0, 9), (0, 0)),
            ((0, 0), (0, 9)),
        ] {
            assert!(!super::reorder_step_photo_within_step(
                &mut steps, source, target
            ));
            assert_eq!(steps, original);
        }
        assert!(super::reorder_step_photo_within_step(
            &mut steps,
            (0, 0),
            (0, 0)
        ));
        assert_eq!(steps, original);
        assert!(!super::reorder_step_photo_within_step(
            &mut [],
            (0, 0),
            (0, 0)
        ));
    }

    #[test]
    fn repeated_editor_field_has_an_accessible_name_and_keeps_its_value() {
        use dioxus::prelude::*;
        let mut dom = VirtualDom::new(|| {
            rsx! {
                super::RecipeEditorInput {
                    accessible_name: "Ingredient 2 amount",
                    value: "400",
                    placeholder: "Amount",
                    oninput: |_| {},
                }
            }
        });
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("aria-label=\"Ingredient 2 amount\""));
        assert!(html.contains("value=\"400\""));
    }
    #[test]
    fn contents_links_expose_optional_content_without_completion_gates() {
        use dioxus::prelude::*;
        let mut dom = VirtualDom::new(|| {
            let active_section = use_signal(|| "basics");
            rsx! {
                super::RecipeEditorContents {
                    active_section,
                    title_present: false,
                    photo_count: 2,
                    ingredient_count: 0,
                    step_count: 1,
                    equipment_count: 0,
                    notes_present: false,
                }
            }
        });
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        for section in [
            "basics",
            "photos",
            "ingredients",
            "method",
            "equipment",
            "notes",
        ] {
            assert!(html.contains(&format!("href=\"#recipe-{section}\"")));
        }
        assert!(html.contains("Optional · 2 photos"));
        assert!(html.contains("1 step added"));
        assert!(html.contains("No ingredients yet"));
        assert!(!html.contains("disabled"));
        assert!(!html.contains("Complete"));
    }
    use super::{
        build_step_tree, can_indent_step, flatten_steps, format_step_photo_file_size,
        step_photo_display_name, StepForm,
    };
    use crate::types::{RecipeImage, RecipeImageEdit, RecipeStep};

    fn photo(url: &str, file_name: &str, file_size: u64) -> RecipeImage {
        RecipeImage::new(
            url.to_string(),
            Some(format!("{url}?thumbnail=true")),
            RecipeImageEdit::default_rect_16x9(),
        )
        .with_file_metadata(file_name.to_string(), file_size)
    }

    #[test]
    fn nested_steps_and_photo_metadata_round_trip_through_the_shared_form() {
        let instructions = vec![
            RecipeStep {
                text: "Prepare".to_string(),
                photos: vec![photo(
                    "https://example.test/prepare.webp",
                    "prepare.webp",
                    1_024,
                )],
                sub_steps: vec![
                    RecipeStep {
                        text: "Chop".to_string(),
                        photos: vec![],
                        sub_steps: vec![RecipeStep {
                            text: "Finely".to_string(),
                            photos: vec![photo(
                                "https://example.test/finely.webp",
                                "finely.webp",
                                2_048,
                            )],
                            sub_steps: vec![],
                        }],
                    },
                    RecipeStep {
                        text: "Mix".to_string(),
                        photos: vec![],
                        sub_steps: vec![],
                    },
                ],
            },
            RecipeStep {
                text: "Serve".to_string(),
                photos: vec![],
                sub_steps: vec![],
            },
        ];

        let flat = flatten_steps(&instructions);

        assert_eq!(
            flat.iter().map(|step| step.depth).collect::<Vec<_>>(),
            vec![0, 1, 2, 1, 0]
        );
        assert_eq!(flat[0].photos[0].file_name.as_deref(), Some("prepare.webp"));
        assert_eq!(flat[2].photos[0].file_size, Some(2_048));
        assert_eq!(build_step_tree(&flat), instructions);
    }

    #[test]
    fn shared_step_form_stores_recipe_images_directly() {
        let image = photo("https://example.test/step.webp", "step-photo.webp", 4_096);
        let form = StepForm {
            text: "Mix".to_string(),
            depth: 0,
            photos: vec![image.clone()],
        };

        assert_eq!(form.photos, vec![image]);
        assert_eq!(step_photo_display_name(&form.photos[0]), "step-photo.webp");
    }

    #[test]
    fn step_photo_display_name_falls_back_to_the_url_file_name() {
        let image = RecipeImage::new(
            "https://example.test/uploads/step-photo.webp?version=2".to_string(),
            None,
            RecipeImageEdit::default_rect_16x9(),
        );

        assert_eq!(step_photo_display_name(&image), "step-photo.webp");
    }

    #[test]
    fn step_photo_size_formatting_handles_saved_and_legacy_metadata() {
        assert_eq!(format_step_photo_file_size(Some(8_192)), "8 KB");
        assert_eq!(format_step_photo_file_size(None), "Size unavailable");
    }

    #[test]
    fn step_photo_upload_errors_are_actionable_and_bound_filename_length() {
        let message = super::step_photo_upload_error_message(
            "sample.png",
            &dioxus::prelude::ServerFnError::Deserialization("lost response".into()),
        );
        assert!(message.contains("sample.png"));
        assert!(message.contains("Choose the photo again"));
        let message = super::step_photo_processing_failure(&"x".repeat(400));
        assert!(message.len() < 240);
        assert!(message.contains("valid JPEG, PNG, or WebP"));
    }

    #[test]
    fn recipe_upload_response_loss_uses_outcome_aware_feedback_in_both_editors() {
        for source in [
            include_str!("recipe_new.rs"),
            include_str!("recipe_edit.rs"),
        ] {
            assert!(source.contains("step_photo_upload_error_message("));
            assert!(source.contains("&problem,"));
            assert!(source.contains("image(s) need attention before saving"));
            assert!(!source.contains("image(s) failed to upload"));
        }
    }

    #[test]
    fn resolved_gallery_uploads_clear_only_their_own_warning_in_both_editors() {
        for source in [
            include_str!("recipe_new.rs"),
            include_str!("recipe_edit.rs"),
        ] {
            let recovery = source
                .split("// Clear only the gallery upload warning")
                .nth(1)
                .unwrap()
                .split("});")
                .next()
                .unwrap();
            assert!(recovery.contains("pending_image_count() == 0"));
            assert!(recovery.contains("gallery_upload_error.set(None)"));
            assert!(!recovery.contains("form_error.set"));
            assert!(!recovery.contains(" error.set("));
            assert!(source.contains("if let Some(err) = gallery_upload_error()"));
        }
    }

    #[test]
    fn step_photo_response_loss_warns_about_extra_copies_without_claiming_a_kept_preview() {
        use dioxus::prelude::ServerFnError;
        for error in [
            ServerFnError::Deserialization("private response detail".into()),
            ServerFnError::new("private backend detail"),
        ] {
            let message = super::step_photo_upload_error_message("sample.png", &error);
            assert!(message.contains("sample.png"));
            assert!(message.contains("couldn't confirm"));
            assert!(message.contains("another copy"));
            assert!(message.contains("Choose the photo again"));
            assert!(message.contains("adjustments must be reapplied"));
            assert!(!message.contains("preview is kept"));
            assert!(!message.contains("private"));
            assert!(!message.contains("Could not upload"));
        }
    }

    #[test]
    fn step_photo_known_rejections_keep_specific_safe_feedback_and_bound_filenames() {
        use dioxus::prelude::ServerFnError;
        let message = super::step_photo_upload_error_message(
            &"x".repeat(400),
            &ServerFnError::new("Upload rate limit exceeded"),
        );
        assert!(message.contains("Too many uploads"));
        assert!(message.len() < 400);
        assert!(!message.contains("couldn't confirm"));
        assert!(!message.contains("another copy"));
        let message = super::step_photo_upload_error_message(
            "sample.png",
            &ServerFnError::new("Not authenticated"),
        );
        assert!(message.contains("Sign in again in another tab"));
    }

    #[test]
    fn step_photo_upload_feedback_announces_pending_and_failed_files() {
        use dioxus::prelude::*;
        let mut dom = VirtualDom::new(|| {
            rsx! {
                super::StepPhotoUploadFeedback {
                    uploading: true,
                    errors: vec![super::step_photo_upload_error_message(
                        "test.png",
                        &ServerFnError::Deserialization("private backend detail".into()),
                    )],
                }
            }
        });
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("role=\"status\""), "{html}");
        assert!(html.contains("role=\"alert\""), "{html}");
        assert!(html.contains("Choose the photo again"), "{html}");
        assert!(html.contains("another copy"), "{html}");
        assert!(!html.contains("private backend detail"), "{html}");
        assert!(html.contains("overflow-wrap:anywhere"), "{html}");
    }

    #[test]
    fn both_step_photo_upload_paths_validate_claim_the_batch_and_release_local_previews() {
        for source in [
            include_str!("recipe_new.rs"),
            include_str!("recipe_edit.rs"),
        ] {
            let upload = source
                .split("let on_step_photo_change =")
                .nth(1)
                .unwrap()
                .split("// ── Submit handler")
                .next()
                .unwrap();
            assert!(upload.contains("validate_image_input"));
            assert!(upload.contains("step_photo_upload_count.set(1)"));
            assert!(upload.contains("let open_preview = files.len() == 1"));
            assert!(upload.contains("step_photo_processing_failure"));
            assert!(upload.contains("step_photo_upload_error_message"));
            assert!(upload.contains("step_photo_upload_errors.write().push"));
            assert!(source.contains("StepPhotoUploadFeedback {"));
            assert!(source.contains("step_photo_previews.write().clear()"));
            assert!(source.contains("if step_photo_controls_locked() { return; }"));
        }
    }

    #[test]
    fn step_photo_edits_preserve_asset_identity_and_other_named_views() {
        let mut image = photo("full.webp", "fixture.png", 4096);
        image.asset_id = Some(uuid::Uuid::from_u128(42));
        let mut rectangle = RecipeImageEdit::default_rect_16x9();
        rectangle.transform.rotation = 25.0;
        let ellipse = RecipeImageEdit::default_ellipse_1x1();
        image.edits = vec![ellipse.clone(), rectangle.clone()];
        let before = image.clone();
        assert_eq!(super::step_photo_editor_edit(&image), rectangle);
        rectangle.color.saturation = 0.5;
        super::apply_step_photo_edit(&mut image, rectangle.clone());
        assert_eq!(image.edits, vec![ellipse, rectangle]);
        assert_eq!(image.asset_id, before.asset_id);
        assert_eq!(image.url, before.url);
        assert_eq!(image.thumb_url, before.thumb_url);
        assert_eq!(image.file_name, before.file_name);
        assert_eq!(image.file_size, before.file_size);
        image.edits.clear();
        assert_eq!(
            super::step_photo_editor_edit(&image),
            RecipeImageEdit::default_rect_16x9()
        );
        super::apply_step_photo_edit(&mut image, RecipeImageEdit::default_rect_16x9());
        assert_eq!(image.edits.len(), 1);
    }

    #[test]
    fn step_photo_preview_has_a_named_native_keyboard_edit_action() {
        use dioxus::prelude::*;
        let mut dom = dioxus::prelude::VirtualDom::new(|| {
            rsx! {
                super::StepPhotoEditButton {
                    image: photo("full.webp", "fixture.png", 4096),
                    disabled: false,
                    onedit: |_| {},
                }
            }
        });
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert!(
            html.contains("aria-label=\"Edit step photo: fixture.png\""),
            "{html}"
        );
        assert!(html.contains("type=\"button\""), "{html}");
        assert!(html.contains("step-photo-preview"), "{html}");
        let mut disabled_dom = VirtualDom::new(|| {
            rsx! {
                super::StepPhotoEditButton {
                    image: photo("full.webp", "fixture.png", 4096),
                    disabled: true,
                    onedit: |_| {},
                }
            }
        });
        disabled_dom.rebuild_in_place();
        assert!(dioxus_ssr::render(&disabled_dom).contains("disabled"));
    }

    #[test]
    fn both_editors_restore_saved_step_photo_metadata_and_use_the_shared_edit_action() {
        for source in [
            include_str!("recipe_new.rs"),
            include_str!("recipe_edit.rs"),
        ] {
            assert!(source.contains("StepPhotoEditButton {"));
            assert!(source.contains("initial_edit: step_editor_initial_edit()"));
            assert!(source.contains("step_editor_modal_url.set(Some(photo.url))"));
            assert!(source.contains("apply_step_photo_edit(photo, edit)"));
            assert!(!source.contains("photo.edits = vec![edit]"));
        }
    }

    #[test]
    fn both_editors_use_the_same_maximum_step_depth() {
        assert!(can_indent_step(0));
        assert!(can_indent_step(2));
        assert!(!can_indent_step(3));
    }
}
