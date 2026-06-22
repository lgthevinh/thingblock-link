//! Per-connection state: the selected port (the helper-side `connect` session
//! concept — the daemon itself is connectionless per-port), the active monitor
//! stream, and the in-flight request ids with their cancellation tokens.

use std::collections::HashMap;

use tokio_util::sync::CancellationToken;

/// State for one browser WS connection.
#[derive(Default)]
pub struct Session {
    /// The port chosen via `connect`, if any (opaque to the JS side).
    selected_port: Option<String>,
    /// Cancellation tokens keyed by in-flight request id, for `cancel{id}`.
    in_flight: HashMap<String, CancellationToken>,
}

impl Session {
    pub fn new() -> Self {
        Self::default()
    }

    /// Read envelopes from the socket and dispatch each via the bridge, writing
    /// streamed and terminal responses back.
    pub async fn run(/* socket, daemon handle */) {
        todo!("M0+: read RequestBody, dispatch via bridge, write ResponseBody")
    }
}
