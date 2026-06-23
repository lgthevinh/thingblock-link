# thingblock-link

The local native **helper** that backs the scratch-editor firmware module's
local-helper compile/flash mode (the "arduino-helper"). It is a single
long-running Rust binary with two faces:

- a **WebSocket server** facing the browser/editor, and
- a **gRPC client** to the bundled [`arduino-cli`](https://github.com/arduino/arduino-cli)
  daemon.

It is a **translating proxy**: the browser speaks a minimal JSON envelope
(documented below) and the helper turns each request into an `ArduinoCoreService`
gRPC streaming call, pumping results back. The arduino-cli gRPC schema never leaks
to the browser, which keeps the daemon swappable and the JS side ignorant of
arduino-cli specifics.

The helper also spawns and owns the `arduino-cli daemon` child process — one
self-contained process for the user to run.

> **Design source of truth:** `.agents/docs/21-06_01.arduino-helper-design.md` owns
> the WS protocol, crate layout, and milestone roadmap. This README mirrors the
> protocol for quick reference; that document is authoritative.

## Status

The WS pipe and the daemon handshake are in place. Of the protocol below, only
**`listBoards`** is implemented today; the other request types return
`error {code: "unimplemented"}` until their milestone lands (compile → upload →
monitor). Each row in the [reference](#websocket-protocol-reference) is tagged with
its status.

## Build, test, lint

Edition 2024. Standard Rust toolchain:

```sh
cargo build                              # debug build
cargo build --release                    # production build
cargo test                               # run tests (some need the bundled arduino-cli)
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

The bundled `arduino-cli` binaries live under `arduino-cli-binaries/`; integration
tests (`tests/daemon.rs`, `tests/ws.rs`) spawn the real daemon and are offline-safe.

## Run

```sh
cargo run -- --port 3030                 # WS server on ws://localhost:3030/
```

`--port` defaults to `3030`. The process starts the arduino-cli daemon on an
OS-assigned loopback port, serves the WebSocket, and renders a tray icon
(`Starting…` → `Running on :PORT`, plus Quit). Quit reaps the daemon.

Set `RUST_LOG` (e.g. `RUST_LOG=thingblock_link=debug`) to adjust log verbosity.

## WebSocket protocol reference

The editor connects to `ws://localhost:<port>/` and exchanges JSON text frames.

### Envelope

Every message — in both directions — is:

```json
{ "id": "<string>", "type": "<string>", "payload": { } }
```

`id` correlates a request with its streamed responses and its single terminal
reply (`result` or `error`). Field names are camelCase. Unsolicited helper messages
(`event`) carry their own `id`.

### Client → helper

| `type` | `payload` | terminal reply | status |
| - | - | - | - |
| `listBoards` | `{ pnpid: string[] }` | `result { targets: ConnectionTarget[] }` | ✅ implemented |
| `connect` | `{ port: string }` | `result {}` | ☐ unimplemented |
| `disconnect` | `{}` | `result {}` | ☐ unimplemented |
| `compile` | `{ fqbn, options, source }` | `result { artifact }` (after `log`/`progress`) | ☐ unimplemented |
| `upload` | `{ fqbn, port, uploadSpeed, artifact }` | `result {}` (after `progress`) | ☐ unimplemented |
| `monitorOpen` | `{ port, baudRate }` | `result {}` then async `monitorData` | ☐ unimplemented |
| `monitorWrite` | `{ data }` | — | ☐ unimplemented |
| `monitorClose` | `{}` | `result {}` | ☐ unimplemented |
| `cancel` | `{}` (targets the request `id`) | `error { code: "cancelled" }` on the cancelled request | ☐ unimplemented |

### Helper → client

| `type` | `payload` | meaning |
| - | - | - |
| `log` | `{ chunk: string }` | streamed stdout/stderr for a request `id` |
| `progress` | `{ phase: string, percent: number }` | streamed progress for a request `id` |
| `result` | request-specific object | terminal success for a request `id` |
| `error` | `{ code: string, message: string }` | terminal failure for a request `id` |
| `monitorData` | `{ data: string }` | inbound serial bytes for the monitor session |
| `event` | request-specific object | unsolicited, e.g. `boardListChanged` |

`error.code` is one of `invalidRequest`, `daemon`, `grpc`, `cancelled`, `io`, or
`unimplemented`. A malformed envelope is answered with `error{invalidRequest}` and
an empty `id` (there is nothing to correlate against).

### Shared shapes

```ts
// A connectable board returned by listBoards. Opaque to the JS Connection
// contract beyond the fields it reads.
type ConnectionTarget = { port: string; label: string };

// A compiled binary the editor hands back to `upload`.
type Artifact = { format: string; path: string };
```

### `listBoards`

`pnpid` is the list of accepted USB device ids, in Windows PNP form
(`USB\VID_xxxx&PID_xxxx`), from the device's upload config. The helper runs gRPC
`BoardList`, reconstructs each connected port's PNP id from its `vid`/`pid`
properties, and keeps the ports whose id matches an entry in `pnpid`
(case-insensitive). A port's `label` is its detected board name, falling back to
the port label, then its address.

**Request**

```json
{ "id": "1", "type": "listBoards", "payload": { "pnpid": ["USB\\VID_2341&PID_0043"] } }
```

**Reply** (an Uno on `/dev/ttyACM0`; empty `targets` when nothing matches)

```json
{ "id": "1", "type": "result", "payload": { "targets": [ { "port": "/dev/ttyACM0", "label": "Arduino Uno" } ] } }
```

## Architecture

```
src/
  main.rs        thin binary: tracing, clap --port, hands main thread to the tray
  daemon.rs      spawns/owns arduino-cli daemon, gRPC channel, Create/Init handshake
  grpc.rs, grpc/ generated `pb` module + `Client` wrapper; one submodule per RPC
    board.rs       BoardList -> pnpid filter -> ConnectionTarget[]
  ws.rs, ws/
    server.rs      axum accept loop -> Session per socket
    session.rs     per-connection state + read/dispatch/write pipe
    protocol.rs    serde structs for the JSON envelope (the cross-repo contract)
  bridge.rs      envelope <-> gRPC translation; the only place the two schemas meet
  tray.rs        tray-icon status/quit UI + main-thread tao event loop
  error.rs
tests/           integration tests (no inline #[cfg(test)] in src/)
```

Stack: `tokio`, `tonic` / `prost`, `axum`, `serde`, `tracing`, `clap`,
`tao` / `tray-icon`.

## License

Licensed under the GNU Affero General Public License v3.0 (AGPL-3.0). See
[`LICENSE`](LICENSE) for the full text.
