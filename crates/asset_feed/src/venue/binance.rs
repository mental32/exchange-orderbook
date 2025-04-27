pub const DEFAULT_WEBSOCKET_ADDRESS: &str = "wss://ws-api.binance.com:443/ws-api/v3";

#[tokio::test]
async fn test_binance_connection() {
    let (stream, response) = tokio_tungstenite::connect_async(DEFAULT_WEBSOCKET_ADDRESS)
        .await
        .expect("Failed to connect");

    dbg!(response.status());
    dbg!(response.headers());
    dbg!(response.version());
}
