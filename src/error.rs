//! Crate-wide error type.
//!
//! Errors originate at the two boundaries we validate (the WS envelope from the
//! browser and the arduino-cli daemon's responses) and are carried back to the
//! editor as the WS `error {code, message}` terminal message. Internal code
//! trusts its inputs (see AGENTS.md), so this stays deliberately small.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    /// A malformed or unexpected WS envelope arrived from the browser.
    #[error("invalid request: {0}")]
    InvalidRequest(String),

    /// The arduino-cli daemon failed to start, or its gRPC channel dropped.
    #[error("daemon: {0}")]
    Daemon(String),

    /// A gRPC call to the daemon returned an error status.
    #[error("grpc: {0}")]
    Grpc(#[from] tonic::Status),

    /// A request was cancelled via `cancel{id}`.
    #[error("cancelled")]
    Cancelled,

    #[error(transparent)]
    Io(#[from] std::io::Error),
}
