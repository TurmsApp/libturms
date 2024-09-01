//! Reading, checking and renewing tokens.
//! 
//! Tokens are used to connect to the discovery server.
//! They are issued (but not revocable) by the discovery server.
//! They are used to verify the user's identity, based on the rules of the
//! discovery server you are using. For example, one server may force you to
//! use a password, while another may let you use any login you like.

use serde::{Serialize, Deserialize};

/// Pieces of information asserted on a JWT.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Recipients that the JWT is intended for.
    #[serde(rename = "aud")]
    pub audience: String,
    /// Identifies the expiration time on  or after which the JWT must not be
    /// accepted for processing.
    #[serde(rename = "exp")]
    pub expire_at: usize,
    /// Identifies the time at which the JWT was issued.
    #[serde(rename = "iat")]
    pub issued_at: usize,
    /// Identifies the organization that issued the JWT.
    /// 
    /// Should be Turms discovery URL, e.g. `turms.domain.tld`
    #[serde(rename = "iss")]
    pub issuer: String,
    /// Identifies the time before which the JWT must not be accepted for
    /// processing.
    #[serde(rename = "nbf")]
    pub not_before: usize,
    /// Subject of the JWT (the user).
    #[serde(rename = "sub")]
    pub subject: String,
}
