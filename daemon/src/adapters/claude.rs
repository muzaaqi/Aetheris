//! Claude Code CLI adapter implementation.
//!
//! Normalizes commands for Anthropic's Claude Code CLI.

use futures_util::future::{self, BoxFuture};

use super::AgentAdapter;
use crate::error::DaemonError;

/// Adapter for Anthropic Claude Code CLI.
#[derive(Debug, Default)]
pub struct ClaudeAdapter {
    mode: std::sync::RwLock<String>,
}

impl ClaudeAdapter {
    /// Creates a new `ClaudeAdapter`.
    pub fn new() -> Self {
        Self {
            mode: std::sync::RwLock::new("safe".to_string()),
        }
    }
}

impl AgentAdapter for ClaudeAdapter {
    fn platform(&self) -> &'static str {
        "claude"
    }

    fn send_prompt<'a>(
        &'a self,
        prompt: &'a str,
        attachments: &'a [String],
    ) -> BoxFuture<'a, Result<(), DaemonError>> {
        let prompt_len = prompt.len();
        let att_len = attachments.len();
        Box::pin(async move {
            tracing::info!(
                "Claude Code adapter received prompt ({prompt_len} chars, {att_len} attachments)"
            );
            Ok(())
        })
    }

    fn set_mode<'a>(&'a self, mode: &'a str) -> BoxFuture<'a, Result<(), DaemonError>> {
        let mode_lower = mode.to_lowercase();
        match mode_lower.as_str() {
            "safe" | "yolo" | "plan" => {
                if let Ok(mut lock) = self.mode.write() {
                    *lock = mode_lower;
                }
                Box::pin(future::ready(Ok(())))
            }
            invalid => {
                let err = DaemonError::Config(format!(
                    "Unsupported mode '{invalid}' for Claude Code. Supported: safe, yolo, plan"
                ));
                Box::pin(future::ready(Err(err)))
            }
        }
    }
}
