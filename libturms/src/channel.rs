//! Handle p2p (webrtc) data channel.

use p2p::triple_diffie_hellman;
use p2p::webrtc::WebRTCManager;
use webrtc::data_channel::data_channel_message::DataChannelMessage;

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
            tracing::trace!(%label, ?msg, "message received");
            Box::pin(async {})
        }));
    }
}
