# PROV-046: History Persistence Before Compaction — Implementation Guide

## Problem

Compaction in `execute_compaction()` is lossy — the full conversation is replaced by a summary. If the summary loses critical context (file paths, specific error messages, decisions, tool call details), it's gone permanently. This is the root cause of PromptCancelled recovery sessions where agents lose track of what was accomplished.

## VTCode Reference

### HistoryFileManager (`vtcode-core/src/context/history_files.rs` lines 1–80)

```rust
/// Chat History Files for Dynamic Context Discovery
///
/// Instead of losing conversation details during lossy summarization:
/// 1. Write full conversation to `.vtcode/history/session_{id}_{turn}.jsonl`
/// 2. Include file reference in summary message
/// 3. Agent can search history with `unified_search` when details are needed

pub struct HistoryConfig {
    pub enabled: bool,
    pub max_files_per_session: usize,  // Default: 10
    pub include_tool_results: bool,    // Default: true
}

pub struct HistoryMessage {
    pub turn: usize,
    pub role: String,
    pub content: String,
    pub tool_call_id: Option<String>,
    pub tool_name: Option<String>,
    pub timestamp: DateTime<Utc>,
}
```

### persist_history_before_summarization (`vtcode-core/src/core/agent/runner/summarize.rs` lines 99–141)

```rust
pub(super) fn persist_history_before_summarization(
    &self,
    conversation: &[Content],
    session_id: &str,
    turn_number: usize,
    modified_files: &[String],
    executed_commands: &[String],
) -> Option<std::path::PathBuf> {
    let mut manager = HistoryFileManager::new(&self._workspace, session_id);
    let messages = messages_to_history_messages(
        &messages_from_conversation(conversation), 0
    );
    match manager.write_history_sync(&messages, turn_number, "summarization",
        modified_files, executed_commands)
    {
        Ok(result) => {
            info!(path = %result.file_path.display(),
                  messages = result.metadata.message_count,
                  "Persisted conversation history before summarization");
            Some(result.file_path)
        }
        Err(e) => {
            warn!(error = %e, "Failed to persist conversation history");
            None
        }
    }
}
```

### Reference in summary message (`summarize.rs` lines 74–82)

```rust
let summary = if let Some(path) = history_file_path {
    format!(
        "{}\n\nFull conversation history saved to: {}\n\
         Use grep_file to search for specific details if needed.",
        base_summary, path.display()
    )
} else {
    base_summary
};
```

## Proposed Implementation for fspec

### 1. Add history persistence function to `interactive_helpers.rs` or new module

```rust
// codelet/cli/src/interactive/history_persistence.rs (new file)
use anyhow::Result;
use rig::message::Message;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Persist full conversation history to JSONL before compaction.
/// Returns the file path if successful, for inclusion in the summary.
pub fn persist_history_before_compaction(
    messages: &[Message],
    session_id: &str,
    workspace: &Path,
) -> Option<PathBuf> {
    let history_dir = workspace.join(".codelet").join("history");
    if let Err(e) = std::fs::create_dir_all(&history_dir) {
        warn!("Failed to create history directory: {}", e);
        return None;
    }

    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("session_{}_{}.jsonl", session_id, timestamp);
    let file_path = history_dir.join(&filename);

    match write_messages_jsonl(messages, &file_path) {
        Ok(count) => {
            info!(
                path = %file_path.display(),
                messages = count,
                "Persisted conversation history before compaction"
            );

            // Rotate old files — keep last 10 per session
            if let Err(e) = rotate_history_files(&history_dir, session_id, 10) {
                warn!("Failed to rotate history files: {}", e);
            }

            Some(file_path)
        }
        Err(e) => {
            warn!("Failed to write history file: {}", e);
            None
        }
    }
}

fn write_messages_jsonl(messages: &[Message], path: &Path) -> Result<usize> {
    use std::io::Write;
    let file = std::fs::File::create(path)?;
    let mut writer = std::io::BufWriter::new(file);
    let mut count = 0;

    for (idx, msg) in messages.iter().enumerate() {
        let (role, text) = match msg {
            Message::User { .. } => ("user", message_to_text(msg)),
            Message::Assistant { .. } => ("assistant", message_to_text(msg)),
        };
        let record = serde_json::json!({
            "index": idx,
            "role": role,
            "content": text,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        writeln!(writer, "{}", serde_json::to_string(&record)?)?;
        count += 1;
    }

    Ok(count)
}

fn message_to_text(msg: &Message) -> String {
    // Extract text content from rig::message::Message
    match msg {
        Message::User { content } => {
            content.iter().filter_map(|c| match c {
                rig::message::UserContent::Text(t) => Some(t.text.clone()),
                _ => None,
            }).collect::<Vec<_>>().join("\n")
        }
        Message::Assistant { content } => {
            content.iter().filter_map(|c| match c {
                rig::message::AssistantContent::Text(t) => Some(t.text.clone()),
                _ => None,
            }).collect::<Vec<_>>().join("\n")
        }
    }
}

fn rotate_history_files(dir: &Path, session_id: &str, max_files: usize) -> Result<()> {
    let prefix = format!("session_{}_", session_id);
    let mut files: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with(&prefix))
        .collect();

    files.sort_by_key(|e| e.file_name());

    while files.len() > max_files {
        if let Some(oldest) = files.first() {
            std::fs::remove_file(oldest.path())?;
            files.remove(0);
        }
    }

    Ok(())
}
```

### 2. Call from execute_compaction() in interactive_helpers.rs

**Note:** Session does not currently have `session_id()` or `workspace_path()` methods.
The session ID is generated as a UUID at the NAPI/CLI layer (see `interactive_helpers.rs` line ~346).
The workspace path is typically `std::env::current_dir()`. Either:
- Pass both as parameters to `execute_compaction()`, or
- Add a `session_id: String` field to Session (populated at construction), or
- Generate a deterministic ID from the conversation hash

```rust
pub async fn execute_compaction(
    session: &mut Session,
    compaction_in_progress: Arc<AtomicBool>,
    pending_prompt: Option<&str>,
) -> Result<()> {
    // PROV-046: Persist full history before lossy compaction
    let workspace = std::env::current_dir().unwrap_or_default();
    let session_id = uuid::Uuid::new_v4().to_string(); // Or pass from caller
    let history_path = persist_history_before_compaction(
        &session.messages,
        &session_id,
        &workspace,
    );

    // ... existing compaction logic ...

    // Include history reference in compaction summary if available
    if let Some(path) = history_path {
        // Append to summary message:
        // "Full conversation history saved to: {path}. Use Read to search for details."
    }
}
```

### 3. Add .codelet/history/ to .gitignore

History files should not be committed — they're ephemeral session data.

## Estimated Effort: 3 story points
