pub mod service;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use service::{
    ActiveFindingsResponse, AppService, BindArtifactRequest, BindArtifactResponse,
    ComponentRegistrationRequest, ProviderScanReportRequest, RecordProviderReportResponse,
    RegisterComponentResponse,
};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct ApiState {
    service: Arc<Mutex<AppService>>,
}

impl ApiState {
    /// Open the API state over one local durable state path.
    ///
    /// # Errors
    ///
    /// Returns an error string when the underlying durable state cannot be opened.
    pub fn open(state_path: impl Into<PathBuf>) -> Result<Self, String> {
        let service = AppService::open(state_path).map_err(|error| error.to_string())?;
        Ok(Self {
            service: Arc::new(Mutex::new(service)),
        })
    }
}

pub fn build_router(state: ApiState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/components", post(register_component))
        .route("/components/{component_key}/artifacts", post(bind_artifact))
        .route("/provider-reports", post(record_provider_report))
        .route("/findings/active", get(list_active_findings))
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}

async fn register_component(
    State(state): State<ApiState>,
    Json(request): Json<ComponentRegistrationRequest>,
) -> Result<Json<RegisterComponentResponse>, ApiError> {
    let response = state
        .service
        .lock()
        .expect("api service mutex should not be poisoned")
        .register_component(request)
        .map_err(ApiError::from)?;
    Ok(Json(response))
}

async fn bind_artifact(
    State(state): State<ApiState>,
    Path(component_key): Path<String>,
    Json(request): Json<BindArtifactRequest>,
) -> Result<Json<BindArtifactResponse>, ApiError> {
    let response = state
        .service
        .lock()
        .expect("api service mutex should not be poisoned")
        .bind_artifact(&component_key, request)
        .map_err(ApiError::from)?;
    Ok(Json(response))
}

async fn record_provider_report(
    State(state): State<ApiState>,
    Json(request): Json<ProviderScanReportRequest>,
) -> Result<Json<RecordProviderReportResponse>, ApiError> {
    let response = state
        .service
        .lock()
        .expect("api service mutex should not be poisoned")
        .record_provider_report(request)
        .map_err(ApiError::from)?;
    Ok(Json(response))
}

async fn list_active_findings(
    State(state): State<ApiState>,
    Query(query): Query<ActiveFindingsQuery>,
) -> Result<Json<ActiveFindingsResponse>, ApiError> {
    let response = state
        .service
        .lock()
        .expect("api service mutex should not be poisoned")
        .list_active_findings(query.into_request())
        .map_err(ApiError::from)?;
    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
struct ActiveFindingsQuery {
    component_key: String,
    artifact_kind: String,
    artifact_identity: String,
}

impl ActiveFindingsQuery {
    fn into_request(self) -> service::ActiveFindingsRequest {
        service::ActiveFindingsRequest {
            component_key: self.component_key,
            artifact_kind: self.artifact_kind,
            artifact_identity: self.artifact_identity,
        }
    }
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: String,
}

struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }
}

impl From<service::AppServiceError> for ApiError {
    fn from(value: service::AppServiceError) -> Self {
        match value {
            service::AppServiceError::InvalidRequest(message) => Self::bad_request(message),
            service::AppServiceError::State(message) => Self::internal(message),
        }
    }
}

impl axum::response::IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (
            self.status,
            Json(ErrorBody {
                error: self.message,
            }),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::ApiState;
    use super::build_router;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use serde_json::json;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    use tower::util::ServiceExt;

    fn temp_path(name: &str) -> std::path::PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("current time should be after unix epoch")
            .as_nanos();
        let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("venom-api-{name}-{nanos}-{counter}.jsonl"))
    }

    #[tokio::test]
    async fn api_registers_binds_reports_and_queries_active_findings() {
        let router =
            build_router(ApiState::open(temp_path("integration")).expect("api state should open"));

        let response = router
            .clone()
            .oneshot(
                Request::post("/components")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "component_key": "component:payments-api",
                            "name": "Payments API"
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("register request should succeed");
        assert_eq!(response.status(), StatusCode::OK);

        let response = router
            .clone()
            .oneshot(
                Request::post("/components/component:payments-api/artifacts")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "artifact_kind": "container-image",
                            "artifact_identity": "registry.example/payments@sha256:111"
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("bind request should succeed");
        assert_eq!(response.status(), StatusCode::OK);

        let response = router
            .clone()
            .oneshot(
                Request::post("/provider-reports")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "provider_key": "fixture-provider",
                            "component_key": "component:payments-api",
                            "artifact_kind": "container-image",
                            "artifact_identity": "registry.example/payments@sha256:111",
                            "freshness": "deterministic",
                            "knowledge_revision": "fixture-db:2026-05-14",
                            "findings": [
                                {
                                    "vulnerability_id": "CVE-2026-0001",
                                    "package_name": "openssl",
                                    "package_version": "3.0.0",
                                    "severity": "high"
                                }
                            ]
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("provider report request should succeed");
        assert_eq!(response.status(), StatusCode::OK);

        let response = router
            .oneshot(
                Request::get(
                    "/findings/active?component_key=component:payments-api&artifact_kind=container-image&artifact_identity=registry.example/payments@sha256:111",
                )
                .body(Body::empty())
                .expect("request should build"),
            )
            .await
            .expect("query request should succeed");
        assert_eq!(response.status(), StatusCode::OK);
    }
}
