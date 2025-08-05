use error::Result;
use webrtc::api::APIBuilder;
use webrtc::api::media_engine::MediaEngine;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

use std::sync::Arc;

/// Simple WebRTC connection manager.
#[derive(Debug, Clone)]
pub struct WebRTCManager {
    /// Granularity.
    pub peer_connection: Arc<RTCPeerConnection>,
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

        let m = MediaEngine::default();
        let api = APIBuilder::new().with_media_engine(m).build();

        let peer_connection = Arc::new(api.new_peer_connection(config).await?);

        Ok(WebRTCManager {
            peer_connection,
            offer: String::default(),
        })
    }

    /// Create an offer.
    pub async fn create_offer(&mut self) -> Result<String> {
        let offer = self.peer_connection.create_offer(None).await?;
        self.peer_connection
            .set_local_description(offer.clone())
            .await?;

        self.offer = serde_json::to_string(&offer)?;

        Ok(self.offer.clone())
    }

    /// Create an answer.
    pub async fn create_answer(&mut self) -> Result<String> {
        let answer = self.peer_connection.create_answer(None).await?;
        self.peer_connection
            .set_local_description(answer.clone())
            .await?;

        self.offer = serde_json::to_string(&answer)?;

        Ok(self.offer.clone())
    }

    /// If peer created answer, connect it via offer.
    pub async fn connect(&mut self, peer_offer: String) -> Result<String> {
        let peer_offer: RTCSessionDescription =
            serde_json::from_str(&peer_offer)?;
        self.peer_connection
            .set_remote_description(peer_offer)
            .await?;

        self.create_answer().await
    }
}
