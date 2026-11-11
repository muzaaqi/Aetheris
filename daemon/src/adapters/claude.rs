//! Claude Code CLI adapter implementation.
//!
//! Normalizes commands for Anthropic's Claude Code CLI.

use futures_util::future::{self, BoxFuture};

use super::AgentAdapter;
use crate::error::DaemonError;
use crate::protocol::ExecutionMode;

/// Adapter for Anthropic Claude Code CLI.
#[derive(Debug, Default)]
pub struct ClaudeAdapter {
    mode: std::sync::RwLock<ExecutionMode>,
}

impl ClaudeAdapter {
    /// Creates a new `ClaudeAdapter`.
    pub fn new() -> Self {
        Self {
            mode: std::sync::RwLock::new(ExecutionMode::Safe),
        }
    }

    /// Returns the current execution mode of the adapter.
    pub fn current_mode(&self) -> ExecutionMode {
        if let Ok(lock) = self.mode.read() {
            *lock
        } else {
            ExecutionMode::Safe
        }
    }

    /// Builds command-line arguments for invoking Claude Code CLI with safety flags.
    pub fn build_cli_args(&self, prompt: &str, mode: ExecutionMode) -> Vec<String> {
        let mut args = vec!["-p".to_string(), prompt.to_string()];
        if mode == ExecutionMode::Yolo {
            args.push("--dangerously-skip-permissions".to_string());
        }
        args
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

    fn set_mode<'a>(&'a self, mode: ExecutionMode) -> BoxFuture<'a, Result<(), DaemonError>> {
        if let Ok(mut lock) = self.mode.write() {
            *lock = mode;
        }
        Box::pin(future::ready(Ok(())))
    }
}
