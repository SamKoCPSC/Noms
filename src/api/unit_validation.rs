//! Unit policy for incoming authoring payloads, before any database write.
use crate::types::RecipeIngredient;
use dioxus::prelude::ServerFnError;

pub(super) fn unit(value: &mut String) -> Result<(), ServerFnError> {
    *value = noms::units::canonicalize(value)
        .map_err(ServerFnError::new)?
        .into();
    Ok(())
}

pub(super) fn ingredients(rows: &mut [RecipeIngredient]) -> Result<(), ServerFnError> {
    for (index, row) in rows.iter_mut().enumerate() {
        unit(&mut row.unit).map_err(|_| {
            ServerFnError::new(format!(
                "Ingredient {}: {}",
                index + 1,
                noms::units::INVALID_UNIT
            ))
        })?;
    }
    Ok(())
}

/// A stored legacy draft must be saved through the authoring boundary before
/// publication. Do not silently modify an immutable snapshot or old recipe.
pub(super) fn saved_ingredients(rows: &[RecipeIngredient]) -> Result<(), ServerFnError> {
    for row in rows {
        let canonical = noms::units::canonicalize(&row.unit).map_err(ServerFnError::new)?;
        if row.unit != canonical {
            return Err(ServerFnError::new(
                "Save this draft with standardized units before publishing.",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_only_units_and_preserves_authored_identity_and_amount() {
        let id = uuid::Uuid::new_v4();
        let mut rows = vec![RecipeIngredient {
            vocabulary_id: Some(id),
            name: "Flour".into(),
            amount: "1/2".into(),
            unit: "grammes".into(),
        }];
        ingredients(&mut rows).unwrap();
        assert_eq!(rows[0].unit, "g");
        assert_eq!(rows[0].amount, "1/2");
        assert_eq!(rows[0].name, "Flour");
        assert_eq!(rows[0].vocabulary_id, Some(id));
        saved_ingredients(&rows).unwrap();
        rows[0].unit = "cups".into();
        assert!(saved_ingredients(&rows).is_err());
        ingredients(&mut rows).unwrap();
        assert_eq!(rows[0].unit, "US cup");
        rows[0].unit = "pinch".into();
        assert!(ingredients(&mut rows)
            .unwrap_err()
            .to_string()
            .contains("Ingredient 1"));
        for blank in ["", "   ", "\t\n"] {
            rows[0].unit = blank.into();
            assert!(ingredients(&mut rows).is_err());
            assert!(saved_ingredients(&rows).is_err());
        }
    }
}
