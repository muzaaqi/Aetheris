# Aetheris 🌌

> **OS-Level AI Agent HUD for Hyprland & Caelestia Shell**

Aetheris adalah antarmuka percakapan dan orkestrasi agent AI tingkat sistem operasi yang tertanam langsung di panel atas (*top bar*) desktop Hyprland.

Ditenagai oleh daemon latar belakang ultra-ringan berbasis **Rust** (~8 MB RAM) dan antarmuka **Caelestia Quickshell (QML)**.

---

## 🏛️ Arsitektur Singkat

- **`daemon/`**: Backend service dalam bahasa Rust (`tokio`, `hyprland-rs`, `notify`, `ignore`). Bertanggung jawab mendeteksi workspace Hyprland aktif, membaca transcript sesi Antigravity secara real-time, memuat skill lokal/global, dan melayani komunikasi WebSocket lokal.
- **`qml/`**: Komponen antarmuka pengguna Caelestia Quickshell (Top bar icon button & dropdown HUD glassmorphic overlay).
- **`docs/`**: Dokumentasi spesifikasi arsitektur (`DESIGN.md`).
- **`systemd/`**: Unit file `aetheris.service` untuk autostart background daemon sebagai user service.

---

## 🚀 Status Proyek

- [x] Spesifikasi Desain & Protokol WebSocket (`docs/DESIGN.md`)
- [ ] **Fase 1:** Rust Daemon Core + QML Top Bar MVP
- [ ] **Fase 2:** Konteks Proyek, Mention `@file`, & Live Antigravity Transcript Stream
- [ ] **Fase 3:** Safety Controls, Human-in-the-Loop Inline Approval, & MCP Visualizer
- [ ] **Fase 4:** Quote Engine, Thinking Slider, & Dynamic Bar Status Icon
- [ ] **Fase 5:** Token Tracker, Snip-to-Prompt Vision, & Artifacts Split-View

Lihat dokumentasi lengkap di [docs/DESIGN.md](docs/DESIGN.md).
