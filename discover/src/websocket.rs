//! Process messages, handle heartbeat...

use error::{Error, Result};
use futures_util::stream::SplitSink;
use futures_util::stream::SplitStream;
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::net::TcpStream;
use tokio::time::Duration;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, WebSocketStream as TungsteniteWebSocket,
    connect_async,
};
use tungstenite::protocol::Message;
use url::Url;

use crate::models::phoenix::Message as PhxMessage;
use crate::models::response::{Response, Status};

type Sender =
    SplitSink<TungsteniteWebSocket<MaybeTlsStream<TcpStream>>, Message>;
type Reader = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

const DEFAULT_QUEUED_MESSAGE: usize = 32;
const SOCKET_PATH: &str = "/socket/websocket";
const AUTH_PATH: &str = "/api/auth";

/// WebSocket client manager.
#[allow(dead_code)]
#[derive(Debug)]
pub struct Client {
    /// Send message via WebSocket.
    pub sender: Sender,
    /// Read message via WebSocket.
    pub reader: Reader,
}

/// WebSocket manager.
#[allow(dead_code)]
#[derive(Debug)]
pub struct WebSocket {
    url: Url,
    /// Access to connection manager.
    pub client: Option<Client>,
    reference: u64,
    /// Time interval for sending a heartbeat.
    pub heartbeat_delay: Duration,
    max_queued_message: usize,
}

#[derive(Debug, Serialize)]
struct Auth {
    vanity: String,
    password: Option<String>,
}

impl WebSocket {
    /// Create a new [`WebSocket`] without connecting it.
    pub fn new<T: AsRef<str>>(url: T) -> Result<Self> {
        let url = Url::parse(url.as_ref())?;

        Ok(WebSocket {
            url,
            client: None,
            reference: 0,
            heartbeat_delay: Duration::from_secs(30),
            max_queued_message: DEFAULT_QUEUED_MESSAGE,
        })
    }

    fn get_scheme(&self, base: &str) -> String {
        match self.url.scheme() {
            "https" | "wss" => format!("{}s", base),
            _ => base.to_owned(),
        }
    }

    /// Send messages to the WebSocket.
    pub async fn send<D>(&mut self, message: PhxMessage<D>) -> Result<()>
    where
        D: Serialize,
    {
        match self.client {
            Some(ref mut client) => {
                // Update reference on message.
                let message = message.r#ref(self.reference);
                self.reference += 1;

                client
                    .sender
                    .send(Message::Text(
                        serde_json::to_string(&message)?.into(),
                    ))
                    .await?;

                Ok(())
            },
            None => Err(Error::Websocket(tungstenite::Error::ConnectionClosed)),
        }
    }

    /// Establish the WebSocket connection.
    ///
    /// First, it makes an HTTP request to get the JWT.
    /// Then, it connects to WebSocket using the token.
    pub async fn connect<T: ToString>(
        &mut self,
        identifier: T,
        password: Option<T>,
    ) -> Result<()> {
        // Ensure the URL has a valid host.
        let host = {
            let host_str = self
                .url
                .host_str()
                .ok_or_else(|| Error::URL(url::ParseError::EmptyHost))?;

            match self.url.port() {
                Some(port) => format!("{host_str}:{port}"),
                None => host_str.to_string(),
            }
        };

        let scheme = self.get_scheme("http");
        let url = format!("{scheme}://{host}{AUTH_PATH}");

        // Send request and get token.
        let token = reqwest::Client::new()
            .post(&url)
            .json(&Auth {
                vanity: identifier.to_string(),
                password: password.map(|p| p.to_string()),
            })
            .send()
            .await?
            .json::<Response>()
            .await?;

        if token.status == Status::Error || token.data.is_empty() {
            return Err(Error::AuthenticationFailed);
        }

        // Establish WebSocket connection.
        let scheme = self.get_scheme("ws");
        let socket_url =
            format!("{scheme}://{host}{SOCKET_PATH}?token={}", token.data);

        let (mut socket, _response) = connect_async(&socket_url).await?;

        // Then join lobby.
        let join_message = PhxMessage::<String>::default()
            .r#ref(self.reference)
            .to_json()?;
        socket.send(Message::text(join_message)).await?;

        // Split socket into writer and reader.
        let (sender, reader) = socket.split();

        self.client = Some(Client { sender, reader });

        Ok(())
    }
}
