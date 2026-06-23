//! Per-connection state and the read→dispatch→write loop for one browser
//! socket. The selected port (the helper-side `connect` session concept — the
//! daemon itself is connectionless per-port) and the in-flight request ids live
//! here; the latter is unused until M2 adds cancellable streams.

use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};

use crate::bridge::{self, Responder};
use crate::daemon::Daemon;
use crate::ws::protocol::{Request, Response, ResponseBody};

/// How many responses may queue toward the socket before backpressure applies.
const RESPONSE_CHANNEL_CAPACITY: usize = 64;

/// State for one browser WS connection.
pub struct Session {
    daemon: Arc<Daemon>,
    /// The port chosen via `connect`, if any (opaque to the JS side).
    selected_port: Option<String>,
    /// Cancellation tokens keyed by in-flight request id, for `cancel{id}`
    /// (wired up in M2 alongside long-running streams).
    in_flight: HashMap<String, CancellationToken>,
}

impl Session {
    pub fn new(daemon: Arc<Daemon>) -> Self {
        Self {
            daemon,
            selected_port: None,
            in_flight: HashMap::new(),
        }
    }

    /// A clone of the daemon handle, for dispatch arms that need a gRPC client.
    pub fn daemon(&self) -> Arc<Daemon> {
        self.daemon.clone()
    }

    /// Store the port chosen via `connect` as this session's selected port.
    pub fn select_port(&mut self, port: String) {
        self.selected_port = Some(port);
    }

    /// Clear the selected port (via `disconnect`).
    pub fn clear_port(&mut self) {
        self.selected_port = None;
    }

    /// Drive the connection until the socket closes: a writer task pumps queued
    /// responses to the sink while this loop reads and dispatches requests.
    pub async fn run(mut self, socket: WebSocket) {
        let (mut sink, mut stream) = socket.split();
        let (tx, mut rx) = mpsc::channel::<Response>(RESPONSE_CHANNEL_CAPACITY);

        let writer = tokio::spawn(async move {
            while let Some(response) = rx.recv().await {
                match serde_json::to_string(&response) {
                    Ok(json) => {
                        if sink.send(Message::Text(json.into())).await.is_err() {
                            break; // socket closed
                        }
                    }
                    // Our own response types always serialize; treat as a bug.
                    Err(e) => warn!(error = %e, "failed to serialize response"),
                }
            }
        });

        while let Some(message) = stream.next().await {
            let message = match message {
                Ok(m) => m,
                Err(e) => {
                    warn!(error = %e, "ws receive error; closing session");
                    break;
                }
            };

            match message {
                Message::Text(text) => self.handle_text(text.as_str(), &tx).await,
                Message::Close(_) => break,
                // No binary/ping/pong handling in the protocol; ignore.
                Message::Binary(_) | Message::Ping(_) | Message::Pong(_) => {}
            }
        }

        // Dropping `tx` ends the writer task, which closes the sink.
        drop(tx);
        let _ = writer.await;
    }

    /// Parse one text frame as a request envelope and dispatch it.
    async fn handle_text(&mut self, text: &str, tx: &mpsc::Sender<Response>) {
        let request: Request = match serde_json::from_str(text) {
            Ok(request) => request,
            Err(e) => {
                debug!(error = %e, "rejecting malformed envelope");
                // No id to correlate against on a parse failure.
                let _ = tx
                    .send(Response {
                        id: String::new(),
                        body: ResponseBody::Error {
                            code: "invalidRequest".into(),
                            message: format!("malformed envelope: {e}"),
                        },
                    })
                    .await;
                return;
            }
        };

        let responder = Responder::new(request.id, tx.clone());
        if let Err(e) = bridge::dispatch(self, request.body, &responder).await {
            warn!(id = %responder.id(), code = e.code(), error = %e, "request failed");
            responder
                .send(ResponseBody::Error {
                    code: e.code().into(),
                    message: e.to_string(),
                })
                .await;
        }
    }
}
