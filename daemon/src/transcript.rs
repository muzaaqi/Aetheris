//! In-kernel inotify watcher for Antigravity and agent transcripts.
//!
//! Streams real-time incremental token chunks from active session transcript
//! files with zero polling overhead and constant memory footprint.

use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio::sync::{Mutex, broadcast, mpsc};
use tokio::task::JoinHandle;

use crate::error::DaemonError;
use crate::protocol::{ServerMessage, TokenStreamPayload};

const READ_CHUNK_SIZE: usize = 64 * 1024; // 64 KB bounded chunk size

/// Finds the most recently modified transcript file under an Antigravity brain directory.
///
/// Searches recursively for `*/.system_generated/logs/transcript.jsonl` files.
pub fn find_latest_transcript(brain_dir: &Path) -> Option<PathBuf> {
    if !brain_dir.is_dir() {
        return None;
    }

    let mut latest_path = None;
    let mut latest_time = std::time::SystemTime::UNIX_EPOCH;

    let walker = ignore::WalkBuilder::new(brain_dir)
        .hidden(false)
        .git_ignore(false)
        .build();

    for entry in walker.flatten() {
        let path = entry.path();
        if path.is_file()
            && path
                .file_name()
                .map(|f| f == "transcript.jsonl")
                .unwrap_or(false)
            && let Ok(metadata) = path.metadata()
            && let Ok(mtime) = metadata.modified()
            && mtime > latest_time
        {
            latest_time = mtime;
            latest_path = Some(path.to_path_buf());
        }
    }

    latest_path
}

/// Parses an individual JSONL transcript line.
///
/// Returns `Some((token, done))` if the line contains a model response or completion event.
pub fn parse_transcript_line(line: &str) -> Option<(String, bool)> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let val: serde_json::Value = serde_json::from_str(trimmed).ok()?;
    let source = val.get("source").and_then(|s| s.as_str());
    let entry_type = val.get("type").and_then(|t| t.as_str());

    if source == Some("MODEL")
        || entry_type == Some("PLANNER_RESPONSE")
        || entry_type == Some("MODEL_RESPONSE")
    {
        let content = val.get("content").and_then(|c| c.as_str()).unwrap_or("");
        let thinking = val.get("thinking").and_then(|t| t.as_str()).unwrap_or("");
        let status = val.get("status").and_then(|s| s.as_str());
        let has_tool_calls = val
            .get("tool_calls")
            .and_then(|tc| tc.as_array())
            .map(|a| !a.is_empty())
            .unwrap_or(false);

        // If content is empty but thinking is present, stream thinking
        let text_to_emit = if !content.is_empty() {
            content
        } else if !thinking.is_empty() {
            thinking
        } else {
            ""
        };

        // If both content and thinking are empty (e.g. tool execution step), skip emitting premature completion
        if text_to_emit.is_empty() && has_tool_calls {
            return None;
        }

        let done =
            status == Some("DONE") || val.get("done").and_then(|d| d.as_bool()).unwrap_or(false);

        if !text_to_emit.is_empty() || done {
            return Some((text_to_emit.to_string(), done));
        }
    }

    None
}

/// Watches an active transcript file and broadcasts incremental token updates.
pub struct TranscriptWatcher {
    event_tx: broadcast::Sender<ServerMessage>,
    watcher: Option<RecommendedWatcher>,
    worker: Option<JoinHandle<()>>,
    notify_tx: Option<mpsc::Sender<()>>,
    state: Arc<Mutex<WatcherState>>,
}

struct WatcherState {
    target_path: PathBuf,
    offset: u64,
    remainder: String,
}

impl TranscriptWatcher {
    /// Creates a new `TranscriptWatcher` connected to the broadcast server channel.
    pub fn new(event_tx: broadcast::Sender<ServerMessage>) -> Self {
        Self {
            event_tx,
            watcher: None,
            worker: None,
            notify_tx: None,
            state: Arc::new(Mutex::new(WatcherState {
                target_path: PathBuf::new(),
                offset: 0,
                remainder: String::new(),
            })),
        }
    }

    /// Attaches the inotify watcher to `path` and begins streaming updates.
    ///
    /// If `path` is a directory, resolves the latest transcript file inside it.
    pub async fn watch_file(&mut self, path: PathBuf) -> Result<(), DaemonError> {
        // Stop any previous worker
        if let Some(worker) = self.worker.take() {
            worker.abort();
        }
        self.watcher = None;

        let resolved_file = if path.is_dir() {
            find_latest_transcript(&path).unwrap_or_else(|| path.clone())
        } else {
            path.clone()
        };

        let initial_offset = if resolved_file.is_file() {
            tokio::fs::metadata(&resolved_file)
                .await
                .map(|m| m.len())
                .unwrap_or(0)
        } else {
            0
        };

        {
            let mut state = self.state.lock().await;
            state.target_path = resolved_file.clone();
            state.offset = initial_offset;
            state.remainder.clear();
        }

        let (notify_tx, mut notify_rx) = mpsc::channel::<()>(32);
        self.notify_tx = Some(notify_tx.clone());

        let callback_tx = notify_tx.clone();
        let mut watcher = RecommendedWatcher::new(
            move |res: std::result::Result<Event, notify::Error>| {
                if let Ok(event) = res
                    && matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_))
                {
                    let _ = callback_tx.try_send(());
                }
            },
            Config::default(),
        )
        .map_err(|e| DaemonError::Internal(format!("Failed to create inotify watcher: {e}")))?;

        // If target is file, watch parent dir if available; if dir, watch dir
        let watch_dir = if resolved_file.is_file() {
            resolved_file.parent().unwrap_or(&path).to_path_buf()
        } else {
            path.clone()
        };

        let mode = if path.is_dir() {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };

        watcher
            .watch(&watch_dir, mode)
            .map_err(|e| DaemonError::Internal(format!("Failed to watch transcript path: {e}")))?;

        self.watcher = Some(watcher);

        let state_clone = self.state.clone();
        let event_tx_clone = self.event_tx.clone();

        let worker = tokio::spawn(async move {
            while notify_rx.recv().await.is_some() {
                Self::process_appended_bytes(&state_clone, &event_tx_clone).await;
            }
        });

        self.worker = Some(worker);

        Ok(())
    }

    /// Reads and processes any newly appended bytes from the watched file in bounded chunks.
    async fn process_appended_bytes(
        state: &Arc<Mutex<WatcherState>>,
        event_tx: &broadcast::Sender<ServerMessage>,
    ) {
        let mut state = state.lock().await;
        let mut file_path = state.target_path.clone();

        // If target is directory, check if a transcript file has appeared
        if file_path.is_dir() {
            if let Some(found) = find_latest_transcript(&file_path) {
                file_path = found;
                state.target_path = file_path.clone();
            } else {
                return;
            }
        }

        let mut file = match File::open(&file_path).await {
            Ok(f) => f,
            Err(_) => return,
        };

        let file_len = match file.metadata().await {
            Ok(m) => m.len(),
            Err(_) => return,
        };

        // File truncated or rotated: reset offset
        if file_len < state.offset {
            tracing::info!(
                "Transcript file truncated or rotated (was {}, now {}), resetting offset",
                state.offset,
                file_len
            );
            state.offset = 0;
            state.remainder.clear();
        }

        if file_len == state.offset {
            return;
        }

        if let Err(e) = file.seek(SeekFrom::Start(state.offset)).await {
            tracing::warn!("Failed to seek transcript file: {e}");
            return;
        }

        let bytes_to_read = (file_len - state.offset) as usize;
        let mut remaining = bytes_to_read;
        let mut chunk_buf = vec![0u8; READ_CHUNK_SIZE];

        while remaining > 0 {
            let to_read = remaining.min(READ_CHUNK_SIZE);
            if let Err(e) = file.read_exact(&mut chunk_buf[..to_read]).await {
                tracing::warn!("Failed to read transcript chunk: {e}");
                return;
            }
            remaining -= to_read;
            state.offset += to_read as u64;

            let incoming_text = String::from_utf8_lossy(&chunk_buf[..to_read]);
            state.remainder.push_str(&incoming_text);

            while let Some(pos) = state.remainder.find('\n') {
                let line = state.remainder[..pos].to_string();
                state.remainder.drain(..=pos);
                if let Some((token, done)) = parse_transcript_line(&line) {
                    let msg = ServerMessage::TokenStream(TokenStreamPayload { token, done });
                    let _ = event_tx.send(msg);
                }
            }
        }
    }
}
