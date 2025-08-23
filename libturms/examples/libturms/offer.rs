use libturms::{Config, IceServer, Turms};
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
            .connect("user", None)
            .await
            .unwrap();

    let offer = managed_turms.create_peer_offer().await.unwrap();
    println!("My offer is: {}", offer);

    loop {}
}
