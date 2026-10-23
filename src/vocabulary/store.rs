//! Transactional, explicitly invoked seed loading and bounded vocabulary reads.
use super::{normalize, validate, SeedFile, SourceManifest, Suggestion};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

type Error = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Default, Serialize)]
pub struct SeedReport {
    pub added: usize,
    pub updated: usize,
    pub retired: usize,
    pub unchanged: usize,
    pub source_updates: usize,
    pub dry_run: bool,
    pub warnings: Vec<String>,
}

pub async fn seed(
    pool: &PgPool,
    file: &SeedFile,
    sources: &SourceManifest,
    dry_run: bool,
) -> Result<SeedReport, Error> {
    let warnings = validate(file, sources)?;
    let mut report = SeedReport {
        dry_run,
        warnings,
        ..Default::default()
    };
    let mut tx = pool.begin().await?;
    // One transaction-scoped lock serializes all vocabulary seed writers.
    sqlx::query("SELECT pg_advisory_xact_lock(170017)")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SELECT pg_advisory_xact_lock(170018)")
        .execute(&mut *tx)
        .await?;
    let existing: Vec<(Uuid, serde_json::Value)> =
        sqlx::query_as("SELECT id, definition FROM vocabulary_entries WHERE dimension = $1")
            .bind(&file.dimension)
            .fetch_all(&mut *tx)
            .await?;
    let existing: HashMap<_, _> = existing.into_iter().collect();
    let incoming: std::collections::HashSet<_> = file.entries.iter().map(|e| e.id).collect();
    for id in existing.keys() {
        if !incoming.contains(id) {
            return Err(format!(
                "Entry {id} disappeared from the seed file; retain it with status retired"
            )
            .into());
        }
    }
    // Store manifest definitions as part of the same transaction.
    for source in &sources.sources {
        let definition = serde_json::to_value(source)?;
        let changed = sqlx::query(
            "INSERT INTO vocabulary_sources (id, definition) VALUES ($1,$2) \
             ON CONFLICT (id) DO UPDATE SET definition = EXCLUDED.definition \
             WHERE vocabulary_sources.definition IS DISTINCT FROM EXCLUDED.definition",
        )
        .bind(&source.id)
        .bind(definition)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        report.source_updates += changed as usize;
    }
    let mut changes = Vec::new();
    for entry in &file.entries {
        let definition = serde_json::to_value(entry)?;
        match existing.get(&entry.id) {
            Some(old) if old == &definition => {
                report.unchanged += 1;
                continue;
            }
            Some(old) => {
                report.updated += 1;
                if entry.status == "retired" && old["status"] != "retired" {
                    report.retired += 1;
                }
            }
            None => report.added += 1,
        }
        changes.push((entry.id, entry.label.clone(), entry.status.clone()));
        sqlx::query(
            "INSERT INTO vocabulary_entries (id,dimension,locale,label,normalized_label,description,status,definition) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8) \
             ON CONFLICT (id) DO UPDATE SET locale=EXCLUDED.locale,label=EXCLUDED.label, \
             normalized_label=EXCLUDED.normalized_label,description=EXCLUDED.description, \
             status=EXCLUDED.status,definition=EXCLUDED.definition,updated_at=now() \
             WHERE vocabulary_entries.dimension=EXCLUDED.dimension"
        ).bind(entry.id).bind(&file.dimension).bind(&entry.locale).bind(&entry.label)
            .bind(normalize(&entry.label)).bind(&entry.description).bind(&entry.status)
            .bind(definition).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM vocabulary_aliases WHERE entry_id=$1")
            .bind(entry.id)
            .execute(&mut *tx)
            .await?;
        for alias in &entry.aliases {
            sqlx::query("INSERT INTO vocabulary_aliases (entry_id,alias,normalized_alias) VALUES ($1,$2,$3)")
                .bind(entry.id).bind(alias).bind(normalize(alias)).execute(&mut *tx).await?;
        }
        sqlx::query("DELETE FROM vocabulary_source_mappings WHERE entry_id=$1")
            .bind(entry.id)
            .execute(&mut *tx)
            .await?;
        for mapping in &entry.source_mappings {
            sqlx::query("INSERT INTO vocabulary_source_mappings (entry_id,source_id,external_id,relationship) VALUES ($1,$2,$3,$4)")
                .bind(entry.id).bind(&mapping.source).bind(&mapping.external_id).bind(&mapping.relationship)
                .execute(&mut *tx).await?;
        }
    }
    if report.added > 0 {
        // A newly seeded ID may already occur in historical recipe JSON.
        reconcile_in_transaction(&mut tx, false).await?;
    }
    if report.added + report.updated + report.source_updates > 0 {
        let digest = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&(file, sources))?)
        );
        sqlx::query("INSERT INTO vocabulary_seed_runs (checksum,entry_count) VALUES ($1,$2)")
            .bind(digest)
            .bind(file.entries.len() as i32)
            .execute(&mut *tx)
            .await?;
    }
    if dry_run {
        tx.rollback().await?;
    } else {
        tx.commit().await?;
    }
    for (id, label, status) in changes {
        // Review output contains public vocabulary only, never connection configuration.
        println!(
            "{} {id} {label:?} ({status})",
            if dry_run { "Would apply" } else { "Applied" }
        );
    }
    Ok(report)
}

#[derive(Debug, Serialize)]
pub struct CountReport {
    pub corrected: u64,
    pub dry_run: bool,
}

/// Deployment-role maintenance, serialized with recipe counter updates.
pub async fn reconcile_counts(pool: &PgPool, dry_run: bool) -> Result<CountReport, Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(170018)")
        .execute(&mut *tx)
        .await?;
    let corrected = reconcile_in_transaction(&mut tx, dry_run).await?;
    tx.commit().await?;
    Ok(CountReport { corrected, dry_run })
}

async fn reconcile_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    dry_run: bool,
) -> Result<u64, Error> {
    // The lock must precede this statement so READ COMMITTED sees every writer
    // that finished before it; later writers apply their deltas after commit.
    let counts = "WITH usage AS (
        SELECT ids.id, count(*) AS recipe_count FROM public.recipes r
        CROSS JOIN LATERAL unnest(public.ingredient_vocabulary_ids(r.ingredients)) AS ids(id)
        WHERE r.visibility='public' AND ids.id IS NOT NULL GROUP BY ids.id
    ), expected AS (
        SELECT e.id, coalesce(u.recipe_count, 0) AS recipe_count
        FROM public.vocabulary_entries e LEFT JOIN usage u ON u.id=e.id
        WHERE e.dimension='ingredient'
    ) ";
    if dry_run {
        let changed: i64 = sqlx::query_scalar(&format!(
            "{counts} SELECT count(*) FROM public.vocabulary_entries e JOIN expected x ON x.id=e.id
             WHERE e.recipe_count IS DISTINCT FROM x.recipe_count"
        ))
        .fetch_one(&mut **tx)
        .await?;
        Ok(changed as u64)
    } else {
        Ok(sqlx::query(&format!(
            "{counts} UPDATE public.vocabulary_entries e SET recipe_count=x.recipe_count
             FROM expected x WHERE e.id=x.id AND e.recipe_count IS DISTINCT FROM x.recipe_count"
        ))
        .execute(&mut **tx)
        .await?
        .rows_affected())
    }
}

#[derive(sqlx::FromRow, Deserialize)]
struct SuggestionRow {
    id: Uuid,
    label: String,
    description: String,
}

pub async fn autocomplete(pool: &PgPool, text: &str) -> Result<Vec<Suggestion>, Error> {
    if text.chars().count() > super::MAX_TEXT {
        return Err("Ingredient query is too long".into());
    }
    let text = normalize(text);
    if text.chars().count() < 2 {
        return Ok(Vec::new());
    }
    // Escape SQL wildcard characters: a typed '%' or '_' is literal input.
    let prefix = format!(
        "{}%",
        text.replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_")
    );
    let rows: Vec<SuggestionRow> = sqlx::query_as(
        "WITH matches AS ( \
           SELECT id, CASE WHEN normalized_label=$1 THEN 0 ELSE 2 END AS rank \
           FROM vocabulary_entries WHERE dimension='ingredient' AND locale='en' AND status='active' AND normalized_label LIKE $2 \
           UNION ALL \
           SELECT e.id, CASE WHEN a.normalized_alias=$1 THEN 1 ELSE 2 END AS rank \
           FROM vocabulary_aliases a JOIN vocabulary_entries e ON e.id=a.entry_id \
           WHERE e.dimension='ingredient' AND e.locale='en' AND e.status='active' AND a.normalized_alias LIKE $2 \
         ), ranked AS (SELECT id,min(rank) AS rank FROM matches GROUP BY id) \
         SELECT e.id,e.label,e.description FROM ranked r JOIN vocabulary_entries e ON e.id=r.id \
         ORDER BY r.rank,e.recipe_count DESC,e.normalized_label,e.id LIMIT $3"
    ).bind(text).bind(prefix).bind(super::SUGGESTION_LIMIT).fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .map(|r| Suggestion {
            id: r.id,
            label: r.label,
            description: r.description,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Keep ranking/transaction fixtures stable as the maintained catalog grows.
    // The full catalog is validated separately by vocabulary::tests and the seed CLI.

    async fn popularity_db() -> (pgtemp::PgTempDB, PgPool, SeedFile, SourceManifest) {
        let db = pgtemp::PgTempDB::async_new().await;
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(12)
            .connect(&db.connection_uri())
            .await
            .unwrap();
        sqlx::raw_sql(
            "CREATE TABLE recipes (id UUID PRIMARY KEY, ingredients JSONB NOT NULL DEFAULT '[]', visibility TEXT NOT NULL DEFAULT 'public')",
        )
        .execute(&pool)
        .await
        .unwrap();
        let schema = include_str!("../../migrations/schema.sql");
        sqlx::raw_sql(&schema[schema.find("-- NOMS-017: maintained").unwrap()..])
            .execute(&pool)
            .await
            .unwrap();
        let file: SeedFile = serde_json::from_str(include_str!(
            "../../data/vocabulary/adapters/foodon-cooking-base-ingredients.json"
        ))
        .unwrap();
        let sources: SourceManifest = serde_json::from_str(include_str!(
            "../../data/vocabulary/adapters/foodon-cooking-base-sources.json"
        ))
        .unwrap();
        seed(&pool, &file, &sources, false).await.unwrap();
        (db, pool, file, sources)
    }

    async fn count(pool: &PgPool, id: Uuid) -> i64 {
        sqlx::query_scalar("SELECT recipe_count FROM vocabulary_entries WHERE id=$1")
            .bind(id)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn popularity_orders_prefixes_after_exact_matches_and_preserves_seed_counts() {
        let (_db, pool, mut file, sources) = popularity_db().await;
        let garlic = file
            .entries
            .iter()
            .find(|e| e.label == "garlic")
            .unwrap()
            .id;
        let paste = file
            .entries
            .iter()
            .find(|e| e.label == "garlic paste")
            .unwrap()
            .id;
        let powder = file
            .entries
            .iter()
            .find(|e| e.label == "garlic powder")
            .unwrap()
            .id;
        for (id, n) in [(powder, 3), (paste, 1)] {
            for _ in 0..n {
                let rows = serde_json::json!([{"vocabulary_id":id}, {"vocabulary_id":id}, {"name":"custom"}]);
                sqlx::query("INSERT INTO recipes (id, ingredients) VALUES ($1,$2)")
                    .bind(Uuid::new_v4())
                    .bind(rows)
                    .execute(&pool)
                    .await
                    .unwrap();
            }
        }
        assert_eq!(count(&pool, powder).await, 3);
        assert_eq!(
            autocomplete(&pool, "garl")
                .await
                .unwrap()
                .iter()
                .map(|r| r.id)
                .collect::<Vec<_>>(),
            vec![powder, paste, garlic]
        );
        assert_eq!(
            autocomplete(&pool, "garlic").await.unwrap()[0].id,
            garlic,
            "exact labels beat popularity"
        );
        let entry = file.entries.iter_mut().find(|e| e.id == garlic).unwrap();
        entry.aliases.push("gar".into());
        seed(&pool, &file, &sources, false).await.unwrap();
        assert_eq!(
            autocomplete(&pool, "gar").await.unwrap()[0].id,
            garlic,
            "exact aliases beat popular prefixes"
        );
        assert_eq!(
            count(&pool, powder).await,
            3,
            "seed updates preserve counters"
        );
        seed(&pool, &file, &sources, true).await.unwrap();
        assert_eq!(count(&pool, powder).await, 3);
        // At equal popularity alphabetical label order is stable.
        sqlx::query("DELETE FROM recipes")
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(
            autocomplete(&pool, "garl")
                .await
                .unwrap()
                .iter()
                .map(|r| r.id)
                .collect::<Vec<_>>(),
            vec![garlic, paste, powder]
        );
    }

    #[tokio::test]
    async fn popularity_reconciliation_backfills_and_repairs_without_double_counting() {
        let (_db, pool, mut file, sources) = popularity_db().await;
        let id = file.entries[0].id;
        let pending = Uuid::new_v4();
        let rows = serde_json::json!([{"vocabulary_id":id},{"vocabulary_id":id},{"vocabulary_id":pending},{"vocabulary_id":"invalid"},{"vocabulary_id":null}]);
        let recipe = Uuid::new_v4();
        sqlx::query("INSERT INTO recipes (id, ingredients) VALUES ($1,$2)")
            .bind(recipe)
            .bind(rows)
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(count(&pool, id).await, 1);
        sqlx::query("UPDATE vocabulary_entries SET recipe_count=7 WHERE id=$1")
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(reconcile_counts(&pool, true).await.unwrap().corrected, 1);
        assert_eq!(count(&pool, id).await, 7, "preview does not mutate");
        assert_eq!(reconcile_counts(&pool, false).await.unwrap().corrected, 1);
        assert_eq!(count(&pool, id).await, 1);
        assert_eq!(reconcile_counts(&pool, false).await.unwrap().corrected, 0);
        let mut added = file.entries[0].clone();
        added.id = pending;
        added.label = "newly recognized ingredient".into();
        added.aliases.clear();
        file.entries.push(added);
        seed(&pool, &file, &sources, false).await.unwrap();
        assert_eq!(
            count(&pool, pending).await,
            1,
            "new IDs backfill references already in recipes"
        );
        sqlx::query("DELETE FROM recipes WHERE id=$1")
            .bind(recipe)
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(count(&pool, id).await, 0);
        assert_eq!(count(&pool, pending).await, 0);
        assert!(
            sqlx::query("UPDATE vocabulary_entries SET recipe_count=-1 WHERE id=$1")
                .bind(id)
                .execute(&pool)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn popularity_concurrent_writes_and_reconciliation_remain_consistent() {
        let (_db, pool, file, _sources) = popularity_db().await;
        let a = file.entries[0].id;
        let b = file.entries[1].id;
        let mut jobs = Vec::new();
        for _ in 0..8 {
            let pool = pool.clone();
            jobs.push(tokio::spawn(async move {
                let id = Uuid::new_v4();
                sqlx::query("INSERT INTO recipes (id, ingredients) VALUES ($1,$2)")
                    .bind(id)
                    .bind(serde_json::json!([{"vocabulary_id":a},{"vocabulary_id":b}]))
                    .execute(&pool)
                    .await
                    .unwrap();
                id
            }));
        }
        let mut ids = Vec::new();
        for job in jobs {
            ids.push(job.await.unwrap());
        }
        assert_eq!((count(&pool, a).await, count(&pool, b).await), (8, 8));
        let mut jobs = Vec::new();
        for (i, id) in ids.into_iter().enumerate() {
            let pool = pool.clone();
            jobs.push(tokio::spawn(async move {
                let remaining = if i % 2 == 0 { a } else { b };
                sqlx::query("UPDATE recipes SET ingredients=$2 WHERE id=$1")
                    .bind(id)
                    .bind(serde_json::json!([{"vocabulary_id":remaining}]))
                    .execute(&pool)
                    .await
                    .unwrap();
            }));
        }
        let repair = tokio::spawn({
            let pool = pool.clone();
            async move { reconcile_counts(&pool, false).await.unwrap() }
        });
        for job in jobs {
            job.await.unwrap();
        }
        repair.await.unwrap();
        assert_eq!((count(&pool, a).await, count(&pool, b).await), (4, 4));
        assert_eq!(reconcile_counts(&pool, false).await.unwrap().corrected, 0);
        // Hold maintenance's exclusive lock: a writer must wait and then apply
        // its delta to the reconciled base instead of losing the increment.
        let mut tx = pool.begin().await.unwrap();
        sqlx::query("SELECT pg_advisory_xact_lock(170018)")
            .execute(&mut *tx)
            .await
            .unwrap();
        let writer = tokio::spawn({
            let pool = pool.clone();
            async move {
                sqlx::query("INSERT INTO recipes (id, ingredients) VALUES ($1,$2)")
                    .bind(Uuid::new_v4())
                    .bind(serde_json::json!([{"vocabulary_id":a}]))
                    .execute(&pool)
                    .await
                    .unwrap();
            }
        });
        tokio::task::yield_now().await;
        reconcile_in_transaction(&mut tx, false).await.unwrap();
        tx.commit().await.unwrap();
        writer.await.unwrap();
        assert_eq!(count(&pool, a).await, 5);
        assert_eq!(reconcile_counts(&pool, false).await.unwrap().corrected, 0);
    }

    #[tokio::test]
    async fn seed_additions_are_visible_and_loading_is_atomic_and_idempotent() {
        let db = pgtemp::PgTempDB::async_new().await;
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(&db.connection_uri())
            .await
            .unwrap();
        let schema = include_str!("../../migrations/schema.sql")
            .split("-- NOMS-017: maintained")
            .nth(1)
            .unwrap();
        sqlx::raw_sql(
            "CREATE TABLE recipes (id UUID PRIMARY KEY, ingredients JSONB NOT NULL DEFAULT '[]', visibility TEXT NOT NULL DEFAULT 'public')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::raw_sql(&format!("-- NOMS-017: maintained{schema}"))
            .execute(&pool)
            .await
            .unwrap();
        let file: SeedFile = serde_json::from_str(include_str!(
            "../../data/vocabulary/adapters/foodon-cooking-base-ingredients.json"
        ))
        .unwrap();
        let sources: SourceManifest = serde_json::from_str(include_str!(
            "../../data/vocabulary/adapters/foodon-cooking-base-sources.json"
        ))
        .unwrap();
        let mut original = file.clone();
        original.entries.truncate(10);
        assert_eq!(
            seed(&pool, &original, &sources, true).await.unwrap().added,
            10
        );
        assert!(autocomplete(&pool, "garbanzo").await.unwrap().is_empty());
        assert_eq!(
            seed(&pool, &original, &sources, false).await.unwrap().added,
            10
        );
        let before: Vec<(Uuid, chrono::DateTime<chrono::Utc>)> =
            sqlx::query_as("SELECT id,updated_at FROM vocabulary_entries ORDER BY id")
                .fetch_all(&pool)
                .await
                .unwrap();
        let report = seed(&pool, &original, &sources, false).await.unwrap();
        assert_eq!((report.added, report.updated, report.unchanged), (0, 0, 10));
        let after: Vec<(Uuid, chrono::DateTime<chrono::Utc>)> =
            sqlx::query_as("SELECT id,updated_at FROM vocabulary_entries ORDER BY id")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(before, after);
        assert_eq!(
            autocomplete(&pool, "garbanzo").await.unwrap()[0].label,
            "chickpea"
        );
        assert!(autocomplete(&pool, "%_").await.unwrap().is_empty());
        assert!(autocomplete(&pool, "x").await.unwrap().is_empty());
        assert!(autocomplete(&pool, &"a".repeat(201)).await.is_err());
        assert_eq!(
            seed(&pool, &file, &sources, false).await.unwrap().added,
            file.entries.len() - 10
        );
        assert_eq!(
            autocomplete(&pool, "tahin").await.unwrap()[0].label,
            "tahini"
        );
        assert_eq!(
            autocomplete(&pool, "sumac").await.unwrap()[0].label,
            "sumac"
        );
        // The comparison-key index supports prefix lookups without wildcard expansion.
        sqlx::query("SET enable_seqscan=off")
            .execute(&pool)
            .await
            .unwrap();
        let plan: Vec<(String,)> = sqlx::query_as("EXPLAIN SELECT id FROM vocabulary_entries WHERE dimension='ingredient' AND locale='en' AND status='active' AND normalized_label LIKE 'tah%'").fetch_all(&pool).await.unwrap();
        assert!(plan
            .iter()
            .any(|r| r.0.contains("idx_vocabulary_active_label")));
        let mut changed = file.clone();
        changed.entries[0].label = "chickpeas".into();
        changed.entries[0].aliases = vec!["chickpea".into()];
        assert_eq!(
            seed(&pool, &changed, &sources, false)
                .await
                .unwrap()
                .updated,
            1
        );
        assert_eq!(
            autocomplete(&pool, "chickpea").await.unwrap()[0].id,
            file.entries[0].id
        );
        assert!(autocomplete(&pool, "garbanzo").await.unwrap().is_empty());
        // A database constraint failure after an earlier update rolls everything back.
        sqlx::raw_sql("CREATE FUNCTION reject_seed_test() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.label='reject-write' THEN RAISE EXCEPTION 'test failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_seed_test BEFORE INSERT OR UPDATE ON vocabulary_entries FOR EACH ROW EXECUTE FUNCTION reject_seed_test();").execute(&pool).await.unwrap();
        let mut broken = changed.clone();
        broken.entries[0].label = "changed-before-failure".into();
        broken.entries[1].label = "reject-write".into();
        assert!(seed(&pool, &broken, &sources, false).await.is_err());
        assert_eq!(
            autocomplete(&pool, "chickpea").await.unwrap()[0].label,
            "chickpeas"
        );
        assert!(autocomplete(&pool, "changed-before-failure")
            .await
            .unwrap()
            .is_empty());
        let mut invalid = changed.clone();
        invalid.entries[0].id = Uuid::nil();
        assert!(seed(&pool, &invalid, &sources, false).await.is_err());
        assert!(
            seed(&pool, &original, &sources, false).await.is_err(),
            "omitted entries must not be deleted"
        );
        changed.entries[0].status = "retired".into();
        assert_eq!(
            seed(&pool, &changed, &sources, false)
                .await
                .unwrap()
                .retired,
            1
        );
        assert!(autocomplete(&pool, "chickpea").await.unwrap().is_empty());
        let still_present: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM vocabulary_entries WHERE id=$1 AND status='retired')",
        )
        .bind(file.entries[0].id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(still_present);
        let runs: i64 = sqlx::query_scalar("SELECT count(*) FROM vocabulary_seed_runs")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(runs, 4);
    }
}
