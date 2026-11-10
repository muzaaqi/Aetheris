//! Aetheris Daemon
//! OS-Level AI Agent HUD background service for Hyprland and Caelestia Shell.

pub mod config;
pub mod error;
pub mod protocol;
pub mod server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Aetheris daemon starting...");
    // Daemon implementation starts in Phase 1
    Ok(())
}
