//! Human-in-the-loop safety interception engine.
//!
//! Manages approval requests, enforces execution modes (Safe, YOLO, Plan-Only),
//! and coordinates async tool authorization via oneshot channels.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::{RwLock, broadcast, oneshot};

use crate::error::{DaemonError, Result};
use crate::protocol::{ExecutionMode, ServerMessage, ToolCallRequestPayload};

/// Thread-safe registry tracking pending tool approvals.
#[derive(Debug, Default)]
pub struct ApprovalRegistry {
    pending: Mutex<HashMap<String, oneshot::Sender<bool>>>,
}

impl ApprovalRegistry {
    /// Creates a new, empty approval registry.
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
        }
    }

    /// Registers a pending approval channel for the given `tool_call_id`.
    pub fn register(&self, tool_call_id: &str) -> oneshot::Receiver<bool> {
        let (tx, rx) = oneshot::channel();
        if let Ok(mut lock) = self.pending.lock() {
            lock.insert(tool_call_id.to_string(), tx);
        }
        rx
    }

    /// Resolves an approval request with the user's decision (`approved`).
    ///
    /// Returns `true` if a pending request existed and was notified.
    pub fn resolve(&self, tool_call_id: &str, approved: bool) -> bool {
        if let Ok(mut lock) = self.pending.lock()
            && let Some(tx) = lock.remove(tool_call_id)
        {
            tx.send(approved).is_ok()
        } else {
            false
        }
    }

    /// Checks whether an approval request is currently pending for `tool_call_id`.
    pub fn has_pending(&self, tool_call_id: &str) -> bool {
        if let Ok(lock) = self.pending.lock() {
            lock.contains_key(tool_call_id)
        } else {
            false
        }
    }

    /// Removes and discards a pending approval channel (e.g. on timeout).
    pub fn remove(&self, tool_call_id: &str) -> Option<oneshot::Sender<bool>> {
        if let Ok(mut lock) = self.pending.lock() {
            lock.remove(tool_call_id)
        } else {
            None
        }
    }
}

/// Evaluates tool calls against the active execution mode and security policies.
pub struct ToolInterceptor {
    mode: Arc<RwLock<ExecutionMode>>,
    registry: Arc<ApprovalRegistry>,
    event_tx: broadcast::Sender<ServerMessage>,
}

impl ToolInterceptor {
    /// Creates a new tool interceptor.
    pub fn new(
        mode: Arc<RwLock<ExecutionMode>>,
        registry: Arc<ApprovalRegistry>,
        event_tx: broadcast::Sender<ServerMessage>,
    ) -> Self {
        Self {
            mode,
            registry,
            event_tx,
        }
    }

    /// Determines whether a tool call performs state mutation (filesystem or process).
    pub fn is_mutating_tool(tool_name: &str) -> bool {
        matches!(
            tool_name.to_lowercase().as_str(),
            "run_command"
                | "write_file"
                | "edit_file"
                | "bash"
                | "execute_command"
                | "terminal"
                | "create_file"
                | "delete_file"
        )
    }

    /// Evaluates a tool call, returning `Ok(true)` if allowed or approved,
    /// `Ok(false)` if rejected by the user, or `Err` if timed out or blocked by policy.
    pub async fn evaluate_tool_call(
        &self,
        tool_call_id: &str,
        tool_name: &str,
        args: serde_json::Value,
        diff: Option<String>,
        timeout: Duration,
    ) -> Result<bool> {
        let current_mode = *self.mode.read().await;

        match current_mode {
            ExecutionMode::PlanOnly => {
                if Self::is_mutating_tool(tool_name) {
                    Err(DaemonError::Interception(format!(
                        "Tool execution rejected: '{tool_name}' mutates state but Plan-Only mode is active"
                    )))
                } else {
                    Ok(true)
                }
            }
            ExecutionMode::Yolo => Ok(true),
            ExecutionMode::Safe => {
                if !Self::is_mutating_tool(tool_name) {
                    return Ok(true);
                }

                let rx = self.registry.register(tool_call_id);

                let request = ServerMessage::ToolCallRequest(ToolCallRequestPayload {
                    tool_call_id: tool_call_id.to_string(),
                    tool_name: tool_name.to_string(),
                    args,
                    requires_approval: true,
                    diff,
                });

                let _ = self.event_tx.send(request);

                match tokio::time::timeout(timeout, rx).await {
                    Ok(Ok(approved)) => Ok(approved),
                    Ok(Err(_)) => Ok(false),
                    Err(_) => {
                        self.registry.remove(tool_call_id);
                        Err(DaemonError::ApprovalTimeout(format!(
                            "Approval timed out after {timeout:?} for tool call '{tool_call_id}'"
                        )))
                    }
                }
            }
        }
    }
}
