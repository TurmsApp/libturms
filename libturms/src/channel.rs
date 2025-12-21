//! Handle p2p (webrtc) data channel.

use p2p::models::{Event, X3DH};
use p2p::webrtc::WebRTCManager;
use p2p::{get_account, triple_diffie_hellman};
use tokio::sync::mpsc;
use vodozemac::olm::{OlmMessage, SessionConfig};
use webrtc::data_channel::data_channel_message::DataChannelMessage;

const MAX_MESSAGE_SIZE_IN_BYTES: usize = 1000 * 1000;

#[derive(Clone, Debug)]
struct Handler {
    webrtc: WebRTCManager,
    label: String,
}

/// Handle channel by parsing income messages.
pub fn handle_channel(sender: mpsc::Sender<Event>, webrtc: WebRTCManager) {
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
        let c = channel.clone();
        channel.on_open(Box::new(move || {
            let label = &h.label;
            tracing::info!(%label, "new channel opened");
            Box::pin(async move {
                if let Err(err) = triple_diffie_hellman(&h.webrtc).await {
                    tracing::error!(%err, "X3DH failed");
                    let _ = c.close().await;
                };
            })
        }));
    }

    {
        let h = handler.clone();
        channel.on_message(Box::new(move |msg: DataChannelMessage| {
            if msg.data.len() > MAX_MESSAGE_SIZE_IN_BYTES {
                return Box::pin(async move {});
            }

            let label = &h.label;
            tracing::trace!(%label, "webrtc message received");

            let webrtc = h.webrtc.clone();
            let sender = sender.clone();
            Box::pin(async move {
                // Only allow raw data if session is not initialized.
                let (data, is_encrypted) = match webrtc.session.lock().as_mut() {
                    Some(session) => {
                        let obj = match vodozemac::olm::Message::from_bytes(&msg.data) {
                            Ok(msg) => msg,
                            Err(_) => {
                                return;
                            }
                        };

                        // If double ratchet cannot decipher it, reject message.
                        match session.decrypt(&OlmMessage::from(obj)) {
                            Ok(d) => (d, true),
                            Err(e) => {
                                tracing::warn!(%e, "decryption failed");
                                return;
                            }
                        }
                    },
                    None => (msg.data.to_vec(), false),
                };

                if data.is_empty() { return; }

                let data = crate::padding::Padding::unpad(data);
                let json = match serde_json::from_slice(&data) {
                    Ok(json) => json,
                    Err(err) => {
                        tracing::error!(%err, len = data.len(), "decoding failed");
                        return;
                    }
                };

                match json {
                    Event::DHKey(x3dh) => handle_dhkey_event(&webrtc, x3dh).await,
                    _ => {
                        // Never accepts raw data for other events.
                        if !is_encrypted { return; }
                        if let Err(err) = sender.send(json.clone()).await {
                            tracing::error!(%err, "failed to send event on mpsc channel");
                        }
                    },
                }
            })
        }));
    }
}

async fn handle_dhkey_event(webrtc: &WebRTCManager, x3dh: X3DH) {
    let mut account = get_account().lock().await;
    let public_key = account.curve25519_key();

    if let Some(otk) = x3dh.otk {
        // Create OLM session.
        let mut session = account.create_outbound_session(
            SessionConfig::version_2(),
            x3dh.public_key,
            otk,
        );
        drop(account); // free access as soon as possible.
        let message = session.encrypt("");

        // Set current session to webrtc handler.
        *webrtc.session.lock() = Some(session);
        let _ = webrtc
            .peer_id
            .set(derive_peer_id(x3dh.public_key.as_bytes()));

        // Generate and send X3DH prekey to peer.
        if let OlmMessage::PreKey(pk) = message {
            match serde_json::to_string(&Event::DHKey(X3DH {
                public_key,
                otk: None,
                prekey: Some(pk),
            })) {
                Ok(message) => {
                    let message = crate::padding::Padding::pad(message);
                    if let Err(err) = webrtc.send(message).await {
                        tracing::error!(%err, "failed to send message");
                    }
                },
                Err(err) => {
                    tracing::error!(%err, "failed to serialize DHKey event")
                },
            }
        }
    } else if let Some(prekey) = x3dh.prekey {
        match account.create_inbound_session(x3dh.public_key, &prekey) {
            Ok(inbound) => {
                *webrtc.session.lock() = Some(inbound.session);
                let _ = webrtc
                    .peer_id
                    .set(derive_peer_id(x3dh.public_key.as_bytes()));
            },
            Err(err) => {
                tracing::error!(%err, "failed to create inbound session")
            },
        }
    } else {
        tracing::error!("received X3DH request without otk nor pre-key");
    }
}

fn derive_peer_id(public_key: impl AsRef<[u8]>) -> String {
    hex::encode(&blake3::hash(public_key.as_ref()).as_bytes()[..16])
}
