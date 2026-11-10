//! Universal JSON-RPC protocol message schemas for Aetheris.
//!
//! Provides bidirectional contracts between the Caelestia QML HUD (client)
//! and the Aetheris Rust background daemon (server).

use serde::{Deserialize, Serialize};

// ============================================================================
// Client -> Daemon Messages
// ============================================================================

/// Incoming messages sent from the Caelestia HUD frontend to the daemon.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum ClientMessage {
    /// Send a user chat prompt, optionally with attachments and quotes.
    #[serde(rename = "send_message")]
    SendMessage(SendMessagePayload),

    /// Approve or reject an intercepted tool execution.
    #[serde(rename = "tool_approval")]
    ToolApproval(ToolApprovalPayload),

    /// Update runtime daemon settings (agent backend, mode, thinking budget).
    #[serde(rename = "update_config")]
    UpdateConfig(UpdateConfigPayload),
}

/// Payload for `send_message` requests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendMessagePayload {
    /// The prompt or instruction text.
    pub text: String,

    /// Absolute file paths of dropped or attached files/images.
    #[serde(default)]
    pub attachments: Vec<String>,

    /// Optional highlighted snippet or code block citation.
    #[serde(default)]
    pub quote: Option<QuotePayload>,
}

/// Quoted snippet citation metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuotePayload {
    /// File path and line range origin (e.g. `src/db.ts:12-20`).
    pub source: String,

    /// The quoted text snippet content.
    pub text: String,
}

/// Payload for `tool_approval` responses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolApprovalPayload {
    /// Unique identifier of the tool call requiring approval.
    pub tool_call_id: String,

    /// Whether the user approved execution.
    pub approved: bool,
}

/// Payload for `update_config` requests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateConfigPayload {
    /// Target platform backend (e.g. `"antigravity"`, `"claude"`, `"direct"`).
    pub platform: String,

    /// Execution mode (e.g. `"safe"`, `"yolo"`, `"plan"`).
    pub mode: String,

    /// Thinking effort budget level (e.g. `"off"`, `"low"`, `"medium"`, `"high"`).
    pub thinking: String,
}

// ============================================================================
// Daemon -> Client Messages
// ============================================================================

/// Outgoing messages pushed from the daemon to the Caelestia HUD frontend.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum ServerMessage {
    /// Real-time token chunk streamed from the active agent backend.
    #[serde(rename = "token_stream")]
    TokenStream(TokenStreamPayload),

    /// Human-in-the-loop tool execution approval request.
    #[serde(rename = "tool_call_request")]
    ToolCallRequest(ToolCallRequestPayload),

    /// Active Hyprland workspace and project context synchronization.
    #[serde(rename = "workspace_sync")]
    WorkspaceSync(WorkspaceSyncPayload),
}

/// Payload for `token_stream` notifications.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenStreamPayload {
    /// The incremental text chunk.
    pub token: String,

    /// Whether this chunk marks the completion of the response stream.
    pub done: bool,
}

/// Payload for `tool_call_request` notifications.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCallRequestPayload {
    /// Unique identifier for this tool call.
    pub tool_call_id: String,

    /// Tool or MCP function name (e.g. `"run_command"`, `"write_file"`).
    pub tool_name: String,

    /// Arguments payload passed to the tool.
    pub args: serde_json::Value,

    /// Whether this tool requires user approval before execution.
    pub requires_approval: bool,
}

/// Payload for `workspace_sync` notifications.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceSyncPayload {
    /// Active Hyprland workspace ID.
    pub workspace_id: i32,

    /// Resolved project root directory path.
    pub path: String,

    /// Current git branch name if inside a git repository.
    #[serde(default)]
    pub git_branch: Option<String>,

    /// Total count of discovered skills available in current context.
    #[serde(default)]
    pub skills_count: u32,

    /// Names of available MCP servers.
    #[serde(default)]
    pub mcp_servers: Vec<String>,
}
