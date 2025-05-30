use libturms::websocket::*;

const LOCAL_URL: &str = "http://localhost:4000";

#[tokio::test]
async fn assert_connect() {
    let _ws = WebSocket::new(LOCAL_URL)
        .unwrap()
        .connect("user", None)
        .await
        .unwrap();
}
