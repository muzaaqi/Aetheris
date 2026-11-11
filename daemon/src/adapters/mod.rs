//! Multi-platform agent adapter layer.
//!
//! Provides a unified interface for interacting with different agent runtimes:
//! Antigravity CLI, Claude Code CLI, and Direct LLM / 9router streaming SSE.

pub mod antigravity;
pub mod claude;
pub mod direct_api;

use futures_util::future::BoxFuture;

use crate::error::DaemonError;
use crate::protocol::ExecutionMode;
pub use antigravity::AntigravityAdapter;
pub use claude::ClaudeAdapter;
pub use direct_api::DirectApiAdapter;

/// Universal interface for interacting with an agent backend.
pub trait AgentAdapter: Send + Sync {
    /// Canonical platform identifier (e.g. `"antigravity"`, `"claude"`, `"direct_api"`).
    fn platform(&self) -> &'static str;

    /// Dispatches a user prompt with optional attachment paths to the agent backend.
    fn send_prompt<'a>(
        &'a self,
        prompt: &'a str,
        attachments: &'a [String],
    ) -> BoxFuture<'a, Result<(), DaemonError>>;

    /// Updates the agent execution mode (`ExecutionMode`).
    fn set_mode<'a>(&'a self, mode: ExecutionMode) -> BoxFuture<'a, Result<(), DaemonError>>;

    /// Convenience helper to set mode from string (returns Err if unrecognized mode).
    fn set_mode_str<'a>(&'a self, mode: &'a str) -> BoxFuture<'a, Result<(), DaemonError>> {
        use std::str::FromStr;
        Box::pin(async move {
            let parsed = ExecutionMode::from_str(mode).map_err(DaemonError::Config)?;
            self.set_mode(parsed).await
        })
    }
}

/// Factory function to instantiate an agent adapter by platform identifier.
///
/// Falls back to `AntigravityAdapter` for unknown platform names.
pub fn create_adapter(platform: &str) -> Box<dyn AgentAdapter> {
    match platform.to_lowercase().as_str() {
        "claude" => Box::new(ClaudeAdapter::new()),
        "direct_api" | "direct" | "9router" => Box::new(DirectApiAdapter::new()),
        _ => Box::new(AntigravityAdapter::new()),
    }
}
