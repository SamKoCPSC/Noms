//! Grocery-list contracts and pure, unit-preserving requirement calculation.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "server")]
pub const MAX_LISTS: usize = 100;
pub const MAX_ITEMS: usize = 1_000;
pub const MAX_RECIPES: usize = 20;
pub const MAX_NAME: usize = 500;
pub const MAX_AMOUNT: usize = 64;
pub const MAX_UNIT: usize = 64;
pub const MAX_PAYLOAD: usize = 2 * 1024 * 1024;
pub const MAX_SERVINGS: i32 = 10_000;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemFields {
    pub name: String,
    pub amount: String,
    pub unit: String,
}
impl ItemFields {
    /// Preserve source-recipe provenance on reads, but require supported units
    /// whenever an author creates or edits fields.
    pub fn validate_input(&self) -> Result<(), String> {
        self.validate()?;
        noms::units::canonicalize(&self.unit).map_err(String::from)?;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() || self.name.chars().count() > MAX_NAME {
            return Err("Item names must contain 1–500 characters.".into());
        }
        if self.amount.chars().count() > MAX_AMOUNT || self.unit.chars().count() > MAX_UNIT {
            return Err("Amounts and units must be 64 characters or fewer.".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GrocerySource {
    pub recipe_id: Uuid,
    pub title: String,
    pub base_servings: Option<i32>,
    pub target_servings: Option<i32>,
    pub revision: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GroceryRecipePreview {
    pub source: GrocerySource,
    pub ingredients: Vec<crate::types::RecipeIngredient>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecipeSelection {
    pub recipe_id: Uuid,
    pub target_servings: Option<i32>,
    pub expected_revision: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Contribution {
    pub source: GrocerySource,
    pub ingredient_index: usize,
    pub original: ItemFields,
    pub scaled_amount: String,
    pub quantity_uncertain: bool,
    pub approximate: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GeneratedItem {
    pub fields: ItemFields,
    pub contributions: Vec<Contribution>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GroceryItem {
    pub id: Uuid,
    pub fields: ItemFields,
    pub original: Option<ItemFields>,
    pub contributions: Vec<Contribution>,
    pub checked: bool,
    pub position: i32,
    pub revision: i64,
}
impl GroceryItem {
    pub fn edited(&self) -> bool {
        self.original
            .as_ref()
            .is_some_and(|original| original != &self.fields)
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GroceryList {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
    pub note: String,
    pub sources: Vec<GrocerySource>,
    pub items: Vec<GroceryItem>,
    pub revision: i64,
    pub created_at: DateTime<Utc>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GroceryListSummary {
    pub id: Uuid,
    pub name: String,
    pub item_count: i64,
    pub checked_count: i64,
    pub recipe_count: usize,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum GroceryItemChange {
    Fields(ItemFields),
    Checked(bool),
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GroceryCreate {
    pub request_id: Uuid,
    pub name: String,
    pub note: String,
    pub recipes: Vec<RecipeSelection>,
    pub manual_items: Vec<ItemFields>,
}
impl GroceryCreate {
    pub fn validate_input(&self) -> Result<(), String> {
        self.validate()?;
        for item in &self.manual_items {
            item.validate_input()?;
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<(), String> {
        validate_list_name(&self.name)?;
        if self.note.chars().count() > 500 {
            return Err("List notes must be 500 characters or fewer.".into());
        }
        if self.recipes.len() > MAX_RECIPES || self.manual_items.len() > MAX_ITEMS {
            return Err("A list supports up to 20 recipes and 1,000 items.".into());
        }
        let mut ids = std::collections::HashSet::new();
        for recipe in &self.recipes {
            if !ids.insert(recipe.recipe_id) {
                return Err("Select each recipe only once.".into());
            }
            if recipe.expected_revision.len() != 64 {
                return Err("Review the recipe before creating this list.".into());
            }
            if recipe
                .target_servings
                .is_some_and(|s| !(1..=MAX_SERVINGS).contains(&s))
            {
                return Err("Target servings must be between 1 and 10,000.".into());
            }
        }
        for item in &self.manual_items {
            item.validate()?;
        }
        if serde_json::to_vec(self)
            .map_err(|_| "Invalid list.".to_string())?
            .len()
            > MAX_PAYLOAD
        {
            return Err("This grocery list is too large.".into());
        }
        Ok(())
    }
}
pub fn validate_list_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() || name.chars().count() > 100 {
        return Err("List names must contain 1–100 characters.".into());
    }
    Ok(())
}

/// Exact bounded rational arithmetic: do not aggregate already-rounded display values.
#[derive(Clone, Copy, Debug)]
struct Quantity {
    n: i128,
    d: i128,
}
impl Quantity {
    fn new(n: i128, d: i128) -> Option<Self> {
        if n < 0 || d <= 0 {
            return None;
        }
        let mut a = n;
        let mut b = d;
        while b != 0 {
            let next = a % b;
            a = b;
            b = next;
        }
        let gcd = a.max(1);
        let n = n / gcd;
        let d = d / gcd;
        if n > 10_i128.pow(30) || d > 10_i128.pow(24) {
            return None;
        }
        Some(Self { n, d })
    }
    fn add(self, rhs: Self) -> Option<Self> {
        Self::new(
            self.n
                .checked_mul(rhs.d)?
                .checked_add(rhs.n.checked_mul(self.d)?)?,
            self.d.checked_mul(rhs.d)?,
        )
    }
    fn scale(self, n: i128, d: i128) -> Option<Self> {
        if n == d && n > 0 {
            return Some(self);
        }
        Self::new(self.n.checked_mul(n)?, self.d.checked_mul(d)?)
    }
    fn display(self) -> (String, bool) {
        const PRECISION: i128 = 1_000_000;
        // Bound the whole amount; safe for scaled numerators before formatting.
        let quotient = self.n / self.d;
        let remainder = self.n % self.d;
        let fractional = remainder * PRECISION;
        let rounded = (fractional + self.d / 2) / self.d;
        let value = quotient * PRECISION + rounded;
        if self.n > 0 && value == 0 {
            return ("<0.000001".into(), true);
        }
        let approximate = fractional % self.d != 0;
        let text = if value % PRECISION == 0 {
            (value / PRECISION).to_string()
        } else {
            format!("{}.{:06}", value / PRECISION, value % PRECISION)
                .trim_end_matches('0')
                .to_string()
        };
        (text, approximate)
    }
}
fn parse_quantity(text: &str) -> Option<Quantity> {
    fn decimal(text: &str) -> Option<Quantity> {
        if text.is_empty()
            || text.len() > 24
            || !text.bytes().all(|c| c.is_ascii_digit() || c == b'.')
        {
            return None;
        }
        let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
        if fraction.len() > 9 || fraction.contains('.') || (whole.is_empty() && fraction.is_empty())
        {
            return None;
        }
        let divisor = 10_i128.pow(fraction.len() as u32);
        let n = whole
            .parse::<i128>()
            .ok()
            .or_else(|| whole.is_empty().then_some(0))?
            .checked_mul(divisor)?
            .checked_add(if fraction.is_empty() {
                0
            } else {
                fraction.parse::<i128>().ok()?
            })?;
        if n / divisor > 1_000_000_000_000 {
            return None;
        }
        Quantity::new(n, divisor)
    }
    fn fraction(text: &str) -> Option<Quantity> {
        if let Some((n, d)) = text.split_once('/') {
            let numerator = decimal(n)?;
            let denominator = decimal(d)?;
            numerator.scale(denominator.d, denominator.n)
        } else {
            decimal(text)
        }
    }
    let text = text.trim();
    let tokens = text.split_whitespace().collect::<Vec<_>>();
    match tokens.as_slice() {
        [single] => fraction(single),
        [whole, part] if !whole.contains('/') && part.contains('/') => {
            decimal(whole)?.add(fraction(part)?)
        }
        [n, "/", d] => fraction(&format!("{n}/{d}")),
        _ => None,
    }
}

/// Shared inventory classification; preserves the grocery parser and its bounds.
pub(crate) fn quantity_is_zero(text: &str) -> Option<bool> {
    parse_quantity(text).map(|q| q.n == 0)
}

pub fn build_requirements(previews: &[GroceryRecipePreview]) -> Result<Vec<GeneratedItem>, String> {
    if previews.len() > MAX_RECIPES {
        return Err("Select up to 20 recipes.".into());
    }
    let mut result: Vec<(GeneratedItem, Option<Quantity>)> = Vec::new();
    for preview in previews {
        if preview.ingredients.len() > 200 {
            return Err("This recipe has too many ingredients.".into());
        }
        let base = preview.source.base_servings.filter(|s| *s > 0);
        let target = preview.source.target_servings;
        if target.is_some_and(|s| !(1..=MAX_SERVINGS).contains(&s)) {
            return Err("Invalid serving count.".into());
        }
        if base.is_none() && target.is_some() {
            return Err(
                "This recipe has no base serving count. Copy its original amounts instead.".into(),
            );
        }
        let factor = (
            target.unwrap_or(base.unwrap_or(1)) as i128,
            base.unwrap_or(1) as i128,
        );
        for (index, ingredient) in preview.ingredients.iter().enumerate() {
            let original = ItemFields {
                name: ingredient.name.clone(),
                amount: ingredient.amount.clone(),
                unit: ingredient.unit.clone(),
            };
            original.validate()?;
            let quantity = parse_quantity(&ingredient.amount)
                .map(|q| {
                    q.scale(factor.0, factor.1)
                        .ok_or_else(|| "Quantity exceeds the supported range.".to_string())
                })
                .transpose()?;
            let (amount, approximate) = quantity
                .map(Quantity::display)
                .unwrap_or_else(|| (ingredient.amount.clone(), false));
            let contribution = Contribution {
                source: preview.source.clone(),
                ingredient_index: index,
                original: original.clone(),
                scaled_amount: amount.clone(),
                quantity_uncertain: quantity.is_none() || base.is_none(),
                approximate,
            };
            if let Some(q) = quantity {
                if let Some((item, total)) = result.iter_mut().find(|(item, total)| {
                    total.is_some()
                        && item.fields.name == original.name
                        && item.fields.unit == original.unit
                }) {
                    let sum = total
                        .unwrap()
                        .add(q)
                        .ok_or("Quantity exceeds the supported range.")?;
                    if sum.n / sum.d > 10_000_000_000_000_000 {
                        return Err("Quantity exceeds the supported range.".into());
                    }
                    item.fields.amount = sum.display().0;
                    item.contributions.push(contribution);
                    *total = Some(sum);
                    continue;
                }
            }
            result.push((
                GeneratedItem {
                    fields: ItemFields { amount, ..original },
                    contributions: vec![contribution],
                },
                quantity,
            ));
            if result.len() > MAX_ITEMS {
                return Err("A grocery list supports up to 1,000 items.".into());
            }
        }
    }
    Ok(result.into_iter().map(|(item, _)| item).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unit_input_policy_keeps_source_provenance_but_validates_manual_fields() {
        let mut fields = ItemFields {
            name: "Rice".into(),
            amount: "2".into(),
            unit: "bag".into(),
        };
        assert!(fields.validate().is_ok());
        assert!(fields.validate_input().is_err());
        fields.unit = "grams".into();
        assert!(fields.validate_input().is_ok());
        fields.unit.clear();
        assert!(fields.validate_input().is_err());
    }
    fn recipe(
        name: &str,
        amount: &str,
        unit: &str,
        base: Option<i32>,
        target: Option<i32>,
    ) -> GroceryRecipePreview {
        GroceryRecipePreview {
            source: GrocerySource {
                recipe_id: Uuid::new_v4(),
                title: "Dinner".into(),
                base_servings: base,
                target_servings: target,
                revision: "a".repeat(64),
            },
            ingredients: vec![crate::types::RecipeIngredient {
                vocabulary_id: None,
                name: name.into(),
                amount: amount.into(),
                unit: unit.into(),
                ..Default::default()
            }],
        }
    }
    #[test]
    fn ingredient_options_leave_grocery_generation_unchanged() {
        let mut input = recipe("Rice", "1", "cup", Some(2), Some(4));
        input.ingredients[0].optional = true;
        input.ingredients[0].text_before = "Acme".into();
        input.ingredients[0].text_after = "rinsed".into();
        input.ingredients[0]
            .substitutes
            .push(crate::types::RecipeIngredient {
                name: "Lentils".into(),
                amount: "300".into(),
                unit: "g".into(),
                ..Default::default()
            });
        let list = build_requirements(&[input]).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].fields.name, "Rice");
        assert_eq!(list[0].fields.amount, "2");
        assert_eq!(list[0].fields.unit, "cup");
    }

    #[test]
    fn serving_aware_exact_unit_matching_and_provenance() {
        let list = build_requirements(&[
            recipe("Rice", "1", "cup", Some(2), Some(4)),
            recipe("Rice", "2", "cup", Some(2), None),
            recipe("Rice", "1", "cups", Some(2), None),
            recipe("rice", "1", "cup", Some(2), None),
        ])
        .unwrap();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].fields.amount, "4");
        assert_eq!(list[0].contributions.len(), 2);
        assert_eq!(list[0].contributions[0].original.amount, "1");
        assert_eq!(list[0].contributions[0].scaled_amount, "2");
    }
    #[test]
    fn fractions_sum_before_rounding_and_small_values_stay_visible() {
        let list = build_requirements(&[
            recipe("Salt", "1/3", "g", Some(1), None),
            recipe("Salt", "1/3", "g", Some(1), None),
            recipe("Salt", "1/3", "g", Some(1), None),
        ])
        .unwrap();
        assert_eq!(list[0].fields.amount, "1");
        assert_eq!(parse_quantity("1 1/2").unwrap().display().0, "1.5");
        assert_eq!(parse_quantity("1 / 2").unwrap().display().0, "0.5");
        let tiny =
            build_requirements(&[recipe("Salt", "0.000000001", "g", Some(1000), Some(1))]).unwrap();
        assert_eq!(tiny[0].fields.amount, "<0.000001");
    }
    #[test]
    fn copy_all_ingredients_and_preserve_unknown_amounts() {
        let list = build_requirements(&[
            recipe("Garnish (optional)", "to taste", "", Some(2), Some(4)),
            recipe("Garnish (optional)", "to taste", "", Some(2), Some(4)),
            recipe("Rice", "200", "g", Some(2), None),
            recipe("Rice", "1", "cup", Some(2), None),
        ])
        .unwrap();
        assert_eq!(list.len(), 4);
        assert!(list[0].contributions[0].quantity_uncertain);
        assert_eq!(list[0].fields.amount, "to taste");
    }
    #[test]
    fn missing_servings_preserve_original_and_cannot_scale() {
        let list = build_requirements(&[recipe("Rice", "2", "cup", None, None)]).unwrap();
        assert_eq!(list[0].fields.amount, "2");
        assert!(list[0].contributions[0].quantity_uncertain);
        assert!(build_requirements(&[recipe("Rice", "2", "cup", None, Some(4))]).is_err());
    }
    #[test]
    fn invalid_numeric_text_is_not_coerced_or_dropped() {
        for text in ["-2", "NaN", "inf", "1/0", "1e300"] {
            assert!(parse_quantity(text).is_none());
            assert_eq!(
                build_requirements(&[recipe("Test", text, "ml", Some(2), Some(4))]).unwrap()[0]
                    .fields
                    .amount,
                text
            );
        }
        assert_eq!(
            build_requirements(&[recipe("Eggs", "2", "", Some(2), Some(4))]).unwrap()[0]
                .fields
                .amount,
            "4"
        );
    }
    #[test]
    fn full_edits_do_not_change_original_requirements() {
        let generated = build_requirements(&[recipe("Rice", "500", "g", Some(2), None)])
            .unwrap()
            .remove(0);
        let mut row = GroceryItem {
            id: Uuid::new_v4(),
            fields: generated.fields.clone(),
            original: Some(generated.fields),
            contributions: generated.contributions,
            checked: true,
            position: 0,
            revision: 0,
        };
        row.fields = ItemFields {
            name: "Brown rice".into(),
            amount: "2".into(),
            unit: "cup".into(),
        };
        assert!(row.edited());
        assert_eq!(row.contributions[0].original.unit, "g");
        assert!(row.checked);
    }
}
