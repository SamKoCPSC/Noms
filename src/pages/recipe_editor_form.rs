use crate::components::base::EditedRecipeImage;
use crate::types::{RecipeImage, RecipeImageEdit, RecipeStep};
use dioxus::prelude::*;
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

#[cfg(test)]
mod tests {
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
