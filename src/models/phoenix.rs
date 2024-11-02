//! Phoenix message model.

use crate::models::string_to_u64;
use serde::ser::{SerializeStruct, Serializer};
use serde::{Deserialize, Serialize};

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
#[derive(Debug, Deserialize)]
pub struct Message<D>
where
    D: Serialize,
{
    /// What happened?
    pub event: Event,
    /// Additional data in message.
    pub payload: D,
    /// Reference of websocket message.
    #[serde(deserialize_with = "string_to_u64")]
    pub reference: u64,
}

impl<D> Serialize for Message<D>
where
    D: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Message", 4)?;
        state.serialize_field("topic", &String::default())?;
        state.serialize_field("event", &self.event)?;
        state.serialize_field("payload", &self.payload)?;
        state.serialize_field("ref", &self.reference.to_string())?;
        state.end()
    }
}
