//! thingblock-link binary entry point — a thin wrapper over the crate library
//! (see `lib.rs`). Builds the tokio runtime and hands the main thread to the
//! tray UI, which owns startup (daemon + WS server) and the tao event loop.

use std::path::PathBuf;

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

    /// Directory of resource packs to serve and resolve compile libs from.
    /// Defaults to `thingblock-resource` next to the executable, which is how the
    /// packaged app ships it; pass `--resource-root ./thingblock-resource` to run
    /// against the in-repo folder during development.
    #[arg(long)]
    resource_root: Option<PathBuf>,
}

/// The packaged default: a `thingblock-resource` directory laid down beside the
/// binary in the install dir. Falls back to a CWD-relative path if the
/// executable location is somehow unavailable.
fn default_resource_root() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("thingblock-resource")))
        .unwrap_or_else(|| PathBuf::from("thingblock-resource"))
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
    let resource_root = args.resource_root.unwrap_or_else(default_resource_root);

    // The tray (and tao's event loop) must own the main thread, so run the
    // daemon + WS server on a runtime and hand control to the tray UI, which
    // never returns — the process exits from within its loop on Quit.
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    tray::run(runtime, args.port, resource_root)
}
