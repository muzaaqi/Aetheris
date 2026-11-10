use aetheris_daemon::protocol::{
    ClientMessage, ServerMessage, TokenStreamPayload, ToolApprovalPayload, WorkspaceSyncPayload,
};

#[test]
fn send_message_deserializes_from_spec_json() {
    let json = r#"{"type":"send_message","payload":{"text":"audit this","attachments":["/tmp/shot.png"],"quote":{"source":"src/db.ts:12-20","text":"const res = await db.query..."}}}"#;
    let msg: ClientMessage = serde_json::from_str(json).unwrap();
    assert!(matches!(msg, ClientMessage::SendMessage(p) if p.text == "audit this"));
}

#[test]
fn tool_approval_roundtrips() {
    let msg = ClientMessage::ToolApproval(ToolApprovalPayload {
        tool_call_id: "call_12345".into(),
        approved: true,
    });
    let json = serde_json::to_string(&msg).unwrap();
    let back: ClientMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(serde_json::to_string(&back).unwrap(), json);
}

#[test]
fn workspace_sync_serializes_to_spec_json() {
    let msg = ServerMessage::WorkspaceSync(WorkspaceSyncPayload {
        workspace_id: 1,
        path: "/home/muzaaqi/Projects/MuMu".into(),
        git_branch: Some("main".into()),
        skills_count: 14,
        mcp_servers: vec!["neon".into(), "cloudflare".into(), "context7".into()],
    });
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains(r#""type":"workspace_sync""#));
    assert!(json.contains(r#""workspace_id":1"#));
}

#[test]
fn token_stream_serializes_done_flag() {
    let msg = ServerMessage::TokenStream(TokenStreamPayload {
        token: "Here is ".into(),
        done: false,
    });
    let v: serde_json::Value = serde_json::to_value(&msg).unwrap();
    assert_eq!(v["payload"]["done"], false);
}

#[test]
fn unknown_type_returns_json_error() {
    let json = r#"{"type":"unknown_thing","payload":{}}"#;
    let result = serde_json::from_str::<ClientMessage>(json);
    assert!(result.is_err());
}

#[test]
fn malformed_json_returns_error() {
    let result = serde_json::from_str::<ClientMessage>("}{broken");
    assert!(result.is_err());
}
