# Aetheris 🌌

> **OS-Level AI Agent HUD for Hyprland & Caelestia Shell**

Aetheris is an operating-system-level AI agent conversational and orchestration interface embedded directly into the top bar of the Hyprland desktop environment.

Powered by an ultra-lightweight **Rust** background daemon (~8 MB RAM) and a native **Caelestia Quickshell (QML)** frontend.

---

## 🏛️ Architecture Overview

- **`daemon/`**: Backend service written in Rust (`tokio`, `hyprland-rs`, `notify`, `ignore`). Responsible for tracking the active Hyprland workspace, watching Antigravity transcript logs in real-time, scanning local/global skills, and serving the local WebSocket JSON-RPC protocol.
- **`qml/`**: Caelestia Quickshell user interface modules (Top bar status icon & glassmorphic dropdown HUD overlay).
- **`docs/`**: Full architectural and protocol specifications (`DESIGN.md`).
- **`systemd/`**: User service unit (`aetheris.service`) for seamless background autostart.

---

## 🚀 Project Roadmap

- [x] Design & WebSocket Protocol Specifications (`docs/DESIGN.md`)
- [ ] **Phase 1:** Rust Daemon Core + QML Top Bar MVP
- [ ] **Phase 2:** Project Context, `@file` Mentions, & Live Antigravity Transcript Streaming
- [ ] **Phase 3:** Safety Controls, Human-in-the-Loop Inline Approval, & MCP Visualizer
- [ ] **Phase 4:** Quote Engine, Thinking Slider, & Dynamic Bar Status Icon
- [ ] **Phase 5:** Token Tracker, Snip-to-Prompt Vision, & Artifacts Split-View

For full technical specifications, see [docs/DESIGN.md](docs/DESIGN.md).
