use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state_path = std::env::var("VENOM_STATE_PATH")
        .unwrap_or_else(|_| "var/state/venom-state.jsonl".to_owned());
    let runtime_path = std::env::var("VENOM_RUNTIME_PATH")
        .unwrap_or_else(|_| "var/state/venom-runtime.jsonl".to_owned());
    let database_url = std::env::var("VENOM_DATABASE_URL").ok();
    let database_schema =
        std::env::var("VENOM_DATABASE_SCHEMA").unwrap_or_else(|_| "public".to_owned());
    let bind = std::env::var("VENOM_API_BIND").unwrap_or_else(|_| "127.0.0.1:3000".to_owned());
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
