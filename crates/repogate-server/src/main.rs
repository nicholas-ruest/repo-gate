//! RepoGate HTTP server entry point.

use clap::Parser;
use repogate_server::db::create_pool;
use repogate_server::http::{build_router, AppState};
use tower_http::services::ServeDir;

#[derive(Parser, Debug)]
#[command(name = "repogate-server", about = "RepoGate HTTP API server")]
struct Args {
    /// Address to listen on.
    #[arg(long, default_value = "0.0.0.0:8080")]
    listen: String,

    /// Database URL (SQLite for dev, e.g. sqlite://./repogate.db).
    #[arg(long, default_value = "sqlite://./repogate.db")]
    database_url: String,

    /// Directory of the Next.js static export to serve.
    #[arg(long, default_value = "static")]
    static_dir: String,

    /// API key (Bearer). When unset, any Bearer token is accepted (dev mode).
    #[arg(long)]
    api_key: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::try_init().ok();
    let args = Args::parse();

    let pool = create_pool(&args.database_url).await?;
    let state = AppState::new(pool);

    let app = build_router(state).fallback_service(ServeDir::new(&args.static_dir));

    let listener = tokio::net::TcpListener::bind(&args.listen).await?;
    tracing::info!("listening on {}", args.listen);
    axum::serve(listener, app).await?;
    Ok(())
}
