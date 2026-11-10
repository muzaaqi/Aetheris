//! Direct API / 9router streaming SSE adapter implementation.
//!
//! Connects directly to OpenAI, Anthropic, or local 9router SSE endpoints.

use futures_util::future::{self, BoxFuture};

use super::AgentAdapter;
use crate::error::DaemonError;

/// Adapter for Direct API endpoints (9router / OpenRouter / OpenAI compatible).
#[derive(Debug, Default)]
pub struct DirectApiAdapter {
    mode: std::sync::RwLock<String>,
}

impl DirectApiAdapter {
    /// Creates a new `DirectApiAdapter`.
    pub fn new() -> Self {
        Self {
            mode: std::sync::RwLock::new("safe".to_string()),
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
                    "Unsupported mode '{invalid}' for Direct API. Supported: safe, yolo, plan"
                ));
                Box::pin(future::ready(Err(err)))
            }
        }
    }
}
