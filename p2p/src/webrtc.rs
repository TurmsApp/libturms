use error::Result;
use vodozemac::olm::Account;
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

use crate::models::X3DH;

/// Simple WebRTC connection manager.
#[derive(Clone)]
pub struct WebRTCManager {
    /// Granularity.
    pub peer_connection: Arc<RTCPeerConnection>,
    /// ICE candidates.
    pub ice: Arc<Mutex<Vec<RTCIceCandidate>>>,
    /// Data channel.
    pub channel: Option<Arc<RTCDataChannel>>,
    is_initiator: bool,
    account: Arc<Mutex<vodozemac::olm::Account>>,
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
            is_initiator: false,
            channel: None,
            account: Arc::new(Mutex::new(Account::new())),
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
                                if let Ok(mut ice_candidates) = ice.lock() {
                                    ice_candidates.push(candidate);
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
        let offer =
            self.peer_connection
                .local_description()
                .await
                .ok_or(error::Error::WebRTC(webrtc::error::Error::ErrPeerConnSDPTypeInvalidValueSetLocalDescription))?;

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
        let answer =
            self.peer_connection
                .local_description()
                .await
                .ok_or(error::Error::WebRTC(webrtc::error::Error::ErrPeerConnSDPTypeInvalidValueSetLocalDescription))?;

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

        let acc = Self {
            channel: Some(Arc::clone(&channel)),
            offer: String::default(),
            ..self.clone() // uses Arc::clone(&T).
        };
        channel.on_open(Box::new(move || {
            Box::pin(async move {
                use crate::models::Event::DHKey;

                // Initate XDH3 key creation from the other side.
                if acc.is_initiator {
                    return;
                }

                // Generate public key.
                acc.account.lock().unwrap().generate_one_time_keys(1);
                let public_key = acc.account.lock().unwrap().curve25519_key().to_vec();
                let otk = acc
                    .account
                    .lock()
                    .unwrap()
                    .one_time_keys();
                let otk = otk.values()
                    .next()
                    .ok_or(error::Error::AuthenticationFailed)
                    .unwrap().to_vec();

                acc
                    .account
                    .lock()
                    .unwrap().mark_keys_as_published();

                // Send to peer second encryption layer.
                acc.channel
                    .unwrap()
                    .send_text(serde_json::to_string(&DHKey(X3DH {public_key, otk})).unwrap())
                    .await
                    .unwrap();
            })
        }));

        self.channel = Some(channel);

        Ok(self
            .channel
            .clone()
            .ok_or(webrtc::Error::ErrConnectionClosed)?)
    }

    /// Convert a [`String`] to [`RTCSessionDescription`].
    pub fn to_session_description(
        &self,
        session: &str,
    ) -> Result<RTCSessionDescription> {
        Ok(serde_json::from_str(session)?)
    }
}

impl fmt::Debug for WebRTCManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WebRTCManager")
            .field("peer_connection", &self.peer_connection)
            .field(
                "channel",
                &self.channel.as_ref().map(|_| "<RTCDataChannel>"),
            )
            .field("offer", &self.offer)
            .finish()
    }
}
