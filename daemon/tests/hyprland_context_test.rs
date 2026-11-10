use aetheris_daemon::hyprland::{detect_git_branch, resolve_window_cwd};
use std::fs;
use tempfile::tempdir;

#[test]
fn resolve_window_cwd_resolves_current_process() {
    let pid = std::process::id();
    let cwd = resolve_window_cwd(pid);
    assert!(cwd.is_some());
    let current_dir = std::env::current_dir().unwrap();
    assert_eq!(cwd.unwrap(), current_dir);
}

#[test]
fn detect_git_branch_reads_standard_head() {
    let dir = tempdir().unwrap();
    let git_dir = dir.path().join(".git");
    fs::create_dir(&git_dir).unwrap();
    fs::write(
        git_dir.join("HEAD"),
        "ref: refs/heads/feat/daemon-context\n",
    )
    .unwrap();

    let branch = detect_git_branch(dir.path());
    assert_eq!(branch, Some("feat/daemon-context".to_string()));
}

#[test]
fn detect_git_branch_returns_none_for_non_git_directory() {
    let dir = tempdir().unwrap();
    let branch = detect_git_branch(dir.path());
    assert_eq!(branch, None);
}

#[test]
fn detect_git_branch_handles_detached_head() {
    let dir = tempdir().unwrap();
    let git_dir = dir.path().join(".git");
    fs::create_dir(&git_dir).unwrap();
    fs::write(
        git_dir.join("HEAD"),
        "0123456789abcdef0123456789abcdef01234567\n",
    )
    .unwrap();

    let branch = detect_git_branch(dir.path());
    assert_eq!(branch, Some("0123456".to_string()));
}
