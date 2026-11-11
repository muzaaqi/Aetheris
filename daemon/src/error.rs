//! Error types for the Aetheris daemon.

use thiserror::Error;

/// Unified error type for all Aetheris daemon operations.
#[derive(Debug, Error)]
pub enum AetherisError {
    /// Standard I/O errors (filesystem, sockets).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// WebSocket protocol and connection errors (boxed to minimize enum footprint).
    #[error("WebSocket error: {0}")]
    WebSocket(Box<tokio_tungstenite::tungstenite::Error>),

    /// Hyprland IPC communication errors (boxed to minimize enum footprint).
    #[error("Hyprland IPC error: {0}")]
    Hyprland(Box<hyprland::error::HyprError>),

    /// JSON serialization and deserialization errors.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Daemon configuration errors.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Tool execution blocked or rejected by safety policy.
    #[error("Interception error: {0}")]
    Interception(String),

    /// Tool approval request timed out.
    #[error("Approval timeout: {0}")]
    ApprovalTimeout(String),

    /// Internal daemon worker or concurrency errors.
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Backward/plan-compatible alias for the daemon error type.
pub type DaemonError = AetherisError;

impl From<tokio_tungstenite::tungstenite::Error> for AetherisError {
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        Self::WebSocket(Box::new(err))
    }
}

impl From<hyprland::error::HyprError> for AetherisError {
    fn from(err: hyprland::error::HyprError) -> Self {
        Self::Hyprland(Box::new(err))
    }
}

/// Convenience Result alias for Aetheris daemon operations.
pub type Result<T> = std::result::Result<T, AetherisError>;
