//! Three-way recipe merge. Lists are indivisible: positions and names are not
//! persistent entry identities, especially after insertion, deletion or reorder.
use crate::types::RecipeEditorState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum MergeChoice {
    Yours,
    Saved,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RecipeField {
    Title,
    Description,
    Commentary,
    PrepTime,
    CookTime,
    Servings,
    Ingredients,
    Instructions,
    Equipment,
    Images,
    Visibility,
    Tags,
}

pub(super) const FIELDS: [RecipeField; 12] = [
    RecipeField::Title,
    RecipeField::Description,
    RecipeField::Commentary,
    RecipeField::PrepTime,
    RecipeField::CookTime,
    RecipeField::Servings,
    RecipeField::Ingredients,
    RecipeField::Instructions,
    RecipeField::Equipment,
    RecipeField::Images,
    RecipeField::Visibility,
    RecipeField::Tags,
];

pub(super) type MergeChoices = [Option<MergeChoice>; FIELDS.len()];

impl RecipeField {
    pub fn label(self) -> &'static str {
        match self {
            Self::Title => "Title",
            Self::Description => "Description",
            Self::Commentary => "Notes",
            Self::PrepTime => "Preparation time",
            Self::CookTime => "Cooking time",
            Self::Servings => "Servings",
            Self::Ingredients => "Ingredients",
            Self::Instructions => "Steps and step photos",
            Self::Equipment => "Equipment",
            Self::Images => "Gallery photos",
            Self::Visibility => "Sharing",
            Self::Tags => "Tags",
        }
    }

    pub fn is_list(self) -> bool {
        matches!(
            self,
            Self::Ingredients | Self::Instructions | Self::Equipment | Self::Images | Self::Tags
        )
    }

    fn preference(
        self,
        base: &RecipeEditorState,
        yours: &RecipeEditorState,
        saved: &RecipeEditorState,
    ) -> (bool, Option<MergeChoice>) {
        fn compare<T: PartialEq>(base: &T, yours: &T, saved: &T) -> (bool, Option<MergeChoice>) {
            if yours == saved {
                (false, Some(MergeChoice::Saved))
            } else if yours == base {
                (true, Some(MergeChoice::Saved))
            } else if saved == base {
                (true, Some(MergeChoice::Yours))
            } else {
                (true, None)
            }
        }
        macro_rules! compare_field {
            ($field:ident) => {
                compare(&base.$field, &yours.$field, &saved.$field)
            };
        }
        match self {
            Self::Title => compare_field!(title),
            Self::Description => compare_field!(description),
            Self::Commentary => compare_field!(commentary),
            Self::PrepTime => compare_field!(prep_time_minutes),
            Self::CookTime => compare_field!(cook_time_minutes),
            Self::Servings => compare_field!(servings),
            Self::Ingredients => compare_field!(ingredients),
            Self::Instructions => compare_field!(instructions),
            Self::Equipment => compare_field!(equipment),
            Self::Images => compare_field!(images),
            Self::Visibility => compare_field!(visibility),
            Self::Tags => compare_field!(tags),
        }
    }

    fn take(self, target: &mut RecipeEditorState, source: &RecipeEditorState) {
        match self {
            Self::Title => target.title = source.title.clone(),
            Self::Description => target.description = source.description.clone(),
            Self::Commentary => target.commentary = source.commentary.clone(),
            Self::PrepTime => target.prep_time_minutes = source.prep_time_minutes,
            Self::CookTime => target.cook_time_minutes = source.cook_time_minutes,
            Self::Servings => target.servings = source.servings,
            Self::Ingredients => target.ingredients = source.ingredients.clone(),
            Self::Instructions => target.instructions = source.instructions.clone(),
            Self::Equipment => target.equipment = source.equipment.clone(),
            Self::Images => target.images = source.images.clone(),
            Self::Visibility => target.visibility = source.visibility.clone(),
            Self::Tags => target.tags = source.tags.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct MergeChange {
    pub field: RecipeField,
    pub suggested: Option<MergeChoice>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct MergePlan {
    pub yours: RecipeEditorState,
    pub saved: RecipeEditorState,
    pub changes: Vec<MergeChange>,
}

impl MergePlan {
    pub fn new(
        base: &RecipeEditorState,
        yours: RecipeEditorState,
        saved: RecipeEditorState,
    ) -> Self {
        let changes = FIELDS
            .into_iter()
            .filter_map(|field| {
                let (different, suggested) = field.preference(base, &yours, &saved);
                different.then_some(MergeChange { field, suggested })
            })
            .collect();
        Self {
            yours,
            saved,
            changes,
        }
    }

    pub fn suggested_choices(&self) -> MergeChoices {
        let mut choices = [None; FIELDS.len()];
        for change in &self.changes {
            choices[change.field as usize] = change.suggested;
        }
        choices
    }

    pub fn resolve(&self, choices: &MergeChoices) -> Option<RecipeEditorState> {
        let mut merged = self.saved.clone();
        for change in &self.changes {
            match choices[change.field as usize]? {
                MergeChoice::Yours => change.field.take(&mut merged, &self.yours),
                MergeChoice::Saved => {}
            }
        }
        Some(merged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{RecipeEquipment, RecipeImage, RecipeIngredient, RecipeStep};

    #[test]
    fn independent_changes_combine_and_agreed_changes_need_no_choice() {
        let base = RecipeEditorState::default();
        let mut yours = base.clone();
        yours.description = Some("Locally written".into());
        yours.servings = Some(4);
        let mut saved = base.clone();
        saved.title = "Saved elsewhere".into();
        saved.servings = Some(4);
        let plan = MergePlan::new(&base, yours.clone(), saved.clone());
        let merged = plan.resolve(&plan.suggested_choices()).unwrap();
        assert_eq!(merged.description, yours.description);
        assert_eq!(merged.title, saved.title);
        assert_eq!(merged.servings, Some(4));
        assert_eq!(plan.changes.len(), 2);
        // An explicit choice may override even a non-conflicting suggestion.
        let mut choices = plan.suggested_choices();
        choices[RecipeField::Title as usize] = Some(MergeChoice::Yours);
        assert_eq!(plan.resolve(&choices).unwrap().title, base.title);
    }

    #[test]
    fn competing_text_and_deletions_require_explicit_choices() {
        let base = RecipeEditorState {
            description: Some("Original".into()),
            ..Default::default()
        };
        let mut yours = base.clone();
        yours.description = None;
        let mut saved = base.clone();
        saved.description = Some("Edited elsewhere".into());
        let plan = MergePlan::new(&base, yours, saved.clone());
        let mut choices = plan.suggested_choices();
        assert!(plan.resolve(&choices).is_none());
        choices[RecipeField::Description as usize] = Some(MergeChoice::Yours);
        assert_eq!(plan.resolve(&choices).unwrap().description, None);
        choices[RecipeField::Description as usize] = Some(MergeChoice::Saved);
        assert_eq!(
            plan.resolve(&choices).unwrap().description,
            saved.description
        );
    }

    #[test]
    fn list_insertions_reorders_deletions_and_photo_edits_stay_whole() {
        let mut base = RecipeEditorState::default();
        base.ingredients.push(RecipeIngredient {
            name: "Flour".into(),
            ..Default::default()
        });
        base.instructions.push(RecipeStep {
            text: "Mix".into(),
            sub_steps: vec![],
            photos: vec![],
        });
        base.equipment.push(RecipeEquipment {
            name: "Bowl".into(),
        });
        base.images.push(RecipeImage::new(
            "/photo".into(),
            None,
            crate::types::RecipeImageEdit::default_rect_16x9(),
        ));
        base.tags = vec!["one".into(), "two".into()];
        let mut yours = base.clone();
        yours.ingredients[0].amount = "2".into();
        yours.instructions[0].text = "Mix gently".into();
        yours.equipment.clear();
        yours.images[0].file_name = Some("My photo".into());
        yours.tags.reverse();
        let mut saved = base.clone();
        saved.ingredients.insert(
            0,
            RecipeIngredient {
                name: "Sugar".into(),
                ..Default::default()
            },
        );
        saved.instructions.clear();
        saved.equipment[0].name = "Large bowl".into();
        saved.images.clear();
        saved.tags.push("three".into());
        let plan = MergePlan::new(&base, yours.clone(), saved.clone());
        assert_eq!(plan.changes.len(), 5);
        assert!(plan
            .changes
            .iter()
            .all(|change| change.suggested.is_none() && change.field.is_list()));
        assert!(plan.resolve(&plan.suggested_choices()).is_none());
        let mut choices = [Some(MergeChoice::Yours); FIELDS.len()];
        assert_eq!(plan.resolve(&choices).unwrap(), yours);
        choices.fill(Some(MergeChoice::Saved));
        assert_eq!(plan.resolve(&choices).unwrap(), saved);
    }

    #[test]
    fn every_editor_field_participates_in_the_merge() {
        let base = RecipeEditorState::default();
        let yours = RecipeEditorState {
            title: "Title".into(),
            description: Some("Description".into()),
            commentary: Some("Notes".into()),
            prep_time_minutes: Some(10),
            cook_time_minutes: Some(20),
            servings: Some(2),
            ingredients: vec![RecipeIngredient {
                name: "Flour".into(),
                ..Default::default()
            }],
            instructions: vec![RecipeStep {
                text: "Mix".into(),
                sub_steps: vec![],
                photos: vec![],
            }],
            equipment: vec![RecipeEquipment {
                name: "Bowl".into(),
            }],
            images: vec![RecipeImage::new(
                "/photo".into(),
                None,
                crate::types::RecipeImageEdit::default_rect_16x9(),
            )],
            visibility: "public".into(),
            tags: vec!["Tag".into()],
        };
        let plan = MergePlan::new(&base, yours.clone(), base.clone());
        assert_eq!(plan.changes.len(), FIELDS.len());
        assert_eq!(plan.resolve(&plan.suggested_choices()), Some(yours));
    }
}
