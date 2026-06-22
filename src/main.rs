//! thingblock-link binary entry point — a thin wrapper over the crate library
//! (see `lib.rs`). Builds the tokio runtime and hands the main thread to the
//! tray UI, which owns startup (daemon + WS server) and the tao event loop.

use clap::Parser;
use thingblock_link::error::Result;
use thingblock_link::tray;

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

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("thingblock_link=info")),
        )
        .init();

    tracing::info!("thingblock-link starting");
    let args = Args::parse();

    // The tray (and tao's event loop) must own the main thread, so run the
    // daemon + WS server on a runtime and hand control to the tray UI, which
    // never returns — the process exits from within its loop on Quit.
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    tray::run(runtime, args.port)
}
