use error::Result;
use tokio::sync::Mutex as AsyncMutex;
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
use std::sync::{Arc, Mutex};

const MAX_ATTEMPTS: u8 = 4;

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
    pub session: Option<Arc<AsyncMutex<vodozemac::olm::Session>>>,
    /// Know if user is offerer.
    pub is_initiator: bool,
    offer: String,
}

impl WebRTCManager {
    /// Init WebRTC basics.
    pub async fn init() -> Result<Self> {
        let config = RTCConfiguration {
            ice_servers: vec![RTCIceServer {
                urls: vec!["stun:stun.l.google.com:19302".to_owned()],
                ..Default::default()
            }],
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
            offer: String::default(),
            is_initiator: false,
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

                                // If Mutex is poisoned, it would be a non-sense.
                                if let Ok(mut ice_candidates) = ice.lock() {
                                    ice_candidates.push(candidate);
                                } else {
                                    tracing::error!(
                                        ?candidate,
                                        "mutex was poisoned, aborting candidate"
                                    );
                                }
                            },
                            None => {
                                tracing::error!("peer connection is closed");
                            },
                        }
                    }
                })
            },
        ));

        Ok(webrtc)
    }

    /// Create an offer.
    pub async fn create_offer(&mut self) -> Result<String> {
        let offer = self.peer_connection.create_offer(None).await?;
        self.peer_connection
            .set_local_description(offer.clone())
            .await?;

        self.peer_connection
            .gathering_complete_promise()
            .await
            .recv()
            .await;

        let offer = self
            .peer_connection
            .local_description()
            .await
            .ok_or(error::Error::WebRTC(
                webrtc::error::Error::ErrPeerConnSDPTypeInvalidValueSetLocalDescription,
            ))?;

        self.offer = serde_json::to_string(&offer)?;
        self.is_initiator = true;

        Ok(self.offer.clone())
    }

    /// Create an answer.
    pub async fn create_answer(&mut self) -> Result<String> {
        let answer = self.peer_connection.create_answer(None).await?;
        self.peer_connection
            .set_local_description(answer.clone())
            .await?;

        self.peer_connection
            .gathering_complete_promise()
            .await
            .recv()
            .await;

        let answer = self
            .peer_connection
            .local_description()
            .await
            .ok_or(error::Error::WebRTC(
                webrtc::error::Error::ErrPeerConnSDPTypeInvalidValueSetLocalDescription,
            ))?;

        self.offer = serde_json::to_string(&answer)?;

        Ok(self.offer.clone())
    }

    /// If peer created answer, connect it via offer.
    pub async fn connect(&mut self, peer_offer: String) -> Result<String> {
        let peer_offer = self.to_session_description(&peer_offer)?;
        self.peer_connection
            .set_remote_description(peer_offer)
            .await?;
        self.is_initiator = false;

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
            .field("offer", &self.offer)
            .field("is_initiator", &self.is_initiator)
            .finish()
    }
}
