//! Models for WebSocket messages and Turms structures.

pub mod phoenix;
pub mod response;

use serde::de::Deserialize;

/// Convert a [`String`] into [`u64`].
pub(crate) fn string_to_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse::<u64>().map_err(serde::de::Error::custom)
}
