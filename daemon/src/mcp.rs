//! Model Context Protocol (MCP) server discovery scanner.
//!
//! Scans global user configurations and workspace-local paths to discover
//! configured MCP server providers (e.g. Neon, Cloudflare, Context7).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Scans global and workspace paths for available MCP server configurations.
pub async fn scan_mcp_servers(workspace_root: Option<&Path>) -> Vec<String> {
    let mut global_roots = Vec::new();
    if let Ok(home) = std::env::var("HOME") {
        let home_path = PathBuf::from(home);
        global_roots.push(home_path.join(".gemini/antigravity/mcp"));
        global_roots.push(home_path.join(".gemini/mcp"));
    }

    scan_mcp_servers_with_roots(workspace_root, &global_roots).await
}

/// Scans provided global roots and workspace directory for MCP servers.
pub async fn scan_mcp_servers_with_roots(
    workspace_root: Option<&Path>,
    global_roots: &[PathBuf],
) -> Vec<String> {
    let ws_path = workspace_root.map(|p| p.to_path_buf());
    let globals = global_roots.to_vec();

    tokio::task::spawn_blocking(move || {
        let mut servers = BTreeSet::new();

        // 1. Scan global directories
        for root in &globals {
            if root.is_dir()
                && let Ok(entries) = std::fs::read_dir(root)
            {
                for entry in entries.flatten() {
                    if let Ok(file_type) = entry.file_type()
                        && file_type.is_dir()
                        && let Some(name) = entry.file_name().to_str()
                        && !name.starts_with('.')
                    {
                        servers.insert(name.to_string());
                    }
                }
            }
        }

        // 2. Scan Claude desktop config if in global paths
        if let Ok(home) = std::env::var("HOME") {
            let claude_cfg = PathBuf::from(home).join(".config/Claude/claude_desktop_config.json");
            if claude_cfg.is_file()
                && let Ok(content) = std::fs::read_to_string(&claude_cfg)
                && let Ok(val) = serde_json::from_str::<serde_json::Value>(&content)
                && let Some(obj) = val.get("mcpServers").and_then(|s| s.as_object())
            {
                for key in obj.keys() {
                    servers.insert(key.clone());
                }
            }
        }

        // 3. Scan workspace directories
        if let Some(ws) = ws_path {
            let candidates = [
                ws.join(".mcp"),
                ws.join(".agents/mcp"),
                ws.join(".gemini/mcp"),
            ];

            for candidate in &candidates {
                if candidate.is_dir()
                    && let Ok(entries) = std::fs::read_dir(candidate)
                {
                    for entry in entries.flatten() {
                        if let Ok(file_type) = entry.file_type()
                            && file_type.is_dir()
                            && let Some(name) = entry.file_name().to_str()
                            && !name.starts_with('.')
                        {
                            servers.insert(name.to_string());
                        }
                    }
                }
            }
        }

        servers.into_iter().collect()
    })
    .await
    .unwrap_or_default()
}
