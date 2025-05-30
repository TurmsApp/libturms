//! Seperate file to manage everything related to future.

use crate::websocket::Sender;
use futures_util::stream::SplitStream;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::time::{interval, Duration};
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::WebSocketStream as TungsteniteWebSocket;
use tungstenite::protocol::Message;

/// Combined function to handle incoming messages and send heartbeat messages.
pub(crate) async fn handle_and_heartbeat(
    delay: Duration,
    mut reader: SplitStream<TungsteniteWebSocket<MaybeTlsStream<TcpStream>>>,
    writer: Sender,
) {
    // Initialize the interval timer for sending heartbeat messages
    let mut heartbeat_interval = interval(delay);

    loop {
        tokio::select! {
            // Handler for receiving and printing messages from the server
            message = reader.next() => {
                match message {
                    Some(Ok(msg)) => {
                        if let Ok(message) = msg.into_text() {
                            println!("Message: {:?}", message);
                        }
                    }
                    Some(Err(e)) => {
                        eprintln!("Error receiving message: {:?}", e);
                        break; // Optionally handle disconnection here
                    }
                    None => {
                        // Connection closed
                        println!("Connection closed by the server.");
                        break;
                    }
                }
            }

            // Heartbeat handler to send periodic messages
            _ = heartbeat_interval.tick() => {
                // Send heartbeat message.
                // If another thread use process, don't worry because this process will do ping
                // for us!
                let _ = writer.lock().await.send(Message::Ping(Vec::new())).await;
            }
        }
    }
}
