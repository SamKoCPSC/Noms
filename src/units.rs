//! Optional, local suggestions for precise cooking measures. No conversions or parsing.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unit {
    /// Text saved only when the author explicitly selects this suggestion.
    pub label: &'static str,
    pub name: &'static str,
    aliases: &'static [&'static str],
}

pub const MAX_SUGGESTIONS: usize = 8;

// Unqualified regional aliases deliberately return multiple choices. Never infer
// a cup/spoon/fluid-ounce standard from an abbreviation or the author's locale.
const UNITS: &[Unit] = &[
    Unit {
        label: "g",
        name: "gram",
        aliases: &["grams", "gramme", "grammes"],
    },
    Unit {
        label: "kg",
        name: "kilogram",
        aliases: &["kilograms", "kilogramme", "kilogrammes", "kilo", "kilos"],
    },
    Unit {
        label: "mL",
        name: "millilitre",
        aliases: &["millilitres", "milliliter", "milliliters", "mls"],
    },
    Unit {
        label: "L",
        name: "litre",
        aliases: &["litres", "liter", "liters"],
    },
    Unit {
        label: "US tsp",
        name: "US teaspoon",
        aliases: &["tsp", "tsps", "teaspoon", "teaspoons", "us teaspoons"],
    },
    Unit {
        label: "US tbsp",
        name: "US tablespoon",
        aliases: &[
            "tbsp",
            "tbsps",
            "tbs",
            "tablespoon",
            "tablespoons",
            "us tablespoons",
        ],
    },
    Unit {
        label: "US cup",
        name: "US customary cup",
        aliases: &["cup", "cups", "us cups", "us cup"],
    },
    Unit {
        label: "oz",
        name: "ounce (weight)",
        aliases: &["ounce", "ounces", "ozs", "avoirdupois ounce"],
    },
    Unit {
        label: "lb",
        name: "pound",
        aliases: &["pounds", "lbs"],
    },
    Unit {
        label: "mg",
        name: "milligram",
        aliases: &["milligrams", "milligramme", "milligrammes"],
    },
    Unit {
        label: "cL",
        name: "centilitre",
        aliases: &["centilitres", "centiliter", "centiliters"],
    },
    Unit {
        label: "dL",
        name: "decilitre",
        aliases: &["decilitres", "deciliter", "deciliters"],
    },
    Unit {
        label: "US fl oz",
        name: "US fluid ounce",
        aliases: &[
            "fl oz",
            "floz",
            "fluid ounce",
            "fluid ounces",
            "us fluid ounces",
        ],
    },
    Unit {
        label: "US pt",
        name: "US liquid pint",
        aliases: &[
            "pt",
            "pint",
            "pints",
            "us pint",
            "us pints",
            "us liquid pints",
        ],
    },
    Unit {
        label: "US qt",
        name: "US liquid quart",
        aliases: &[
            "qt",
            "quart",
            "quarts",
            "us quart",
            "us quarts",
            "us liquid quarts",
        ],
    },
    Unit {
        label: "US gal",
        name: "US liquid gallon",
        aliases: &[
            "gal",
            "gallon",
            "gallons",
            "us gallon",
            "us gallons",
            "us liquid gallons",
        ],
    },
    Unit {
        label: "metric tsp",
        name: "metric teaspoon (5 mL)",
        aliases: &[
            "tsp",
            "tsps",
            "teaspoon",
            "teaspoons",
            "metric teaspoon",
            "metric teaspoons",
        ],
    },
    Unit {
        label: "metric tbsp",
        name: "metric tablespoon (15 mL)",
        aliases: &[
            "tbsp",
            "tbsps",
            "tbs",
            "tablespoon",
            "tablespoons",
            "metric tablespoon",
            "metric tablespoons",
        ],
    },
    Unit {
        label: "metric cup",
        name: "metric cup (250 mL)",
        aliases: &["cup", "cups", "metric cups"],
    },
    Unit {
        label: "AU tbsp",
        name: "Australian tablespoon (20 mL)",
        aliases: &[
            "tbsp",
            "tbsps",
            "tablespoon",
            "tablespoons",
            "australian tablespoon",
            "australian tablespoons",
            "au tablespoon",
            "au tablespoons",
        ],
    },
    Unit {
        label: "Imperial fl oz",
        name: "Imperial fluid ounce",
        aliases: &[
            "fl oz",
            "floz",
            "fluid ounce",
            "fluid ounces",
            "imperial fluid ounces",
            "uk fl oz",
            "uk fluid ounce",
            "uk fluid ounces",
        ],
    },
    Unit {
        label: "Imperial pt",
        name: "Imperial pint",
        aliases: &[
            "pt",
            "pint",
            "pints",
            "imperial pints",
            "uk pint",
            "uk pints",
        ],
    },
    Unit {
        label: "Imperial qt",
        name: "Imperial quart",
        aliases: &[
            "qt",
            "quart",
            "quarts",
            "imperial quarts",
            "uk quart",
            "uk quarts",
        ],
    },
    Unit {
        label: "Imperial gal",
        name: "Imperial gallon",
        aliases: &[
            "gal",
            "gallon",
            "gallons",
            "imperial gallons",
            "uk gallon",
            "uk gallons",
        ],
    },
];

fn normalize(text: &str) -> String {
    text.to_lowercase()
        .replace('.', "")
        .replace(['(', ')', '-'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Exact aliases rank first, then prefixes; multiword queries allow word order
/// changes. The author must choose a result; this never rewrites typed text.
pub fn suggestions(query: &str) -> Vec<&'static Unit> {
    if query.chars().count() > 64 {
        return Vec::new();
    }
    let query = normalize(query);
    if query.is_empty() {
        return UNITS.iter().take(MAX_SUGGESTIONS).collect();
    }
    let words: Vec<_> = query.split_whitespace().collect();
    let mut matches = Vec::new();
    for (position, unit) in UNITS.iter().enumerate() {
        let terms: Vec<_> = [unit.label, unit.name]
            .into_iter()
            .chain(unit.aliases.iter().copied())
            .map(normalize)
            .collect();
        let rank = if terms.iter().any(|term| term == &query) {
            0
        } else if terms.iter().any(|term| term.starts_with(&query)) {
            1
        } else if words.len() > 1
            && words.iter().all(|word| {
                terms
                    .iter()
                    .any(|term| term.split_whitespace().any(|part| part.starts_with(word)))
            })
        {
            2
        } else {
            continue;
        };
        matches.push((rank, position, unit));
    }
    matches.sort_unstable_by_key(|(rank, position, _)| (*rank, *position));
    matches
        .into_iter()
        .take(MAX_SUGGESTIONS)
        .map(|(_, _, unit)| unit)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(query: &str) -> Vec<&'static str> {
        suggestions(query).into_iter().map(|u| u.label).collect()
    }

    #[test]
    fn abbreviations_full_names_plurals_and_punctuation_find_the_same_measure() {
        for query in ["g", "GRAMS", " grammes "] {
            assert_eq!(labels(query)[0], "g");
        }
        for query in ["ml", "mL", "milliliters", "millilitres"] {
            assert_eq!(labels(query)[0], "mL");
        }
        assert_eq!(labels("lbs.")[0], "lb");
        assert_eq!(labels("fl. oz."), vec!["US fl oz", "Imperial fl oz"]);
        assert_eq!(labels("millig"), vec!["mg"]);
    }

    #[test]
    fn regional_standards_and_weight_fluid_ounces_stay_distinct() {
        assert_eq!(labels("cups"), vec!["US cup", "metric cup"]);
        assert_eq!(
            labels("tablespoons"),
            vec!["US tbsp", "metric tbsp", "AU tbsp"]
        );
        assert_eq!(labels("metric cups"), vec!["metric cup"]);
        assert_eq!(labels("Australian tablespoons"), vec!["AU tbsp"]);
        assert_eq!(labels("pints UK"), vec!["Imperial pt"]);
        assert_eq!(labels("oz"), vec!["oz"]);
        assert_eq!(labels("ounces"), vec!["oz"]);
        assert_eq!(labels("US fluid ounces"), vec!["US fl oz"]);
    }

    #[test]
    fn approximate_count_container_and_custom_units_are_not_suggested() {
        for query in [
            "pinch",
            "dash",
            "handful",
            "piece",
            "clove",
            "sprig",
            "bunch",
            "can",
            "packet",
            "heaped tsp",
            "to taste",
            "my custom unit",
        ] {
            assert!(suggestions(query).is_empty(), "{query}");
        }
        assert!(suggestions(&"g".repeat(65)).is_empty());
        assert_eq!(suggestions("").len(), MAX_SUGGESTIONS);
    }

    #[test]
    fn every_saved_label_is_unique_bounded_and_finds_itself_first() {
        let mut seen = std::collections::HashSet::new();
        for unit in UNITS {
            assert!(seen.insert(unit.label));
            assert!(unit.label.len() <= 64);
            assert_eq!(suggestions(unit.label)[0], unit);
            assert!(suggestions(unit.name).contains(&unit));
        }
        for query in ["m", "t", "US", "", "Imperial"] {
            assert!(suggestions(query).len() <= MAX_SUGGESTIONS);
        }
    }
}
