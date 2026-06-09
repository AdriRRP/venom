use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let state_path = std::env::var("VENOM_STATE_PATH")
        .unwrap_or_else(|_| "var/state/venom-state.jsonl".to_owned());
    let runtime_path = std::env::var("VENOM_RUNTIME_PATH")
        .unwrap_or_else(|_| "var/state/venom-runtime.jsonl".to_owned());
    let database_url = std::env::var("VENOM_DATABASE_URL").ok();
    let database_schema =
        std::env::var("VENOM_DATABASE_SCHEMA").unwrap_or_else(|_| "public".to_owned());
    let bind = std::env::var("VENOM_API_BIND").unwrap_or_else(|_| "127.0.0.1:3000".to_owned());
    if let Some(command) = args.next() {
        match command.as_str() {
            venom_api::LEGACY_REPAIR_BOOTSTRAP_COMMAND => {
                let database_url = database_url.ok_or_else(|| {
                    format!(
                        "VENOM_DATABASE_URL is required for {}",
                        venom_api::LEGACY_REPAIR_BOOTSTRAP_COMMAND
                    )
                })?;
                venom_api::bootstrap_postgres_legacy_repair_state(&database_url, &database_schema)
                    .await?;
                return Ok(());
            }
            other => return Err(format!("unknown command: {other}").into()),
        }
    }
    let state = if let Some(database_url) = database_url {
        venom_api::ApiState::open_postgres(&database_url, &database_schema).await?
    } else {
        venom_api::ApiState::open(state_path, runtime_path)?
    };
    let app = venom_api::build_router(state);
    let listener = tokio::net::TcpListener::bind(bind.parse::<SocketAddr>()?).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
