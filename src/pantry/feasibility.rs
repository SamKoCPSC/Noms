//! Read-only ingredient presence checks. Amounts and units are never consulted.
use super::{PantryEntry, PantryKind};
use crate::types::RecipeIngredient;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IngredientStatus {
    Available,
    Missing,
    OptionalMissing,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IngredientCheck {
    pub name: String,
    pub status: IngredientStatus,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PantryCheck {
    pub ingredients: Vec<IngredientCheck>,
}
impl PantryCheck {
    pub fn is_cookable(&self) -> bool {
        !self.ingredients.is_empty()
            && !self
                .ingredients
                .iter()
                .any(|row| row.status == IngredientStatus::Missing)
    }

    pub fn missing_optional_count(&self) -> usize {
        self.ingredients
            .iter()
            .filter(|row| row.status == IngredientStatus::OptionalMissing)
            .count()
    }

    pub fn label(&self) -> &'static str {
        if self.ingredients.is_empty() {
            "No ingredients to check"
        } else if !self.is_cookable() {
            "Missing ingredients"
        } else if self.missing_optional_count() > 0 {
            "Optional ingredients missing"
        } else {
            "All ingredients available"
        }
    }
}

pub fn check_pantry(ingredients: &[RecipeIngredient], entries: &[PantryEntry]) -> PantryCheck {
    PantryMatcher::new(entries).check(ingredients)
}

/// Reuse one presence index when checking a library against the same pantry.
pub struct PantryMatcher {
    available: HashSet<String>,
}
impl PantryMatcher {
    pub fn new(entries: &[PantryEntry]) -> Self {
        // Availability is an explicit user choice. Do not use PantryEntry::usable(),
        // which also excludes recorded zero amounts that may no longer be current.
        let available = entries
            .iter()
            .filter(|entry| entry.kind == PantryKind::Ingredient && entry.stock.available)
            .map(|entry| entry.stock.fields.name.to_lowercase())
            .collect();
        Self { available }
    }

    pub fn is_available(&self, name: &str) -> bool {
        !name.trim().is_empty() && self.available.contains(&name.to_lowercase())
    }

    pub fn check(&self, ingredients: &[RecipeIngredient]) -> PantryCheck {
        self.check_requirements(ingredients.iter().map(|i| (i.name.as_str(), i.optional)))
    }

    /// A required occurrence wins over an optional one with the same name.
    /// Keep the first authored spelling/order, regardless of occurrence order.
    pub fn check_requirements<'a>(
        &self,
        requirements: impl IntoIterator<Item = (&'a str, bool)>,
    ) -> PantryCheck {
        let mut positions = HashMap::new();
        let mut ingredients: Vec<IngredientCheck> = Vec::new();
        for (name, optional) in requirements {
            let key = name.to_lowercase();
            let status = if name.trim().is_empty() {
                IngredientStatus::Missing
            } else if self.is_available(name) {
                IngredientStatus::Available
            } else if optional {
                IngredientStatus::OptionalMissing
            } else {
                IngredientStatus::Missing
            };
            if let Some(&index) = positions.get(&key) {
                let row: &mut IngredientCheck = &mut ingredients[index];
                if status == IngredientStatus::Missing {
                    row.status = status;
                }
            } else {
                positions.insert(key, ingredients.len());
                ingredients.push(IngredientCheck {
                    name: name.to_string(),
                    status,
                });
            }
        }
        PantryCheck { ingredients }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pantry::{PantryFields, PantryStock};

    fn ingredient(name: &str, amount: &str, unit: &str) -> RecipeIngredient {
        RecipeIngredient {
            vocabulary_id: None,
            name: name.into(),
            amount: amount.into(),
            unit: unit.into(),
            ..Default::default()
        }
    }
    fn stock(name: &str, amount: &str, unit: &str, available: bool) -> PantryEntry {
        PantryEntry {
            id: uuid::Uuid::new_v4(),
            kind: PantryKind::Ingredient,
            stock: PantryStock {
                fields: PantryFields {
                    name: name.into(),
                    amount: amount.into(),
                    unit: unit.into(),
                },
                available,
            },
            quantity_zero: amount == "0",
            position: 0,
            revision: 0,
        }
    }

    #[test]
    fn optional_metadata_is_used_but_modifiers_and_substitutions_do_not_change_identity() {
        let mut rice = ingredient("Rice", "1", "cup");
        rice.optional = true;
        rice.text_before = "Acme".into();
        rice.text_after = "rinsed".into();
        rice.substitutes.push(ingredient("Lentils", "300", "g"));
        assert_eq!(
            check_pantry(&[rice.clone()], &[stock("Lentils", "", "", true)]).label(),
            "Optional ingredients missing"
        );
        assert_eq!(
            check_pantry(&[rice], &[stock("rice", "", "", true)]).label(),
            "All ingredients available"
        );
    }

    #[test]
    fn amounts_and_units_do_not_affect_presence_including_recorded_zero() {
        for (amount, unit) in [("0", "g"), ("", ""), ("a little", "cups"), ("0.001", "kg")] {
            let check = check_pantry(
                &[ingredient("Rice", "1000", "cup")],
                &[stock("Rice", amount, unit, true)],
            );
            assert_eq!(check.label(), "All ingredients available");
        }
    }

    #[test]
    fn explicit_out_overrides_recorded_amount_and_any_available_row_satisfies_presence() {
        let recipe = [ingredient("Rice", "1", "cup")];
        let out = stock("Rice", "1000", "cup", false);
        assert_eq!(
            check_pantry(&recipe, std::slice::from_ref(&out)).label(),
            "Missing ingredients"
        );
        assert_eq!(
            check_pantry(&recipe, &[out, stock("rICE", "0", "g", true)]).label(),
            "All ingredients available"
        );
    }

    #[test]
    fn names_ignore_case_but_not_spelling_or_whitespace() {
        for (recipe_name, pantry_name) in
            [("Rice", "rice"), ("rice", "RICE"), ("Jalapeño", "JALAPEÑO")]
        {
            assert_eq!(
                check_pantry(
                    &[ingredient(recipe_name, "1", "cup")],
                    &[stock(pantry_name, "2", "g", true)]
                )
                .label(),
                "All ingredients available"
            );
        }
        let mut recipe = ingredient("Rice", "1", "cup");
        recipe.vocabulary_id = Some(uuid::Uuid::new_v4());
        for name in ["Rice ", "Brown rice"] {
            assert_eq!(
                check_pantry(
                    std::slice::from_ref(&recipe),
                    &[stock(name, "2", "cup", true)]
                )
                .label(),
                "Missing ingredients"
            );
        }
    }

    #[test]
    fn repeated_names_ignore_case_and_units_preserving_first_authored_spelling_and_order() {
        let recipe = [
            ingredient("Salt", "1", "g"),
            ingredient("Rice", "1", "cup"),
            ingredient("rICE", "500", "g"),
            ingredient("SALT", "2", "tsp"),
        ];
        let check = check_pantry(&recipe, &[stock("Rice", "", "", true)]);
        assert_eq!(
            check
                .ingredients
                .iter()
                .map(|row| row.name.as_str())
                .collect::<Vec<_>>(),
            ["Salt", "Rice"]
        );
        assert_eq!(check.ingredients[0].status, IngredientStatus::Missing);
        assert_eq!(check.ingredients[1].status, IngredientStatus::Available);
        let missing = check_pantry(&recipe, &[]);
        assert_eq!(missing.ingredients.len(), 2);
        assert_eq!(missing.ingredients[1].name, "Rice");
    }

    #[test]
    fn equipment_never_satisfies_an_ingredient() {
        let mut entry = stock("Rice", "", "", true);
        entry.kind = PantryKind::Equipment;
        assert_eq!(
            check_pantry(&[ingredient("Rice", "1", "cup")], &[entry]).label(),
            "Missing ingredients"
        );
    }

    #[test]
    fn all_authored_ingredients_are_checked_regardless_of_quantity_or_optional_wording() {
        for ingredient in [
            ingredient("Rice", "0", "cup"),
            ingredient("Garnish (optional)", "to taste", ""),
        ] {
            assert_eq!(
                check_pantry(&[ingredient], &[]).label(),
                "Missing ingredients"
            );
        }
    }

    #[test]
    fn empty_recipe_does_not_claim_availability() {
        assert_eq!(check_pantry(&[], &[]).label(), "No ingredients to check");
        assert!(!check_pantry(&[], &[]).is_cookable());
    }

    #[test]
    fn missing_optional_items_allow_cooking_but_are_distinct_from_full_matches() {
        let mut garnish = ingredient("Parsley", "1", "g");
        garnish.optional = true;
        let recipe = [ingredient("Rice", "100", "g"), garnish];
        let partial = check_pantry(&recipe, &[stock("rice", "0", "kg", true)]);
        assert!(partial.is_cookable());
        assert_eq!(partial.label(), "Optional ingredients missing");
        assert_eq!(partial.missing_optional_count(), 1);
        let full = check_pantry(
            &recipe,
            &[stock("rice", "", "", true), stock("PARSLEY", "", "", true)],
        );
        assert!(full.is_cookable());
        assert_eq!(full.label(), "All ingredients available");
        assert_eq!(full.missing_optional_count(), 0);
        let missing = check_pantry(&recipe, &[]);
        assert!(!missing.is_cookable());
        assert_eq!(missing.label(), "Missing ingredients");
        assert_eq!(missing.missing_optional_count(), 1);
    }

    #[test]
    fn required_duplicate_wins_in_either_order_and_optional_duplicates_count_once() {
        let matcher = PantryMatcher::new(&[]);
        for requirements in [
            vec![("Salt", true), ("SALT", false)],
            vec![("Salt", false), ("SALT", true)],
        ] {
            let result = matcher.check_requirements(requirements);
            assert!(!result.is_cookable());
            assert_eq!(result.ingredients.len(), 1);
            assert_eq!(result.ingredients[0].name, "Salt");
            assert_eq!(result.missing_optional_count(), 0);
        }
        let optional_only = matcher.check_requirements([("Salt", true), ("SALT", true)]);
        assert!(optional_only.is_cookable());
        assert_eq!(optional_only.missing_optional_count(), 1);
    }
}
