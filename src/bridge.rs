//! Translates the WS envelope into arduino-cli gRPC streaming calls and pumps
//! results back as `log` / `progress` / `result` / `error` (and `monitorData`).
//!
//! This is the only place the two schemas meet; neither leaks past it. A request
//! produces zero or more streamed responses and one terminal `result`/`error`,
//! all sharing the request `id` carried by the [`Responder`].

use std::sync::Arc;

use tokio::sync::mpsc;
use tracing::warn;

use crate::daemon::Daemon;
use crate::error::Result;
use crate::ws::protocol::{ListBoardsResult, RequestBody, Response, ResponseBody};

/// Sends responses for one request back to the session's writer task, stamping
/// each with the request `id`. Cloneable/`&`-shareable so a streaming handler can
/// emit many `log`/`progress` messages before its terminal reply.
#[derive(Clone)]
pub struct Responder {
    id: String,
    tx: mpsc::Sender<Response>,
}

impl Responder {
    pub fn new(id: String, tx: mpsc::Sender<Response>) -> Self {
        Self { id, tx }
    }

    /// Send one response body for this request. A closed channel (browser gone)
    /// is not actionable here, so it is logged and dropped.
    pub async fn send(&self, body: ResponseBody) {
        let response = Response {
            id: self.id.clone(),
            body,
        };
        if self.tx.send(response).await.is_err() {
            warn!(id = %self.id, "response dropped: ws writer closed");
        }
    }
}

/// Dispatch one request body to its gRPC translation, streaming responses back
/// through `responder`.
///
/// Returns `Err` only for failures the session should turn into a terminal
/// `error`; handlers that own their own terminal reply return `Ok`.
pub async fn dispatch(body: RequestBody, responder: &Responder, daemon: Arc<Daemon>) -> Result<()> {
    // Remaining arms land their real `ArduinoCoreService` translations in later
    // milestones; until then they report themselves unimplemented.
    let unimplemented = |what: &str| {
        responder.send(ResponseBody::Error {
            code: "unimplemented".into(),
            message: format!("{what} is not implemented yet"),
        })
    };

    match body {
        RequestBody::ListBoards { pnpid } => {
            let targets = daemon.client().board_list(&pnpid).await?;
            responder
                .send(ResponseBody::Result(
                    serde_json::to_value(ListBoardsResult { targets })
                        .expect("ListBoardsResult always serializes"),
                ))
                .await;
        }
        RequestBody::Connect { .. } => unimplemented("connect").await,
        RequestBody::Disconnect {} => unimplemented("disconnect").await,
        RequestBody::Compile { .. } => unimplemented("compile").await,
        RequestBody::Upload { .. } => unimplemented("upload").await,
        RequestBody::MonitorOpen { .. } => unimplemented("monitorOpen").await,
        RequestBody::MonitorWrite { .. } => unimplemented("monitorWrite").await,
        RequestBody::MonitorClose {} => unimplemented("monitorClose").await,
        RequestBody::Cancel {} => unimplemented("cancel").await,
    }

    Ok(())
}
