//! WS accept loop. Serves `ws://localhost:PORT` (localhost is a secure context,
//! so no TLS pre-M5) on a caller-provided listener, upgrades each connection,
//! and drives a [`Session`] per socket.

use std::sync::Arc;

use axum::Router;
use axum::extract::ws::WebSocket;
use axum::extract::{State, WebSocketUpgrade};
use axum::response::Response;
use axum::routing::any;
use tokio::net::TcpListener;
use tracing::info;

use crate::daemon::Daemon;
use crate::error::Result;
use crate::ws::session::Session;

/// Serve WS connections on `listener` until shutdown, sharing one daemon handle
/// across sessions. The caller binds the listener so it can choose the port (or
/// bind `:0` and read the assigned address, as the tests do).
pub async fn serve(listener: TcpListener, daemon: Arc<Daemon>) -> Result<()> {
    let app = Router::new().route("/", any(ws_handler)).with_state(daemon);
    info!(addr = ?listener.local_addr()?, "ws server listening");
    axum::serve(listener, app).await?;
    Ok(())
}

/// Upgrade an HTTP connection to WebSocket and hand the socket to a `Session`.
async fn ws_handler(ws: WebSocketUpgrade, State(daemon): State<Arc<Daemon>>) -> Response {
    ws.on_upgrade(move |socket: WebSocket| Session::new(daemon).run(socket))
}
