//! High-level API for Turms.

pub extern crate discover;
pub extern crate p2p;

mod channel;

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
    pub turms_url: Option<String>,
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
    turms: Option<WebSocket>,
    queued_connection: HashMap<String, WebRTCManager>,
    peers_connection: HashMap<String, WebRTCManager>,
}

impl Turms {
    /// Init [`Turms`] by parsing config.
    pub fn from_config<C: AsRef<Path>>(
        config: ConfigFinder<C>,
    ) -> Result<Self> {
        let config = match config {
            ConfigFinder::Path(path) => fs::read_to_string(path)?,
            ConfigFinder::Text(str) => str,
        };
        let config: Config = serde_yaml::from_str(&config)?;

        let turms =
            config.turms_url.as_ref().map(WebSocket::new).transpose()?;

        Ok(Self {
            config,
            turms,
            queued_connection: HashMap::new(),
            peers_connection: HashMap::new(),
        })
    }

    /// Init WebSocket connection and handle messages.
    pub async fn connect<T: ToString>(
        mut self,
        identifier: T,
        password: Option<T>,
    ) -> Result<Self> {
        if let Some(ref mut turms) = self.turms {
            turms.connect(identifier, password).await?;

            let ws = turms.reader.clone().ok_or(error::ConnectionClosed)?;
            tokio::spawn(async move {
                let mut reader = ws.lock().await;
                while let Ok(Some(msg)) = reader.try_next().await {
                    tracing::info!(%msg, "new message from x");
                }
                tracing::warn!("discovery WebSocket disconnected");
            });

            spawn_heartbeat!(turms);
        }

        Ok(self)
    }

    /// Create a WebRTC offer.
    pub async fn create_peer_offer(&mut self) -> Result<String> {
        let mut webrtc = WebRTCManager::init().await?;

        let channel = webrtc.create_channel().await?;
        channel::handle_channel(self, channel);

        let offer = webrtc.create_offer().await?;
        // use offer-answer common datas later.
        // this ID is not secure.
        self.queued_connection.insert("1".into(), webrtc);
        Ok(offer)
    }

    /// Inits connection.
    /// If you initiated connection only.
    pub async fn i_got_answer(&mut self, answer: String) -> Result<()> {
        let webrtc = self.queued_connection.get_mut("1").unwrap();
        let session = webrtc.to_session_description(&answer)?;

        webrtc
            .peer_connection
            .set_remote_description(session)
            .await?;

        self.peers_connection.insert("1".into(), webrtc.clone());
        self.queued_connection.remove("1");

        Ok(())
    }

    /// Answer to a WebRTC offer.
    pub async fn answer_to_peer(&mut self, offer: String) -> Result<String> {
        let mut webrtc = WebRTCManager::init().await?;

        let channel = webrtc.create_channel().await?;
        channel::handle_channel(self, channel);

        let offer = webrtc.connect(offer).await?;

        self.queued_connection.insert("1".into(), webrtc);
        Ok(offer)
    }
}
