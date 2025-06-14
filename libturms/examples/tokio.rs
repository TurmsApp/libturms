use libturms::discover::*;
use tracing_subscriber::prelude::*;

const LOCAL_URL: &str = "http://localhost:4000";

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| {
                    format!("{}=debug", env!("CARGO_CRATE_NAME")).into()
                }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let mut ws = websocket::WebSocket::new(LOCAL_URL).expect("URL is invalid.");

    ws.connect("user", None)
        .await
        .expect("Is the password wrong? Or server offline?");

    tracing::info!("Discovery WebSocket connected!");

    spawn_heartbeat!(mut ws);

    loop {}
}
