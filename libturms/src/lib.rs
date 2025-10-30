//! High-level API for Turms.

pub extern crate discover;
pub extern crate p2p;

mod channel;

use discover::spawn_heartbeat;
use discover::websocket::WebSocket;
use error::Result;
use futures_util::TryStreamExt;
use p2p::models::Event;
use p2p::webrtc::{WebRTCManager, to_session_description};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
pub use webrtc::ice_transport::ice_server::RTCIceServer;

use std::{collections::HashMap, fs, path::Path};

const CONCURRENT_MESSAGES: usize = 1;

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
    pub rtc: Vec<RTCIceServer>,
    pub turms_url: Option<String>,
}

/// High level API to facilitate Turms usage.
#[derive(Debug)]
pub struct Turms {
    /// Parsed configuration.
    pub config: Config,
    turms: Option<WebSocket>,
    sender: mpsc::Sender<Event>,
    queued_connection: HashMap<String, WebRTCManager>,
    peers_connection: HashMap<String, WebRTCManager>,
}

impl Turms {
    /// Init [`Turms`] by parsing config.
    pub fn from_config<C: AsRef<Path>>(
        config: ConfigFinder<C>,
    ) -> Result<(Self, mpsc::Receiver<Event>)> {
        let config = match config {
            ConfigFinder::Path(path) => fs::read_to_string(path)?,
            ConfigFinder::Text(text) => text,
        };
        let config: Config = serde_yaml::from_str(&config)?;

        let turms =
            config.turms_url.as_ref().map(WebSocket::new).transpose()?;

        let (sender, receiver) = mpsc::channel::<Event>(CONCURRENT_MESSAGES);

        Ok((
            Self {
                config,
                turms,
                sender,
                queued_connection: HashMap::new(),
                peers_connection: HashMap::new(),
            },
            receiver,
        ))
    }

    /// Init WebSocket connection and handle messages.
    pub async fn connect<T: ToString>(mut self, token: T) -> Result<Self> {
        if let Some(ref mut turms) = self.turms {
            turms.connect(token.to_string()).await?;

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

    /// Heuristic SDP session id (`sess-id`) extractor.
    fn extract_session_id(sdp: &str) -> Option<&str> {
        if let Some(o_line) = sdp.lines().find(|line| line.starts_with("o=")) {
            let parts: Vec<&str> = o_line.split(' ').collect();

            if parts.len() >= 2 {
                return Some(parts[1]);
            }
        }

        None
    }

    /// Create a WebRTC offer.
    pub async fn create_peer_offer(&mut self) -> Result<String> {
        let mut webrtc = WebRTCManager::init(self.config.rtc.clone()).await?;

        let _channel = webrtc.create_channel().await?;
        channel::handle_channel(self.sender.clone(), webrtc.clone());

        let offer = webrtc.create_offer().await?;
        let id = Self::extract_session_id(&offer.sdp)
            .ok_or(error::Error::MissingSessionId)?;
        self.queued_connection.insert(id.to_string(), webrtc);
        Ok(serde_json::to_string(&offer)?)
    }

    /// Inits connection.
    /// If you initiated connection only.
    pub async fn i_got_answer(&mut self, answer: String) -> Result<()> {
        let session = to_session_description(&answer)?;
        let id = Self::extract_session_id(&session.sdp)
            .ok_or(error::Error::MissingSessionId)?;
        let webrtc = self
            .queued_connection
            .get_mut(id)
            .ok_or(error::Error::MissingSessionId)?;

        webrtc
            .peer_connection
            .set_remote_description(session.clone())
            .await?;

        self.peers_connection.insert(id.to_string(), webrtc.clone());
        self.queued_connection.remove(id);

        Ok(())
    }

    /// Answer to a WebRTC offer.
    pub async fn answer_to_peer(&mut self, offer: String) -> Result<String> {
        let mut webrtc = WebRTCManager::init(self.config.rtc.clone()).await?;

        let _channel = webrtc.create_channel().await?;
        channel::handle_channel(self.sender.clone(), webrtc.clone());

        let offer = webrtc.connect(offer).await?;
        let id = Self::extract_session_id(&offer.sdp)
            .ok_or(error::Error::MissingSessionId)?;

        self.queued_connection.insert(id.to_string(), webrtc);
        Ok(serde_json::to_string(&offer)?)
    }
}
