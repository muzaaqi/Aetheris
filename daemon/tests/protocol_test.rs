use aetheris_daemon::protocol::{
    ClientMessage, FileMatch, ServerMessage, SkillInfo, SkillScope, TokenStreamPayload,
    ToolApprovalPayload, WorkspaceSyncPayload,
};

#[test]
fn search_files_deserializes_and_roundtrips() {
    let json = r#"{"type":"search_files","payload":{"query":"main.rs","limit":10}}"#;
    let msg: ClientMessage = serde_json::from_str(json).expect("deserialize failed");
    match msg {
        ClientMessage::SearchFiles(p) => {
            assert_eq!(p.query, "main.rs");
            assert_eq!(p.limit, Some(10));
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn list_skills_deserializes() {
    let json = r#"{"type":"list_skills"}"#;
    let msg: ClientMessage = serde_json::from_str(json).expect("deserialize failed");
    assert!(matches!(msg, ClientMessage::ListSkills));
}

#[test]
fn file_results_serializes_to_spec() {
    let msg = ServerMessage::FileResults(aetheris_daemon::protocol::FileResultsPayload {
        query: "main".into(),
        matches: vec![FileMatch {
            path: "daemon/src/main.rs".into(),
            is_dir: false,
        }],
    });
    let json = serde_json::to_string(&msg).expect("serialize failed");
    assert!(json.contains(r#""type":"file_results""#));
    assert!(json.contains("daemon/src/main.rs"));
}

#[test]
fn skills_list_serializes_with_scope() {
    let msg = ServerMessage::SkillsList(aetheris_daemon::protocol::SkillsListPayload {
        skills: vec![SkillInfo {
            name: "rust-testing".into(),
            description: "Testing guidelines".into(),
            scope: SkillScope::Workspace,
            path: ".agents/skills/rust-testing".into(),
        }],
    });
    let json = serde_json::to_string(&msg).expect("serialize failed");
    assert!(json.contains(r#""type":"skills_list""#));
    assert!(json.contains("workspace"));
}

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
