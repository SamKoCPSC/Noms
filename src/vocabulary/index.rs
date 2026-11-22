//! Portable, bounded local search with the same normalization and ranking as SQL.
use super::{normalize, Suggestion, MAX_TEXT, SUGGESTION_LIMIT};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;

pub const INDEX_SCHEMA: u32 = 1;
pub const MAX_INDEX_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_INDEX_ENTRIES: usize = 100_000;
pub const MANIFEST_PATH: &str = "/api/vocabulary/ingredients/version.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndexEntry {
    pub id: Uuid,
    pub label: String,
    pub aliases: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    pub schema_version: u32,
    /// Server order is normalized label, then UUID, using the database collation.
    pub entries: Vec<IndexEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u32,
    pub version: String,
    pub url: String,
    /// Public-recipe counts only; zeros are omitted. Separate from the catalog hash.
    pub popularity: Vec<(Uuid, i64)>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CachedIndex {
    pub version: String,
    pub catalog: Catalog,
    pub popularity: Vec<(Uuid, i64)>,
}

pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn valid_version(version: &str) -> bool {
    version.len() == 64
        && version
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

pub fn index_path(version: &str) -> String {
    format!("/api/vocabulary/ingredients/{version}.json")
}

impl Manifest {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != INDEX_SCHEMA
            || !valid_version(&self.version)
            || self.url != index_path(&self.version)
            || self.popularity.len() > MAX_INDEX_ENTRIES
        {
            return Err("Invalid vocabulary manifest".into());
        }
        Ok(())
    }
}

impl Catalog {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != INDEX_SCHEMA || self.entries.len() > MAX_INDEX_ENTRIES {
            return Err("Unsupported vocabulary index".into());
        }
        let mut ids = std::collections::HashSet::new();
        let valid_name = |s: &str| !normalize(s).is_empty() && s.chars().count() <= MAX_TEXT;
        for entry in &self.entries {
            if entry.id.is_nil()
                || !ids.insert(entry.id)
                || !valid_name(&entry.label)
                || entry.aliases.len() > 1024
                || entry.aliases.iter().any(|s| !valid_name(s))
            {
                return Err("Invalid vocabulary entry".into());
            }
        }
        Ok(())
    }
}

struct Term {
    key: String,
    entry: usize,
    exact_rank: u8,
}

struct PreparedCatalog {
    catalog: Catalog,
    version: String,
    terms: Vec<Term>,
    by_id: HashMap<Uuid, usize>,
}

#[derive(Clone)]
pub struct SearchIndex {
    prepared: Arc<PreparedCatalog>,
    counts: Vec<i64>,
}

impl SearchIndex {
    pub fn new(
        version: &str,
        catalog: Catalog,
        popularity: &[(Uuid, i64)],
    ) -> Result<Self, String> {
        catalog.validate()?;
        let encoded = serde_json::to_vec(&catalog).map_err(|e| e.to_string())?;
        if encoded.len() > MAX_INDEX_BYTES || !valid_version(version) || digest(&encoded) != version
        {
            return Err("Vocabulary index checksum mismatch".into());
        }
        let mut terms = Vec::new();
        let mut by_id = HashMap::new();
        for (entry, row) in catalog.entries.iter().enumerate() {
            by_id.insert(row.id, entry);
            terms.push(Term {
                key: normalize(&row.label),
                entry,
                exact_rank: 0,
            });
            terms.extend(row.aliases.iter().map(|alias| Term {
                key: normalize(alias),
                entry,
                exact_rank: 1,
            }));
        }
        terms.sort_unstable_by(|a, b| a.key.cmp(&b.key));
        let counts = vec![0; catalog.entries.len()];
        let index = Self {
            prepared: Arc::new(PreparedCatalog {
                catalog,
                version: version.into(),
                terms,
                by_id,
            }),
            counts,
        };
        index.with_popularity(popularity)
    }

    pub fn version(&self) -> &str {
        &self.prepared.version
    }
    pub fn catalog(&self) -> &Catalog {
        &self.prepared.catalog
    }

    /// Updating counts shares all prepared strings and prefix terms.
    pub fn with_popularity(&self, popularity: &[(Uuid, i64)]) -> Result<Self, String> {
        if popularity.len() > self.prepared.catalog.entries.len() {
            return Err("Invalid popularity snapshot".into());
        }
        let mut counts = vec![0; self.prepared.catalog.entries.len()];
        let mut seen = std::collections::HashSet::new();
        for (id, count) in popularity {
            let Some(index) = self.prepared.by_id.get(id) else {
                return Err("Unknown popularity identity".into());
            };
            if *count < 0 || !seen.insert(id) {
                return Err("Invalid popularity count".into());
            }
            counts[*index] = *count;
        }
        Ok(Self {
            prepared: self.prepared.clone(),
            counts,
        })
    }

    pub fn search(&self, query: &str) -> Vec<Suggestion> {
        if query.chars().count() > MAX_TEXT {
            return Vec::new();
        }
        let query = normalize(query);
        if query.chars().count() < 2 {
            return Vec::new();
        }
        let first = self.prepared.terms.partition_point(|term| term.key < query);
        let mut matches: HashMap<usize, u8> = HashMap::new();
        for term in self.prepared.terms[first..]
            .iter()
            .take_while(|t| t.key.starts_with(&query))
        {
            let rank = if term.key == query {
                term.exact_rank
            } else {
                2
            };
            matches
                .entry(term.entry)
                .and_modify(|r| *r = (*r).min(rank))
                .or_insert(rank);
        }
        let mut matches: Vec<_> = matches.into_iter().collect();
        matches.sort_unstable_by_key(|(entry, rank)| {
            (*rank, std::cmp::Reverse(self.counts[*entry]), *entry)
        });
        matches
            .into_iter()
            .take(SUGGESTION_LIMIT as usize)
            .map(|(i, _)| {
                let e = &self.prepared.catalog.entries[i];
                Suggestion {
                    id: e.id,
                    label: e.label.clone(),
                    description: String::new(),
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (String, Catalog) {
        let rows = [
            (1, "garlic", vec!["garlic cloves"]),
            (2, "garlic powder", vec!["ground garlic"]),
            (3, "garlic salt", vec!["garlic seasoning"]),
            (4, "jelly ear", vec!["wood ear mushrooms"]),
            (5, "soy sauce", vec!["shoyu"]),
            (6, "%agave", vec![]),
        ];
        let catalog = Catalog {
            schema_version: INDEX_SCHEMA,
            entries: rows
                .into_iter()
                .map(|(id, label, aliases)| IndexEntry {
                    id: Uuid::from_u128(id),
                    label: label.into(),
                    aliases: aliases.into_iter().map(String::from).collect(),
                })
                .collect(),
        };
        (digest(&serde_json::to_vec(&catalog).unwrap()), catalog)
    }
    #[test]
    fn local_matching_preserves_exact_priority_aliases_literal_prefixes_and_normalization() {
        let (version, catalog) = fixture();
        let index = SearchIndex::new(&version, catalog, &[(Uuid::from_u128(3), 50)]).unwrap();
        assert_eq!(index.search("garlic")[0].id, Uuid::from_u128(1));
        assert_eq!(index.search("gar")[0].id, Uuid::from_u128(3));
        assert_eq!(index.search("WOOD  EAR")[0].id, Uuid::from_u128(4));
        assert_eq!(index.search(" ＳＯＹ　sauce ")[0].id, Uuid::from_u128(5));
        assert_eq!(index.search("%a")[0].id, Uuid::from_u128(6));
        assert!(index.search("_a").is_empty());
        assert!(index.search("a").is_empty());
        assert!(index.search(&"x".repeat(201)).is_empty());
        assert_eq!(index.search("garlic cl").len(), 1);
    }
    #[test]
    fn popularity_refresh_does_not_change_catalog_version_and_ties_follow_server_order() {
        let (version, catalog) = fixture();
        let index = SearchIndex::new(&version, catalog, &[]).unwrap();
        assert_eq!(
            index
                .search("gar")
                .iter()
                .map(|x| x.id.as_u128())
                .collect::<Vec<_>>(),
            [1, 2, 3]
        );
        let updated = index.with_popularity(&[(Uuid::from_u128(2), 99)]).unwrap();
        assert_eq!(updated.version(), version);
        assert_eq!(updated.search("gar")[0].id.as_u128(), 2);
        assert_eq!(updated.search("garlic")[0].id.as_u128(), 1);
    }
    #[test]
    fn corrupt_cache_unknown_versions_and_invalid_counts_fail_closed() {
        let (version, mut catalog) = fixture();
        assert!(SearchIndex::new(&"0".repeat(64), catalog.clone(), &[]).is_err());
        assert!(SearchIndex::new(&version, catalog.clone(), &[(Uuid::nil(), 1)]).is_err());
        assert!(SearchIndex::new(&version, catalog.clone(), &[(Uuid::from_u128(1), -1)]).is_err());
        catalog.entries[0].id = Uuid::nil();
        assert!(catalog.validate().is_err());
        assert!(!valid_version(&"A".repeat(64)));
        let bad = Manifest {
            schema_version: INDEX_SCHEMA,
            version: version.clone(),
            url: "https://elsewhere.test/index".into(),
            popularity: vec![],
        };
        assert!(bad.validate().is_err());
    }
}
