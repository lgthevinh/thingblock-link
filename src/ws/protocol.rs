//! Serde structs for the WS `{id, type, payload}` envelope — the single contract
//! between this helper and the browser/editor (see the design doc). The
//! arduino-cli gRPC schema never appears here, keeping the daemon swappable and
//! the JS side ignorant of arduino-cli specifics.
//!
//! `id` correlates a request with its streamed responses and its one terminal
//! reply. Wire field names are camelCase to match the JS side.

use serde::{Deserialize, Serialize};

/// A message from the browser to the helper.
#[derive(Debug, Deserialize)]
pub struct Request {
    pub id: String,
    #[serde(flatten)]
    pub body: RequestBody,
}

/// Client → helper message bodies, discriminated by `type` with the variant data
/// carried under `payload` (adjacently tagged).
#[derive(Debug, Deserialize)]
#[serde(
    tag = "type",
    content = "payload",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum RequestBody {
    ListBoards {
        pnpid: Vec<String>,
    },
    Connect {
        port: String,
    },
    Disconnect {},
    Compile {
        fqbn: String,
        options: serde_json::Value,
        source: String,
    },
    Upload {
        fqbn: String,
        port: String,
        upload_speed: u32,
        artifact: Artifact,
    },
    MonitorOpen {
        port: String,
        baud_rate: u32,
    },
    MonitorWrite {
        data: String,
    },
    MonitorClose {},
    /// Targets an in-flight request `id`; drops its underlying tonic stream.
    Cancel {},
}

/// A message from the helper to the browser.
#[derive(Debug, Serialize)]
pub struct Response {
    pub id: String,
    #[serde(flatten)]
    pub body: ResponseBody,
}

/// Helper → client message bodies. Streamed (`log`, `progress`, `monitorData`),
/// terminal (`result`, `error`), or unsolicited (`event`).
///
/// `result` and `event` payloads vary per request, so they carry a free-form
/// `Value`; the typed helper structs below serialize into it.
#[derive(Debug, Serialize)]
#[serde(
    tag = "type",
    content = "payload",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ResponseBody {
    /// Streamed stdout/stderr chunk for a request `id`.
    Log { chunk: String },
    /// Streamed progress for a request `id`.
    Progress { phase: String, percent: f32 },
    /// Terminal success for a request `id`.
    Result(serde_json::Value),
    /// Terminal failure for a request `id`.
    Error { code: String, message: String },
    /// Inbound serial bytes for the monitor session.
    MonitorData { data: String },
    /// Unsolicited event, e.g. `boardListChanged` from `BoardListWatch`.
    Event(serde_json::Value),
}

/// A compiled binary the editor can hand back to `upload`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    pub format: String,
    pub path: String,
}

/// A connectable board, as returned by `listBoards`. Shape is opaque to the JS
/// `Connection` contract beyond the fields it reads.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTarget {
    pub port: String,
    pub label: String,
}

/// `result` payload for `listBoards`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListBoardsResult {
    pub targets: Vec<ConnectionTarget>,
}

/// `result` payload for `compile`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompileResult {
    pub artifact: Artifact,
}
