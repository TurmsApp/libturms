use libturms::{Config, IceServer, Turms};
use tracing_subscriber::prelude::*;

use std::io::{self, BufRead};

const LOCAL_URL: &str = "http://localhost:4000";

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| {
                    format!(
                        "{}=debug,libturms=debug,webrtc=error,p2p=info",
                        env!("CARGO_CRATE_NAME")
                    )
                    .into()
                }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config {
        turms_url: Some(LOCAL_URL.into()),
        rtc: vec![IceServer {
            urls: vec!["stun:stun.l.google.com:19302".into()],
            ..Default::default()
        }],
    };
    let config = serde_yaml::to_string(&config).unwrap();

    let mut managed_turms =
        Turms::from_config(libturms::ConfigFinder::<String>::Text(config))
            .unwrap()
            .connect("user", None)
            .await
            .unwrap();

    let offer = managed_turms.create_peer_offer().await.unwrap();
    println!("My offer is: {offer}");

    println!("Enter peer *answer*: ");

    let mut buffer = String::new();
    let stdin = io::stdin();
    let mut handle = stdin.lock();

    handle.read_line(&mut buffer).unwrap();

    managed_turms.i_got_answer(buffer).await.unwrap();

    loop {}
}
