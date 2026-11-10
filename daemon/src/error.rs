//! Error types for the Aetheris daemon.

use thiserror::Error;

/// Unified error type for all Aetheris daemon operations.
#[derive(Debug, Error)]
pub enum AetherisError {
    /// Standard I/O errors (filesystem, sockets).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// WebSocket protocol and connection errors.
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    /// Hyprland IPC communication errors.
    #[error("Hyprland IPC error: {0}")]
    Hyprland(#[from] hyprland::error::HyprError),

    /// JSON serialization and deserialization errors.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Daemon configuration errors.
    #[error("Configuration error: {0}")]
    Config(String),
}

/// Convenience Result alias for Aetheris daemon operations.
pub type Result<T> = std::result::Result<T, AetherisError>;
