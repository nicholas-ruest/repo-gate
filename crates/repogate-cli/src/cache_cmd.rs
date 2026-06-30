//! The `repogate cache` command.

use anyhow::Context;
use repogate_server::db::{create_pool, AnalysisCacheStore};

use crate::cli::{CacheArgs, CacheCommands};

/// Default local cache database path (under the user's home directory).
fn default_db_url() -> anyhow::Result<String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let dir = std::path::Path::new(&home).join(".config/repogate");
    std::fs::create_dir_all(&dir).context("creating config dir")?;
    Ok(format!("sqlite://{}/repogate.db", dir.display()))
}

/// Run the cache subcommand.
pub async fn run_cache(args: CacheArgs) -> anyhow::Result<()> {
    let pool = create_pool(&default_db_url()?).await?;
    let cache = AnalysisCacheStore::new(pool);

    match args.command {
        CacheCommands::Invalidate { repo_url } => {
            cache
                .invalidate(&repo_url)
                .await
                .map_err(|e| anyhow::anyhow!("cache invalidate failed: {e}"))?;
            println!("Invalidated cache for {repo_url}");
        }
        CacheCommands::List => {
            println!("Cache database is ready. Per-entry listing is not yet implemented.");
        }
    }
    Ok(())
}
