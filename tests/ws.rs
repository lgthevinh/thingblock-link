//! End-to-end check of the WS pipe: accept → upgrade → parse envelope →
//! dispatch → serialize → `id` correlation, with a live daemon behind it. The
//! M0 dispatch returns `error{unimplemented}` for every request type; this test
//! verifies the whole round-trip, not the (later) RPC logic.

use std::net::Ipv4Addr;
use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use thingblock_link::daemon::Daemon;
use thingblock_link::ws;
use tokio::net::TcpListener;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

#[tokio::test]
async fn ws_round_trip_returns_unimplemented_error() {
    let daemon = Arc::new(Daemon::start().await.expect("daemon should start"));

    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .await
        .expect("bind ws listener");
    let addr = listener.local_addr().expect("listener address");
    tokio::spawn(async move {
        ws::server::serve(listener, daemon).await.expect("serve");
    });

    let url = format!("ws://{addr}/");
    let (mut socket, _resp) = connect_async(url.as_str()).await.expect("ws connect");

    socket
        .send(Message::Text(
            r#"{"id":"42","type":"listBoards","payload":{"pnpid":[]}}"#.into(),
        ))
        .await
        .expect("send request");

    let reply = loop {
        match socket
            .next()
            .await
            .expect("stream ended")
            .expect("ws message")
        {
            Message::Text(text) => break text,
            _ => continue,
        }
    };

    let json: serde_json::Value = serde_json::from_str(reply.as_str()).expect("parse reply");
    assert_eq!(json["id"], "42", "terminal reply must carry the request id");
    assert_eq!(json["type"], "error");
    assert_eq!(json["payload"]["code"], "unimplemented");
}
