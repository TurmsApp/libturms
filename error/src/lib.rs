use thiserror::Error;
pub use tungstenite::Error::ConnectionClosed;

/// Custom [`std::result::Result`] type with Turms' [`Error`]s as fallback.
pub type Result<T> = std::result::Result<T, Error>;

/// The enum that lists errors.
#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    JsonParsing(#[from] serde_json::Error),
    #[error(transparent)]
    YamlParsing(#[from] serde_yaml::Error),
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
    Websocket(Box<tungstenite::Error>),
    #[error(transparent)]
    HTTP(Box<reqwest::Error>),
    #[error("message failed to be sent")]
    MessageSendFailed,

    #[error(transparent)]
    WebRTC(#[from] webrtc::error::Error),
    #[error("mutex is poisoned")]
    MutexPoisoned,

    #[error("authentication failed")]
    AuthenticationFailed,
    #[error("sess-id does not exist on sdp")]
    MissingSessionId,
    #[error(transparent)]
    RandOs(#[from] rand::rand_core::OsError),
}

impl From<tungstenite::Error> for Error {
    fn from(err: tungstenite::Error) -> Self {
        Error::Websocket(Box::new(err))
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::HTTP(Box::new(err))
    }
}
