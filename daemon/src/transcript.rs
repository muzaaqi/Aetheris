//! In-kernel inotify watcher for Antigravity and agent transcripts.
//!
//! Streams real-time incremental token chunks from active session transcript
//! files with zero polling overhead and constant memory footprint.

use std::io::SeekFrom;
use std::path::PathBuf;
use std::sync::Arc;

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio::sync::{Mutex, broadcast, mpsc};
use tokio::task::JoinHandle;

use crate::error::DaemonError;
use crate::protocol::{ServerMessage, TokenStreamPayload};

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
        let status = val.get("status").and_then(|s| s.as_str());
        let done =
            status == Some("DONE") || val.get("done").and_then(|d| d.as_bool()).unwrap_or(false);

        if !content.is_empty() || done {
            return Some((content.to_string(), done));
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
    path: PathBuf,
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
                path: PathBuf::new(),
                offset: 0,
                remainder: String::new(),
            })),
        }
    }

    /// Attaches the inotify watcher to `path` and begins streaming updates.
    pub async fn watch_file(&mut self, path: PathBuf) -> Result<(), DaemonError> {
        // Stop any previous worker
        if let Some(worker) = self.worker.take() {
            worker.abort();
        }
        self.watcher = None;

        let initial_offset = tokio::fs::metadata(&path)
            .await
            .map(|m| m.len())
            .unwrap_or(0);

        {
            let mut state = self.state.lock().await;
            state.path = path.clone();
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

        // If path exists, watch it directly or watch parent
        let watch_target = if path.exists() {
            path.clone()
        } else if let Some(parent) = path.parent() {
            parent.to_path_buf()
        } else {
            path.clone()
        };

        watcher
            .watch(&watch_target, RecursiveMode::NonRecursive)
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

    /// Reads and processes any newly appended bytes from the watched file.
    async fn process_appended_bytes(
        state: &Arc<Mutex<WatcherState>>,
        event_tx: &broadcast::Sender<ServerMessage>,
    ) {
        let mut state = state.lock().await;
        let path = state.path.clone();

        let mut file = match File::open(&path).await {
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
        let mut buffer = vec![0u8; bytes_to_read];
        if let Err(e) = file.read_exact(&mut buffer).await {
            tracing::warn!("Failed to read new transcript bytes: {e}");
            return;
        }

        state.offset = file_len;

        let incoming_text = String::from_utf8_lossy(&buffer);
        let full_text = format!("{}{}", state.remainder, incoming_text);

        let mut lines: Vec<&str> = full_text.split('\n').collect();
        // The last element is either empty (if ended with \n) or incomplete remainder
        if let Some(tail) = lines.pop() {
            state.remainder = tail.to_string();
        } else {
            state.remainder.clear();
        }

        for line in lines {
            if let Some((token, done)) = parse_transcript_line(line) {
                let msg = ServerMessage::TokenStream(TokenStreamPayload { token, done });
                let _ = event_tx.send(msg);
            }
        }
    }
}
