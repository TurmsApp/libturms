use bitflags::bitflags;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use vodozemac::Curve25519PublicKey;
use vodozemac::olm::PreKeyMessage;

/// Encapsulates events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    /// Init second security layer.
    DHKey(X3DH),
    /// Share either current user (after [`DHKey`]) or a peer (DHT).
    User(User),
    /// Encrypted message.
    Message(Message),
    /// Notifies the peer system that the user is typing a message.
    Typing,
}

/// Triple Diffie-Hellman exchange.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct X3DH {
    /// Curve25519 public key.
    pub public_key: Curve25519PublicKey,
    /// One-time key.
    pub otk: Option<Curve25519PublicKey>,
    /// Receiver pre-key.
    pub prekey: Option<PreKeyMessage>,
}

/// Represents a peer for a presentation, update, or share (DHT).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// SHA256 of public key OR discovery ID.
    id: String,
    /// Custom name of peer.
    username: String,
}

/// Represents a message in a chat.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// The author of the message.
    pub author: User,
    /// The recipient of the message.
    pub recipient: User,
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
        /// Message flagged as urgent.
        const URGENT = 1 << 0;
        /// Message MUST NOT be saved.
        const EPHEMERAL = 1 << 1;
    }
}
