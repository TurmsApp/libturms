//! Process messages, handle heartbeat...

use error::{Error, Result};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::net::TcpStream;
use tokio::sync::{Mutex, mpsc};
use tokio::time::Duration;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, WebSocketStream as TungsteniteWebSocket,
    connect_async,
};
use tungstenite::protocol::Message;
use url::Url;

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::models::phoenix::Message as PhxMessage;
use crate::models::response::{Response, Status};

type _Sender =
    SplitSink<TungsteniteWebSocket<MaybeTlsStream<TcpStream>>, Message>;
type Reader =
    Arc<Mutex<SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>>>;

const DEFAULT_QUEUED_MESSAGE: usize = 32;
const SOCKET_PATH: &str = "/socket/websocket";
const AUTH_PATH: &str = "/api/auth";

/// WebSocket client manager.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Client {
    /// Send message via WebSocket.
    pub sender: Option<mpsc::Sender<tungstenite::Message>>,
    reference: Arc<AtomicU64>,
}

impl Client {
    /// Send messages to the WebSocket.
    pub async fn send<D>(&mut self, message: PhxMessage<D>) -> Result<()>
    where
        D: Serialize,
    {
        // Update reference on message.
        let reference = self.reference.fetch_add(1, Ordering::Relaxed);
        let message = message.r#ref(reference);

        self.sender
            .as_ref()
            .ok_or(error::ConnectionClosed)?
            .send(Message::Text(serde_json::to_string(&message)?.into()))
            .await
            .map_err(|_| Error::MessageSendFailed)?;

        Ok(())
    }
}

/// WebSocket manager.
#[allow(dead_code)]
#[derive(Debug)]
pub struct WebSocket {
    url: Url,
    /// Access to connection manager.
    pub client: Client,
    /// Read message via WebSocket.
    pub reader: Option<Reader>,
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
            client: Client {
                sender: None,
                reference: Arc::new(AtomicU64::new(0)),
            },
            reader: None,
            heartbeat_delay: Duration::from_secs(45),
            max_queued_message: DEFAULT_QUEUED_MESSAGE,
        })
    }

    fn get_scheme(&self, base: &str) -> String {
        match self.url.scheme() {
            "https" | "wss" => format!("{base}s"),
            _ => base.to_owned(),
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
        let join_message =
            PhxMessage::<String>::default().r#ref(0).to_json()?;
        socket.send(Message::text(join_message)).await?;

        // Split socket into writer and reader.
        let (mut sender, reader) = socket.split();
        self.reader = Some(Arc::new(Mutex::new(reader)));

        // Create MPSC channel to handle multiple senders at same time.
        // For instance, user and heartbeat manager.
        let (tx, mut rx) = mpsc::channel(self.max_queued_message);
        self.client.sender = Some(tx);

        tokio::spawn(async move {
            while let Some(wrapper) = rx.recv().await {
                if let Err(err) = sender.send(wrapper).await {
                    tracing::error!(%err, "failed to send message over WebSocket")
                }
            }
        });

        Ok(())
    }
}
