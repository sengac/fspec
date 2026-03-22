# PROV-046: Refactoring Guide — History Persistence + Compaction Path Cleanup

## Refactoring Opportunity

PROV-046 adds pre-compaction history persistence. The compaction path currently lives in `interactive_helpers.rs` with tight coupling to `stream_loop.rs`. This card cleans up that interface.

## Current Compaction Path Smell: Scattered State

Compaction logic is split across 4 files with no cohesive boundary:

| File | What It Does |
|------|-------------|
| `stream_loop.rs` | Pre-prompt check (lines 654-705), hook detection, signal_compaction_needed(), post-loop retry (lines 2013-2301) |
| `interactive_helpers.rs` | `execute_compaction()` — the actual orchestrator |
| `compaction_dag.rs` | DAG instructions, detection, force-inject |
| `compaction_threshold.rs` | Threshold math |

The problem: `stream_loop.rs` has ~200 lines of compaction orchestration (pre-check + post-loop retry) that rightfully belong in the compaction subsystem.

## What This Card Extracts

### New Module: `history_persistence.rs` (~80 lines)

```rust
use std::path::PathBuf;
use chrono::Utc;
use rig::message::Message;

const MAX_HISTORY_FILES: usize = 10;

/// Persist full conversation history to JSONL before lossy compaction.
/// Returns the path where history was saved.
pub async fn persist_before_compaction(
    messages: &[Message],
    session_id: &str,
    history_dir: &Path,
) -> Result<PathBuf> {
    let timestamp = Utc::now().format("%Y%m%dT%H%M%S");
    let filename = format!("session_{}_{}.jsonl", session_id, timestamp);
    let path = history_dir.join(&filename);

    // Ensure directory exists
    tokio::fs::create_dir_all(history_dir).await?;

    // Write each message as a JSON line
    let mut file = tokio::fs::File::create(&path).await?;
    for msg in messages {
        let line = serde_json::to_string(msg)?;
        file.write_all(line.as_bytes()).await?;
        file.write_all(b"\n").await?;
    }

    // Rotate: keep only MAX_HISTORY_FILES most recent
    rotate_history_files(history_dir, session_id, MAX_HISTORY_FILES).await?;

    Ok(path)
}

async fn rotate_history_files(dir: &Path, session_id: &str, keep: usize) -> Result<()> {
    let prefix = format!("session_{}_", session_id);
    let mut entries: Vec<_> = tokio::fs::read_dir(dir).await?
        .filter_map(|e| async { e.ok() })
        .filter(|e| e.file_name().to_string_lossy().starts_with(&prefix))
        .collect().await;

    entries.sort_by_key(|e| e.file_name());

    if entries.len() > keep {
        for entry in &entries[..entries.len() - keep] {
            let _ = tokio::fs::remove_file(entry.path()).await;
        }
    }
    Ok(())
}
```

### Changes to `interactive_helpers.rs`

In `execute_compaction()`, before `reset_session_to_reminders()`:

```rust
// PROV-046: Persist full history before lossy compaction
let history_dir = session.data_dir().join("history");
let history_path = persist_before_compaction(
    &session.messages,
    session.session_id(),
    &history_dir,
).await.unwrap_or_else(|e| {
    warn!("Failed to persist history before compaction: {}", e);
    PathBuf::from("(failed to save)")
});
info!("PROV-046: Pre-compaction history saved to {:?}", history_path);
```

Then append a reference to the compaction instruction:

```rust
// After building the compaction instruction, append history reference
let instruction = format!(
    "{}\n\n---\nFull conversation history before compaction: {}. Use Read tool to search for specific details.",
    compaction_instruction,
    history_path.display()
);
```

## Session ID Gap

The current `Session` struct has no `session_id` field. Options:

1. **Pass it in**: `execute_compaction(session, ..., session_id)` — simple, minimal change
2. **Add to Session**: `Session::new()` generates a UUID — cleaner but larger change
3. **Derive from provider+timestamp**: fragile, not recommended

Recommendation: **Option 2** — add `session_id: String` to Session, generated in `new()`. This is needed by PROV-048 (streaming diagnostics) too.

```rust
impl Session {
    pub fn session_id(&self) -> &str { &self.session_id }
}
```

## .gitignore Update

```
# PROV-046: Pre-compaction conversation history
.codelet/history/
```

## SOLID Alignment

| Principle | How |
|-----------|-----|
| **SRP** | History persistence is a single module, not mixed into compaction logic |
| **OCP** | Adding history formats (JSON, Markdown) = extend persist fn, don't modify compaction |
| **DRY** | History path reference built once, used in instruction template |

## Estimated Impact

- **Lines added to stream_loop.rs**: 0 (all changes in interactive_helpers.rs)
- **New module**: `history_persistence.rs` (~80 lines)
- **Modified**: `interactive_helpers.rs` (~15 lines added to execute_compaction)
- **Modified**: `session/mod.rs` (~5 lines for session_id)
