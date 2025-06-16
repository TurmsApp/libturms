use futures_util::TryStreamExt;
use libturms::discover::*;
use tracing_subscriber::prelude::*;

const LOCAL_URL: &str = "http://localhost:4000";

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| {
                    format!("{}=debug,discover=info", env!("CARGO_CRATE_NAME"))
                        .into()
                }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let mut ws = websocket::WebSocket::new(LOCAL_URL).expect("URL is invalid.");

    ws.connect("user", None)
        .await
        .expect("Is the password wrong? Or server offline?");

    tracing::info!("discovery WebSocket connected");

    // You can also manage it yourself via `ws.client`.
    spawn_heartbeat!(ws);

    // Then read every inbound messages.
    let mut reader = ws.reader.unwrap();
    while let Ok(Some(msg)) = reader.try_next().await {
        tracing::info!(%msg, "new message from discovery");
    }
}
