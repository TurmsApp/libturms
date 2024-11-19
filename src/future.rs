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
