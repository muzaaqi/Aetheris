//! Unified diff generation for file tool execution preview.
//!
//! Generates standard Git/POSIX unified diff strings for code additions,
//! modifications, and deletions without requiring external dependencies.

use std::path::Path;

/// Generates a standard unified diff string between old and new text.
pub fn generate_unified_diff(
    old_content: Option<&str>,
    new_content: &str,
    file_label: &str,
) -> String {
    let clean_label = file_label.trim_start_matches('/');

    match old_content {
        None => {
            let lines: Vec<&str> = new_content.lines().collect();
            if lines.is_empty() {
                return String::new();
            }
            let mut out = format!(
                "--- /dev/null\n+++ b/{clean_label}\n@@ -0,0 +1,{} @@\n",
                lines.len()
            );
            for line in lines {
                out.push('+');
                out.push_str(line);
                out.push('\n');
            }
            out
        }
        Some(old) => {
            if old == new_content {
                return String::new();
            }

            let old_lines: Vec<&str> = old.lines().collect();
            let new_lines: Vec<&str> = new_content.lines().collect();

            let m = old_lines.len();
            let n = new_lines.len();

            let mut dp = vec![vec![0u32; n + 1]; m + 1];
            for i in 0..m {
                for j in 0..n {
                    if old_lines[i] == new_lines[j] {
                        dp[i + 1][j + 1] = dp[i][j] + 1;
                    } else {
                        dp[i + 1][j + 1] = dp[i][j + 1].max(dp[i + 1][j]);
                    }
                }
            }

            #[derive(Debug, PartialEq, Eq)]
            enum DiffOp<'a> {
                Keep(&'a str),
                Delete(&'a str),
                Insert(&'a str),
            }

            let mut ops = Vec::new();
            let mut i = m;
            let mut j = n;
            while i > 0 || j > 0 {
                if i > 0 && j > 0 && old_lines[i - 1] == new_lines[j - 1] {
                    ops.push(DiffOp::Keep(old_lines[i - 1]));
                    i -= 1;
                    j -= 1;
                } else if j > 0 && (i == 0 || dp[i][j - 1] >= dp[i - 1][j]) {
                    ops.push(DiffOp::Insert(new_lines[j - 1]));
                    j -= 1;
                } else if i > 0 && (j == 0 || dp[i][j - 1] < dp[i - 1][j]) {
                    ops.push(DiffOp::Delete(old_lines[i - 1]));
                    i -= 1;
                }
            }
            ops.reverse();

            let mut out = format!(
                "--- a/{clean_label}\n+++ b/{clean_label}\n@@ -1,{} +1,{} @@\n",
                m, n
            );
            for op in ops {
                match op {
                    DiffOp::Keep(line) => {
                        out.push(' ');
                        out.push_str(line);
                        out.push('\n');
                    }
                    DiffOp::Delete(line) => {
                        out.push('-');
                        out.push_str(line);
                        out.push('\n');
                    }
                    DiffOp::Insert(line) => {
                        out.push('+');
                        out.push_str(line);
                        out.push('\n');
                    }
                }
            }
            out
        }
    }
}

/// Asynchronously computes a unified diff between an existing on-disk file and proposed new content.
pub async fn compute_file_diff(target_path: &Path, new_content: &str) -> Option<String> {
    let label = target_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file");

    let old_content = match tokio::fs::read_to_string(target_path).await {
        Ok(s) => Some(s),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(_) => return None,
    };

    let diff = generate_unified_diff(old_content.as_deref(), new_content, label);
    if diff.is_empty() { None } else { Some(diff) }
}
