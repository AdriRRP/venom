use crate::app::service::{
    self, ActiveFindingsResponse, AppReadSnapshot, AppService, BindArtifactRequest,
    BindArtifactResponse, ComponentRegistrationRequest, ConfigureIntegrationRuntimeRequest,
    ConfigureIntegrationRuntimeResponse, ConfigureProviderRequest, ConfigureProviderResponse,
    DrainIntegrationWorkerCommand, DrainIntegrationWorkerResponse, DrainWorkerCommand,
    DrainWorkerResponse, ProviderScanReportRequest, RecordProviderReportResponse,
    RegisterComponentResponse, RequestScanCommand, RequestScanResponse, RunNextScanCommand,
    RunNextScanResponse, ScanCommandStatusResponse,
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, watch};

#[derive(Clone)]
pub struct ApiState {
    inner: Arc<ApiStateInner>,
}

struct ApiStateInner {
    service: Mutex<AppService>,
    read_snapshot_tx: watch::Sender<Arc<AppReadSnapshot>>,
    read_snapshot_rx: watch::Receiver<Arc<AppReadSnapshot>>,
}

impl ApiState {
    /// Open the API state over one local durable state path.
    ///
    /// # Errors
    ///
    /// Returns an error string when the underlying durable state or runtime cannot be opened.
    pub fn open(
        state_path: impl Into<PathBuf>,
        runtime_path: impl Into<PathBuf>,
    ) -> Result<Self, String> {
        let service =
            AppService::open_local(state_path, runtime_path).map_err(|error| error.to_string())?;
        Ok(Self::new(service))
    }

    /// Open the API state over a Postgres durable backend.
    ///
    /// # Errors
    ///
    /// Returns an error string when the Postgres durable backend cannot be opened.
    pub async fn open_postgres(database_url: &str, schema: &str) -> Result<Self, String> {
        let service = AppService::open_postgres(database_url, schema)
            .await
            .map_err(|error| error.to_string())?;
        Ok(Self::new(service))
    }

    fn new(service: AppService) -> Self {
        let snapshot = Arc::new(service.read_snapshot());
        let (read_snapshot_tx, read_snapshot_rx) = watch::channel(snapshot);
        Self {
            inner: Arc::new(ApiStateInner {
                service: Mutex::new(service),
                read_snapshot_tx,
                read_snapshot_rx,
            }),
        }
    }

    fn read_snapshot(&self) -> Arc<AppReadSnapshot> {
        self.inner.read_snapshot_rx.borrow().clone()
    }

    fn refresh_snapshot(&self, service: &AppService) {
        self.inner
            .read_snapshot_tx
            .send_replace(Arc::new(service.read_snapshot()));
    }
}

pub fn build_router(state: ApiState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/components", post(register_component))
        .route("/components/{component_key}/artifacts", post(bind_artifact))
        .route(
            "/components/{component_key}/provider-runtime",
            post(configure_provider),
        )
        .route("/integration-runtime", post(configure_integration_runtime))
        .route("/scan-requests", post(request_scan))
        .route("/scan-commands/{command_id}", get(scan_command_status))
        .route("/scan-workers/run-next", post(run_next_scan))
        .route("/scan-workers/drain", post(drain_worker))
        .route("/integration-workers/drain", post(drain_integration_worker))
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
    let response = {
        let mut service = state.inner.service.lock().await;
        let response = service
            .register_component(request)
            .await
            .map_err(ApiError::from)?;
        state.refresh_snapshot(&service);
        response
    };
    Ok(Json(response))
}

async fn bind_artifact(
    State(state): State<ApiState>,
    Path(component_key): Path<String>,
    Json(request): Json<BindArtifactRequest>,
) -> Result<Json<BindArtifactResponse>, ApiError> {
    let response = {
        let mut service = state.inner.service.lock().await;
        let response = service
            .bind_artifact(&component_key, request)
            .await
            .map_err(ApiError::from)?;
        state.refresh_snapshot(&service);
        response
    };
    Ok(Json(response))
}

async fn configure_provider(
    State(state): State<ApiState>,
    Path(component_key): Path<String>,
    Json(request): Json<ConfigureProviderRequest>,
) -> Result<Json<ConfigureProviderResponse>, ApiError> {
    let response = {
        let mut service = state.inner.service.lock().await;
        let response = service
            .configure_provider(&component_key, request)
            .await
            .map_err(ApiError::from)?;
        state.refresh_snapshot(&service);
        response
    };
    Ok(Json(response))
}

async fn configure_integration_runtime(
    State(state): State<ApiState>,
    Json(request): Json<ConfigureIntegrationRuntimeRequest>,
) -> Result<Json<ConfigureIntegrationRuntimeResponse>, ApiError> {
    let response = {
        let mut service = state.inner.service.lock().await;
        let response = service
            .configure_integration_runtime(request)
            .await
            .map_err(ApiError::from)?;
        state.refresh_snapshot(&service);
        response
    };
    Ok(Json(response))
}

async fn record_provider_report(
    State(state): State<ApiState>,
    Json(request): Json<ProviderScanReportRequest>,
) -> Result<Json<RecordProviderReportResponse>, ApiError> {
    let response = {
        let mut service = state.inner.service.lock().await;
        let response = service
            .record_provider_report(request)
            .await
            .map_err(ApiError::from)?;
        state.refresh_snapshot(&service);
        response
    };
    Ok(Json(response))
}

async fn request_scan(
    State(state): State<ApiState>,
    Json(request): Json<RequestScanCommand>,
) -> Result<Json<RequestScanResponse>, ApiError> {
    let response = {
        let mut service = state.inner.service.lock().await;
        let response = service
            .request_scan(request)
            .await
            .map_err(ApiError::from)?;
        state.refresh_snapshot(&service);
        response
    };
    Ok(Json(response))
}

async fn scan_command_status(
    State(state): State<ApiState>,
    Path(command_id): Path<String>,
) -> Result<Json<ScanCommandStatusResponse>, ApiError> {
    let response = state
        .read_snapshot()
        .scan_command_status(&command_id)
        .map_err(ApiError::from)?;
    Ok(Json(response))
}

async fn run_next_scan(
    State(state): State<ApiState>,
    Json(request): Json<RunNextScanCommand>,
) -> Result<Json<RunNextScanResponse>, ApiError> {
    let response = {
        let mut service = state.inner.service.lock().await;
        let response = service
            .run_next_scan(request)
            .await
            .map_err(ApiError::from)?;
        state.refresh_snapshot(&service);
        response
    };
    Ok(Json(response))
}

async fn drain_worker(
    State(state): State<ApiState>,
    Json(request): Json<DrainWorkerCommand>,
) -> Result<Json<DrainWorkerResponse>, ApiError> {
    let response = {
        let mut service = state.inner.service.lock().await;
        let response = service
            .run_worker_until_idle(request)
            .await
            .map_err(ApiError::from)?;
        state.refresh_snapshot(&service);
        response
    };
    Ok(Json(response))
}

async fn drain_integration_worker(
    State(state): State<ApiState>,
    Json(request): Json<DrainIntegrationWorkerCommand>,
) -> Result<Json<DrainIntegrationWorkerResponse>, ApiError> {
    let response = {
        let mut service = state.inner.service.lock().await;
        let response = service
            .publish_integration_events_until_idle(request)
            .await
            .map_err(ApiError::from)?;
        state.refresh_snapshot(&service);
        response
    };
    Ok(Json(response))
}

async fn list_active_findings(
    State(state): State<ApiState>,
    Query(query): Query<ActiveFindingsQuery>,
) -> Result<Json<ActiveFindingsResponse>, ApiError> {
    let response = state
        .read_snapshot()
        .list_active_findings(query.into_request())
        .map_err(ApiError::from)?;
    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
struct ActiveFindingsQuery {
    component_key: String,
    artifact_kind: String,
    artifact_identity: String,
    min_severity: Option<String>,
    package_name: Option<String>,
    offset: Option<usize>,
    limit: Option<usize>,
}

impl ActiveFindingsQuery {
    fn into_request(self) -> service::ActiveFindingsRequest {
        service::ActiveFindingsRequest {
            component_key: self.component_key,
            artifact_kind: self.artifact_kind,
            artifact_identity: self.artifact_identity,
            min_severity: self.min_severity,
            package_name: self.package_name,
            offset: self.offset,
            limit: self.limit,
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
            service::AppServiceError::NotFound(message) => Self {
                status: StatusCode::NOT_FOUND,
                message,
            },
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

    fn temp_path(name: &str, suffix: &str) -> std::path::PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("current time should be after unix epoch")
            .as_nanos();
        let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("venom-api-{name}-{suffix}-{nanos}-{counter}.jsonl"))
    }

    fn temp_schema(name: &str) -> String {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("current time should be after unix epoch")
            .as_nanos();
        let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
        format!("venom_{name}_{nanos}_{counter}")
    }

    fn postgres_test_url() -> Option<String> {
        std::env::var("VENOM_TEST_POSTGRES_URL").ok()
    }

    #[tokio::test]
    async fn api_registers_binds_reports_and_queries_active_findings() {
        let router = build_router(
            ApiState::open(
                temp_path("integration", "state"),
                temp_path("integration", "runtime"),
            )
            .expect("api state should open"),
        );

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

        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = record_provider_report(router.clone()).await;
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

    #[tokio::test]
    async fn api_queries_active_findings_with_filter_and_page_metadata() {
        let router = build_router(
            ApiState::open(
                temp_path("active-findings-query", "state"),
                temp_path("active-findings-query", "runtime"),
            )
            .expect("api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = record_provider_report_with_two_findings(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = router
            .oneshot(
                Request::get(
                    "/findings/active?component_key=component:payments-api&artifact_kind=container-image&artifact_identity=registry.example/payments@sha256:111&min_severity=high&limit=1&offset=0",
                )
                .body(Body::empty())
                .expect("request should build"),
            )
            .await
            .expect("query request should succeed");
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["total_active_findings"], 1);
        assert_eq!(payload["returned"], 1);
        assert_eq!(payload["limit"], 1);
        assert_eq!(payload["offset"], 0);
        assert_eq!(
            payload["active_findings"][0]["vulnerability_id"],
            "CVE-2026-0001"
        );
    }

    #[tokio::test]
    async fn api_enqueues_scan_requests_and_exposes_pending_status() {
        let router = build_router(
            ApiState::open(
                temp_path("scan-request", "state"),
                temp_path("scan-request", "runtime"),
            )
            .expect("api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = enqueue_scan_request(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        let command_id = payload["command_id"]
            .as_str()
            .expect("command id should be present")
            .to_owned();
        assert_eq!(payload["status"], "pending");

        let response = router
            .oneshot(
                Request::get(format!("/scan-commands/{command_id}"))
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("status request should succeed");
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn api_runs_next_scan_and_marks_command_completed() {
        let router = build_router(
            ApiState::open(
                temp_path("run-next", "state"),
                temp_path("run-next", "runtime"),
            )
            .expect("api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = configure_fixture_provider(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = enqueue_scan_request(router.clone()).await;
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        let command_id = payload["command_id"]
            .as_str()
            .expect("command id should be present")
            .to_owned();

        let response = run_next_scan_with_fixture(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = router
            .clone()
            .oneshot(
                Request::get(format!("/scan-commands/{command_id}"))
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("status request should succeed");
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["status"], "completed");
    }

    #[tokio::test]
    async fn api_drains_pending_scan_commands_until_idle() {
        let router = build_router(
            ApiState::open(
                temp_path("drain-worker", "state"),
                temp_path("drain-worker", "runtime"),
            )
            .expect("api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = configure_fixture_provider(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = configure_fixture_integration_runtime(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = enqueue_scan_request(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = enqueue_scan_request(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = drain_worker_with_fixture(router.clone(), 8).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["outcome"], "drained");
        assert_eq!(payload["processed"], 2);
        assert_eq!(payload["completed"], 2);
        assert_eq!(payload["failed"], 0);
        assert_eq!(payload["pending_remaining"], 0);
    }

    #[tokio::test]
    async fn api_drains_pending_integration_events_from_state_and_runtime() {
        let router = build_router(
            ApiState::open(
                temp_path("drain-integration-worker", "state"),
                temp_path("drain-integration-worker", "runtime"),
            )
            .expect("api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = configure_fixture_provider(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = configure_fixture_integration_runtime(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = enqueue_scan_request(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = drain_worker_with_fixture(router.clone(), 8).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = drain_integration_worker_with_success(router.clone(), 8).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["outcome"], "drained");
        assert_eq!(payload["attempted"], 2);
        assert_eq!(payload["published"], 2);
        assert_eq!(payload["pending_remaining"], 0);
        assert_eq!(payload["last_event_kind"], "scan-command-completed");
        assert!(payload["last_error"].is_null());
    }

    #[tokio::test]
    async fn api_keeps_pending_integration_events_on_publication_failure() {
        let router = build_router(
            ApiState::open(
                temp_path("fail-integration-worker", "state"),
                temp_path("fail-integration-worker", "runtime"),
            )
            .expect("api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = configure_fixture_provider(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = configure_fixture_integration_runtime(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = enqueue_scan_request(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = drain_worker_with_fixture(router.clone(), 8).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = drain_integration_worker_with_failure(
            router.clone(),
            8,
            "fixture publish failed",
            true,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["outcome"], "limited");
        assert_eq!(payload["attempted"], 1);
        assert_eq!(payload["published"], 0);
        assert_eq!(payload["pending_remaining"], 2);
        assert_eq!(payload["last_event_kind"], "finding-changes-observed");
        assert_eq!(payload["last_error"], "fixture publish failed");
        assert_eq!(payload["last_retryable"], true);
    }

    #[tokio::test]
    async fn api_exposes_http_publisher_transport_failure_explicitly() {
        let router = build_router(
            ApiState::open(
                temp_path("http-integration-worker", "state"),
                temp_path("http-integration-worker", "runtime"),
            )
            .expect("api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = configure_fixture_provider(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response =
            configure_http_integration_runtime(router.clone(), "http://127.0.0.1:9/publish", 3_000)
                .await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = enqueue_scan_request(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = drain_worker_with_fixture(router.clone(), 8).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = drain_integration_worker_with_success(router.clone(), 8).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["outcome"], "limited");
        assert_eq!(payload["attempted"], 1);
        assert_eq!(payload["published"], 0);
        assert_eq!(payload["pending_remaining"], 2);
        assert_eq!(payload["last_retryable"], true);
        assert!(payload["last_error"].as_str().is_some());
    }

    #[tokio::test]
    async fn api_rejects_fixture_failure_controls_for_http_publisher() {
        let router = build_router(
            ApiState::open(
                temp_path("http-integration-failure", "state"),
                temp_path("http-integration-failure", "runtime"),
            )
            .expect("api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = configure_fixture_provider(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response =
            configure_http_integration_runtime(router.clone(), "http://127.0.0.1:9/publish", 3_000)
                .await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = enqueue_scan_request(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = drain_worker_with_fixture(router.clone(), 8).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = drain_integration_worker_with_failure(
            router.clone(),
            8,
            "fixture publish failed",
            true,
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(
            payload["error"],
            "http publisher does not accept fixture failure controls"
        );
    }

    #[tokio::test]
    async fn postgres_backend_reloads_findings_and_scan_status() {
        let Some(database_url) = postgres_test_url() else {
            return;
        };
        let schema = temp_schema("reload");
        let router = build_router(
            ApiState::open_postgres(&database_url, &schema)
                .await
                .expect("postgres api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = configure_fixture_provider(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = configure_fixture_integration_runtime(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = enqueue_scan_request(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        let command_id = payload["command_id"]
            .as_str()
            .expect("command id should be present")
            .to_owned();

        let response = run_next_scan_with_fixture(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let reloaded = build_router(
            ApiState::open_postgres(&database_url, &schema)
                .await
                .expect("postgres api state should reopen"),
        );

        let response = reloaded
            .clone()
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
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["active_findings"].as_array().map_or(0, Vec::len), 1);

        let response = reloaded
            .oneshot(
                Request::get(format!("/scan-commands/{command_id}"))
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("status request should succeed");
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["status"], "completed");
    }

    #[tokio::test]
    async fn postgres_worker_loop_drains_until_idle() {
        let Some(database_url) = postgres_test_url() else {
            return;
        };
        let schema = temp_schema("drain");
        let router = build_router(
            ApiState::open_postgres(&database_url, &schema)
                .await
                .expect("postgres api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = configure_fixture_provider(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = configure_fixture_integration_runtime(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = enqueue_scan_request(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = enqueue_scan_request(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = drain_worker_with_fixture(router.clone(), 8).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["outcome"], "drained");
        assert_eq!(payload["completed"], 2);
        assert_eq!(payload["pending_remaining"], 0);
    }

    #[tokio::test]
    async fn postgres_integration_publication_worker_is_bounded_and_durable() {
        let Some(database_url) = postgres_test_url() else {
            return;
        };
        let schema = temp_schema("publish");
        let router = build_router(
            ApiState::open_postgres(&database_url, &schema)
                .await
                .expect("postgres api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = configure_fixture_provider(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = configure_fixture_integration_runtime(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = enqueue_scan_request(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = drain_worker_with_fixture(router.clone(), 8).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = drain_integration_worker_with_success(router.clone(), 1).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["outcome"], "limited");
        assert_eq!(payload["attempted"], 1);
        assert_eq!(payload["published"], 1);
        assert_eq!(payload["pending_remaining"], 1);
        assert_eq!(payload["last_event_kind"], "finding-changes-observed");

        let reloaded = build_router(
            ApiState::open_postgres(&database_url, &schema)
                .await
                .expect("postgres api state should reopen"),
        );

        let response = drain_integration_worker_with_success(reloaded.clone(), 8).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["outcome"], "drained");
        assert_eq!(payload["attempted"], 1);
        assert_eq!(payload["published"], 1);
        assert_eq!(payload["pending_remaining"], 0);
        assert_eq!(payload["last_event_kind"], "scan-command-completed");
    }

    #[tokio::test]
    async fn postgres_integration_runtime_reloads_and_publishes_over_http() {
        let Some(database_url) = postgres_test_url() else {
            return;
        };
        let schema = temp_schema("publish_http");
        let router = build_router(
            ApiState::open_postgres(&database_url, &schema)
                .await
                .expect("postgres api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = configure_fixture_provider(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response =
            configure_http_integration_runtime(router.clone(), "http://127.0.0.1:9/publish", 3_000)
                .await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = enqueue_scan_request(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = drain_worker_with_fixture(router.clone(), 8).await;
        assert_eq!(response.status(), StatusCode::OK);

        let reloaded = build_router(
            ApiState::open_postgres(&database_url, &schema)
                .await
                .expect("postgres api state should reopen"),
        );

        let response = drain_integration_worker_with_success(reloaded.clone(), 8).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["outcome"], "limited");
        assert_eq!(payload["attempted"], 1);
        assert_eq!(payload["published"], 0);
        assert_eq!(payload["pending_remaining"], 2);
        assert_eq!(payload["last_retryable"], true);
        assert!(payload["last_error"].as_str().is_some());
    }

    async fn bind_owned_artifact(router: axum::Router) -> axum::response::Response {
        router
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
            .expect("bind request should succeed")
    }

    async fn register_payments_component(router: axum::Router) -> axum::response::Response {
        router
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
            .expect("register request should succeed")
    }

    async fn configure_fixture_provider(router: axum::Router) -> axum::response::Response {
        router
            .oneshot(
                Request::post("/components/component:payments-api/provider-runtime")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "provider_key": "fixture-provider"
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("configure provider request should succeed")
    }

    async fn configure_fixture_integration_runtime(
        router: axum::Router,
    ) -> axum::response::Response {
        router
            .oneshot(
                Request::post("/integration-runtime")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "publisher_key": "fixture-publisher"
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("configure integration runtime request should succeed")
    }

    async fn configure_http_integration_runtime(
        router: axum::Router,
        endpoint_url: &str,
        timeout_ms: u32,
    ) -> axum::response::Response {
        router
            .oneshot(
                Request::post("/integration-runtime")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "publisher_key": "http-publisher",
                            "endpoint_url": endpoint_url,
                            "timeout_ms": timeout_ms
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("configure integration runtime request should succeed")
    }

    async fn record_provider_report(router: axum::Router) -> axum::response::Response {
        router
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
            .expect("provider report request should succeed")
    }

    async fn record_provider_report_with_two_findings(
        router: axum::Router,
    ) -> axum::response::Response {
        router
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
                            "knowledge_revision": "fixture-db:2026-05-16",
                            "findings": [
                                {
                                    "vulnerability_id": "CVE-2026-0001",
                                    "package_name": "openssl",
                                    "package_version": "3.0.0",
                                    "severity": "critical"
                                },
                                {
                                    "vulnerability_id": "CVE-2026-0002",
                                    "package_name": "busybox",
                                    "package_version": "1.36.0",
                                    "severity": "low"
                                }
                            ]
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("provider report request should succeed")
    }

    async fn enqueue_scan_request(router: axum::Router) -> axum::response::Response {
        router
            .oneshot(
                Request::post("/scan-requests")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "component_key": "component:payments-api",
                            "artifact_kind": "container-image",
                            "artifact_identity": "registry.example/payments@sha256:111",
                            "freshness": "deterministic"
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("scan request should succeed")
    }

    async fn run_next_scan_with_fixture(router: axum::Router) -> axum::response::Response {
        router
            .oneshot(
                Request::post("/scan-workers/run-next")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
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
            .expect("run-next request should succeed")
    }

    async fn drain_worker_with_fixture(
        router: axum::Router,
        max_commands: usize,
    ) -> axum::response::Response {
        router
            .oneshot(
                Request::post("/scan-workers/drain")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "max_commands": max_commands,
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
            .expect("drain request should succeed")
    }

    async fn drain_integration_worker_with_success(
        router: axum::Router,
        max_events: usize,
    ) -> axum::response::Response {
        router
            .oneshot(
                Request::post("/integration-workers/drain")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "max_events": max_events
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("integration drain request should succeed")
    }

    async fn drain_integration_worker_with_failure(
        router: axum::Router,
        max_events: usize,
        error_message: &str,
        retryable: bool,
    ) -> axum::response::Response {
        router
            .oneshot(
                Request::post("/integration-workers/drain")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "max_events": max_events,
                            "error_message": error_message,
                            "retryable": retryable
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("integration drain request should succeed")
    }
}
