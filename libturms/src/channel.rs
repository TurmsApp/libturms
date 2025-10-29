//! Handle p2p (webrtc) data channel.

use p2p::models::{Event, X3DH};
use p2p::webrtc::WebRTCManager;
use p2p::{get_account, triple_diffie_hellman};
use tokio::sync::Mutex;
use vodozemac::olm::{OlmMessage, SessionConfig};
use webrtc::data_channel::data_channel_message::DataChannelMessage;

use std::sync::Arc;

#[derive(Clone, Debug)]
struct Handler {
    webrtc: WebRTCManager,
    label: String,
}

/// Handle channel by parsing income messages.
pub fn handle_channel(webrtc: WebRTCManager) {
    let Some(channel) = webrtc.channel.clone() else {
        tracing::error!("no WebRTC channel");
        return;
    };

    let handler = Handler {
        webrtc,
        label: channel.label().to_owned(),
    };

    {
        let h = handler.clone();
        channel.on_open(Box::new(move || {
            let label = &h.label;
            tracing::info!(%label, "new channel opened");
            Box::pin(async move {
                if let Err(err) = triple_diffie_hellman(&h.webrtc).await {
                    tracing::error!(%err, "X3DH failed");
                };
            })
        }));
    }

    {
        let h = handler.clone();
        channel.on_message(Box::new(move |msg: DataChannelMessage| {
            let label = &h.label;
            tracing::trace!(%label, ?msg, "webrtc message received");

            let mut webrtc = h.webrtc.clone();
            Box::pin(async move {
                let data = match webrtc.session {
                    Some(session) => {
                        let message = match vodozemac::olm::Message::from_bytes(&msg.data) {
                            Ok(msg) => msg,
                            Err(_) => {
                                return;
                            }
                        };

                        session.lock().await.decrypt(&OlmMessage::from(message)).unwrap_or_else(|_| msg.data.to_vec())
                    }
                    None => msg.data.to_vec(),
                };

                let Ok(json) = serde_json::from_slice(&data) else {
                    tracing::debug!("decoding failed");
                    return;
                };
                tracing::debug!(?json, "decoded webrtc message");

                match json {
                    Event::DHKey(x3dh) => {
                        let mut account = get_account().lock().await;
                        let public_key = account.curve25519_key();
                        if let Some(otk) = x3dh.otk {
                            let mut session: vodozemac::olm::Session = account.create_outbound_session(SessionConfig::version_2(), x3dh.public_key, otk);
                            let message = session.encrypt("");
                            webrtc.session = Some(Arc::new(Mutex::new(session)));
                            if let OlmMessage::PreKey(pk) = message {
                                match serde_json::to_string(&Event::DHKey(X3DH { public_key, otk: None, prekey: Some(pk) })) {
                                    Ok(message) => {
                                        if let Err(err) = webrtc.send(message).await {
                                            tracing::error!(%err, "failed to send message");
                                        }
                                    }
                                    Err(err) => tracing::error!(%err, "failed to serialize DHKey event"),
                                }
                            }
                        } else if let Some(prekey) = x3dh.prekey {
                            if let Err(err) = account.create_inbound_session(x3dh.public_key, &prekey) {
                                tracing::error!(%err, "failed to create inbound session");
                            }
                        } else {
                            tracing::error!("received X3DH request without otk nor pre-key");
                        }
                    },
                    _ => unimplemented!(),
                }
            })
        }));
    }
}
