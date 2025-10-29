use error::Result;
use tokio::sync::Mutex;
use tokio::time::{Duration, sleep};
use webrtc::api::APIBuilder;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::data_channel::RTCDataChannel;
use webrtc::data_channel::data_channel_init::RTCDataChannelInit;
use webrtc::ice_transport::ice_candidate::RTCIceCandidate;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

use std::fmt;
use std::sync::Arc;

const MAX_ATTEMPTS: u8 = 4;

/// WebRTC session descriptor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Description {
    /// WebRTC description is answer.
    Answer(String),
    /// WebRTC description is offer.
    Offer(String),
    /// WebRTC description is not loaded.
    None,
}

/// Simple WebRTC connection manager.
#[derive(Clone)]
pub struct WebRTCManager {
    /// Granularity.
    pub peer_connection: Arc<RTCPeerConnection>,
    /// ICE candidates.
    pub ice: Arc<Mutex<Vec<RTCIceCandidate>>>,
    /// Data channel.
    pub channel: Option<Arc<RTCDataChannel>>,
    /// Cryptographic session.
    pub session: Option<Arc<Mutex<vodozemac::olm::Session>>>,
    /// Session descriptor.
    pub description: Description,
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
            ice: Arc::new(Mutex::new(Vec::new())),
            description: Description::None,
            channel: None,
            session: None,
        };

        let ice = Arc::downgrade(&webrtc.ice);
        webrtc.peer_connection.on_ice_candidate(Box::new(
            move |candidate: Option<RTCIceCandidate>| {
                let ice = ice.clone();
                Box::pin(async move {
                    if let Some(candidate) = candidate {
                        match ice.upgrade() {
                            Some(ice) => {
                                tracing::debug!(
                                    ?candidate,
                                    "new ice candidate"
                                );
                                ice.lock().await.push(candidate);
                            },
                            None => {
                                tracing::error!("peer connection is closed")
                            },
                        }
                    }
                })
            },
        ));

        Ok(webrtc)
    }

    async fn common_description(
        &mut self,
        description: RTCSessionDescription,
    ) -> Result<String> {
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
            .ok_or(error::Error::WebRTC(
                webrtc::error::Error::ErrPeerConnSDPTypeInvalidValueSetLocalDescription,
            ))?;

        Ok(serde_json::to_string(&description)?)
    }

    /// Create an offer.
    pub async fn create_offer(&mut self) -> Result<String> {
        let offer = self.peer_connection.create_offer(None).await?;
        let offer = self.common_description(offer).await?;

        self.description = Description::Offer(offer.clone());

        Ok(offer)
    }

    /// Create an answer.
    pub async fn create_answer(&mut self) -> Result<String> {
        let answer = self.peer_connection.create_answer(None).await?;
        let answer = self.common_description(answer).await?;

        self.description = Description::Answer(answer.clone());

        Ok(answer)
    }

    /// If peer created answer, connect it via offer.
    pub async fn connect(&mut self, peer_offer: String) -> Result<String> {
        let peer_offer = self.to_session_description(&peer_offer)?;
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

    /// Convert a [`String`] to [`RTCSessionDescription`].
    pub fn to_session_description(
        &self,
        session: &str,
    ) -> Result<RTCSessionDescription> {
        Ok(serde_json::from_str(session)?)
    }

    /// Sender with retries.
    /// Useful during X3DH negociation.
    pub async fn send(&self, message: String) -> Result<()> {
        let msg = match self.session.clone() {
            Some(session) => {
                session.lock().await.encrypt(message).message().to_vec()
            },
            None => message.as_bytes().to_vec(),
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
                                return Err(error::Error::MessageSendFailed);
                            }
                        },
                    }
                }

                Ok(())
            },
            None => {
                Err(error::Error::WebRTC(webrtc::Error::ErrDataChannelNotOpen))
            },
        }
    }
}

impl fmt::Debug for WebRTCManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WebRTCManager")
            .field("peer_connection", &self.peer_connection)
            .field("description", &self.description)
            .finish()
    }
}
