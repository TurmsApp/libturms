//! Error manager.

use std::error::Error as StdError;
use std::fmt;

/// Boxed error to bypass specific [Error](StdError).
type BError = Box<dyn StdError + Send + Sync>;

/// The struct that represents an error.
#[derive(Debug)]
pub struct Error {
    /// The error type.
    pub etype: ErrorType,
    /// The cause of this error.
    pub cause: Option<BError>,
    /// Explains the context in which the error occurs.
    pub context: Option<String>,
}

impl Error {
    /// Throw an [`Error`].
    pub fn new(
        etype: ErrorType,
        cause: Option<BError>,
        context: Option<String>,
    ) -> Self {
        Error {
            etype,
            cause,
            context,
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.etype)
    }
}
impl StdError for Error {}

/// Errors in Squid.
#[derive(Debug)]
pub enum ErrorType {
    /// Generic error that returns no additional information.
    Unspecified,
    /// IO errors, related to [`std::fs`]`.
    InputOutput(IoError),
    /// JWT errors.
    Token(TokenError),
}

impl fmt::Display for ErrorType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ErrorType::Unspecified => {
                write!(
                    f,
                    "An error has occurred, but no further information is provided."
                )
            },
            ErrorType::InputOutput(error) => write!(f, "{:?}", error),
            ErrorType::Token(error) => write!(f, "{:?}", error),
        }
    }
}
impl StdError for ErrorType {}

/// Errors related to [`std`].
#[derive(Debug)]
pub enum IoError {
    /// Cannot read file.
    ReadingError,
    /// URL cannot be parsed.
    ParsingError,
    /// Error related to [ureq].
    HTTPError,
    /// Vanity or password is invalid.
    Credidentials,
    /// WebSocket connection failed.
    ConnectionError,
}

impl fmt::Display for IoError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            IoError::ReadingError => {
                write!(f, "Cannot read file with provided path.")
            },
            IoError::ParsingError => {
                write!(f, "URL cannot be parsed. Double check Discovery URL.")
            },
            IoError::HTTPError => {
                write!(f, "HTTP request cannot be sent or decoded.")
            },
            IoError::Credidentials => {
                write!(f, "Vanity or password is invalid.")
            },
            IoError::ConnectionError => {
                write!(f, "WebSocket connection failed.")
            },
        }
    }
}
impl StdError for IoError {}

/// Errors related to [jsonwebtoken].
#[derive(Debug)]
pub enum TokenError {
    /// JWT cannot be encoded or decoded.
    Fail,
    /// JWT has expired.
    Expired,
    /// JWT is used too early.
    Early,
}

impl fmt::Display for TokenError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TokenError::Fail => {
                write!(f, "Token cannot be encoded or decoded.")
            },
            TokenError::Expired => write!(f, "Invalid token: expired."),
            TokenError::Early => write!(f, "Invalid token: used too early."),
        }
    }
}
impl StdError for TokenError {}
