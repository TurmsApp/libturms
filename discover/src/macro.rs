//! 

/// d
#[macro_export]
macro_rules! spawn_heartbeat {
    (mut $ws:expr) => {{
        if let Some(_client) = $ws.client.as_ref() {
            println!("pass");

            tokio::spawn(async move {});
        }
    }};
}
