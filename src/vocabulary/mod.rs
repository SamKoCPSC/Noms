//! Maintained suggestions, independent of authored recipe text.
use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;
use uuid::Uuid;

pub mod index;
#[cfg(feature = "server")]
pub mod index_store;
#[cfg(feature = "server")]
pub mod store;

pub const MAX_TEXT: usize = 200;
pub const SUGGESTION_LIMIT: i64 = 8;

/// Comparison keys only: never replace the author's display text.
pub fn normalize(text: &str) -> String {
    text.nfkc()
        .collect::<String>()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeedFile {
    pub schema_version: u32,
    pub dimension: String,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub id: Uuid,
    pub label: String,
    pub locale: String,
    pub aliases: Vec<String>,
    pub description: String,
    pub status: String,
    pub source_mappings: Vec<SourceMapping>,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceMapping {
    pub source: String,
    pub external_id: String,
    pub relationship: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    pub origin: String,
    pub references: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceManifest {
    pub schema_version: u32,
    pub sources: Vec<Source>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub id: String,
    pub url: String,
    pub version: String,
    pub checksum: String,
    pub license: String,
    pub attribution: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Suggestion {
    pub id: Uuid,
    pub label: String,
    pub description: String,
}

/// Validate the whole input before acquiring a database connection.
pub fn validate(seed: &SeedFile, sources: &SourceManifest) -> Result<Vec<String>, String> {
    use std::collections::{HashMap, HashSet};
    if seed.schema_version != 1 || sources.schema_version != 1 || seed.dimension != "ingredient" {
        return Err("Expected schema version 1 and ingredient dimension".into());
    }
    if seed.entries.is_empty() || seed.entries.len() > 100_000 {
        return Err("Expected 1–100000 seed entries; retire entries explicitly rather than emptying the file".into());
    }
    let valid_text = |s: &str| {
        !s.trim().is_empty() && s.chars().count() <= MAX_TEXT && !s.chars().any(char::is_control)
    };
    let mut source_ids = HashSet::new();
    for s in &sources.sources {
        if !valid_text(&s.id)
            || !source_ids.insert(s.id.as_str())
            || !s.url.starts_with("https://")
            || s.version.trim().is_empty()
            || s.checksum.len() != 64
            || !s.checksum.bytes().all(|c| c.is_ascii_hexdigit())
            || s.license.trim().is_empty()
            || s.attribution.trim().is_empty()
        {
            return Err(format!("Invalid source manifest entry: {}", s.id));
        }
    }
    let mut ids = HashSet::new();
    let mut names: HashMap<(String, String), Uuid> = HashMap::new();
    let mut warnings = Vec::new();
    for e in &seed.entries {
        if e.id.is_nil()
            || !ids.insert(e.id)
            || !valid_text(&e.label)
            || e.locale != "en"
            || !matches!(e.status.as_str(), "active" | "retired")
            || e.description.chars().count() > 2000
            || e.aliases.len() > 100
            || !matches!(e.provenance.origin.as_str(), "curated" | "imported")
        {
            return Err(format!("Invalid or duplicate ingredient entry: {}", e.id));
        }
        let mut local = HashSet::new();
        for text in std::iter::once(&e.label).chain(e.aliases.iter()) {
            let key = normalize(text);
            if !valid_text(text) || !local.insert(key.clone()) {
                return Err(format!("Invalid or repeated label/alias in {}", e.id));
            }
            if e.status == "active" {
                if let Some(other) = names.insert((e.locale.clone(), key), e.id) {
                    warnings.push(format!(
                        "Ambiguous suggestion {text:?}: {other} and {}; entries remain separate",
                        e.id
                    ));
                }
            }
        }
        let mut refs = HashSet::new();
        for reference in &e.provenance.references {
            if !source_ids.contains(reference.as_str()) || !refs.insert(reference) {
                return Err(format!(
                    "Unknown or repeated provenance reference in {}",
                    e.id
                ));
            }
        }
        if e.provenance.origin == "imported" && refs.is_empty() {
            return Err(format!("Imported entry {} needs source provenance", e.id));
        }
        let mut mappings = HashSet::new();
        for m in &e.source_mappings {
            if !source_ids.contains(m.source.as_str())
                || !refs.contains(&m.source)
                || !valid_text(&m.external_id)
                || !matches!(
                    m.relationship.as_str(),
                    "exact" | "broader" | "narrower" | "related"
                )
                || !mappings.insert((&m.source, &m.external_id, &m.relationship))
            {
                return Err(format!("Invalid source mapping in {}", e.id));
            }
        }
    }
    Ok(warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn inputs() -> (SeedFile, SourceManifest) {
        (
            serde_json::from_str(include_str!("../../data/vocabulary/ingredients.json")).unwrap(),
            serde_json::from_str(include_str!("../../data/vocabulary/sources.json")).unwrap(),
        )
    }
    #[test]
    fn curated_seed_and_ambiguous_aliases_validate_without_merging() {
        let (mut seed, sources) = inputs();
        assert!(validate(&seed, &sources).unwrap().is_empty());
        let mut another = seed.entries[0].clone();
        another.id = Uuid::new_v4();
        seed.entries.push(another);
        assert!(!validate(&seed, &sources).unwrap().is_empty());
        assert_eq!(normalize("  ＧＡＲＢＡＮＺＯ   Bean "), "garbanzo bean");
    }
    #[test]
    fn invalid_identity_alias_and_provenance_are_rejected() {
        let (seed, sources) = inputs();
        let mut invalid = seed.clone();
        invalid.entries[1].id = invalid.entries[0].id;
        assert!(validate(&invalid, &sources).is_err());
        let mut invalid = seed.clone();
        invalid.entries[0].aliases.push(" CHICKPEA ".into());
        assert!(validate(&invalid, &sources).is_err());
        let mut invalid = seed;
        invalid.entries[0]
            .provenance
            .references
            .push("unknown-source".into());
        assert!(validate(&invalid, &sources).is_err());
    }
}
