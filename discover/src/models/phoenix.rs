//! Phoenix message model.

use crate::models::string_to_u64;
use error::Result;
use serde::ser::{SerializeStruct, Serializer};
use serde::{Deserialize, Serialize};

/// Enumerate all events usable with Turms.
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Event {
    /// Join a Phoenix channel.
    #[serde(rename = "phx_join")]
    #[default]
    Join,
    /// I'm still alive!
    Heartbeat,
    /// Only send by server.
    /// Sent after joining, it enumerates every messages sent by relations while offline.
    #[serde(rename = "pending_messages")]
    UnreadMessages,
}

/// Message to send towards WebSocket.
#[derive(Debug, Default, Deserialize)]
pub struct Message<D>
where
    D: Serialize,
{
    /// What happened?
    event: Event,
    /// Additional data in message.
    payload: D,
    /// Reference of websocket message.
    #[serde(deserialize_with = "string_to_u64")]
    reference: u64,
}

impl<D> Serialize for Message<D>
where
    D: Serialize,
{
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let topic = if self.event == Event::Heartbeat {
            "phoenix"
        } else {
            Default::default()
        };

        let mut state = serializer.serialize_struct("Message", 4)?;
        state.serialize_field("topic", topic)?;
        state.serialize_field("event", &self.event)?;
        state.serialize_field("payload", &self.payload)?;
        state.serialize_field("ref", &self.reference.to_string())?;
        state.end()
    }
}

impl<D> Message<D>
where
    D: Serialize,
{
    /// Update `event` field on [`Message`].
    pub fn event(mut self, event: Event) -> Self {
        self.event = event;
        self
    }

    /// Update `reference` field on [`Message`].
    pub fn r#ref(mut self, reference: u64) -> Self {
        self.reference = reference;
        self
    }

    /// Convert [`Message`] to a JSON structure.
    pub fn to_json(self) -> Result<String> {
        Ok(serde_json::to_string(&self)?)
    }
}
