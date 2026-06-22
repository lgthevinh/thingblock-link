//! WebSocket server facing the browser/editor.
//!
//! [`protocol`] defines the `{id, type, payload}` envelope (the cross-repo
//! contract), [`server`] accepts connections, and [`session`] holds per-socket
//! state.

pub mod protocol;
pub mod server;
pub mod session;
