//! Fast git-aware file search indexer and 2-tier skill scanner.
//!
//! Provides non-blocking context discovery for active workspaces, including
//! `@file` fuzzy mentions and hierarchical agent skills (User-scope and Workspace-scope).

use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

use crate::error::DaemonError;
use crate::protocol::{FileMatch, SkillInfo, SkillScope};

/// Finds the root project directory by traversing upward until `.git` or `.agents` is found.
pub fn find_workspace_root(start: &Path) -> PathBuf {
    let mut current = start.to_path_buf();
    loop {
        if current.join(".git").exists() || current.join(".agents").exists() {
            return current;
        }
        if !current.pop() {
            return start.to_path_buf();
        }
    }
}

/// Search files under `root` matching `query`, respecting gitignore rules.
///
/// Executes on a blocking worker thread to avoid stalling Tokio async runtimes.
pub async fn search_files(
    root: &Path,
    query: &str,
    limit: usize,
) -> Result<Vec<FileMatch>, DaemonError> {
    let root = root.to_path_buf();
    let query = query.to_lowercase();
    let limit = if limit == 0 { 20 } else { limit };

    tokio::task::spawn_blocking(move || {
        let mut matches = Vec::new();
        let walker = WalkBuilder::new(&root)
            .hidden(true)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .build();

        for entry_res in walker {
            let entry = match entry_res {
                Ok(e) => e,
                Err(e) => {
                    tracing::debug!("Skipping inaccessible path during file walk: {e}");
                    continue;
                }
            };

            let path = entry.path();
            if path == root {
                continue;
            }

            let rel_path = match path.strip_prefix(&root) {
                Ok(p) => p,
                Err(_) => path,
            };

            let rel_str = rel_path.to_string_lossy();
            if query.is_empty() || rel_str.to_lowercase().contains(&query) {
                let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
                matches.push(FileMatch {
                    path: rel_str.into_owned(),
                    is_dir,
                });

                if matches.len() >= limit {
                    break;
                }
            }
        }

        Ok(matches)
    })
    .await
    .map_err(|e| DaemonError::Internal(format!("File search worker panicked: {e}")))?
}

/// Parses the name and description from a `SKILL.md` markdown file.
///
/// Looks for YAML frontmatter between `---` fences. If not found or missing a name,
/// falls back to the parent directory name.
pub fn parse_skill_markdown(path: &Path, content: &str) -> (String, String) {
    let mut name = String::new();
    let mut description = String::new();

    let lines: Vec<&str> = content.lines().collect();
    if lines.first().map(|l| l.trim()) == Some("---") {
        for line in lines.iter().skip(1) {
            let trimmed = line.trim();
            if trimmed == "---" {
                break;
            }
            if let Some(rest) = trimmed.strip_prefix("name:") {
                name = rest.trim().to_string();
            } else if let Some(rest) = trimmed.strip_prefix("description:") {
                description = rest.trim().to_string();
            }
        }
    }

    if name.is_empty() {
        name = path
            .parent()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unnamed-skill".to_string());
    }

    (name, description)
}

/// Discovers global User-scope and project Workspace-scope skills.
///
/// Scans:
/// - Tier 1 (User): `~/.gemini/antigravity/builtin/skills`, `~/.gemini/skills`, `~/.agents/skills`
/// - Tier 2 (Workspace): `<workspace_root>/.agents/skills`
pub async fn scan_skills(workspace_root: Option<&Path>) -> Result<Vec<SkillInfo>, DaemonError> {
    let workspace_root = workspace_root.map(|p| p.to_path_buf());

    tokio::task::spawn_blocking(move || {
        let mut skills = Vec::new();

        // 1. User tier
        if let Some(home) = std::env::var_os("HOME") {
            let home_path = PathBuf::from(home);
            let user_paths = [
                home_path.join(".gemini/antigravity/builtin/skills"),
                home_path.join(".gemini/skills"),
                home_path.join(".agents/skills"),
            ];

            for path in &user_paths {
                scan_skill_directory(path, SkillScope::User, &mut skills);
            }
        }

        // 2. Workspace tier
        if let Some(ws_path) = workspace_root {
            let ws_skills_dir = ws_path.join(".agents/skills");
            scan_skill_directory(&ws_skills_dir, SkillScope::Workspace, &mut skills);
        }

        Ok(skills)
    })
    .await
    .map_err(|e| DaemonError::Internal(format!("Skill scanner worker panicked: {e}")))?
}

fn scan_skill_directory(base_dir: &Path, scope: SkillScope, out: &mut Vec<SkillInfo>) {
    if !base_dir.is_dir() {
        return;
    }

    let entries = match std::fs::read_dir(base_dir) {
        Ok(e) => e,
        Err(err) => {
            tracing::debug!(
                "Could not read skills directory {}: {err}",
                base_dir.display()
            );
            return;
        }
    };

    for entry_res in entries {
        let entry = match entry_res {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();
        if path.is_dir() {
            let skill_md = path.join("SKILL.md");
            if let Ok(content) = std::fs::read_to_string(&skill_md) {
                let (name, description) = parse_skill_markdown(&skill_md, &content);
                out.push(SkillInfo {
                    name,
                    description,
                    scope,
                    path: path.to_string_lossy().into_owned(),
                });
            }
        }
    }
}
