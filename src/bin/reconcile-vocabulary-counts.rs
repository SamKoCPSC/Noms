//! Rebuild ingredient popularity after schema rollout or repair counter drift.
#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Vocabulary count reconciliation failed: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut dry_run = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--dry-run" => dry_run = true,
            "--help" | "-h" => {
                println!("reconcile-vocabulary-counts [--dry-run]\nRequires deployment-role DATABASE_URL and an already-applied schema.");
                return Ok(());
            }
            _ => return Err(format!("Unknown argument: {arg}").into()),
        }
    }
    let url = std::env::var("DATABASE_URL").map_err(|_| "DATABASE_URL is required")?;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .connect(&url)
        .await?;
    let report = noms::vocabulary::store::reconcile_counts(&pool, dry_run).await?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    pool.close().await;
    Ok(())
}
