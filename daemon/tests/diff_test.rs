use aetheris_daemon::diff::generate_unified_diff;

#[test]
fn generates_diff_for_new_file() {
    let diff = generate_unified_diff(None, "line1\nline2\n", "test.txt");
    assert!(diff.contains("--- /dev/null"));
    assert!(diff.contains("+++ b/test.txt"));
    assert!(diff.contains("+line1"));
    assert!(diff.contains("+line2"));
}

#[test]
fn generates_diff_for_modified_file() {
    let old = "fn main() {\n    println!(\"hello\");\n}\n";
    let new = "fn main() {\n    println!(\"hello world\");\n}\n";
    let diff = generate_unified_diff(Some(old), new, "src/main.rs");
    assert!(diff.contains("--- a/src/main.rs"));
    assert!(diff.contains("+++ b/src/main.rs"));
    assert!(diff.contains("-    println!(\"hello\");"));
    assert!(diff.contains("+    println!(\"hello world\");"));
}

#[test]
fn identical_files_produce_empty_diff() {
    let content = "same content\n";
    let diff = generate_unified_diff(Some(content), content, "same.txt");
    assert!(diff.is_empty());
}

#[tokio::test]
async fn compute_file_diff_handles_new_and_existing() {
    use tempfile::tempdir;
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("code.rs");

    // File doesn't exist yet -> treated as new file
    let diff1 = aetheris_daemon::diff::compute_file_diff(&file_path, "fn add() {}\n")
        .await
        .unwrap();
    assert!(diff1.contains("--- /dev/null"));
    assert!(diff1.contains("+fn add() {}"));

    // Write file to disk
    tokio::fs::write(&file_path, "fn add() {}\n").await.unwrap();

    // Identical content -> returns None
    let diff2 = aetheris_daemon::diff::compute_file_diff(&file_path, "fn add() {}\n").await;
    assert_eq!(diff2, None);

    // Modified content -> returns unified diff
    let diff3 = aetheris_daemon::diff::compute_file_diff(&file_path, "fn add(a: i32) {}\n")
        .await
        .unwrap();
    assert!(diff3.contains("-fn add() {}"));
    assert!(diff3.contains("+fn add(a: i32) {}"));
}
