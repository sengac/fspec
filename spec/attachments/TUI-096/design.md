# TUI-096 — Resume View Rich Session Display

## Problem

The Rust TUI `/resume` view currently renders session rows as:
```
▸ 550e8400-e29b-41d4-a716-446655440004 (idle)
```

This is useless for identifying sessions. The TypeScript version renders:
```
> 🟢 Session Name
    12 messages | openai/gpt-4 | 2 hours ago
```

## Root Cause Analysis

### Current Rendering (`mode_view_render.rs:117`)
```rust
let label = format!(" {marker} {} ({})", info.id, info.status);
```
Only `id` (UUID) and `status` are used. `name`, `message_count`, `provider_id`, `model_id` are ignored.

### SessionInfo Fields Available (`codelet/rpc-types/src/lib.rs:240-254`)
```rust
pub struct SessionInfo {
    pub id: String,
    pub name: String,              // ← EXISTS but unused in rendering
    pub status: String,
    pub project: String,
    pub message_count: u32,        // ← EXISTS but unused in rendering
    pub provider_id: Option<String>, // ← EXISTS but unused in rendering
    pub model_id: Option<String>,    // ← EXISTS but unused in rendering
    pub is_isolated: bool,
    pub worktree_path: Option<String>,
    pub role: Option<String>,
}
```

### Missing: Timestamp

`SessionInfo` has **no timestamp field**. The TypeScript `MergedSession` has `createdAt` and `updatedAt` (ISO strings), but the Rust `SessionInfo` struct doesn't carry this data.

**Where timestamps exist:**
- `SessionManifest` (`codelet/core/src/persistence/manifest.rs:99`) has `created_at: DateTime<Utc>` and `updated_at: DateTime<Utc>` fields
- `list_sessions` in `session_manager.rs:361-408` builds `SessionInfo` from both in-memory `BackgroundSession` and persisted `SessionManifest`
- `BackgroundSession::get_info()` (`background_session.rs:1545-1584`) builds `SessionInfo` but doesn't include timestamps

**Solution:** Add `updated_at_ms: Option<i64>` to `SessionInfo` (Unix epoch milliseconds for cross-language compatibility). Populate it from:
- `BackgroundSession`: use `Utc::now().timestamp_millis()` (no stored timestamp in memory)
- `SessionManifest`: use `manifest.updated_at.timestamp_millis()`

## Design

### Row Format

Each session renders as **two lines** (matching TS parity):

**Line 1 (name):**
```
▸ Session Name
```
- `▸` marker for selected, ` ` for unselected
- Session name (truncated if too long)
- Selected: REVERSED background

**Line 2 (detail):**
```
    12 messages | openai/gpt-4 | 2 hours ago
```
- Indented with 4 spaces
- `message_count` messages
- `provider_id/model_id` (or "unknown" if neither)
- Time ago string (e.g., "2 hours ago", "3 days ago", "just now")
- Selected: REVERSED background, dim text

### Time Ago Formatting

Implement a `format_time_ago` helper in `mode_view_render.rs`:
```rust
fn format_time_ago(updated_at_ms: i64) -> &'static str {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    let diff = now - updated_at_ms;

    if diff < 60_000 { "just now" }
    else if diff < 3_600_000 { format!("{}m ago", diff / 60_000) }
    else if diff < 86_400_000 { format!("{}h ago", diff / 3_600_000) }
    else if diff < 604_800_000 { format!("{}d ago", diff / 86_400_000) }
    else if diff < 2_592_000_000 { format!("{}w ago", diff / 604_800_000) }
    else { format!("{}mo ago", diff / 2_592_000_000) }
}
```

### Scroll Math Impact

With 2 lines per session, `visible_rows` for session count purposes becomes `area.height / 2`. The scroll offset still tracks session index (not visual row), but the rendering loop outputs 2 visual rows per session.

### Files to Modify

1. **`codelet/rpc-types/src/lib.rs`** — Add `updated_at_ms: Option<i64>` to `SessionInfo`
2. **`codelet/sessions/src/background_session.rs`** — Populate `updated_at_ms` in `get_info()`
3. **`codelet/sessions/src/session_manager.rs`** — Populate `updated_at_ms` from `SessionManifest.updated_at` in `list_sessions()`
4. **`codelet/fspec-tui/src/views/agent/mode_view_render.rs`** — Rewrite `render_session_rows` to show rich 2-line format
5. **`codelet/fspec-tui/src/views/agent/resume_session_view.rs`** — Adjust scroll math for 2-line rows

### NAPI Feature Gate

The `SessionInfo` struct has `#[cfg_attr(feature = "napi", napi_derive::napi(object))]`. Adding `updated_at_ms` is backward compatible (new field with `Option<i64>` defaulting to `None`). The NAPI binding auto-generates the JS type with the new optional field.

### Testing

- Unit test for `format_time_ago` with various time deltas
- Integration test for `render_session_rows` verifying 2-line output
- Verify `list_sessions` populates `updated_at_ms` from manifests
