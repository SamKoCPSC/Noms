//! Explicit data-plane command; never runs automatically at server startup.
use noms::vocabulary::{self, SeedFile, SourceManifest};

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Vocabulary seed failed: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut file = "data/vocabulary/ingredients.json".to_string();
    let mut sources = "data/vocabulary/sources.json".to_string();
    let mut dry_run = false;
    let mut validate_only = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--dry-run" => dry_run = true,
            "--validate-only" => validate_only = true,
            "--file" => file = args.next().ok_or("--file requires a path")?,
            "--sources" => sources = args.next().ok_or("--sources requires a path")?,
            "--help" | "-h" => {
                println!("seed-vocabulary [--dry-run | --validate-only] [--file PATH] [--sources PATH]\n--validate-only checks files without connecting to a database. Other modes require DATABASE_URL and an already-applied schema.");
                return Ok(());
            }
            _ => return Err(format!("Unknown argument: {arg}").into()),
        }
    }
    let seed: SeedFile = serde_json::from_slice(&std::fs::read(file)?)?;
    let sources: SourceManifest = serde_json::from_slice(&std::fs::read(sources)?)?;
    if dry_run && validate_only {
        return Err("--dry-run and --validate-only are mutually exclusive".into());
    }
    let warnings = vocabulary::validate(&seed, &sources)?;
    if validate_only {
        println!(
            "{}",
            serde_json::json!({
                "valid": true,
                "entries": seed.entries.len(),
                "sources": sources.sources.len(),
                "warnings": warnings,
            })
        );
        return Ok(());
    }
    let url = std::env::var("DATABASE_URL").map_err(|_| "DATABASE_URL is required")?;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .connect(&url)
        .await?;
    let report = vocabulary::store::seed(&pool, &seed, &sources, dry_run).await?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    pool.close().await;
    Ok(())
}
