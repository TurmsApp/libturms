//! Reading, checking and renewing tokens.
//!
//! Tokens are used to connect to the discovery server.
//! They are issued (but not revocable) by the discovery server.
//! They are used to verify the user's identity, based on the rules of the
//! discovery server you are using. For example, one server may force you to
//! use a password, while another may let you use any login you like.

use crate::error::{Error, ErrorType, IoError, TokenError};
use jsonwebtoken::{
    decode, encode, DecodingKey, EncodingKey, Header, Validation,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::ops::Add;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub use jsonwebtoken::Algorithm;

/// Pieces of information asserted on a JWT.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Claims {
    /// Recipients that the JWT is intended for.
    #[serde(rename = "aud")]
    pub audience: String,
    /// Identifies the expiration time on  or after which the JWT must not be
    /// accepted for processing.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "exp")]
    pub expire_at: Option<u64>,
    /// Identifies the time at which the JWT was issued.
    #[serde(rename = "iat")]
    pub issued_at: u64,
    /// Identifies the organization that issued the JWT.
    ///
    /// Should be Turms discovery URL, e.g. `turms.domain.tld`
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "iss")]
    pub issuer: Option<String>,
    /// Identifies the time before which the JWT must not be accepted for
    /// processing.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "nbf")]
    pub not_before: Option<u64>,
    /// Subject of the JWT (the user).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "sub")]
    pub subject: Option<String>,
}

impl Claims {
    /// Create new [`Claims`] with pre-filled fields.
    pub fn new(audience: String) -> Claims {
        Claims {
            audience,
            issued_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            ..Default::default()
        }
    }

    /// Make token expire after a defined [std::time::Duration].
    pub fn expire_after(mut self, duration: Duration) -> Self {
        self.expire_at = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .add(duration)
                .as_secs(),
        );
        self
    }

    /// Set emitter of the token.
    pub fn issuer(mut self, issuer: String) -> Self {
        self.issuer = Some(issuer);
        self
    }

    /// Set after a defined [std::time::Duration] token should be accepted.
    pub fn not_before(mut self, duration: Duration) -> Self {
        self.not_before = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .add(duration)
                .as_secs(),
        );
        self
    }
}

/// Method to extract key.
#[derive(Debug)]
pub enum Key<P: AsRef<Path>> {
    /// Extract key from a file.
    Path(P),
    /// Extract key directly from a string.
    Text(String),
}

/// Manage JWT.
/// Only supports asymmetric encryption.
#[allow(missing_debug_implementations)]
pub struct TokenManager {
    private_key: Option<EncodingKey>,
    public_key: DecodingKey,
    algorithm: Algorithm,
}

impl TokenManager {
    /// Create a new [`TokenManager`].
    pub fn new<P: AsRef<Path>>(
        private_key: Option<Key<P>>,
        public_key: Key<P>,
    ) -> Result<Self, Error> {
        let private_key = if let Some(private_key) = private_key {
            match private_key {
                Key::Path(path) => {
                    let bytes = fs::read(path).map_err(|error| {
                        Error::new(
                            ErrorType::InputOutput(IoError::ReadingError),
                            Some(Box::new(error)),
                            Some("while opening file".to_owned()),
                        )
                    })?;

                    Some(EncodingKey::from_rsa_pem(&bytes).map_err(
                        |error| {
                            Error::new(
                                ErrorType::InputOutput(IoError::ReadingError),
                                Some(Box::new(error)),
                                Some("decoding rsa key".to_owned()),
                            )
                        },
                    )?)
                },
                Key::Text(str) => Some(
                    EncodingKey::from_rsa_pem(str.to_string().as_bytes())
                        .map_err(|error| {
                            Error::new(
                                ErrorType::InputOutput(IoError::ReadingError),
                                Some(Box::new(error)),
                                Some("decoding rsa key".to_owned()),
                            )
                        })?,
                ),
            }
        } else {
            None
        };

        let public_key = match public_key {
            Key::Path(path) => {
                let bytes = fs::read(path).map_err(|error| {
                    Error::new(
                        ErrorType::InputOutput(IoError::ReadingError),
                        Some(Box::new(error)),
                        Some("while opening file".to_owned()),
                    )
                })?;

                DecodingKey::from_rsa_pem(&bytes).map_err(|error| {
                    Error::new(
                        ErrorType::InputOutput(IoError::ReadingError),
                        Some(Box::new(error)),
                        Some("decoding rsa key".to_owned()),
                    )
                })?
            },
            Key::Text(str) => DecodingKey::from_rsa_pem(
                str.to_string().as_bytes(),
            )
            .map_err(|error| {
                Error::new(
                    ErrorType::InputOutput(IoError::ReadingError),
                    Some(Box::new(error)),
                    Some("decoding rsa key".to_owned()),
                )
            })?,
        };

        Ok(TokenManager {
            private_key,
            public_key,
            algorithm: Algorithm::RS256,
        })
    }

    /// Create a new custom JWT.
    ///
    /// `private_key` must be set.
    pub fn create_token(&self, claims: &Claims) -> Result<String, Error> {
        if let Some(private_key) = &self.private_key {
            let token =
                encode(&Header::new(self.algorithm), claims, private_key)
                    .map_err(|error| {
                        Error::new(
                            ErrorType::Token(TokenError::Fail),
                            Some(Box::new(error)),
                            Some("encoding jwt".to_owned()),
                        )
                    })?;

            Ok(token)
        } else {
            Ok(String::default())
        }
    }

    /// Decode and check a JWT.
    pub fn decode(&self, token: &str) -> Result<Claims, Error> {
        let claims: Claims =
            decode(token, &self.public_key, &Validation::new(self.algorithm))
                .map_err(|error| {
                    Error::new(
                        ErrorType::Token(TokenError::Fail),
                        Some(Box::new(error)),
                        Some("decoding jwt".to_owned()),
                    )
                })?
                .claims;

        if claims
            .expire_at
            .and_then(|expire_at| {
                (expire_at
                    < std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs())
                .then_some(..)
            })
            .is_some()
        {
            return Err(Error::new(
                ErrorType::Token(TokenError::Expired),
                None,
                Some("token is expired".to_owned()),
            ));
        }

        if claims
            .not_before
            .and_then(|not_before| {
                (not_before
                    > std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs())
                .then_some(..)
            })
            .is_some()
        {
            return Err(Error::new(
                ErrorType::Token(TokenError::Early),
                None,
                Some(
                    "`not_before` claim is older than actual timestamp"
                        .to_owned(),
                ),
            ));
        }

        Ok(claims)
    }
}
