//! WS accept loop. Binds `ws://localhost:PORT` (localhost is a secure context,
//! so no TLS pre-M5), upgrades each connection, and drives a [`Session`] per
//! socket.
//!
//! [`Session`]: crate::ws::session::Session

use crate::error::Result;

/// Bind the WS endpoint and serve connections until shutdown.
pub async fn serve(/* addr, daemon handle */) -> Result<()> {
    todo!("M0: axum router with a WS upgrade route -> Session::run")
}
