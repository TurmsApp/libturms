use std::fmt;
use std::sync::{Arc, OnceLock};

use error::{Error, Result};
use parking_lot::Mutex;
use tokio::time::{Duration, sleep};
use webrtc::api::APIBuilder;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::data_channel::RTCDataChannel;
use webrtc::data_channel::data_channel_init::RTCDataChannelInit;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

const MAX_ATTEMPTS: u8 = 4;

/// WebRTC session descriptor.
#[derive(Clone, Debug)]
pub enum Description {
    /// WebRTC description is answer.
    Answer(RTCSessionDescription),
    /// WebRTC description is offer.
    Offer(RTCSessionDescription),
    /// WebRTC description is not loaded.
    None,
}

/// Simple WebRTC connection manager.
#[derive(Clone)]
pub struct WebRTCManager {
    /// WebRTC connection.
    pub peer_connection: Arc<RTCPeerConnection>,
    /// Peer ID dervied from public key using SHA2.
    pub peer_id: Arc<OnceLock<String>>,
    /// Data channel.
    pub channel: Option<Arc<RTCDataChannel>>,
    /// Cryptographic session.
    pub session: Arc<Mutex<Option<vodozemac::olm::Session>>>,
    /// Session descriptor.
    pub(crate) description: Description,
}

impl WebRTCManager {
    /// Init WebRTC basics.
    pub async fn init(ice_servers: Vec<RTCIceServer>) -> Result<Self> {
        let config = RTCConfiguration {
            ice_servers,
            ..Default::default()
        };

        let mut media = MediaEngine::default();
        let mut registry = Registry::new();
        registry = register_default_interceptors(registry, &mut media)?;

        let api = APIBuilder::new()
            .with_media_engine(media)
            .with_interceptor_registry(registry)
            .build();

        // If account is not initialized, consider creating a new one.
        crate::get_account();

        let peer_connection = Arc::new(api.new_peer_connection(config).await?);
        let webrtc = WebRTCManager {
            peer_connection,
            peer_id: Arc::new(OnceLock::new()),
            description: Description::None,
            channel: None,
            session: Arc::new(Mutex::new(None)),
        };

        Ok(webrtc)
    }

    async fn common_description(
        &mut self,
        description: RTCSessionDescription,
    ) -> Result<RTCSessionDescription> {
        self.peer_connection
            .set_local_description(description)
            .await?;

        self.peer_connection
            .gathering_complete_promise()
            .await
            .recv()
            .await;

        let description = self
            .peer_connection
            .local_description()
            .await
            .ok_or(Error::WebRTC(
                webrtc::error::Error::ErrPeerConnSDPTypeInvalidValueSetLocalDescription,
            ))?;

        Ok(description)
    }

    /// Create an offer.
    pub async fn create_offer(&mut self) -> Result<RTCSessionDescription> {
        let offer = self.peer_connection.create_offer(None).await?;
        let offer = self.common_description(offer).await?;

        self.description = Description::Offer(offer.clone());

        Ok(offer)
    }

    /// Create an answer.
    pub async fn create_answer(&mut self) -> Result<RTCSessionDescription> {
        let answer = self.peer_connection.create_answer(None).await?;
        let answer = self.common_description(answer).await?;

        self.description = Description::Answer(answer.clone());

        Ok(answer)
    }

    /// If peer created answer, connect it via offer.
    pub async fn connect(
        &mut self,
        peer_offer: RTCSessionDescription,
    ) -> Result<RTCSessionDescription> {
        self.peer_connection
            .set_remote_description(peer_offer)
            .await?;

        self.create_answer().await
    }

    /// Create a new channel to communicate with a peer.
    pub async fn create_channel(&mut self) -> Result<Arc<RTCDataChannel>> {
        let dc_init = RTCDataChannelInit {
            ..Default::default()
        };

        let channel = self
            .peer_connection
            .create_data_channel("data", Some(dc_init))
            .await?;

        self.channel = Some(Arc::clone(&channel));

        Ok(channel)
    }

    /// Sender with retries.
    pub async fn send(&self, message: impl AsRef<[u8]>) -> Result<()> {
        // Encryption is CPU-bound. Async have no effect. May use
        // `spawn_blocking` thread later.
        let msg = match self.session.clone().lock().as_mut() {
            Some(session) => session.encrypt(message).message().to_vec(),
            None => message.as_ref().to_vec(),
        };

        match self.channel.as_ref() {
            Some(ch) => {
                for n in 0..MAX_ATTEMPTS {
                    if n > 0 {
                        // Wait only if first try is failed.
                        sleep(Duration::from_secs(u64::from(n) * 5)).await;
                    }

                    match ch.send(&bytes::Bytes::from(msg.clone())).await {
                        Ok(_) => break,
                        Err(err) => {
                            tracing::error!(%err, "{n}th attempt to send message failed");
                            if n == MAX_ATTEMPTS - 1 {
                                return Err(Error::MessageSendFailed);
                            }
                        },
                    }
                }

                Ok(())
            },
            None => Err(Error::WebRTC(webrtc::Error::ErrDataChannelNotOpen)),
        }
    }
}

impl fmt::Debug for WebRTCManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WebRTCManager")
            .field("peer_connection", &self.peer_connection)
            .field("peer_id", &self.peer_id)
            .field("session", &self.session)
            .field("description", &self.description)
            .finish_non_exhaustive()
    }
}

/// Convert a [`String`] to [`RTCSessionDescription`].
pub fn to_session_description(session: &str) -> Result<RTCSessionDescription> {
    Ok(serde_json::from_str(session)?)
}
