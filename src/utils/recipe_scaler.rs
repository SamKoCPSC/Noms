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
    TargetIngredient { ingredient_index: usize, target_amount: f64 },
}

/// A scaled ingredient result.
#[derive(Clone, Debug)]
pub struct ScaledIngredient {
    pub amount: String,       // formatted fraction string (may be empty)
    pub unit: String,
    pub name: String,
    #[allow(dead_code)]
    pub scaled: bool,         // true if this ingredient was scaled (used in tests)
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

/// Format a numeric value as a cooking-friendly fraction string.
///
/// Rounds to nearest 1/8 precision. Uses common fractions:
/// 1/8, 3/8, 1/4, 1/3, 1/2, 5/8, 2/3, 3/4, 7/8.
/// Whole numbers render without fraction ("2" not "2 0/1").
/// Zero or near-zero returns empty string. Negative values use absolute value.
pub fn format_amount(value: f64) -> String {
    let value = value.abs();
    if value <= 0.0 {
        return String::new();
    }

    // Check original value's fractional part for 1/3 and 2/3 detection
    // before rounding, since 1/3 and 2/3 don't align with 1/8 rounding.
    let orig_frac = value - (value as i64) as f64;
    let is_third = (orig_frac - 1.0_f64 / 3.0).abs() < 0.04;
    let is_two_thirds = (orig_frac - 2.0_f64 / 3.0).abs() < 0.04;

    // Round to nearest 1/8
    let rounded = (value * 8.0).round() / 8.0;
    if rounded <= 0.0 {
        return String::new();
    }

    let whole = rounded as i64;
    let fractional = rounded - whole as f64;

    // Below 1/16 threshold → whole number
    if fractional < 0.015625 {
        return whole.to_string();
    }

    // Match fractional to nearest common fraction.
    // 1/8-based fractions are checked first (exact after rounding),
    // then 1/3 and 2/3 as special cases for values that landed near those boundaries.
    let fraction_str = match fractional {
        f if (f - 0.125).abs() < 0.03 => "1/8",
        f if (f - 0.25).abs() < 0.03 => "1/4",
        f if (f - 0.375).abs() < 0.03 => {
            if is_third { "1/3" } else { "3/8" }
        }
        f if (f - 0.5).abs() < 0.03 => "1/2",
        f if (f - 0.625).abs() < 0.03 => {
            if is_two_thirds { "2/3" } else { "5/8" }
        }
        f if (f - 0.75).abs() < 0.03 => "3/4",
        f if (f - 0.875).abs() < 0.03 => "7/8",
        _ => {
            // Fallback: format as decimal, strip trailing zeros
            let s = format!("{:.2}", fractional);
            let s = s.trim_end_matches('0').trim_end_matches('.');
            return if whole > 0 {
                format!("{} {}", whole, s)
            } else {
                s.to_string()
            };
        }
    };

    if whole > 0 {
        format!("{} {}", whole, fraction_str)
    } else {
        fraction_str.to_string()
    }
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
        if target_amount <= 0.0
            || ingredient_index >= self.original_ingredients.len()
        {
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
        self.original_servings.map(|s| (s as f64 * self.multiplier()).round() as i32)
    }

    /// Return scaled prep time (rounded), or None if original is None.
    pub fn scaled_prep_time(&self) -> Option<i32> {
        self.original_prep_time.map(|t| (t as f64 * self.multiplier()).round() as i32)
    }

    /// Return scaled cook time (rounded), or None if original is None.
    pub fn scaled_cook_time(&self) -> Option<i32> {
        self.original_cook_time.map(|t| (t as f64 * self.multiplier()).round() as i32)
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
    fn format_common_fractions() {
        assert_eq!(format_amount(0.5), "1/2");
        assert_eq!(format_amount(0.25), "1/4");
        assert_eq!(format_amount(0.333), "1/3");
        assert_eq!(format_amount(0.667), "2/3");
    }

    #[test]
    fn format_mixed_numbers() {
        assert_eq!(format_amount(1.5), "1 1/2");
        assert_eq!(format_amount(2.25), "2 1/4");
    }

    #[test]
    fn format_eighths() {
        assert_eq!(format_amount(0.125), "1/8");
        assert_eq!(format_amount(0.375), "3/8");
        assert_eq!(format_amount(0.625), "5/8");
        assert_eq!(format_amount(0.875), "7/8");
    }

    #[test]
    fn format_zero_and_negative() {
        assert_eq!(format_amount(0.0), "");
        assert_eq!(format_amount(-1.0), "1");
    }

    #[test]
    fn format_rounding() {
        assert_eq!(format_amount(3.33), "3 1/3");
        assert_eq!(format_amount(1.875), "1 7/8");
    }

    #[test]
    fn format_three_thirds() {
        assert_eq!(format_amount(3.333), "3 1/3");
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
        assert_eq!(calc.scaled_prep_time(), Some(30));
        assert_eq!(calc.scaled_cook_time(), Some(60));
    }

    #[test]
    fn multiplier_mode_halves() {
        let mut calc = ScaleCalculator::new(test_ingredients(), Some(4), Some(15), Some(30));
        calc.set_multiplier(0.5);
        let scaled = calc.scaled_ingredients();

        assert_eq!(scaled[0].amount, "1");
        assert_eq!(scaled[1].amount, "1/4");
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
        assert_eq!(calc.scaled_prep_time(), Some(15)); // 10 * 1.5 = 15
        assert_eq!(calc.scaled_cook_time(), Some(30)); // 20 * 1.5 = 30
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
