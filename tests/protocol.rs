//! Round-trip tests for the WS `{id, type, payload}` envelope (the cross-repo
//! contract). Kept here rather than inline in `src/` so source files stay focused
//! on implementation.

use thingblock_link::ws::protocol::{
    ListBoardsResult, Request, RequestBody, Response, ResponseBody,
};

#[test]
fn deserializes_list_boards_request() {
    let raw = r#"{"id":"1","type":"listBoards","payload":{"pnpid":["2341:0043"]}}"#;
    let req: Request = serde_json::from_str(raw).expect("parse listBoards");
    assert_eq!(req.id, "1");
    match req.body {
        RequestBody::ListBoards { pnpid } => assert_eq!(pnpid, ["2341:0043"]),
        other => panic!("unexpected body: {other:?}"),
    }
}

#[test]
fn deserializes_upload_request_with_camelcase_fields() {
    let raw = r#"{
        "id":"7","type":"upload",
        "payload":{"fqbn":"arduino:avr:uno","port":"/dev/ttyACM0",
                   "uploadSpeed":115200,
                   "artifact":{"format":"hex","path":"/tmp/sketch.hex"}}
    }"#;
    let req: Request = serde_json::from_str(raw).expect("parse upload");
    match req.body {
        RequestBody::Upload {
            upload_speed,
            artifact,
            ..
        } => {
            assert_eq!(upload_speed, 115200);
            assert_eq!(artifact.format, "hex");
        }
        other => panic!("unexpected body: {other:?}"),
    }
}

#[test]
fn serializes_result_envelope() {
    let resp = Response {
        id: "1".into(),
        body: ResponseBody::Result(
            serde_json::to_value(ListBoardsResult { targets: vec![] }).unwrap(),
        ),
    };
    let json = serde_json::to_value(&resp).unwrap();
    assert_eq!(json["id"], "1");
    assert_eq!(json["type"], "result");
    assert!(json["payload"]["targets"].is_array());
}
