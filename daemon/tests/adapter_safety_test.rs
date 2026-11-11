use aetheris_daemon::adapters::ClaudeAdapter;
use aetheris_daemon::protocol::ExecutionMode;

#[test]
fn claude_adapter_sets_yolo_flag() {
    let adapter = ClaudeAdapter::new();
    let safe_args = adapter.build_cli_args("hello", ExecutionMode::Safe);
    assert!(!safe_args.contains(&"--dangerously-skip-permissions".to_string()));

    let yolo_args = adapter.build_cli_args("hello", ExecutionMode::Yolo);
    assert!(yolo_args.contains(&"--dangerously-skip-permissions".to_string()));
}
