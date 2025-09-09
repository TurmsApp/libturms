use error::Result;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::data_channel::RTCDataChannel;
use webrtc::data_channel::data_channel_init::RTCDataChannelInit;
use webrtc::ice_transport::ice_candidate::RTCIceCandidate;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::{
    api::APIBuilder,
    peer_connection::sdp::session_description::RTCSessionDescription,
};

use std::sync::{Arc, Mutex};

/// Simple WebRTC connection manager.
#[derive(Debug, Clone)]
pub struct WebRTCManager {
    /// Granularity.
    pub peer_connection: Arc<RTCPeerConnection>,
    /// Candidate ICE.
    pub ice: Arc<Mutex<Vec<RTCIceCandidate>>>,
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

        let peer_connection = Arc::new(api.new_peer_connection(config).await?);
        let webrtc = WebRTCManager {
            peer_connection,
            ice: Arc::new(Mutex::new(Vec::new())),
            offer: String::default(),
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
                                // Leave the unusable connection as it is.
                                // Connection can't be properly closed here.
                                let mut ice_candidates = ice.lock().unwrap();
                                ice_candidates.push(candidate);
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
        let offer = self.peer_connection.local_description().await.unwrap();

        self.offer = serde_json::to_string(&offer)?;

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
        let answer = self.peer_connection.local_description().await.unwrap();

        self.offer = serde_json::to_string(&answer)?;

        Ok(self.offer.clone())
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
        Ok(self
            .peer_connection
            .create_data_channel("data", Some(dc_init))
            .await?)
    }

    /// Convert a [`String`] to [`RTCSessionDescription`].
    pub fn to_session_description(
        &self,
        session: &str,
    ) -> Result<RTCSessionDescription> {
        Ok(serde_json::from_str(session)?)
    }
}
