//! thingblock-link binary entry point — a thin wrapper over the crate library
//! (see `lib.rs`). Starts the arduino-cli daemon manager and the WS server.

use std::net::Ipv4Addr;
use std::sync::Arc;

use clap::Parser;
use thingblock_link::daemon::Daemon;
use thingblock_link::error::Result;
use thingblock_link::ws;
use tokio::net::TcpListener;

/// WS port the editor connects to. A contract detail with the editor; override
/// with `--port` until the two sides are pinned together.
const DEFAULT_WS_PORT: u16 = 3030;

#[derive(Parser)]
#[command(
    name = "thingblock-link",
    about = "Local arduino-cli helper for the scratch-editor"
)]
struct Args {
    /// TCP port for the WebSocket server (bound on localhost).
    #[arg(long, default_value_t = DEFAULT_WS_PORT)]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("thingblock_link=info")),
        )
        .init();

    tracing::info!("thingblock-link starting");
    let args = Args::parse();

    // Spawn and own the arduino-cli daemon, then serve the editor over WS.
    let daemon = Arc::new(Daemon::start().await?);
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, args.port)).await?;
    ws::server::serve(listener, daemon).await?;

    // Tray UI (tray.rs) is deferred: once implemented, the tao event loop must
    // own the main thread, so this will be restructured to run the daemon + WS
    // server on a tokio runtime spawned from `main` while the tray drives here.
    Ok(())
}
