use crate::types::{RecipeImage, RecipeStep};

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
    fn both_editors_use_the_same_maximum_step_depth() {
        assert!(can_indent_step(0));
        assert!(can_indent_step(2));
        assert!(!can_indent_step(3));
    }
}
