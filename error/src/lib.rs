use thiserror::Error;

/// Custom [`std::result::Result`] type with Turms' [`Error`]s as fallback.
pub type Result<T> = std::result::Result<T, Error>;

/// The enum that lists errors.
#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Parsing(#[from] serde_json::Error),
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    URL(#[from] url::ParseError),

    #[error("jwt have expired since {expire_at}")]
    TokenExpired { expire_at: u64 },
    #[error("jwt must not be used before {not_before}")]
    TooEarly { not_before: u64 },
    #[error(transparent)]
    JWT(#[from] jsonwebtoken::errors::Error),

    #[error(transparent)]
    Websocket(#[from] tungstenite::Error),
    #[error(transparent)]
    HTTP(#[from] reqwest::Error),

    #[error("authentication failed")]
    AuthenticationFailed,
}
