//! thingblock-link binary entry point — a thin wrapper over the crate library
//! (see `lib.rs`). Sets up logging and, once milestones land, wires the tokio
//! runtime (daemon manager + WS server) to the tray's event loop.

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("thingblock_link=info")),
        )
        .init();

    tracing::info!("thingblock-link starting");

    // M0 wiring (see design doc milestones):
    //   1. Build a tokio runtime; spawn the arduino-cli daemon manager
    //      (`daemon::Daemon::start`) and the WS server (`ws::server::serve`).
    //   2. Run the tao event loop on this (main) thread via `tray::run` to drive
    //      the tray icon, surfacing status and a Quit action.
    //   3. Bridge status/control between the tokio side and the tray over a
    //      channel.
}
