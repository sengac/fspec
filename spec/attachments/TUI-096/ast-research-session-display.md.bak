# AST Research: SessionInfo struct and render_session_rows

## SessionInfo struct (codelet/rpc-types/src/lib.rs:240-254)
```rust
pub struct SessionInfo {
    pub id: String,
    pub name: String,
    pub status: String,
    pub project: String,
    pub message_count: u32,
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
    pub is_isolated: bool,
    pub worktree_path: Option<String>,
    pub role: Option<String>,
}
```
**Gap:** No `updated_at_ms` field. Need to add `pub updated_at_ms: Option<i64>`.

## render_session_rows (codelet/fspec-tui/src/views/agent/mode_view_render.rs:85-132)
```rust
pub(super) fn render_session_rows(
    area: Rect,
    buf: &mut Buffer,
    sessions: &[SessionInfo],
    selected_index: usize,
    scroll_offset: usize,
) {
    // ... empty placeholder at line 92-103
    // ... single-line rendering at line 109-131
    let label = format!(" {marker} {} ({})", info.id, info.status);
}
```
**Gap:** Only renders `info.id` and `info.status`. Needs to render name, message_count, provider, timestamp.

## list_sessions (codelet/sessions/src/session_manager.rs:361-408)
```rust
pub fn list_sessions(&self) -> Vec<SessionInfo> {
    // In-memory sessions from BackgroundSession::get_info()
    // Persisted sessions from SessionManifest
    SessionInfo {
        id: m.id.to_string(),
        name: m.name,
        status: "idle".to_string(),
        project: m.project.to_string_lossy().to_string(),
        message_count: m.messages.len() as u32,
        provider_id: ...,
        model_id: ...,
        is_isolated: false,
        worktree_path: None,
        role: None,
    }
}
```
**Gap:** SessionManifest has `updated_at: DateTime<Utc>` but it's not passed to SessionInfo.

## BackgroundSession::get_info (codelet/sessions/src/background_session.rs:1545-1584)
```rust
pub fn get_info(&self) -> SessionInfo {
    SessionInfo {
        id: self.id.to_string(),
        name: self.name.read().expect("name lock poisoned").clone(),
        status: self.get_status().as_str().to_string(),
        project: self.project.clone(),
        message_count,
        provider_id: ...,
        model_id: ...,
        is_isolated: self.worktree_path.is_some(),
        worktree_path: ...,
        role: None,
    }
}
```
**Gap:** No timestamp available in memory. Will use `Utc::now().timestamp_millis()`.

## render_pane_scrollbar (codelet/fspec-tui/src/views/diff_common/mod.rs:28-47)
```rust
pub fn render_pane_scrollbar(
    content: Rect,
    buf: &mut Buffer,
    list_width: u16,
    scroll: usize,
    visible: usize,
    total: usize,
) {
    crate::components::list_scrollbar::render_list_scrollbar(
        Rect {
            x: content.x + list_width,
            y: content.y,
            width: 1,
            height: content.height,
        },
        buf,
        scroll,
        visible,
        total,
    );
}
```
**Pattern:** Already exists, ready to use.

## render_list_scrollbar (codelet/fspec-tui/src/components/list_scrollbar.rs:23-50)
```rust
pub fn render_list_scrollbar(
    area: Rect,
    buf: &mut Buffer,
    scroll_offset: usize,
    visible: usize,
    total: usize,
) {
    // thumb_h = ((visible * h) / total).max(1)
    // thumb_pos = (scroll_offset * h) / total
    // Glyphs: ■ (thumb) / │ (track), both Modifier::DIM
}
```
**Pattern:** Already exists, ready to use.
