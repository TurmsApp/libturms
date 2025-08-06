use anyhow::Result;
use futures_util::TryStreamExt;
use libturms::discover::*;
use libturms::p2p::models::*;
use libturms::p2p::webrtc;
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
    tokio::spawn(async move {
        let mut reader = ws.reader.unwrap();
        while let Ok(Some(msg)) = reader.try_next().await {
            tracing::info!(%msg, "new message from discovery");
        }
        tracing::warn!("discovery WebSocket disconnected");
    });

    // Create WebRTC connection.
    let mut webrtc = webrtc::WebRTCManager::init()
        .await
        .expect("cannot init WebRTC");
    let offer = webrtc.create_offer().await.expect("cannot create SDP");

    // Send SDP offer.
    ws.client
        .send(
            models::phoenix::Message::default()
                .event(models::phoenix::Event::Offer)
                .payload(offer),
        )
        .await
        .expect("Turms message failed");

    // Create a channel for one peer.
    let channel = webrtc
        .create_channel()
        .await
        .expect("cannot create WebRTC channel");
    channel.clone().on_open(Box::new(move || {
        Box::pin(async move {
            // On connection with a peer, send a typing event.
            let message = Event::Typing;
            let message = serde_json::to_string(&message)
                .expect("event to string failed");
            channel
                .send_text(message)
                .await
                .expect("WebRTC message failed");
        })
    }));
}

fn parse_message<D: serde::ser::Serialize>(
    msg: impl ToString,
) -> Result<models::phoenix::Message<D>> {
    // Implémentation de parsing robuste ici
    todo!()
}
