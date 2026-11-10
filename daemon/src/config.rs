//! Daemon configuration management for Aetheris.

/// Configuration parameters for the Aetheris background daemon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonConfig {
    /// Host interface to bind the WebSocket server.
    pub host: String,
    /// TCP port to listen for HUD WebSocket connections.
    pub port: u16,
    /// Capacity of the internal broadcast channel for event fan-out.
    pub broadcast_capacity: usize,
    /// Maximum allowable WebSocket frame size in bytes.
    pub max_frame_size: usize,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 21420,
            broadcast_capacity: 64,
            max_frame_size: 1_048_576,
        }
    }
}

impl DaemonConfig {
    /// Formats the socket address string (`host:port`).
    pub fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_binds_to_expected_address() {
        let config = DaemonConfig::default();
        assert_eq!(config.addr(), "127.0.0.1:21420");
        assert_eq!(config.broadcast_capacity, 64);
        assert_eq!(config.max_frame_size, 1_048_576);
    }
}
