# Spesifikasi Desain & Arsitektur: Aetheris — OS-Level AI Agent HUD

> **Nama Proyek:** Aetheris  
> **Status:** Approved / Ready for Build  
> **Platform Target:** Linux (Hyprland + Caelestia Quickshell)  
> **Teknologi Utama:** Rust (Backend Daemon) + QML / Quickshell (Frontend Bar & HUD)  
> **Integrasi Agent:** Antigravity (Google DeepMind), Claude Code (Anthropic), Direct LLM API (via 9router / OpenRouter)

---

## 1. Ringkasan Eksekutif & Visi Produk

**Aetheris** adalah antarmuka percakapan dan orkestrasi agent AI tingkat sistem operasi yang tertanam langsung di panel atas (*top bar*) desktop Hyprland. 

Nama **Aetheris** diambil dari *Aether* (udara murni para dewa di langit tertinggi dalam mitologi Yunani), selaras dengan ekosistem desktop *Caelestia* dan *Hyprland*.

Berbeda dengan antarmuka chat berbasis web atau terminal terpisah, Aetheris bertindak sebagai **pusat kendali agent cerdas yang sadar konteks (*context-aware*)**:
- Menempel secara visual dan mulus di bar Caelestia Shell (Glassmorphic, Material 3, mengikuti `scheme.current`).
- Otomatis mendeteksi workspace aktif di Hyprland (berpindah ke workspace proyek langsung berganti konteks, file repo, sesi, dan skill terkait).
- Menjadi jembatan visual untuk engine agent CLI yang sudah ada (Antigravity CLI, Claude Code), membaca transcript secara live, serta menyediakan kontrol perizinan (*human-in-the-loop approval*) dan pratinjau perubahan kode (*diff viewer*).

---

## 2. Arsitektur Sistem

```
┌────────────────────────────────────────────────────────────────────────┐
│                      HYPRLAND DESKTOP ENVIRONMENT                      │
├────────────────────────────────────────────────────────────────────────┤
│  Top Bar (Caelestia QML)                                               │
│  [ Workspace Pager ]    [ Agent HUD Icon 🟢 ]    [ Clock & Status ]   │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ Klik / Shortcut (SUPER + Space)
┌───────────────────────────────────▼────────────────────────────────────┐
│                  CAELESTIA QML HUD OVERLAY (FRONTEND)                  │
│  - Mode Bar: [Antigravity ▼] [⚡ YOLO / 🛡️ Safe] [Thinking: Med ▼]    │
│  - Tabs: [ 💬 Chat & Tools ]  [ 📄 Artifacts & Plans ]                 │
│  - Chat View: Streaming Markdown, Tool Cards, Collapsible Diffs        │
│  - Input Box: Native DropArea (drag files), @file fuzzy picker, Quote  │
└───────────────────────────────────▲────────────────────────────────────┘
                                    │ Local WebSocket (ws://127.0.0.1:21420)
┌───────────────────────────────────▼────────────────────────────────────┐
│                    AGENT-HUD-DAEMON (RUST BACKEND)                     │
│  Memory Footprint: ~8 MB RAM | Zero-GC | tokio async runtime           │
│                                                                        │
│  ┌───────────────────────┐  ┌──────────────────────────────────────┐  │
│  │   Hyprland Monitor    │  │          Transcript Watcher          │  │
│  │   (hyprland-rs IPC)   │  │       (notify inotify engine)        │  │
│  │ - Deteksi workspace   │  │ - Watch Antigravity transcript.jsonl │  │
│  │ - Auto-detect active  │  │ - Stream token & tool calls live     │  │
│  │   working directory   │  │                                      │  │
│  └───────────────────────┘  └──────────────────────────────────────┘  │
│                                                                        │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │                    Platform Normalizer Layer                     │  │
│  │  - Antigravity Adapter (CLI wrapper / transcript pipe)           │  │
│  │  - Claude Code Adapter (CLI child process / stdio JSON)          │  │
│  │  - Direct LLM / 9router Adapter (Streaming SSE API)             │  │
│  │  - Universal Mode & Thinking Effort Translator                   │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                                                        │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │                     Context & Tool Discovery                     │  │
│  │  - Skill Scanner (~/.gemini, ~/.agents, <workspace>/.agents)     │  │
│  │  - MCP Discovery (Neon, Context7, git, project-scoped)           │  │
│  │  - Fast Fuzzy File Indexer (ignore & walkdir crates)             │  │
│  │  - Quota & Token Usage Tracker                                   │  │
│  └──────────────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Spesifikasi Fitur Terinci

### A. Workspace & Context Binding
- **Deteksi Otomatis:** Daemon mendengarkan event socket Hyprland (`activeworkspace` dan `activewindow`).
- **Path Resolution:** Menentukan direktori kerja aktif berdasarkan jendela yang sedang fokus (misal terminal WezTerm di `~/Projects/MuMu` atau Zed editor di `~/.config/hypr`).
- **Session Isolation:** Setiap path memiliki sesi obrolannya sendiri. Saat berpindah dari Workspace 1 ke Workspace 2, HUD otomatis mengalihkan riwayat dan konteks agent.

### B. Universal Multi-Platform Adapter
Menyediakan pengalaman terpadu untuk engine yang berbeda:
1. **Antigravity Adapter:**
   - Memantau `<appDataDir>/brain/<id>/.system_generated/logs/transcript.jsonl` menggunakan `notify`.
   - Mengirim input via subprocess atau IPC socket.
2. **Claude Code Adapter:**
   - Membaca dan menulis ke sesi Claude Code (`claude --dangerously-skip-permissions` atau standard mode).
3. **Direct API Adapter:**
   - Menghubungkan langsung ke endpoint OpenAI/Anthropic/Gemini melalui proxy lokal **9router** (`http://127.0.0.1:20128`).

### C. Normalisasi Mode & Effort
- **Mode Slider:**
  - `Safe Mode`: Mencegat pemanggilan tool dan menunggu klik persetujuan `[Approve]` dari pengguna.
  - `Auto / YOLO Mode`: Memasukkan parameter *auto-accept* (`--mode=accept-edits` untuk Antigravity, `--dangerously-skip-permissions` untuk Claude Code).
  - `Plan-Only Mode`: Membatasi agent ke mode analisis dan penulisan rencana tanpa izin mengedit file.
- **Thinking Slider:**
  - Pengatur universal: `Off` | `Low` | `Medium` | `High`.
  - Diterjemahkan ke parameter asli masing-masing model (misal `budget_tokens` untuk Claude 3.7, `reasoning_effort` untuk OpenAI o3-mini, `thinkingBudget` untuk Gemini Thinking).

### D. Skill & MCP Discovery
- **User-Scope Skills:** Membaca skill dari `~/.gemini/antigravity-cli/builtin/skills`, `~/.gemini/skills`, dan `~/.agents/skills`.
- **Workspace-Scope Skills:** Membaca skill lokal dari `<workspace>/.agents/skills`.
- **MCP Visualizer:** Saat agent menjalankan tool MCP (misal: query Neon DB atau dokumentasi Context7), HUD menampilkan kartu tool informatif lengkap dengan status eksekusi.

### E. Interaksi File & Quote Engine
- **Mention `@file` & `@dir`:** Mengetik `@` memunculkan dropdown autocomplete file proyek menggunakan algoritma fuzzy search super cepat (`ignore` crate).
- **Drag-and-Drop:** File atau gambar yang di-drop ke area chat otomatis dilampirkan ke pesan.
- **Quote Selection:** Meng-highlight teks pada jawaban agent memunculkan tombol `[Quote]` untuk memasukkan teks ke prompt sebagai blok kutipan `> ...`.
- **Smart Code Quote:** Tombol `[Quote]` pada setiap blok kode otomatis menyertakan nama file dan rentang baris (`> [src/index.ts:40-55]`).

### F. Status Top Bar & Notifikasi
- Icon di bar Caelestia memiliki 4 status warna:
  - 🟢 **Idle / Siap**
  - 🟡 **Thinking / Menjalankan Task**
  - 🔴 **Perlu Persetujuan (Approval Diperlukan)**
  - 🔵 **Subagent / Background Job Berjalan**
- Notifikasi desktop via `notify-send` dikirimkan saat task selesai atau saat membutuhkan perhatian pengguna.

---

## 4. Protokol Komunikasi (WebSocket JSON-RPC)

Frontend QML dan Backend Rust berkomunikasi melalui WebSocket lokal (`ws://127.0.0.1:21420`):

### Client $\rightarrow$ Daemon
```json
// Mengirim pesan
{
  "type": "send_message",
  "payload": {
    "text": "Tolong audit performa query ini",
    "attachments": ["/path/to/file.png"],
    "quote": { "source": "src/db.ts:12-20", "text": "const res = await db.query..." }
  }
}

// Menyetujui eksekusi tool
{
  "type": "tool_approval",
  "payload": { "tool_call_id": "call_12345", "approved": true }
}

// Mengubah setting
{
  "type": "update_config",
  "payload": { "platform": "antigravity", "mode": "safe", "thinking": "medium" }
}
```

### Daemon $\rightarrow$ Client
```json
// Token streaming
{
  "type": "token_stream",
  "payload": { "token": "Berikut ", "done": false }
}

// Permintaan izin tool
{
  "type": "tool_call_request",
  "payload": {
    "tool_call_id": "call_12345",
    "tool_name": "run_command",
    "args": { "CommandLine": "bun test tests/api" },
    "requires_approval": true
  }
}

// Status workspace
{
  "type": "workspace_sync",
  "payload": {
    "workspace_id": 1,
    "path": "/home/muzaaqi/Projects/MuMu",
    "git_branch": "main",
    "skills_count": 14,
    "mcp_servers": ["neon", "cloudflare", "context7"]
  }
}
```

---

## 5. Rencana Tahapan Implementasi (Implementation Roadmap)

| Tahap | Fokus Pekerjaan | Deliverable |
|---|---|---|
| **Fase 1 (MVP)** | Rust Daemon Core + QML Top Bar | WebSocket server, deteksi workspace Hyprland, widget QML top bar dengan popout chat, streaming teks dasar. |
| **Fase 2** | Konteks & Multi-Platform | File mention `@`, drag-and-drop file QML, scanner skill 2-tingkat, adapter Antigravity live transcript watcher. |
| **Fase 3** | Keamanan & Eksekusi | Tool call visualizer, tombol inline approval, mode switcher (Safe / YOLO / Plan), MCP support, diff viewer. |
| **Fase 4** | UX Ergonomics | Quote selection & code block quote, slider thinking effort, status icon dinamis di top bar, notifikasi desktop. |
| **Fase 5** | Polish & Vision | Token tracker, kamera screenshot snip-to-prompt, tab viewer dokumen artefak & diagram Mermaid. |

---

## 6. Lokasi Proyek & File Structure

```
~/.config/agent-hud/
├── Cargo.toml                    # Rust crate manifest
├── src/
│   ├── main.rs                   # Entry point, tokio WebSocket server
│   ├── hyprland.rs               # Hyprland IPC listener & workspace tracker
│   ├── transcript.rs             # Inotify watcher untuk Antigravity transcript.jsonl
│   ├── context.rs                # File indexer (@file) & skill scanner
│   ├── adapters/                 # Multi-platform adapters
│   │   ├── mod.rs
│   │   ├── antigravity.rs
│   │   ├── claude.rs
│   │   └── direct_api.rs
│   └── protocol.rs               # Serde models JSON-RPC
└── systemd/
    └── agent-hud.service         # User systemd service unit

~/.config/quickshell/caelestia/
└── modules/bar/
    ├── components/
    │   └── AgentHudButton.qml    # Icon status di bar atas
    └── popouts/
        ├── AgentHudPopout.qml    # Window overlay chat & tool visualizer
        └── components/
            ├── ChatView.qml
            ├── ToolCard.qml
            ├── ModeBar.qml
            └── FileDropArea.qml
```
