//! Macro-commands for easly manage async stuffs.

/// Spawn heartbeat to keep WebSocket connection active.
#[macro_export]
macro_rules! spawn_heartbeat {
    (mut $ws:expr) => {{
        if let Some(client) = $ws.client.as_ref() {
            use tokio::time::interval;
            use futures_util::sink::SinkExt;
            use $crate::models::phoenix::Message;

            // Initialize the interval timer for sending heartbeat messages
            let mut heartbeat_interval = interval($ws.heartbeat_delay);

            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        // Heartbeat handler to send periodic messages
                        _ = heartbeat_interval.tick() => {
                            // Send heartbeat message.
                            // If another thread use process, don't worry because this process will do ping
                            // for us!
                            let msg: Message<String> = Message::default().event(models::phoenix::Event::Heartbeat);
                            match $ws.send(msg).await {
                                Ok(_) => tracing::debug!("heartbeat sent"),
                                Err(err) => {
                                    tracing::error!(%err, "failed to send heartbeat");
                                }
                            }
                        }
                    }
                }
            });
        }
    }};
}
