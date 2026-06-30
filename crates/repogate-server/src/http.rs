//! HTTP API: routes, handlers, bearer auth, and shared state (ADR-015).

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use axum::{
    extract::{Path, Request, State},
    http::StatusCode,
    middleware::{from_fn, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::db::job_store::SqlxAssessmentJobStore;
use repogate_orchestrator::job::{AssessmentJob, AssessmentJobStore};

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::SqlitePool,
    pub job_queue: Arc<Mutex<VecDeque<String>>>,
}

impl AppState {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self {
            pool,
            job_queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    fn job_store(&self) -> SqlxAssessmentJobStore {
        SqlxAssessmentJobStore::new(self.pool.clone())
    }
}

/// Request body for `POST /assessments`.
#[derive(Debug, Deserialize)]
pub struct AnalysisRequest {
    pub repo_url: String,
    pub budget_usd: f32,
    #[serde(default)]
    pub model_override: Option<String>,
    #[serde(default)]
    pub weights: Option<serde_json::Value>,
}

/// Response for a newly created assessment job.
#[derive(Debug, Serialize, Deserialize)]
pub struct JobResponse {
    pub job_id: String,
    pub estimated_cost_min: f32,
    pub estimated_cost_max: f32,
}

/// Polling response for job status.
#[derive(Debug, Serialize, Deserialize)]
pub struct StatusResponse {
    pub status: String,
    pub current_phase: String,
    pub progress_pct: u8,
    pub tokens_used: u64,
}

/// API error type.
#[derive(Debug)]
pub enum AppError {
    NotFound,
    BadRequest(String),
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            AppError::NotFound => (StatusCode::NOT_FOUND, "not_found", "not found".to_string()),
            AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, "bad_request", m),
            AppError::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, "internal", m),
        };
        (
            status,
            Json(serde_json::json!({ "error": message, "code": code })),
        )
            .into_response()
    }
}

/// Build the API router with auth applied (static serving is added in `main`).
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/assessments", post(create_assessment))
        .route(
            "/assessments/:id",
            get(get_assessment).delete(delete_assessment),
        )
        .route("/assessments/:id/status", get(get_assessment_status))
        .route("/assessments/:id/report", get(get_report))
        .route("/assessments/:id/report.pdf", get(get_report_pdf))
        .layer(from_fn(auth_middleware))
        .with_state(state)
}

async fn health() -> StatusCode {
    StatusCode::OK
}

fn estimate_cost(_repo_url: &str) -> (f32, f32) {
    (1.0, 15.0)
}

async fn create_assessment(
    State(state): State<AppState>,
    Json(req): Json<AnalysisRequest>,
) -> Result<Json<JobResponse>, AppError> {
    if req.repo_url.trim().is_empty() {
        return Err(AppError::BadRequest("repo_url is required".to_string()));
    }

    let job = AssessmentJob::new(&req.repo_url, "pending");
    let job_id = job.id.clone();
    state
        .job_store()
        .save(job)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    state
        .job_queue
        .lock()
        .map_err(|_| AppError::Internal("queue lock".to_string()))?
        .push_back(job_id.clone());

    let (estimated_cost_min, estimated_cost_max) = estimate_cost(&req.repo_url);
    Ok(Json(JobResponse {
        job_id,
        estimated_cost_min,
        estimated_cost_max,
    }))
}

async fn get_assessment(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let job = state
        .job_store()
        .find_by_id(&id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or(AppError::NotFound)?;
    Ok(Json(serde_json::to_value(&job).unwrap_or_default()))
}

async fn get_assessment_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<StatusResponse>, AppError> {
    let job = state
        .job_store()
        .find_by_id(&id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or(AppError::NotFound)?;

    let status = format!("{:?}", job.status).to_lowercase();
    let current_phase = job
        .current_phase
        .map(|p| format!("{p:?}").to_lowercase())
        .unwrap_or_else(|| "queued".to_string());
    Ok(Json(StatusResponse {
        status,
        current_phase,
        progress_pct: 0,
        tokens_used: job.tokens_used,
    }))
}

async fn get_report(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<String, AppError> {
    use sqlx::Row;
    let row = sqlx::query("SELECT markdown_content FROM reports WHERE job_id = ?1")
        .bind(&id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    match row {
        Some(r) => r
            .try_get::<Option<String>, _>("markdown_content")
            .map_err(|e| AppError::Internal(e.to_string()))?
            .ok_or(AppError::NotFound),
        None => Err(AppError::NotFound),
    }
}

async fn get_report_pdf(
    State(_state): State<AppState>,
    Path(_id): Path<String>,
) -> Result<Vec<u8>, AppError> {
    Err(AppError::NotFound)
}

async fn delete_assessment(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    sqlx::query("DELETE FROM jobs WHERE id = ?1")
        .bind(&id)
        .execute(&state.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

/// Bearer-token auth middleware. `/health` is exempt.
async fn auth_middleware(request: Request, next: Next) -> Response {
    if request.uri().path() == "/health" {
        return next.run(request).await;
    }
    let authorized = request
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.starts_with("Bearer "))
        .unwrap_or(false);

    if authorized {
        next.run(request).await
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "unauthorized", "code": "unauthorized" })),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request as HttpRequest;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn test_state() -> (tempfile::TempDir, AppState) {
        let dir = tempfile::tempdir().unwrap();
        let url = format!("sqlite://{}/test.db", dir.path().display());
        let pool = crate::db::create_pool(&url).await.unwrap();
        (dir, AppState::new(pool))
    }

    #[tokio::test]
    async fn health_ok_without_auth() {
        let (_dir, state) = test_state().await;
        let app = build_router(state);
        let resp = app
            .oneshot(
                HttpRequest::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn create_requires_auth() {
        let (_dir, state) = test_state().await;
        let app = build_router(state);
        let resp = app
            .oneshot(
                HttpRequest::builder()
                    .method("POST")
                    .uri("/assessments")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"repo_url":"https://github.com/a/b","budget_usd":5.0}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn submit_poll_fetch_flow() {
        let (_dir, state) = test_state().await;
        let app = build_router(state);

        // Submit.
        let resp = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .method("POST")
                    .uri("/assessments")
                    .header("content-type", "application/json")
                    .header("Authorization", "Bearer test-key")
                    .body(Body::from(
                        r#"{"repo_url":"https://github.com/a/b","budget_usd":5.0}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let job: JobResponse = serde_json::from_slice(&body).unwrap();
        assert!(!job.job_id.is_empty());

        // Poll status -> queued.
        let resp = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .uri(format!("/assessments/{}/status", job.job_id))
                    .header("Authorization", "Bearer test-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let status: StatusResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(status.status, "queued");

        // Fetch the job document.
        let resp = app
            .oneshot(
                HttpRequest::builder()
                    .uri(format!("/assessments/{}", job.job_id))
                    .header("Authorization", "Bearer test-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn unknown_job_status_is_404() {
        let (_dir, state) = test_state().await;
        let app = build_router(state);
        let resp = app
            .oneshot(
                HttpRequest::builder()
                    .uri("/assessments/nope/status")
                    .header("Authorization", "Bearer test-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
