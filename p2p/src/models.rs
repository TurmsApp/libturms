use bitflags::bitflags;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Encapsulates events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    DHKey(X3DH),
    Message(Message),
    Typing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct X3DH {
    /// Curve25519 public key.
    pub public_key: Vec<u8>,
    /// One-time-key.
    pub otk: Vec<u8>,
}

/// Represents a message in a chat.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// The author of the message.
    pub author: String,
    /// The recipient of the message.
    pub recipient: String,
    /// The content of the message.
    pub content: String,
    /// The timestamp when the message was sent.
    pub timestamp: DateTime<Utc>,
    /// The timestamp when the message was last edited.
    pub edited_timestamp: DateTime<Utc>,
    /// A list of reactions to the message.
    pub reactions: Vec<char>,
    /// A list of attachments associated with the message.
    pub attachments: Vec<Attachment>,
    /// Flags.
    pub flags: Flags,
}

/// Represents an attachment to a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    /// The filename of the attachment.
    pub filename: String,
    /// The MIME type of the attachment.
    pub mime_type: Option<String>,
    /// The URL (or path) of the attachment.
    pub url: Option<String>,
    /// The binary data of the attachment.
    pub blob: Option<Vec<u8>>,
    /// Flags.
    pub flags: Flags,
}

bitflags! {
    /// Represents a set of message/attachment flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Flags: u32 {
        const URGENT = 1 << 0;
        /// Message MUST NOT be saved.
        const EPHEMERAL = 1 << 1;
    }
}
