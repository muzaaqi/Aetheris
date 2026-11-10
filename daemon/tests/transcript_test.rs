use aetheris_daemon::protocol::ServerMessage;
use aetheris_daemon::transcript::{TranscriptWatcher, parse_transcript_line};
use std::fs::{self, OpenOptions};
use std::io::Write;
use tempfile::tempdir;
use tokio::sync::broadcast;
use tokio::time::Duration;

#[test]
fn parse_transcript_line_extracts_planner_response() {
    let json = r#"{"step_index":1,"source":"MODEL","type":"PLANNER_RESPONSE","content":"Processing complete."}"#;
    let (token, done) = parse_transcript_line(json).expect("parse failed");
    assert_eq!(token, "Processing complete.");
    assert!(!done);
}

#[test]
fn parse_transcript_line_extracts_done_flag() {
    let json = r#"{"step_index":2,"source":"MODEL","type":"PLANNER_RESPONSE","content":"All done.","status":"DONE"}"#;
    let (token, done) = parse_transcript_line(json).expect("parse failed");
    assert_eq!(token, "All done.");
    assert!(done);
}

#[tokio::test]
async fn transcript_watcher_streams_new_lines_and_handles_truncation() {
    let dir = tempdir().unwrap();
    let transcript_path = dir.path().join("transcript.jsonl");
    fs::write(&transcript_path, "").unwrap();

    let (tx, mut rx) = broadcast::channel(16);
    let mut watcher = TranscriptWatcher::new(tx);
    watcher.watch_file(transcript_path.clone()).await.unwrap();

    // Append a line
    let mut file = OpenOptions::new()
        .append(true)
        .open(&transcript_path)
        .unwrap();
    writeln!(
        file,
        r#"{{"step_index":1,"source":"MODEL","type":"PLANNER_RESPONSE","content":"Hello world"}}"#
    )
    .unwrap();
    file.flush().unwrap();

    // Trigger explicit check / wait for inotify event
    let received = tokio::time::timeout(Duration::from_secs(3), rx.recv())
        .await
        .expect("timeout waiting for transcript stream")
        .expect("channel error");

    match received {
        ServerMessage::TokenStream(payload) => {
            assert_eq!(payload.token, "Hello world");
            assert!(!payload.done);
        }
        _ => panic!("unexpected message"),
    }
}
