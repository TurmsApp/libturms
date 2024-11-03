//! Seperate file to manage everything related to future.

use crate::models::phoenix::{Event, Message as PhxMessage};
use crate::websocket::WebSocket;
use futures_util::{SinkExt, StreamExt};
use tokio::time::{interval, Duration};
use tungstenite::protocol::Message;

/// Combined function to handle incoming messages and send heartbeat messages.
pub(crate) async fn handle_and_heartbeat(delay: Duration, ws: WebSocket) {
    // Initialize the interval timer for sending heartbeat messages
    let mut heartbeat_interval = interval(delay);
    let heartbeat_message = PhxMessage::<String>::default()
        .event(Event::Heartbeat)
        .to_json()
        .unwrap_or_default();

    if let Some(mut socket) = ws.client {
        loop {
            tokio::select! {
                // Handler for receiving and printing messages from the server
                Some(Ok(msg)) = socket.next() => {
                    match msg {
                        Message::Text(text) => {
                            println!("Received: {}", text);
                        }
                        Message::Close(_) => {
                            break;
                        }
                        _ => {} // Traiter d'autres types de messages si nécessaire
                    }
                }
                // Heartbeat handler to send periodic messages
                _ = heartbeat_interval.tick() => {
                    // Send heartbeat message.
                    if socket.send(Message::text(heartbeat_message.clone())).await.is_err() {
                        // Add tracing here later.
                    }
                }
            }
        }
    }
}
