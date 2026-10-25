//! Shared pantry inventory contracts. Authored names and units are never normalized.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[cfg(feature = "server")]
pub const MAX_PROFILES: usize = 100;
pub const MAX_INGREDIENTS: usize = 1_000;
pub const MAX_EQUIPMENT: usize = 200;
pub const MAX_PAYLOAD: usize = 2 * 1024 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PantryKind {
    Ingredient,
    Equipment,
}
impl PantryKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ingredient => "Ingredients",
            Self::Equipment => "Equipment",
        }
    }
    pub fn limit(self) -> usize {
        match self {
            Self::Ingredient => MAX_INGREDIENTS,
            Self::Equipment => MAX_EQUIPMENT,
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PantryFields {
    pub name: String,
    pub amount: String,
    pub unit: String,
}
impl PantryFields {
    pub fn validate(&self, kind: PantryKind) -> Result<(), String> {
        text(&self.name, 500, true, "Ingredient or equipment name")?;
        text(&self.amount, 64, false, "Amount")?;
        text(&self.unit, 64, false, "Unit")?;
        if kind == PantryKind::Equipment && (!self.amount.is_empty() || !self.unit.is_empty()) {
            return Err("Equipment tracks availability without amounts or units.".into());
        }
        quantity_zero(&self.amount)?;
        Ok(())
    }
}
fn text(value: &str, max: usize, required: bool, label: &str) -> Result<(), String> {
    if value.contains('\0') || value.chars().count() > max || (required && value.trim().is_empty())
    {
        return Err(format!(
            "{label} must {}contain at most {max} characters.",
            if required { "be nonblank and " } else { "" }
        ));
    }
    Ok(())
}
pub fn validate_profile(name: &str, note: &str) -> Result<(), String> {
    text(name, 100, true, "Kitchen name")?;
    text(note, 500, false, "Kitchen note")
}
/// None means descriptive/unknown; numeric-looking invalid values are rejected,
/// rather than disguised as presence-only stock. Parsing shares grocery bounds.
pub fn quantity_zero(amount: &str) -> Result<Option<bool>, String> {
    let value = amount.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if let Some(zero) = crate::grocery::quantity_is_zero(value) {
        return Ok(Some(zero));
    }
    let lower = value.to_ascii_lowercase();
    let numeric = value
        .chars()
        .all(|c| c.is_ascii_digit() || c.is_ascii_whitespace() || "+-./eE".contains(c));
    let negative = value.starts_with(['-', '−'])
        && value
            .trim_start_matches(['-', '−'])
            .trim_start()
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_digit() || c == '.');
    if numeric
        || negative
        || matches!(
            lower.trim_start_matches(['+', '-', '−']).trim_start(),
            "nan" | "inf" | "infinity" | "∞"
        )
    {
        return Err("Enter a nonnegative amount within the supported range, a fraction, descriptive text, or leave it blank.".into());
    }
    Ok(None)
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PantryStock {
    pub fields: PantryFields,
    pub available: bool,
}
impl Default for PantryStock {
    fn default() -> Self {
        Self {
            fields: PantryFields::default(),
            available: true,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PantryEntry {
    pub id: Uuid,
    pub kind: PantryKind,
    pub stock: PantryStock,
    pub quantity_zero: bool,
    pub position: i32,
    pub revision: i64,
}
impl PantryEntry {
    pub fn usable(&self) -> bool {
        self.stock.available && !self.quantity_zero
    }
    pub fn amount_label(&self) -> String {
        if self.kind == PantryKind::Equipment {
            return String::new();
        }
        if self.stock.fields.amount.trim().is_empty() {
            "Quantity unknown".into()
        } else {
            format!("{} {}", self.stock.fields.amount, self.stock.fields.unit)
                .trim_end()
                .into()
        }
    }
    pub fn status(&self) -> &'static str {
        if !self.stock.available {
            if self.kind == PantryKind::Equipment {
                "Unavailable"
            } else {
                "Out"
            }
        } else if self.quantity_zero {
            "No measured stock"
        } else if self.kind == PantryKind::Equipment {
            "Available"
        } else if quantity_zero(&self.stock.fields.amount)
            .ok()
            .flatten()
            .is_none()
        {
            "Present · Quantity unknown"
        } else {
            "In stock"
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PantryKitchen {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
    pub note: String,
    pub entries: Vec<PantryEntry>,
    pub revision: i64,
    pub created_at: DateTime<Utc>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PantrySummary {
    pub id: Uuid,
    pub name: String,
    pub ingredient_count: i64,
    pub equipment_count: i64,
    pub in_stock_count: i64,
    pub available_equipment_count: i64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PantryCopy {
    pub id: Uuid,
    pub expected_revision: i64,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PantryCreate {
    pub request_id: Uuid,
    pub name: String,
    pub note: String,
    pub ingredients: Vec<PantryStock>,
    pub equipment: Vec<PantryStock>,
    pub copy: Option<PantryCopy>,
}
impl PantryCreate {
    pub fn validate(&self) -> Result<(), String> {
        validate_profile(&self.name, &self.note)?;
        if self.ingredients.len() > MAX_INGREDIENTS || self.equipment.len() > MAX_EQUIPMENT {
            return Err("A kitchen supports up to 1,000 ingredients and 200 tools.".into());
        }
        if self.copy.is_some() && (!self.ingredients.is_empty() || !self.equipment.is_empty()) {
            return Err("A kitchen copy cannot also submit starting inventory.".into());
        }
        for x in &self.ingredients {
            x.fields.validate(PantryKind::Ingredient)?
        }
        for x in &self.equipment {
            x.fields.validate(PantryKind::Equipment)?
        }
        if serde_json::to_vec(self)
            .map_err(|_| "Invalid kitchen data.")?
            .len()
            > MAX_PAYLOAD
        {
            return Err("This kitchen is too large.".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PantryChange {
    Fields(PantryFields),
    Availability(bool),
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quantity_keeps_unknown_zero_and_units_distinct() {
        for s in ["0", "0.000", "0/2", "0 / 3"] {
            assert_eq!(quantity_zero(s), Ok(Some(true)));
        }
        for s in ["2", "1/2", "1 1/2", "0.000000001"] {
            assert_eq!(quantity_zero(s), Ok(Some(false)));
        }
        for s in ["", "a little", "about 2"] {
            assert_eq!(quantity_zero(s), Ok(None));
        }
    }
    #[test]
    fn invalid_numbers_are_not_unknown_stock() {
        for s in [
            "-1",
            "-1/2",
            "NaN",
            "+NaN",
            "-NaN",
            "− 1",
            "Infinity",
            "1e999",
            "1/0",
            "99999999999999999999999",
            "0.0000000001",
        ] {
            assert!(quantity_zero(s).is_err(), "{s}");
        }
    }
    #[test]
    fn validation_keeps_authored_fields_and_rejects_invalid_equipment() {
        let f = PantryFields {
            name: " Rice ".into(),
            amount: "2".into(),
            unit: " cup ".into(),
        };
        assert!(f.validate(PantryKind::Ingredient).is_ok());
        assert_eq!(f.name, " Rice ");
        assert!(f.validate(PantryKind::Equipment).is_err());
        assert!(validate_profile(" ", "").is_err());
        assert!(validate_profile("Cabin", "note\0").is_err());
    }
}
