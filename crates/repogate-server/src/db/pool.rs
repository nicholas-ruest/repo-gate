//! SQLite connection pool creation and migration (ADR-014).
//!
//! Uses runtime sqlx queries (not the compile-time macros) so the crate builds
//! without a live database or an offline cache. SQLite backs dev/CLI; Postgres
//! support is a future addition behind the same store traits.

use std::str::FromStr;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;

/// Create a SQLite pool for `database_url` (e.g. `sqlite://./repogate.db`),
/// creating the database if missing and running all migrations.
pub async fn create_pool(database_url: &str) -> anyhow::Result<SqlitePool> {
    let options = SqliteConnectOptions::from_str(database_url)?.create_if_missing(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}
