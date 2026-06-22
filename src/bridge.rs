//! Translates the WS envelope into arduino-cli gRPC streaming calls and pumps
//! results back as `log` / `progress` / `result` / `error` (and `monitorData`).
//!
//! This is the only place the two schemas meet; neither leaks past it. A
//! cancellation token (from the session's in-flight map) drops the underlying
//! tonic stream when the browser sends `cancel{id}`.

use crate::error::Result;
use crate::ws::protocol::RequestBody;

/// Dispatch one request body to its gRPC translation. Streamed output is written
/// back over the session's response sink (wired in when the WS write half and
/// daemon handle are threaded through).
pub async fn dispatch(_req: RequestBody /* , daemon, sink, cancel */) -> Result<()> {
    todo!("translate each RequestBody variant into its ArduinoCoreService RPC")
}
