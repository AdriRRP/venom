#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = std::env::var("VENOM_DATABASE_URL").map_err(|_| {
        "VENOM_DATABASE_URL is required for bootstrap-legacy-repair-state".to_owned()
    })?;
    let database_schema =
        std::env::var("VENOM_DATABASE_SCHEMA").unwrap_or_else(|_| "public".to_owned());
    venom_api::bootstrap_postgres_legacy_repair_state(&database_url, &database_schema).await?;
    Ok(())
}
