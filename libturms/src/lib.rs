//! High-level API for Turms.

pub extern crate discover;
pub extern crate p2p;

use discover::{spawn_heartbeat, websocket::WebSocket};
use error::Result;
use futures_util::TryStreamExt;
use p2p::webrtc::WebRTCManager;
use serde::{Deserialize, Serialize};

use std::{collections::HashMap, fs, path::Path};

/// Method to extract config.
#[derive(Debug)]
pub enum ConfigFinder<P: AsRef<Path>> {
    /// Extract config from a file.
    Path(P),
    /// Extract config directly from a string.
    Text(String),
}

/// Turms configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub rtc: Vec<IceServer>,
    pub turms_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IceServer {
    pub urls: Vec<String>,
    pub username: String,
    pub credential: String,
}

/// High level API to facilitate Turms usage.
#[derive(Debug)]
pub struct Turms {
    /// Parsed configuration.
    pub config: Config,
    turms: WebSocket,
    queued_connection: HashMap<String, WebRTCManager>,
    _peers_connection: HashMap<String, WebRTCManager>,
}

impl Turms {
    /// Init [`Turms`] by parsing config.
    pub async fn from_config<C: AsRef<Path>>(
        config: ConfigFinder<C>,
    ) -> Result<Self> {
        let config = match config {
            ConfigFinder::Path(path) => fs::read_to_string(path)?,
            ConfigFinder::Text(str) => str,
        };
        let config: Config = serde_yaml::from_str(&config)?;

        let turms = WebSocket::new(&config.turms_url)?;

        Ok(Self {
            config,
            turms,
            queued_connection: HashMap::new(),
            _peers_connection: HashMap::new(),
        })
    }

    /// Init WebSocket connection and handle messages.
    pub async fn connect<T: ToString>(
        mut self,
        identifier: T,
        password: Option<T>,
    ) -> Result<Self> {
        self.turms.connect(identifier, password).await?;

        let ws = self.turms.reader.clone().unwrap();
        tokio::spawn(async move {
            let mut reader = ws.lock().await;
            while let Ok(Some(msg)) = reader.try_next().await {
                tracing::info!(%msg, "new message from x");
            }
            tracing::warn!("discovery WebSocket disconnected");
        });

        spawn_heartbeat!(self.turms);

        Ok(self)
    }

    /// Create a WebRTC offer.
    pub async fn create_peer_offer(&mut self) -> Result<String> {
        let mut webrtc = WebRTCManager::init().await?;
        let offer = webrtc.create_offer().await?;
        // use offer-answer common datas later.
        // this ID is not secure.
        self.queued_connection.insert("1".into(), webrtc);
        Ok(offer)
    }

    /// Answer to a WebRTC offer.
    pub async fn answer_to_peer(&mut self, offer: String) -> Result<String> {
        let mut webrtc = WebRTCManager::init().await?;
        let offer = webrtc.connect(offer).await?;
        Ok(offer)
    }
}
