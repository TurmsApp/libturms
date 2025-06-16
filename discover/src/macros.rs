//! Macro-commands for easly manage async stuffs.

/// Spawn heartbeat to keep WebSocket connection active.
#[macro_export]
macro_rules! spawn_heartbeat {
    ($ws:expr) => {{
        let mut client = $ws.client.clone();
            use tokio::time::interval;
            use futures_util::sink::SinkExt;
            use $crate::models::phoenix::Message;

            // Initialize the interval timer for sending heartbeat messages.
            let mut heartbeat_interval = interval($ws.heartbeat_delay);

            tokio::spawn(async move {
                loop {
                    // Heartbeat handler to send periodic messages.
                    heartbeat_interval.tick().await;

                    let msg: Message<String> = Message::default().event(models::phoenix::Event::Heartbeat);
                    match client.send(msg).await {
                        Ok(_) => tracing::debug!("heartbeat sent"),
                        Err(err) => {
                            tracing::error!(%err, "failed to send heartbeat");
                            break; // automatic deconnection after a certain time.
                        }
                    }
                }
            });
    }};
}
