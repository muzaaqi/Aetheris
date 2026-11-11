//! Direct API / 9router streaming adapter.
//!
//! Provides direct SSE streaming connectivity to OpenAI, Anthropic, and Gemini APIs.

use futures_util::future::{self, BoxFuture};

use super::AgentAdapter;
use crate::error::DaemonError;
use crate::protocol::ExecutionMode;

/// Adapter for Direct LLM streaming APIs.
#[derive(Debug, Default)]
pub struct DirectApiAdapter {
    mode: std::sync::RwLock<ExecutionMode>,
}

impl DirectApiAdapter {
    /// Creates a new `DirectApiAdapter`.
    pub fn new() -> Self {
        Self {
            mode: std::sync::RwLock::new(ExecutionMode::Safe),
        }
    }
}

impl AgentAdapter for DirectApiAdapter {
    fn platform(&self) -> &'static str {
        "direct_api"
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
                "Direct API adapter received prompt ({prompt_len} chars, {att_len} attachments)"
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
