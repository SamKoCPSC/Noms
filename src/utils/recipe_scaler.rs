//! Pure-logic recipe scaling calculator.
//!
//! Parses cooking amounts (integers, decimals, fractions, mixed numbers),
//! formats them as cooking-friendly fractions, and scales recipe ingredients
//! proportionally by multiplier or by targeting a specific ingredient.
//!
//! This module has no Dioxus or WASM dependencies and compiles on all targets,
//! allowing unit tests to run natively.

/// Lightweight reference to an ingredient's display fields.
/// Used by ScaleCalculator to avoid coupling to ParsedIngredient.
#[derive(Clone, Debug)]
pub struct IngredientRef {
    pub amount: String,
    pub unit: String,
    pub name: String,
}

/// Scaling mode selector.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum ScaleMode {
    #[default]
    None,
    Multiplier(f64),
    TargetIngredient {
        ingredient_index: usize,
        target_amount: f64,
    },
}

/// A scaled ingredient result.
#[derive(Clone, Debug)]
pub struct ScaledIngredient {
    pub amount: String, // formatted fraction string (may be empty)
    pub unit: String,
    pub name: String,
    #[allow(dead_code)]
    pub scaled: bool, // true if this ingredient was scaled (used in tests)
}

/// Recipe scaling calculator.
///
/// Accepts a generic ingredient type that provides amount, unit, and name
/// as string references. This avoids coupling to any specific struct
/// (e.g., ParsedIngredient) and keeps the module self-contained.
///
/// NOTE: Must derive Clone — Dioxus 0.7's `use_signal(|| Option::<T>::None)`
/// requires `T: Clone + 'static` because `impl<T: Clone> Clone for Option<T>`.
#[derive(Clone)]
pub struct ScaleCalculator {
    original_ingredients: Vec<IngredientRef>,
    original_servings: Option<i32>,
    original_prep_time: Option<i32>,
    original_cook_time: Option<i32>,
    mode: ScaleMode,
}

// ── Parsing ──────────────────────────────────────────────────────────────────

/// Parse a cooking amount string into a numeric value.
///
/// Supports: integers ("2"), decimals ("2.5"), fractions ("1/2"),
/// mixed numbers ("1 1/2"). Returns None for non-numeric strings
/// like "pinch", "to taste", or empty strings.
pub fn parse_amount(s: &str) -> Option<f64> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }

    let tokens: Vec<&str> = trimmed.split_whitespace().collect();
    let mut sum = 0.0_f64;
    let mut parsed_any = false;
    let mut last_number: Option<f64> = None;
    let mut i = 0;

    while i < tokens.len() {
        let token = tokens[i];

        // Try parsing as f64 (handles integers and decimals)
        if let Ok(v) = token.parse::<f64>() {
            sum += v;
            parsed_any = true;
            last_number = Some(v);
            i += 1;
            continue;
        }

        // Handle standalone "/" as separator: "1 / 2"
        // Must check BEFORE the contains('/') branch which would catch "/" as a fraction.
        if token == "/" && i + 1 < tokens.len() {
            if let Some(numerator) = last_number {
                if let Ok(den) = tokens[i + 1].parse::<f64>() {
                    if den != 0.0 {
                        // Subtract the numerator (already added as whole number)
                        // and add the actual fraction
                        sum -= numerator;
                        sum += numerator / den;
                        parsed_any = true;
                        last_number = None;
                        i += 2;
                        continue;
                    }
                }
            }
            return None;
        }

        // If token contains '/', try fraction (e.g., "1/2")
        if token.contains('/') {
            let parts: Vec<&str> = token.split('/').collect();
            if parts.len() == 2 {
                if let (Ok(num), Ok(den)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
                    if den != 0.0 {
                        sum += num / den;
                        parsed_any = true;
                        last_number = None;
                        i += 1;
                        continue;
                    }
                }
            }
            return None;
        }

        // Token has no '/' and did not parse as f64 → non-numeric
        return None;
    }

    if parsed_any {
        Some(sum)
    } else {
        None
    }
}

// ── Formatting ───────────────────────────────────────────────────────────────

/// Format a numeric value as a decimal string truncated to 2 decimal places.
/// Zero or near-zero returns empty string. Negative values use absolute value.
pub fn format_amount(value: f64) -> String {
    let value = value.abs();
    if value <= 0.0 {
        return String::new();
    }

    // Truncate to 2 decimal places
    let truncated = (value * 100.0).floor() / 100.0;
    if truncated <= 0.0 {
        return String::new();
    }

    // Format with up to 2 decimal places, stripping trailing zeros
    let s = format!("{:.2}", truncated);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    s.to_string()
}

// ── ScaleCalculator ──────────────────────────────────────────────────────────

impl ScaleCalculator {
    pub fn new(
        ingredients: Vec<IngredientRef>,
        servings: Option<i32>,
        prep_time: Option<i32>,
        cook_time: Option<i32>,
    ) -> Self {
        Self {
            original_ingredients: ingredients,
            original_servings: servings,
            original_prep_time: prep_time,
            original_cook_time: cook_time,
            mode: ScaleMode::None,
        }
    }

    /// Set scaling by a numeric multiplier. Validates m > 0.0.
    pub fn set_multiplier(&mut self, m: f64) {
        if m > 0.0 {
            self.mode = ScaleMode::Multiplier(m);
        }
    }

    /// Set scaling by targeting a specific ingredient's desired amount.
    ///
    /// Validates ingredient_index < len and target_amount > 0.0.
    /// If the original amount for that ingredient cannot be parsed, does nothing.
    pub fn set_target_ingredient(&mut self, ingredient_index: usize, target_amount: f64) {
        if target_amount <= 0.0 || ingredient_index >= self.original_ingredients.len() {
            return;
        }

        let original_amount = parse_amount(&self.original_ingredients[ingredient_index].amount);
        if let Some(orig) = original_amount {
            if orig > 0.0 {
                self.mode = ScaleMode::TargetIngredient {
                    ingredient_index,
                    target_amount,
                };
            }
        }
    }

    /// Return the effective multiplier for the current mode.
    /// Returns 1.0 for None mode.
    pub fn multiplier(&self) -> f64 {
        match &self.mode {
            ScaleMode::None => 1.0,
            ScaleMode::Multiplier(m) => *m,
            ScaleMode::TargetIngredient {
                ingredient_index,
                target_amount,
            } => {
                if let Some(orig) =
                    parse_amount(&self.original_ingredients[*ingredient_index].amount)
                {
                    if orig > 0.0 {
                        return *target_amount / orig;
                    }
                }
                1.0
            }
        }
    }

    /// Return ingredients with scaled amounts.
    ///
    /// If mode is None, returns originals with formatted amounts and scaled=false.
    pub fn scaled_ingredients(&self) -> Vec<ScaledIngredient> {
        let m = self.multiplier();
        let is_scaling = m != 1.0;

        self.original_ingredients
            .iter()
            .map(|ing| {
                if let Some(orig_val) = parse_amount(&ing.amount) {
                    let scaled_val = orig_val * m;
                    ScaledIngredient {
                        amount: format_amount(scaled_val),
                        unit: ing.unit.clone(),
                        name: ing.name.clone(),
                        scaled: is_scaling,
                    }
                } else {
                    // Non-numeric amount — keep original display
                    ScaledIngredient {
                        amount: ing.amount.clone(),
                        unit: ing.unit.clone(),
                        name: ing.name.clone(),
                        scaled: false,
                    }
                }
            })
            .collect()
    }

    /// Return scaled servings (rounded), or None if original is None.
    pub fn scaled_servings(&self) -> Option<i32> {
        self.original_servings
            .map(|s| (s as f64 * self.multiplier()).round() as i32)
    }

    /// Return original prep time (does not scale with recipe size).
    pub fn scaled_prep_time(&self) -> Option<i32> {
        self.original_prep_time
    }

    /// Return original cook time (does not scale with recipe size).
    pub fn scaled_cook_time(&self) -> Option<i32> {
        self.original_cook_time
    }

    /// Reset scaling back to None mode.
    pub fn reset(&mut self) {
        self.mode = ScaleMode::None;
    }

    /// Return reference to current mode.
    pub fn mode(&self) -> &ScaleMode {
        &self.mode
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_amount tests ─────────────────────────────────────────────

    #[test]
    fn parse_integer() {
        assert_eq!(parse_amount("2"), Some(2.0));
        assert_eq!(parse_amount("100"), Some(100.0));
        assert_eq!(parse_amount("0"), Some(0.0));
    }

    #[test]
    fn parse_decimal() {
        assert_eq!(parse_amount("2.5"), Some(2.5));
        assert_eq!(parse_amount("0.5"), Some(0.5));
    }

    #[test]
    fn parse_fraction() {
        assert_eq!(parse_amount("1/2"), Some(0.5));
        assert_eq!(parse_amount("3/4"), Some(0.75));
        assert!((parse_amount("1/3").unwrap() - 0.3333).abs() < 0.0001);
        assert_eq!(parse_amount("0/4"), Some(0.0));
    }

    #[test]
    fn parse_mixed_number() {
        assert_eq!(parse_amount("1 1/2"), Some(1.5));
        assert_eq!(parse_amount("2 3/4"), Some(2.75));
    }

    #[test]
    fn parse_whitespace_trimming() {
        assert_eq!(parse_amount(" 1 / 2 "), Some(0.5));
    }

    #[test]
    fn parse_non_numeric_returns_none() {
        assert_eq!(parse_amount("pinch"), None);
        assert_eq!(parse_amount("to taste"), None);
        assert_eq!(parse_amount(""), None);
    }

    // ── format_amount tests ────────────────────────────────────────────

    #[test]
    fn format_whole_numbers() {
        assert_eq!(format_amount(2.0), "2");
        assert_eq!(format_amount(3.0), "3");
    }

    #[test]
    fn format_decimals() {
        assert_eq!(format_amount(0.5), "0.5");
        assert_eq!(format_amount(0.25), "0.25");
        assert_eq!(format_amount(0.333), "0.33");
        assert_eq!(format_amount(0.667), "0.66");
    }

    #[test]
    fn format_mixed_numbers() {
        assert_eq!(format_amount(1.5), "1.5");
        assert_eq!(format_amount(2.25), "2.25");
    }

    #[test]
    fn format_truncation() {
        assert_eq!(format_amount(0.125), "0.12");
        assert_eq!(format_amount(0.375), "0.37");
        assert_eq!(format_amount(0.625), "0.62");
        assert_eq!(format_amount(0.875), "0.87");
    }

    #[test]
    fn format_zero_and_negative() {
        assert_eq!(format_amount(0.0), "");
        assert_eq!(format_amount(-1.0), "1");
    }

    #[test]
    fn format_truncation_not_rounding() {
        assert_eq!(format_amount(3.339), "3.33");
        assert_eq!(format_amount(1.875), "1.87");
    }

    #[test]
    fn format_small_values() {
        assert_eq!(format_amount(0.009), ""); // truncates to 0.00
        assert_eq!(format_amount(0.015), "0.01");
    }

    // ── ScaleCalculator tests ──────────────────────────────────────────

    fn test_ingredients() -> Vec<IngredientRef> {
        vec![
            IngredientRef {
                amount: "2".to_string(),
                unit: "cups".to_string(),
                name: "flour".to_string(),
            },
            IngredientRef {
                amount: "1/2".to_string(),
                unit: "tsp".to_string(),
                name: "salt".to_string(),
            },
            IngredientRef {
                amount: "pinch".to_string(),
                unit: "".to_string(),
                name: "nutmeg".to_string(),
            },
        ]
    }

    #[test]
    fn calculator_initializes_with_none_mode() {
        let calc = ScaleCalculator::new(test_ingredients(), Some(4), Some(15), Some(30));
        assert!(*calc.mode() == ScaleMode::None);
        assert_eq!(calc.multiplier(), 1.0);
    }

    #[test]
    fn multiplier_mode_doubles() {
        let mut calc = ScaleCalculator::new(test_ingredients(), Some(4), Some(15), Some(30));
        calc.set_multiplier(2.0);
        let scaled = calc.scaled_ingredients();

        assert_eq!(scaled[0].amount, "4");
        assert_eq!(scaled[0].scaled, true);
        assert_eq!(scaled[1].amount, "1");
        assert_eq!(scaled[1].scaled, true);
        // "pinch" stays unscaled
        assert_eq!(scaled[2].amount, "pinch");
        assert_eq!(scaled[2].scaled, false);

        assert_eq!(calc.scaled_servings(), Some(8));
        // Prep/cook times do not scale
        assert_eq!(calc.scaled_prep_time(), Some(15));
        assert_eq!(calc.scaled_cook_time(), Some(30));
    }

    #[test]
    fn multiplier_mode_halves() {
        let mut calc = ScaleCalculator::new(test_ingredients(), Some(4), Some(15), Some(30));
        calc.set_multiplier(0.5);
        let scaled = calc.scaled_ingredients();

        assert_eq!(scaled[0].amount, "1");
        assert_eq!(scaled[1].amount, "0.25");
        assert_eq!(calc.scaled_servings(), Some(2));
    }

    #[test]
    fn target_ingredient_mode_scales_proportionally() {
        let mut calc = ScaleCalculator::new(test_ingredients(), Some(4), Some(15), Some(30));
        // Original flour is "2", target is "4" → multiplier = 2.0
        calc.set_target_ingredient(0, 4.0);
        let scaled = calc.scaled_ingredients();

        assert_eq!(scaled[0].amount, "4");
        assert_eq!(scaled[1].amount, "1");
        assert_eq!(calc.multiplier(), 2.0);
    }

    #[test]
    fn target_ingredient_fractional_scaling() {
        let mut calc = ScaleCalculator::new(test_ingredients(), Some(4), Some(15), Some(30));
        // Original flour is "2", target is "1" → multiplier = 0.5
        calc.set_target_ingredient(0, 1.0);
        assert_eq!(calc.multiplier(), 0.5);
        assert_eq!(calc.scaled_servings(), Some(2));
    }

    #[test]
    fn target_ingredient_unscaleable_does_nothing() {
        let mut calc = ScaleCalculator::new(test_ingredients(), Some(4), None, None);
        // "pinch" cannot be parsed — should not change mode
        calc.set_target_ingredient(2, 10.0);
        assert!(*calc.mode() == ScaleMode::None);
        assert_eq!(calc.multiplier(), 1.0);
    }

    #[test]
    fn reset_clears_mode() {
        let mut calc = ScaleCalculator::new(test_ingredients(), Some(4), None, None);
        calc.set_multiplier(3.0);
        assert!(*calc.mode() == ScaleMode::Multiplier(3.0));
        calc.reset();
        assert!(*calc.mode() == ScaleMode::None);
        assert_eq!(calc.multiplier(), 1.0);
    }

    #[test]
    fn time_and_servings_round_correctly() {
        let mut calc = ScaleCalculator::new(test_ingredients(), Some(3), Some(10), Some(20));
        calc.set_multiplier(1.5);
        assert_eq!(calc.scaled_servings(), Some(5)); // 3 * 1.5 = 4.5 → 5 (round)
                                                     // Prep/cook times do not scale
        assert_eq!(calc.scaled_prep_time(), Some(10));
        assert_eq!(calc.scaled_cook_time(), Some(20));
    }

    #[test]
    fn none_optionals_return_none() {
        let calc = ScaleCalculator::new(test_ingredients(), None, None, None);
        assert_eq!(calc.scaled_servings(), None);
        assert_eq!(calc.scaled_prep_time(), None);
        assert_eq!(calc.scaled_cook_time(), None);
    }

    #[test]
    fn invalid_multiplier_ignored() {
        let mut calc = ScaleCalculator::new(test_ingredients(), None, None, None);
        calc.set_multiplier(0.0);
        assert!(*calc.mode() == ScaleMode::None);
        calc.set_multiplier(-1.0);
        assert!(*calc.mode() == ScaleMode::None);
    }

    #[test]
    fn invalid_target_ingredient_ignored() {
        let mut calc = ScaleCalculator::new(test_ingredients(), None, None, None);
        calc.set_target_ingredient(99, 1.0); // out of bounds
        assert!(*calc.mode() == ScaleMode::None);
        calc.set_target_ingredient(0, 0.0); // zero target
        assert!(*calc.mode() == ScaleMode::None);
    }
}
