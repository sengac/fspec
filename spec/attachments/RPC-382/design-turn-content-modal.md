# RPC-382 — Port AgentView turn content modal (Enter on selected turn) to Rust

> **Depends on RPC-381** (turn-selection / SELECT mode). This card adds the modal that
> opens when the user presses **Enter** on a selected turn.

## 1. Problem Statement

In the TypeScript reference `AgentView`, while turn-selection mode is active (TUI-042),
pressing **Enter** on the selected turn opens a **`TurnContentModal`** (TUI-045) that
displays the turn's **full** content (`fullContent || content`). This replaced the old
`/expand` command. **Esc** closes the modal first; a **second Esc** exits select mode.

The Rust port has:
- No turn-content modal widget.
- No "full content" source on chunks distinct from the wrapped/collapsed view.
- No Esc-cascade level for "close modal before exiting select mode".

## 2. TypeScript Reference Behaviour

### 2.1 State (`AgentView.tsx:856-857`)
- `showTurnModal` (default `false`).
- `modalMessageIndex: number | null`.

### 2.2 Enter opens the modal (`AgentView.tsx:5467-5474`)
The `VirtualList`'s `onSelect` (active only in select mode):
```ts
onSelect={isTurnSelectMode ? line => {
  setModalMessageIndex(line.messageIndex);
  setShowTurnModal(true);
} : undefined}
```
Enter in item mode routes to `onSelect(item, selectedIndex)` (`VirtualList.tsx:619-624`).

### 2.3 Modal render (`AgentView.tsx:5546-5560`)
Renders only when `showTurnModal && modalMessageIndex !== null && conversation[idx]`:
```tsx
<TurnContentModal
  content={conversation[modalMessageIndex].fullContent
        || conversation[modalMessageIndex].content}
  role={conversation[modalMessageIndex].role}
  terminalWidth={...} terminalHeight={...}
  isFocused={true}
/>
```
- Uses the **full** content (`fullContent` falls back to `content`).
- The underlying list **loses focus** while the modal is open
  (`isFocused={... && !showTurnModal}`, `AgentView.tsx:5458`).
- `fullContent` lives on `ConversationMessage` (`types/conversation.ts:21`).
- Collapsed lines in the main view carry the hint
  `"... +N lines (select turn to /expand)"` (`AgentView.tsx:696, 769`).

### 2.4 Esc cascade (`AgentView.tsx:4792-4834`)
Priority order — the two levels relevant to this card:
- Priority 2: if `showTurnModal` → `setShowTurnModal(false)` and consume.
- Priority 4: if `isTurnSelectMode` → exit select mode (RPC-381).

So one Esc closes the modal; a second Esc leaves SELECT mode.

### 2.5 Tab while modal open (`AgentView.tsx:4840-4844`)
Disabling select mode via Tab also tears down the modal
(`setShowTurnModal(false); setModalMessageIndex(null)`).

## 3. Rust Architecture — Current State

| Concern | Rust location | Status |
|---|---|---|
| Modal state | — | None — add to `AgentView` (`views/agent.rs:85`) |
| Chunk full content | `rendered_chunk.rs` `ChunkSource.text` | Holds body; see §4.2 |
| Existing overlays | `views/agent/confirm_dialog.rs`, `merge_confirm_dialog.rs` | Pattern to follow |
| Enter handling | `views/agent/dispatch.rs` | Suppressed in select mode (RPC-381) |
| Esc cascade | `views/agent/dispatch.rs:182` | Extend with modal level |
| Selected turn | `ScrollbackList::selected_seq()` (RPC-381) | Source of which turn to show |

## 4. Recommended Design

### 4.1 State (`views/agent.rs`)
Add to `AgentView`:
- `turn_modal_seq: Option<u64>` — `Some(seq)` ⇒ modal open for that turn; `None` ⇒ closed.

> Using the chunk `seq` (stable id) instead of an index keeps the modal pinned to the
> correct turn even if chunks mutate while the modal is open. Mirrors RPC-381's decision to
> key selection by `seq`.

### 4.2 Full-content source
Each `RenderedChunk` already carries `source: Option<ChunkSource>` whose `text` is the full
(un-truncated) body that the scrollback re-wraps. The modal should render this `text`
re-wrapped to the modal's inner width. If a chunk has `source: None` (legacy pre-rendered),
fall back to joining its `lines`. No new field is strictly required, but document the
chosen accessor (e.g. `ScrollbackList::full_text_for_seq(seq) -> Option<String>` plus the
chunk's `ChunkKind`/color for role coloring).

### 4.3 `TurnContentModal` widget (new file, e.g. `views/agent/turn_modal.rs`)
- Centered overlay (follow `confirm_dialog.rs` / `merge_confirm_dialog.rs` layout helpers),
  sized relative to the terminal rect, with a bordered block titled by the turn's role
  (e.g. `You` / `Agent` / tool name) using the same color the scrollback uses for that
  `ChunkKind`.
- Body = the full turn text, wrapped to inner width, vertically clipped to the modal
  height (scrolling within the modal is OPTIONAL — out of scope unless trivial; document
  if deferred).
- Keep the file under 300 lines.

### 4.4 Enter opens it (`dispatch.rs`)
In select mode (RPC-381 suppresses Enter-submit). Replace that suppression with:
```rust
if self.turn_select_mode && key.code == KeyCode::Enter {
    if let Some(seq) = current_selected_seq { self.emit(Action::OpenTurnModal(seq)); }
    return EventResult::consumed();
}
```

### 4.5 Esc cascade (`dispatch.rs`)
Order (highest priority first), all consuming:
1. If `turn_modal_seq.is_some()` → `Action::CloseTurnModal` (close modal only).
2. Else if `turn_select_mode` → exit select mode (RPC-381).
3. Else → existing `AgentEscPressed`.

### 4.6 Focus gating
While `turn_modal_seq.is_some()`, the scrollback / turn navigation must not consume keys
(other than Esc to close). Mirror the TS `isFocused={... && !showTurnModal}` so ↑/↓ do not
move the underlying selection while the modal is up. Document where the gate is applied.

### 4.7 Tab tear-down
When `ToggleTurnSelectMode` disables select mode, also clear `turn_modal_seq = None`
(parity with `AgentView.tsx:4840-4844`).

### 4.8 Actions / reducer (`components/mod.rs` + App dispatch)
- `Action::OpenTurnModal(u64)` — set `turn_modal_seq = Some(seq)`.
- `Action::CloseTurnModal` — set `turn_modal_seq = None`.

### 4.9 Rendering hook
In `views/agent.rs` `render_with_store` (or the chrome painter), after painting the
scrollback, if `turn_modal_seq.is_some()`, paint the `TurnContentModal` overlay on top.

## 5. Acceptance-Test Strategy (Rust / `cargo test`)
- **Dispatch**: with select mode active and a turn selected, Enter emits
  `Action::OpenTurnModal(seq)` (NOT `InputSubmitted`).
- **Esc cascade**: with modal open, Esc emits `CloseTurnModal` and does NOT exit select
  mode; with modal closed but select mode on, Esc exits select mode.
- **Tab tear-down**: toggling select mode off clears the modal.
- **Render**: when `turn_modal_seq = Some(seq)`, the rendered buffer contains the selected
  turn's full text and a bordered modal titled by role; scrollback keys are gated.
- **Full content**: a turn whose scrollback view is collapsed still shows the complete
  `ChunkSource.text` in the modal.

## 6. Out of Scope
- In-modal scrolling for very long turns (document if deferred).
- "Discuss Selected" prefill (subordinate-pane feature, not wired into AgentView's Enter).

## 7. Reference File Index
| Purpose | TS | Rust |
|---|---|---|
| Modal state | `AgentView.tsx:856-857` | `views/agent.rs:85` (add `turn_modal_seq`) |
| Enter→open | `AgentView.tsx:5467-5474`, `VirtualList.tsx:619-624` | `views/agent/dispatch.rs` |
| Modal render | `AgentView.tsx:5546-5560` + `TurnContentModal` | `views/agent/turn_modal.rs` (new) |
| Full content | `types/conversation.ts:21` (`fullContent`) | `rendered_chunk.rs` `ChunkSource.text` |
| Esc cascade | `AgentView.tsx:4792-4834` | `views/agent/dispatch.rs:182` |
| Focus gate | `AgentView.tsx:5458` | dispatch focus check |
