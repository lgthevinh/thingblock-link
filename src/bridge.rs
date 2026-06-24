//! Translates the WS envelope into arduino-cli gRPC streaming calls and pumps
//! results back as `log` / `progress` / `result` / `error` (and `monitorData`).
//!
//! This is the only place the two schemas meet; neither leaks past it. A request
//! produces zero or more streamed responses and one terminal `result`/`error`,
//! all sharing the request `id` carried by the [`Responder`].

use std::path::{Path, PathBuf};
use std::sync::Arc;

use futures::StreamExt;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};

use crate::daemon::Daemon;
use crate::error::{Error, Result};
use crate::grpc::compile::CompileEvent;
use crate::utils::tempdir::TempDir;
use crate::ws::protocol::{
    CompileOptions, CompileResult, ListBoardsResult, RequestBody, Response, ResponseBody,
};
use crate::ws::session::{InFlight, Session};

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

    /// The request `id` this responder stamps onto every reply, for log context.
    pub fn id(&self) -> &str {
        &self.id
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

    /// Send a terminal `error` derived from an [`Error`].
    async fn send_error(&self, error: &Error) {
        self.send(ResponseBody::Error {
            code: error.code().into(),
            message: error.to_string(),
        })
        .await;
    }
}

/// Dispatch one request body to its gRPC translation, streaming responses back
/// through `responder`.
///
/// Returns `Err` only for failures the session should turn into a terminal
/// `error`; handlers that own their own terminal reply (e.g. the spawned
/// `compile` task) return `Ok`.
pub async fn dispatch(
    session: &mut Session,
    body: RequestBody,
    responder: &Responder,
) -> Result<()> {
    let id = responder.id();

    // Remaining arms land their real `ArduinoCoreService` translations in later
    // milestones; until then they report themselves unimplemented.
    let unimplemented = |what: &str| {
        debug!(id, request = what, "unimplemented request");
        responder.send(ResponseBody::Error {
            code: "unimplemented".into(),
            message: format!("{what} is not implemented yet"),
        })
    };

    match body {
        RequestBody::ListBoards { pnpid } => {
            let targets = session.daemon().client().board_list(&pnpid).await?;
            debug!(id, count = targets.len(), "listBoards: returning targets");
            responder
                .send(ResponseBody::Result(
                    serde_json::to_value(ListBoardsResult { targets })
                        .expect("ListBoardsResult always serializes"),
                ))
                .await;
        }
        // The daemon is connectionless per-port, so `connect` is a session-side
        // concept: store the chosen port; existence is checked by upload/monitor.
        RequestBody::Connect { port } => {
            if port.is_empty() {
                return Err(Error::InvalidRequest(
                    "connect requires a non-empty port".into(),
                ));
            }
            debug!(id, %port, "connect: selected port");
            session.select_port(port);
            responder
                .send(ResponseBody::Result(serde_json::json!({})))
                .await;
        }
        RequestBody::Disconnect {} => {
            // M4 closes any open monitor stream here before clearing the port.
            debug!(id, "disconnect: cleared selected port");
            session.clear_port();
            responder
                .send(ResponseBody::Result(serde_json::json!({})))
                .await;
        }
        // Long-running and cancellable: run on a spawned task so the read loop
        // stays responsive (notably to `cancel`). The task owns its terminal.
        RequestBody::Compile {
            fqbn,
            options,
            source,
        } => {
            let opts: CompileOptions = serde_json::from_value(options)
                .map_err(|e| Error::InvalidRequest(format!("compile options: {e}")))?;
            let temp_base = session.ensure_temp_base()?;
            let in_flight = session.in_flight();
            let token = CancellationToken::new();
            in_flight
                .lock()
                .expect("in_flight mutex")
                .insert(id.to_string(), token.clone());
            debug!(id, %fqbn, "compile: spawning");

            tokio::spawn(run_compile(
                session.daemon(),
                responder.clone(),
                in_flight,
                temp_base,
                token,
                fqbn,
                opts,
                source,
            ));
        }
        // `cancel`'s envelope `id` is the in-flight request's id (per the
        // protocol). Fire its token; the task emits the terminal `error{cancelled}`.
        RequestBody::Cancel {} => {
            if let Some(token) = session.in_flight().lock().expect("in_flight mutex").get(id) {
                debug!(id, "cancel: signalling in-flight request");
                token.cancel();
            } else {
                debug!(id, "cancel: no in-flight request for id");
            }
        }
        RequestBody::Upload { .. } => unimplemented("upload").await,
        RequestBody::MonitorOpen { .. } => unimplemented("monitorOpen").await,
        RequestBody::MonitorWrite { .. } => unimplemented("monitorWrite").await,
        RequestBody::MonitorClose {} => unimplemented("monitorClose").await,
    }

    Ok(())
}

/// Drive one `compile` to its terminal reply, then deregister from `in_flight`.
/// Owns every terminal for its request id (`result`, `error`, `error{cancelled}`).
#[allow(clippy::too_many_arguments)]
async fn run_compile(
    daemon: Arc<Daemon>,
    responder: Responder,
    in_flight: InFlight,
    temp_base: Arc<TempDir>,
    token: CancellationToken,
    fqbn: String,
    opts: CompileOptions,
    source: String,
) {
    let id = responder.id().to_string();
    compile_stream(
        &daemon, &responder, &temp_base, &token, &fqbn, &opts, &source,
    )
    .await;
    in_flight.lock().expect("in_flight mutex").remove(&id);
}

/// The cancellable compile pump. Materializes the sketch, opens the stream, and
/// translates each event into a WS response until a terminal one is sent.
async fn compile_stream(
    daemon: &Daemon,
    responder: &Responder,
    temp_base: &TempDir,
    token: &CancellationToken,
    fqbn: &str,
    opts: &CompileOptions,
    source: &str,
) {
    let sketch_dir = match write_sketch(temp_base.path(), responder.id(), source) {
        Ok(dir) => dir,
        Err(e) => return responder.send_error(&e).await,
    };

    let mut client = daemon.client();
    let stream = match client.compile(fqbn, &sketch_dir, opts).await {
        Ok(stream) => stream,
        Err(e) => return responder.send_error(&e).await,
    };
    tokio::pin!(stream);

    loop {
        tokio::select! {
            biased;
            () = token.cancelled() => {
                return responder
                    .send(ResponseBody::Error {
                        code: Error::Cancelled.code().into(),
                        message: Error::Cancelled.to_string(),
                    })
                    .await;
            }
            item = stream.next() => match item {
                Some(Ok(CompileEvent::Log(chunk))) => {
                    responder.send(ResponseBody::Log { chunk }).await;
                }
                Some(Ok(CompileEvent::Progress { phase, percent })) => {
                    responder.send(ResponseBody::Progress { phase, percent }).await;
                }
                Some(Ok(CompileEvent::Done(artifact))) => {
                    return responder
                        .send(ResponseBody::Result(
                            serde_json::to_value(CompileResult { artifact })
                                .expect("CompileResult always serializes"),
                        ))
                        .await;
                }
                Some(Err(e)) => return responder.send_error(&e).await,
                // Stream ended without a `Done`: no artifact was produced.
                None => {
                    return responder
                        .send_error(&Error::Daemon(
                            "compile ended without producing an artifact".into(),
                        ))
                        .await;
                }
            },
        }
    }
}

/// Materialize `source` as a sketch under `base`. arduino-cli compiles a sketch
/// *directory* whose main `.ino` shares the folder's name, so we create
/// `<base>/<name>/<name>.ino`, keyed by the request id (sanitized for the FS).
fn write_sketch(base: &Path, id: &str, source: &str) -> Result<PathBuf> {
    let name = sketch_name(id);
    let dir = base.join(&name);
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join(format!("{name}.ino")), source)?;
    Ok(dir)
}

/// A filesystem-safe sketch folder name for a request id. The `sketch_` prefix
/// guards against ids that would otherwise be empty or lead with a digit.
fn sketch_name(id: &str) -> String {
    let safe: String = id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    format!("sketch_{safe}")
}
