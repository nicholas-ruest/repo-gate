# P15 — `repogate-server`: `axum` HTTP Server, API Endpoints, Static Serving

## Context

**You are implementing the HTTP API server: REST endpoints, job management, report serving.**

**Prerequisites:** P11 (orchestration), P12 (report), P13 (stores) are complete.

---

## Phase & Dependencies

- **Phase:** UX
- **Depends on:** P11, P12, P13

---

## Scope & Deliverables

Implement `repogate-server/src/main.rs` and route handlers.

### File: `src/main.rs`

```rust
use axum::{
    extract::{Path, Json, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, delete},
    Router,
};
use std::sync::{Arc, Mutex};
use tower_http::services::ServeDir;

#[derive(Clone)]
struct AppState {
    pool: sqlx::AnyPool,
    pipeline_runner: Arc<PipelineRunner>,
    job_queue: Arc<Mutex<VecDeque<String>>>,
}

#[derive(serde::Deserialize)]
struct AnalysisRequest {
    repo_url: String,
    budget_usd: f32,
    model_override: Option<String>,
    weights: Option<serde_json::Value>,
}

#[derive(serde::Serialize)]
struct JobResponse {
    job_id: String,
    estimated_cost_min: f32,
    estimated_cost_max: f32,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let pool = create_pool(&args.database_url).await?;
    
    let app_state = AppState {
        pool,
        pipeline_runner: Arc::new(PipelineRunner::new(/* ... */)),
        job_queue: Arc::new(Mutex::new(VecDeque::new())),
    };
    
    let app = Router::new()
        .route("/health", get(health))
        .route("/assessments", post(create_assessment))
        .route("/assessments/:id", get(get_assessment))
        .route("/assessments/:id/status", get(get_assessment_status))
        .route("/assessments/:id/report", get(get_report))
        .route("/assessments/:id/report.pdf", get(get_report_pdf))
        .route("/assessments/:id", delete(delete_assessment))
        .nest_service("/", ServeDir::new("static"))
        .layer(axum::middleware::Next::layer(auth_middleware))
        .with_state(app_state)
        .fallback(|| async { (StatusCode::NOT_FOUND, "404") })
        .into_make_service_with_connect_info::<std::net::SocketAddr>();
    
    let listener = tokio::net::TcpListener::bind(&args.listen).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}

async fn health() -> StatusCode {
    StatusCode::OK
}

async fn create_assessment(
    State(state): State<AppState>,
    Json(req): Json<AnalysisRequest>,
) -> Result<Json<JobResponse>, AppError> {
    let job_id = uuid::Uuid::new_v4().to_string();
    
    // Estimate cost
    let (min, max) = estimate_cost(&req.repo_url).await?;
    
    // Queue job
    state.job_queue.lock().unwrap().push_back(job_id.clone());
    
    Ok(Json(JobResponse {
        job_id,
        estimated_cost_min: min,
        estimated_cost_max: max,
    }))
}

async fn get_assessment(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Fetch from DB
    Ok(Json(serde_json::json!({})))
}

async fn get_assessment_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<StatusResponse>, AppError> {
    Ok(Json(StatusResponse {
        status: "queued".to_string(),
        current_phase: "ingesting".to_string(),
        progress_pct: 10,
        tokens_used: 0,
    }))
}

async fn get_report(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<String, AppError> {
    // Fetch markdown from DB
    Ok("# Report".to_string())
}

async fn get_report_pdf(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Vec<u8>, AppError> {
    Err(AppError::NotFound)
}

async fn delete_assessment(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> StatusCode {
    StatusCode::NO_CONTENT
}

async fn auth_middleware(
    request: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::http::Response<axum::body::Body> {
    if request.uri().path() == "/health" {
        return next.run(request).await;
    }
    
    if let Some(auth) = request.headers().get("Authorization") {
        if let Ok(auth_str) = auth.to_str() {
            if auth_str.starts_with("Bearer ") {
                return next.run(request).await;
            }
        }
    }
    
    axum::http::Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .body(axum::body::Body::from("Unauthorized"))
        .unwrap()
}

#[derive(serde::Serialize)]
struct StatusResponse {
    status: String,
    current_phase: String,
    progress_pct: u8,
    tokens_used: u64,
}

#[derive(Debug)]
enum AppError {
    NotFound,
    InternalError(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        match self {
            AppError::NotFound => (StatusCode::NOT_FOUND, "Not found").into_response(),
            AppError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response(),
        }
    }
}

#[derive(clap::Parser)]
struct Args {
    #[arg(long, default_value = "0.0.0.0:8080")]
    listen: String,
    
    #[arg(long, default_value = "sqlite://repogate.db")]
    database_url: String,
    
    #[arg(long, default_value = "static")]
    static_dir: String,
    
    #[arg(long)]
    api_key: Option<String>,
}

async fn estimate_cost(repo_url: &str) -> Result<(f32, f32), AppError> {
    Ok((1.0, 15.0))
}
```

---

## Source Documents to Read

- **`docs/adr/ADR-015-web-api-layer-axum-nextjs.md`** — axum endpoints, bearer auth, polling, static export

---

## Acceptance Criteria

- ✅ `cargo build -p repogate-server` produces executable
- ✅ `POST /assessments` with valid body → 200 JSON
- ✅ `POST /assessments` without `Authorization` → 401
- ✅ `GET /assessments/:id/status` → `{status: "queued"}`
- ✅ `GET /health` → 200 without auth
- ✅ Integration test: submit → poll → fetch

---

## Language

**Rust** — axum, async HTTP, bearer auth, static file serving.

---

## Out-of-Scope

- Do NOT implement WebSocket (polling is MVP)
- Do NOT implement job history pagination
