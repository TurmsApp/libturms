use libturms::discover::*;

const LOCAL_URL: &str = "http://localhost:4000";

#[tokio::main]
async fn main() {
    let mut ws = websocket::WebSocket::new(LOCAL_URL)
        .expect("URL is invalid.");
    
    ws.connect("user", None)
        .await
        .expect("Is the password wrong? Or server offline?");

    spawn_heartbeat!(mut ws);

    loop {}
}
