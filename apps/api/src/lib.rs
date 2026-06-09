mod app;
mod http;
mod infra;

pub use http::{ApiState, build_router};

pub const LEGACY_REPAIR_BOOTSTRAP_COMMAND: &str = "bootstrap-legacy-repair-state";

/// Run one explicit legacy bootstrap seeding pass for canonical Postgres repair
/// state.
///
/// # Errors
///
/// Returns an error string when Postgres cannot be reached or legacy repair
/// state cannot be seeded.
pub async fn bootstrap_postgres_legacy_repair_state(
    database_url: &str,
    schema: &str,
) -> Result<(), String> {
    infra::postgres_backend::PostgresStore::bootstrap_legacy_repair_state(database_url, schema)
        .await
}
