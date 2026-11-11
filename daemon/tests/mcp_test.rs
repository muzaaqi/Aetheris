use aetheris_daemon::mcp::scan_mcp_servers_with_roots;
use std::fs;
use tempfile::tempdir;

#[tokio::test]
async fn scans_workspace_and_global_mcp_servers() {
    let dir = tempdir().unwrap();
    let mcp_dir = dir.path().join(".gemini/antigravity/mcp");
    fs::create_dir_all(mcp_dir.join("neon")).unwrap();
    fs::create_dir_all(mcp_dir.join("cloudflare")).unwrap();
    fs::write(mcp_dir.join("neon/run_sql.json"), "{}").unwrap();

    let ws_dir = tempdir().unwrap();
    let ws_mcp = ws_dir.path().join(".mcp/custom_server");
    fs::create_dir_all(&ws_mcp).unwrap();

    let servers = scan_mcp_servers_with_roots(Some(ws_dir.path()), &[mcp_dir]).await;

    assert!(servers.contains(&"neon".to_string()));
    assert!(servers.contains(&"cloudflare".to_string()));
    assert!(servers.contains(&"custom_server".to_string()));
}

#[tokio::test]
async fn handles_non_existent_directories_gracefully() {
    let servers = scan_mcp_servers_with_roots(None, &[]).await;
    assert!(servers.is_empty());
}
