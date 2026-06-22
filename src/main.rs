//! thingblock-link — the local native helper that backs the scratch-editor
//! firmware module's local-helper compile/flash mode.
//!
//! Two faces: a WebSocket server for the browser/editor, and a gRPC client to
//! the `arduino-cli` daemon. A system-tray icon gives this background process a
//! minimal status/quit UI. See `.agents/docs/21-06_01.arduino-helper-design.md`
//! for the WS protocol and the milestone roadmap.

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("thingblock_link=info")),
        )
        .init();

    tracing::info!("thingblock-link starting");

    // Architecture to implement (see design doc milestones):
    //   1. Build a tokio runtime; spawn the arduino-cli daemon manager
    //      (daemon.rs) and the WebSocket server (ws/server.rs) onto it.
    //   2. Run the tao event loop on this (main) thread to drive the tray icon
    //      (tray-icon), surfacing status and a Quit action.
    //   3. Bridge status/control between the tokio side and the tray over a
    //      channel.
}
