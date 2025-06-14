//! Authentification models.

use serde::{Deserialize, Serialize};

/// Response status.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Status {
    /// Server failed to process request.
    #[serde(rename = "error")]
    Error,
    /// Server managed to process request.
    #[serde(rename = "success")]
    Success,
}

/// Structure of a Turms response.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Response {
    /// Whether server managed to handle request.
    pub status: Status,
    /// Content of response.
    pub data: String,
    /// If `status` is errored, give a reason.
    pub error: Option<String>,
}
