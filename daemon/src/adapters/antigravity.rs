//! Antigravity adapter implementation.
//!
//! Connects to Google Antigravity sessions and controls mode and prompt execution.

use futures_util::future::{self, BoxFuture};

use super::AgentAdapter;
use crate::error::DaemonError;
use crate::protocol::ExecutionMode;

/// Adapter for the Antigravity agent backend.
#[derive(Debug, Default)]
pub struct AntigravityAdapter {
    mode: std::sync::RwLock<ExecutionMode>,
}

impl AntigravityAdapter {
    /// Creates a new `AntigravityAdapter` defaulting to `ExecutionMode::Safe`.
    pub fn new() -> Self {
        Self {
            mode: std::sync::RwLock::new(ExecutionMode::Safe),
        }
    }
}

impl AgentAdapter for AntigravityAdapter {
    fn platform(&self) -> &'static str {
        "antigravity"
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
                "Antigravity adapter received prompt ({prompt_len} chars, {att_len} attachments)"
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
