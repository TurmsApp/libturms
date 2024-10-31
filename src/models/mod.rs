//! Models for WebSocket messages and Turms structures.

pub mod response;
pub mod pheonix;

use serde::de::Deserialize;

/// Convert [`u64`] into [`String`].
pub(crate) fn u64_to_string<S>(num: &u64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&num.to_string())
}

/// Convert a [`String`] into [`u64`].
pub(crate) fn string_to_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse::<u64>().map_err(serde::de::Error::custom)
}
