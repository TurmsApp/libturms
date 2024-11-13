//! Process messages, handle heartbeat...

use crate::error::{Error, ErrorType, IoError};
use crate::future::handle_and_heartbeat;
use crate::models::phoenix::Message as PhxMessage;
use crate::models::response::{Response, Status};
use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::time::Duration;
use tokio::sync::Mutex;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::WebSocketStream as TungsteniteWebSocket;
use tungstenite::protocol::Message;
use url::Url;

use std::future::Future;
use std::sync::Arc;

pub(crate) type Sender = Arc<Mutex<SplitSink<TungsteniteWebSocket<MaybeTlsStream<TcpStream>>, Message>>>;

/// WebSocket manager.
#[derive(Debug)]
pub struct WebSocket {
    url: Url,
    client: Option<Sender>,
    reference: u64,
    heartbeat_delay: Duration,
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

        Ok(WebSocket {
            url,
            client: None,
            reference: 0,
            heartbeat_delay: Duration::from_secs(30),
        })
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
    pub async fn connect<T: AsRef<str>>(
        mut self,
        identifier: T,
        password: Option<T>,
    ) -> Result<(impl Future<Output = ()>, Self), Error> {
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

        let (mut socket, _response) =
            connect_async(&socket_url).await.map_err(|error| {
                Error::new(
                    ErrorType::InputOutput(IoError::ConnectionError),
                    Some(Box::new(error)),
                    Some(
                        "Failed to establish WebSocket connection.".to_owned(),
                    ),
                )
            })?;

        // Then join lobby.
        let join_message = PhxMessage::<String>::default()
            .r#ref(self.reference)
            .to_json()?;
        socket
            .send(Message::text(join_message))
            .await
            .map_err(|error| {
                Error::new(
                    ErrorType::InputOutput(IoError::SendError),
                    Some(Box::new(error)),
                    None,
                )
            })?;

        // Split socket into writer and reader.
        let (write, read) = socket.split();

        let writer = Arc::new(Mutex::new(write));

        // Useless for now, useful in the future.
        self.client = Some(Arc::clone(&writer));

        let handler = handle_and_heartbeat(self.heartbeat_delay, read, Arc::clone(&writer));
        
        Ok((handler, self))
    }
}
