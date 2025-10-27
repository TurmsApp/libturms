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

type _Sender =
    SplitSink<TungsteniteWebSocket<MaybeTlsStream<TcpStream>>, Message>;
type Reader =
    Arc<Mutex<SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>>>;

const DEFAULT_QUEUED_MESSAGE: usize = 32;
const SOCKET_PATH: &str = "/socket/websocket";

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

    /// Establish the WebSocket connection.
    ///
    /// Uses pre-generated JWT by Turms.
    pub async fn connect(&mut self, token: String) -> Result<()> {
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

        // Establish WebSocket connection.
        let scheme = self.url.scheme();
        let socket_url =
            format!("{scheme}://{host}{SOCKET_PATH}?token={token}");

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
