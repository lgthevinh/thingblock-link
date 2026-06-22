//! System-tray status/quit UI for this background process.
//!
//! `tray-icon` + `tao`: the tao event loop must own the main thread, so this
//! runs there while the tokio runtime drives the daemon + WS server on worker
//! threads. A channel (set up in `main`) bridges status updates and the Quit
//! action between the two sides.

/// Build the tray icon + menu and run the event loop. Blocks the main thread.
pub fn run() {
    todo!("UI: build a TrayIcon with a status item + Quit, run the tao event loop")
}
