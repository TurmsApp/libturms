//! Process messages, handle heartbeat...

use crate::error::{Error, ErrorType, IoError};
use crate::models::response::{Response, Status};
use std::net::TcpStream;
use tungstenite::connect;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::WebSocket as TungsteniteWebSocket;
use url::Url;

/// WebSocket manager.
#[derive(Debug)]
pub struct WebSocket {
    url: Url,
    client: Option<TungsteniteWebSocket<MaybeTlsStream<TcpStream>>>,
}

impl WebSocket {
    /// Create a new [`WebSocket`] without connecting it.
    pub fn new<T: AsRef<str>>(url: T) -> Result<Self, Error> {
        let url = Url::parse(url.as_ref()).map_err(|error| {
            Error::new(
                ErrorType::InputOutput(IoError::ParsingError),
                Some(Box::new(error)),
                None,
            )
        })?;

        Ok(WebSocket { url, client: None })
    }

    fn get_scheme(&self, base: &str) -> String {
        match self.url.scheme() {
            "https" | "wss" => format!("{}s", base),
            _ => base.to_owned(),
        }
    }

    /// Establish the WebSocket connection.
    ///
    /// First, it makes an HTTP request to get the JWT.
    /// Then, it connects to WebSocket using the token.
    pub fn connect<T: AsRef<str>>(
        mut self,
        identifier: T,
        password: Option<T>,
    ) -> Result<Self, Error> {
        // Ensure the URL has a valid host.
        let host = {
            let host_str = self.url.host_str().ok_or_else(|| {
                Error::new(
                    ErrorType::InputOutput(IoError::ParsingError),
                    None,
                    Some(format!(
                        "URL {:?} does not contain a valid host.",
                        self.url.to_string()
                    )),
                )
            })?;

            match self.url.port() {
                Some(port) => format!("{host_str}:{port}"),
                None => host_str.to_string(),
            }
        };

        let scheme = self.get_scheme("http");
        let url = format!("{scheme}://{host}/api/auth");

        // Send request and get token.
        let token = ureq::post(&url)
            .send_json(ureq::json!({
                "vanity": identifier.as_ref(),
                "password": password.as_ref().map(|p| p.as_ref()),
            }))
            .map_err(|error| {
                Error::new(
                    ErrorType::InputOutput(IoError::HTTPError),
                    Some(Box::new(error)),
                    None,
                )
            })?
            .into_json::<Response>()
            .map_err(|error| {
                Error::new(
                    ErrorType::InputOutput(IoError::HTTPError),
                    Some(Box::new(error)),
                    Some("Received invalid JSON response.".to_owned()),
                )
            })?;

        if token.status == Status::Error || token.data.is_empty() {
            return Err(Error::new(
                ErrorType::InputOutput(IoError::Credidentials),
                None,
                Some("Authentication failed.".to_owned()),
            ));
        }

        // Establish WebSocket connection.
        let scheme = self.get_scheme("ws");
        let socket_url =
            format!("{scheme}://{host}/socket/websocket?token={}", token.data);

            println!("{:?}", socket_url);

        let (socket, _response) = connect(&socket_url).map_err(|error| {
            Error::new(
                ErrorType::InputOutput(IoError::ConnectionError),
                Some(Box::new(error)),
                Some("Failed to establish WebSocket connection.".to_owned()),
            )
        })?;

        self.client = Some(socket);

        Ok(self)
    }
}
