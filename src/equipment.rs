//! Small maintained equipment catalog, searched locally without a database lookup.
use crate::vocabulary::{normalize, MAX_TEXT, SUGGESTION_LIMIT};
use serde::Deserialize;
use std::{collections::BTreeMap, sync::OnceLock};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalog {
    schema_version: u32,
    dimension: String,
    entries: Vec<Entry>,
    ambiguous_search_terms: BTreeMap<String, Vec<String>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    label: String,
    aliases: Vec<String>,
    scope: String,
}

struct PreparedEntry {
    label: String,
    key: String,
    names: Vec<String>,
}

fn catalog() -> &'static [PreparedEntry] {
    static INDEX: OnceLock<Vec<PreparedEntry>> = OnceLock::new();
    INDEX.get_or_init(|| {
        let catalog: Catalog =
            serde_json::from_str(include_str!("../data/vocabulary/equipment.json"))
                .expect("maintained equipment catalog must be valid");
        assert_eq!(catalog.schema_version, 1);
        assert_eq!(catalog.dimension, "equipment");
        catalog
            .entries
            .into_iter()
            .map(|entry| {
                assert!(matches!(
                    entry.scope.as_str(),
                    "equipment" | "related_supply"
                ));
                let key = normalize(&entry.label);
                let mut names = vec![key.clone()];
                names.extend(entry.aliases.iter().map(|name| normalize(name)));
                // Broad search terms return each reviewed target. They are not
                // aliases that identify one subtype or rewrite authored text.
                names.extend(
                    catalog
                        .ambiguous_search_terms
                        .iter()
                        .filter(|(_, targets)| targets.contains(&entry.label))
                        .map(|(term, _)| normalize(term)),
                );
                PreparedEntry {
                    label: entry.label,
                    key,
                    names,
                }
            })
            .collect()
    })
}

/// Up to eight preferred labels, matching labels, aliases and broad search terms.
/// Supplies such as parchment paper are useful recipe requirements too.
pub fn suggestions(query: &str) -> Vec<&'static str> {
    if query.chars().count() > MAX_TEXT {
        return Vec::new();
    }
    let query = normalize(query);
    if query.chars().count() < 2 {
        return Vec::new();
    }
    let words: Vec<_> = query.split_whitespace().collect();
    let mut matches: Vec<_> = catalog()
        .iter()
        .filter_map(|entry| {
            let rank = entry
                .names
                .iter()
                .enumerate()
                .filter_map(|(position, name)| {
                    if name == &query {
                        Some(if position == 0 { 0 } else { 1 })
                    } else if name.starts_with(&query) {
                        Some(2)
                    } else if words
                        .iter()
                        .all(|word| name.split_whitespace().any(|part| part.starts_with(word)))
                    {
                        Some(3)
                    } else {
                        None
                    }
                })
                .min()?;
            Some((rank, entry))
        })
        .collect();
    matches.sort_unstable_by(|(rank_a, a), (rank_b, b)| (rank_a, &a.key).cmp(&(rank_b, &b.key)));
    matches
        .into_iter()
        .take(SUGGESTION_LIMIT as usize)
        .map(|(_, entry)| entry.label.as_str())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn maintained_catalog_has_unique_names_and_resolved_broad_search_targets() {
        let data: Catalog =
            serde_json::from_str(include_str!("../data/vocabulary/equipment.json")).unwrap();
        assert_eq!(data.schema_version, 1);
        assert_eq!(data.dimension, "equipment");
        assert_eq!(
            data.entries
                .iter()
                .filter(|e| e.scope == "equipment")
                .count(),
            427
        );
        assert_eq!(
            data.entries
                .iter()
                .filter(|e| e.scope == "related_supply")
                .count(),
            16
        );
        let mut names = HashSet::new();
        let labels: HashSet<_> = data.entries.iter().map(|e| e.label.as_str()).collect();
        for entry in &data.entries {
            for name in std::iter::once(&entry.label).chain(&entry.aliases) {
                assert!(names.insert(normalize(name)), "duplicate name: {name}");
                assert!(
                    suggestions(name).contains(&entry.label.as_str()),
                    "not discoverable: {name}"
                );
            }
        }
        for (term, targets) in &data.ambiguous_search_terms {
            assert!(!names.contains(&normalize(term)));
            assert!(targets.len() > 1);
            for target in targets {
                assert!(labels.contains(target.as_str()));
                assert!(
                    suggestions(term).contains(&target.as_str()),
                    "missing target: {term} -> {target}"
                );
            }
        }
    }

    #[test]
    fn search_uses_aliases_normalization_and_word_prefixes() {
        assert_eq!(suggestions("pizza steel")[0], "baking steel");
        assert_eq!(suggestions("pastry sheeter")[0], "dough sheeter");
        assert_eq!(suggestions("baking paper")[0], "parchment paper");
        assert_eq!(suggestions("  ＰＩＺＺＡ　ＳＴＥＥＬ  ")[0], "baking steel");
        assert!(suggestions("steel baking").contains(&"baking steel"));
        assert!(suggestions("mixing bo").contains(&"mixing bowl"));
        assert_eq!(suggestions("frying pan")[0], "frying pan");
        assert!(suggestions("pan").len() <= SUGGESTION_LIMIT as usize);
    }

    #[test]
    fn broad_terms_keep_distinct_targets_and_unknown_text_has_no_completion() {
        let mixers = suggestions("mixer");
        assert!(mixers.contains(&"stand mixer") && mixers.contains(&"hand mixer"));
        let kettles = suggestions("kettle");
        assert!(kettles.contains(&"electric kettle") && kettles.contains(&"stovetop kettle"));
        assert!(suggestions("ice shaver").contains(&"ice shaver"));
        assert!(suggestions("ice crusher").contains(&"ice crusher"));
        for query in [
            "",
            " ",
            "a",
            "%_",
            "my family heirloom pan",
            &"a".repeat(MAX_TEXT + 1),
        ] {
            assert!(
                suggestions(query).is_empty(),
                "unexpected match for {query:?}"
            );
        }
    }
}
