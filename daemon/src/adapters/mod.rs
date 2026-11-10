//! Multi-platform agent adapter layer.
//!
//! Provides a unified interface for interacting with different agent runtimes:
//! Antigravity CLI, Claude Code CLI, and Direct LLM / 9router streaming SSE.

pub mod antigravity;
pub mod claude;
pub mod direct_api;

use futures_util::future::BoxFuture;

use crate::error::DaemonError;
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

    /// Updates the agent execution mode (`"safe"`, `"yolo"`, `"plan"`).
    fn set_mode<'a>(&'a self, mode: &'a str) -> BoxFuture<'a, Result<(), DaemonError>>;
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
