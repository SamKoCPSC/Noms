//! Read-only ingredient presence checks. Amounts and units are never consulted.
use super::{PantryEntry, PantryKind};
use crate::types::RecipeIngredient;
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IngredientStatus {
    Available,
    Missing,
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
    pub fn label(&self) -> &'static str {
        if self.ingredients.is_empty() {
            "No ingredients to check"
        } else if self
            .ingredients
            .iter()
            .any(|row| row.status == IngredientStatus::Missing)
        {
            "Missing ingredients"
        } else {
            "All ingredients available"
        }
    }
}

pub fn check_pantry(ingredients: &[RecipeIngredient], entries: &[PantryEntry]) -> PantryCheck {
    // Availability is an explicit user choice. Do not use PantryEntry::usable(),
    // which also excludes recorded zero amounts that may no longer be current.
    let available: HashSet<String> = entries
        .iter()
        .filter(|entry| entry.kind == PantryKind::Ingredient && entry.stock.available)
        .map(|entry| entry.stock.fields.name.to_lowercase())
        .collect();
    let mut seen = HashSet::new();
    let ingredients = ingredients
        .iter()
        .filter_map(|ingredient| {
            let name = ingredient.name.to_lowercase();
            if !seen.insert(name.clone()) {
                return None;
            }
            Some(IngredientCheck {
                name: ingredient.name.clone(),
                status: if available.contains(&name) {
                    IngredientStatus::Available
                } else {
                    IngredientStatus::Missing
                },
            })
        })
        .collect();
    PantryCheck { ingredients }
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
    }
}
