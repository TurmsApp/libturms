//! Handle p2p (webrtc) data channel.

use webrtc::data_channel::RTCDataChannel;
use webrtc::data_channel::data_channel_message::DataChannelMessage;

use std::sync::Arc;

use crate::Turms;

/// Handle channel by parsing income messages.
pub fn handle_channel(_turms: &mut Turms, channel: Arc<RTCDataChannel>) {
    let label = channel.label().to_owned();
    let d_label = label.clone();

    channel.on_open(Box::new(move || {
        tracing::info!(%label, "new channel opened");
        Box::pin(async {})
    }));

    channel.on_message(Box::new(move |msg: DataChannelMessage| {
        tracing::trace!(%d_label, ?msg, "message received");
        Box::pin(async {})
    }));
}
