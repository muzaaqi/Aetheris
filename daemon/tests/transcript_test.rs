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

#[test]
fn parse_transcript_line_extracts_thinking_when_content_empty() {
    let json = r#"{"step_index":3,"source":"MODEL","type":"PLANNER_RESPONSE","content":"","thinking":"Thinking about query plan...","status":"RUNNING"}"#;
    let (token, done) = parse_transcript_line(json).expect("parse failed");
    assert_eq!(token, "Thinking about query plan...");
    assert!(!done);
}

#[test]
fn parse_transcript_line_skips_empty_tool_call_without_content_or_thinking() {
    let json = r#"{"step_index":4,"source":"MODEL","type":"PLANNER_RESPONSE","content":"","status":"DONE","tool_calls":[{"name":"run_command"}]}"#;
    assert!(parse_transcript_line(json).is_none());
}

#[test]
fn find_latest_transcript_locates_nested_session_log() {
    use aetheris_daemon::transcript::find_latest_transcript;

    let dir = tempdir().unwrap();
    let brain = dir.path();

    // Create session 1
    let s1 = brain.join("session_1/.system_generated/logs");
    fs::create_dir_all(&s1).unwrap();
    fs::write(s1.join("transcript.jsonl"), "old").unwrap();

    // Sleep a tiny fraction to ensure timestamp difference
    std::thread::sleep(Duration::from_millis(50));

    // Create session 2
    let s2 = brain.join("session_2/.system_generated/logs");
    fs::create_dir_all(&s2).unwrap();
    let s2_file = s2.join("transcript.jsonl");
    fs::write(&s2_file, "new").unwrap();

    let found = find_latest_transcript(brain);
    assert_eq!(found, Some(s2_file));
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
