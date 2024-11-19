use futures_util::SinkExt;
use libturms::websocket::*;

const LOCAL_URL: &str = "http://localhost:4000";

#[tokio::main]
async fn main() {
    let (receiver, ws) = WebSocket::new(LOCAL_URL)
        .expect("URL is invalid.")
        .connect("user", None)
        .await
        .expect("Is the password wrong? Or server offline?");

    // To avoid the end of program, we use the second loop here.
    // However, if we have another program running (such as a web server), we could use another
    // ``tokio::spawn`.
    tokio::spawn(receiver);

    loop {}
}
