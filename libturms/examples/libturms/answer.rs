use libturms::{Config, IceServer, Turms};
use std::io::{self, BufRead};
use tracing_subscriber::prelude::*;

const LOCAL_URL: &str = "http://localhost:4000";

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| {
                    format!("{}=debug,libturms=debug", env!("CARGO_CRATE_NAME"))
                        .into()
                }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config {
        turms_url: LOCAL_URL.into(),
        rtc: vec![IceServer {
            urls: vec!["stun:stun.l.google.com:19302".into()],
            ..Default::default()
        }],
    };
    let config = serde_yaml::to_string(&config).unwrap();

    let mut managed_turms =
        Turms::from_config(libturms::ConfigFinder::<String>::Text(config))
            .await
            .unwrap()
            .connect("user2", None)
            .await
            .unwrap();

    println!("Peer offer: ");

    let mut buffer = String::new();
    let stdin = io::stdin();
    let mut handle = stdin.lock();

    handle.read_line(&mut buffer).unwrap();

    managed_turms.answer_to_peer(buffer).await.unwrap();

    loop {}
}
