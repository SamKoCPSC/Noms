//! Shared, bounded snapshots for client autocomplete. No authored data is included.
use super::index::{
    digest, index_path, Catalog, IndexEntry, Manifest, INDEX_SCHEMA, MAX_INDEX_BYTES,
    MAX_INDEX_ENTRIES,
};
use chrono::{DateTime, Utc};
use flate2::{write::GzEncoder, Compression};
use sqlx::PgPool;
use std::{
    collections::VecDeque,
    io::Write,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Signature = (i64, Option<DateTime<Utc>>);

pub const SIGNATURE_SQL: &str = "SELECT count(*),max(updated_at) FROM vocabulary_entries WHERE dimension='ingredient' AND locale='en'";
pub const CATALOG_SQL: &str = "SELECT e.id,e.label,e.recipe_count,coalesce(array_agg(a.alias ORDER BY a.normalized_alias,a.alias) FILTER (WHERE a.alias IS NOT NULL),ARRAY[]::text[]) AS aliases \
    FROM vocabulary_entries e LEFT JOIN vocabulary_aliases a ON a.entry_id=e.id \
    WHERE e.dimension='ingredient' AND e.locale='en' AND e.status='active' \
    GROUP BY e.id ORDER BY e.normalized_label,e.id LIMIT $1";

#[derive(sqlx::FromRow)]
struct Row {
    id: uuid::Uuid,
    label: String,
    recipe_count: i64,
    aliases: Vec<String>,
}

pub struct Snapshot {
    pub version: String,
    pub body: Arc<[u8]>,
    pub gzip: Arc<[u8]>,
    pub manifest: Arc<[u8]>,
    pub manifest_version: String,
    signature: Signature,
    built_at: Instant,
}

#[derive(Default)]
struct Cache {
    current: Option<Arc<Snapshot>>,
    previous: VecDeque<Arc<Snapshot>>,
    checked_at: Option<Instant>,
}

#[derive(Clone)]
pub struct IndexService {
    pool: PgPool,
    cache: Arc<Mutex<Cache>>,
    validation_interval: Duration,
    ranking_interval: Duration,
}

impl IndexService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            cache: Arc::new(Mutex::new(Cache::default())),
            validation_interval: Duration::from_secs(1),
            ranking_interval: Duration::from_secs(300),
        }
    }

    pub async fn current(&self) -> Result<Arc<Snapshot>, Error> {
        // A single builder coalesces concurrent first loads, without holding a DB
        // connection during JSON encoding or compression.
        let mut cache = self.cache.lock().await;
        if let Some(current) = &cache.current {
            if cache
                .checked_at
                .is_some_and(|t| t.elapsed() < self.validation_interval)
                && current.built_at.elapsed() < self.ranking_interval
            {
                return Ok(current.clone());
            }
        }
        let signature: Signature = sqlx::query_as(SIGNATURE_SQL).fetch_one(&self.pool).await?;
        if let Some(current) = &cache.current {
            if current.signature == signature && current.built_at.elapsed() < self.ranking_interval
            {
                let result = current.clone();
                cache.checked_at = Some(Instant::now());
                return Ok(result);
            }
        }
        // All rows, aliases, counts and the stamp come from one committed snapshot.
        let mut tx = self.pool.begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
            .execute(&mut *tx)
            .await?;
        let signature: Signature = sqlx::query_as(SIGNATURE_SQL).fetch_one(&mut *tx).await?;
        let rows: Vec<Row> = sqlx::query_as(CATALOG_SQL)
            .bind((MAX_INDEX_ENTRIES + 1) as i64)
            .fetch_all(&mut *tx)
            .await?;
        tx.commit().await?;
        let snapshot =
            Arc::new(tokio::task::spawn_blocking(move || encode(rows, signature)).await??);
        if let Some(old) = cache.current.take() {
            if old.version != snapshot.version {
                cache.previous.push_front(old);
                cache.previous.truncate(2);
            }
        }
        cache.current = Some(snapshot.clone());
        cache.checked_at = Some(Instant::now());
        Ok(snapshot)
    }

    /// A known content-addressed response is immutable and needs no further DB read.
    pub async fn version(&self, version: &str) -> Result<Option<Arc<Snapshot>>, Error> {
        {
            let cache = self.cache.lock().await;
            if let Some(snapshot) = cache
                .current
                .iter()
                .chain(cache.previous.iter())
                .find(|s| s.version == version)
            {
                return Ok(Some(snapshot.clone()));
            }
        }
        let current = self.current().await?;
        Ok((current.version == version).then_some(current))
    }
}

fn encode(rows: Vec<Row>, signature: Signature) -> Result<Snapshot, Error> {
    let popularity = rows
        .iter()
        .filter(|r| r.recipe_count > 0)
        .map(|r| (r.id, r.recipe_count))
        .collect();
    let catalog = Catalog {
        schema_version: INDEX_SCHEMA,
        entries: rows
            .into_iter()
            .map(|r| IndexEntry {
                id: r.id,
                label: r.label,
                aliases: r.aliases,
            })
            .collect(),
    };
    catalog.validate()?;
    let body = serde_json::to_vec(&catalog)?;
    if body.len() > MAX_INDEX_BYTES {
        return Err("Vocabulary index is too large".into());
    }
    let version = digest(&body);
    let manifest = serde_json::to_vec(&Manifest {
        schema_version: INDEX_SCHEMA,
        url: index_path(&version),
        version: version.clone(),
        popularity,
    })?;
    let manifest_version = digest(&manifest);
    let mut compressed = GzEncoder::new(Vec::new(), Compression::default());
    compressed.write_all(&body)?;
    let gzip = compressed.finish()?;
    Ok(Snapshot {
        version,
        body: body.into(),
        gzip: gzip.into(),
        manifest: manifest.into(),
        manifest_version,
        signature,
        built_at: Instant::now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vocabulary::{index::SearchIndex, store, SeedFile, SourceManifest};
    #[tokio::test]
    async fn snapshot_matches_sql_and_refreshes_without_changing_name_hash_for_popularity() {
        let db = pgtemp::PgTempDB::async_new().await;
        let pool = PgPool::connect(&db.connection_uri()).await.unwrap();
        let schema = include_str!("../../migrations/schema.sql");
        sqlx::raw_sql("CREATE TABLE recipes(id uuid PRIMARY KEY,ingredients jsonb NOT NULL DEFAULT '[]',visibility text NOT NULL DEFAULT 'public')").execute(&pool).await.unwrap();
        sqlx::raw_sql(&schema[schema.find("-- NOMS-017: maintained").unwrap()..])
            .execute(&pool)
            .await
            .unwrap();
        let mut seed: SeedFile = serde_json::from_str(include_str!(
            "../../data/vocabulary/adapters/foodon-cooking-base-ingredients.json"
        ))
        .unwrap();
        let sources: SourceManifest = serde_json::from_str(include_str!(
            "../../data/vocabulary/adapters/foodon-cooking-base-sources.json"
        ))
        .unwrap();
        store::seed(&pool, &seed, &sources, false).await.unwrap();
        let mut service = IndexService::new(pool.clone());
        service.validation_interval = Duration::ZERO;
        service.ranking_interval = Duration::ZERO;
        let first = service.current().await.unwrap();
        let build_index = |s: &Snapshot| {
            let catalog = serde_json::from_slice(&s.body).unwrap();
            let manifest: Manifest = serde_json::from_slice(&s.manifest).unwrap();
            SearchIndex::new(&s.version, catalog, &manifest.popularity).unwrap()
        };
        for text in [
            "gar",
            "garlic",
            "garbanzo",
            "tah",
            "ＳＯＹ",
            "%a",
            "_a",
            "chickpea",
        ] {
            assert_eq!(
                build_index(&first)
                    .search(text)
                    .iter()
                    .map(|x| x.id)
                    .collect::<Vec<_>>(),
                store::autocomplete(&pool, text)
                    .await
                    .unwrap()
                    .iter()
                    .map(|x| x.id)
                    .collect::<Vec<_>>(),
                "{text}"
            );
        }
        sqlx::query("UPDATE vocabulary_entries SET recipe_count=7 WHERE id=$1")
            .bind(seed.entries[0].id)
            .execute(&pool)
            .await
            .unwrap();
        let ranked = service.current().await.unwrap();
        assert_eq!(first.version, ranked.version);
        assert_ne!(first.manifest_version, ranked.manifest_version);
        seed.entries[0]
            .aliases
            .push("new catalog test alias".into());
        seed.entries[1].status = "retired".into();
        store::seed(&pool, &seed, &sources, false).await.unwrap();
        let changed = service.current().await.unwrap();
        assert_ne!(first.version, changed.version);
        assert_eq!(
            build_index(&changed).search("new catalog test alias")[0].id,
            seed.entries[0].id
        );
        assert!(!build_index(&changed)
            .catalog()
            .entries
            .iter()
            .any(|r| r.id == seed.entries[1].id));
        assert_eq!(
            service.version(&first.version).await.unwrap().unwrap().body,
            first.body
        );
        assert!(service.version(&"0".repeat(64)).await.unwrap().is_none());
        let mut decoder = flate2::read::GzDecoder::new(&changed.gzip[..]);
        let mut uncompressed = Vec::new();
        std::io::Read::read_to_end(&mut decoder, &mut uncompressed).unwrap();
        assert_eq!(&uncompressed[..], &changed.body[..]);
    }
}
