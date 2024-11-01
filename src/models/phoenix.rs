//! Phoenix message model.

use serde::{Deserialize, Serialize};
use crate::models::{u64_to_string, string_to_u64};

/// Enumerate all events usable with Turms.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Event {
    /// Join a Phoenix channel.
    #[serde(rename = "phx_join")]
    Join,
    /// I'm still alive!
    Heartbeat,
}

/// Message to send towards WebSocket.
#[derive(Debug, Serialize, Deserialize)]
pub struct Message<D>
where
    D: Serialize,
{
    /// 
    pub topic: String,
    /// What happened?
    pub event: Event,
    /// Additional data in message.
    pub payload: D,
    /// Reference of websocket message.
    #[serde(serialize_with = "u64_to_string", deserialize_with = "string_to_u64")]
    pub r#ref: u64,
}
