# RPC-391 — Render Colored Edit/Write Diffs in the Rust Agent View

> **Depends on:** RPC-390 (diff generation + `[R]-`/`[A]+` marker encoding).

## Problem Statement

After RPC-390 the Rust `fspec-tui` crate can turn an Edit/Write tool call into a
marker-encoded diff string. This card **wires that generator into the live agent view** and
**renders the markers as colored lines**, achieving parity with the TypeScript reference
(`src/tui/components/AgentView.tsx`, TUI-038):

- Removed lines: dark-red background `#8B0000` (rgb 139,0,0), white text.
- Added lines: dark-green background `#006400` (rgb 0,100,0), white text.
- Context lines: line-number gutter in gray, content default white.

## How the TS Reference Works (data flow)

1. **At tool-call time** the Edit/Write input (`old_string`, `new_string`, `content`,
   `file_path`) is stashed in `pendingToolDiffsRef` keyed by tool-call id, and the edit's
   start line is precomputed via `calculateStartLine` (`AgentView.tsx:2067-2113`,
   `:781-813`). `PendingToolDiff` interface: `AgentView.tsx:866-874`.
2. **At ToolResult time** the stashed entry is looked up; `formatEditDiff`/`formatWriteDiff`
   + `formatDiffForDisplay` produce two strings:
   - `toolResultContent` — collapsed at `DIFF_COLLAPSED_LINES = 25`.
   - `toolResultFullContent` — full (for the `/expand` modal).
   (`AgentView.tsx:2173-2208`, mirrored `:3300-3331`; also `processChunksToConversation`
   `:390-410`.)
3. **At render time** `VirtualList renderItem` (`AgentView.tsx:5310-5448`) decodes the
   markers when `line.role === 'tool'`:
   - `[R]`/`[A]` present → strip the 3-char marker, render the whole line with the colored
     **background** (`DIFF_COLORS.removed`/`added`) + white fg.
   - No marker but matches `/^[L ]?\s*\d+\s{3}/` → context line: gray line-number gutter,
     default content.

## Rust Architecture (mirror, adapted to ratatui)

The Rust port computes wrapped `Line<'static>` values at the **store/wrap layer**
(`store/agent_view/chunk_wrap.rs`) rather than in a React render callback. So the marker
decode happens in `wrap_*` instead of a separate render component, but the **encode-in-store
/ decode-at-wrap** split is preserved.

### Step 1 — Capture Edit/Write input at tool-call time
- In `chunk_processor.rs::handle_tool_call` (`:108`), when `info.name` lowercases to
  `edit`/`replace`/`write`/`write_file`, parse `info.input` JSON and stash a
  `PendingToolDiff { tool_name, file_path, old_string, new_string, content, start_line }`
  keyed by `info.id` on `SessionContext` (new field, e.g. `pending_tool_diffs: HashMap<String, PendingToolDiff>`).
- Compute `start_line` via RPC-390's `calculate_start_line`.

### Step 2 — Produce the diff on ToolResult
- In `chunk_processor.rs::handle_tool_result` (`:128`), look up the pending entry by
  `info.tool_call_id`. If present:
  - Edit family → `format_edit_diff(old, new)`; Write family → `format_write_diff(content)`.
  - `format_diff_for_display(lines, DIFF_COLLAPSED_LINES, start_line)` → the marker string.
  - Store the marker string as the tool card body (replacing the raw `info.content` append),
    and keep a **full** (uncollapsed) variant for the `TurnContentModal` (the Rust modal
    that mirrors the TS `/expand` view). The full text must stay in `ChunkSource.text` (or a
    parallel field) so the modal shows the complete diff.
  - Remove the pending entry after use.
- Non-Edit/Write tools keep the existing raw behaviour untouched.

### Step 3 — Mark the chunk as a diff card
- `ChunkKind::ToolCall` needs to know it carries a diff so `wrap_*` decodes markers AND
  bypasses the generic RPC-389 8-line/streaming-window collapse (diffs do their own
  collapse inside `format_diff_for_display`). Options (pick the cleanest):
  - Add `is_diff: bool` to `ChunkKind::ToolCall { tool_call_id, is_error, is_diff }`, OR
  - Detect by checking for `[R]`/`[A]`/diff-context markers in the body.
  Prefer an explicit `is_diff` flag (no string sniffing).

### Step 4 — Decode markers into colored lines at the wrap layer
- In `chunk_wrap.rs`, for a diff tool card, do NOT apply the 8-line collapse
  (`collapse_tool_body`). Instead, split the (already collapsed by RPC-390) body into lines
  and for each line:
  - Contains `[R]` → strip the 3-char `[R]` marker; emit a `Line` whose span style is
    `Style::default().bg(Color::Rgb(139,0,0)).fg(Color::White)`.
  - Contains `[A]` → strip `[A]`; `bg(Color::Rgb(0,100,0)).fg(Color::White)`.
  - Matches the context pattern `^[L ]?\s*\d+\s{3}` → split the line-number gutter from the
    content; gutter span `fg(Color::Gray)`, content span default white.
  - Otherwise → existing default styling.
- Keep the `● ` header prefix on the first (header) line exactly as the current
  `wrap_tool_call` does.

### DIFF_COLORS (parity constants)
```rust
const DIFF_BG_REMOVED: Color = Color::Rgb(139, 0, 0); // #8B0000
const DIFF_BG_ADDED:   Color = Color::Rgb(0, 100, 0); // #006400
```

## Important Interactions / Pitfalls
- **RPC-389 collapse must NOT double-apply.** Diff bodies are already collapsed to 25 lines
  with their own `... +N lines (select turn to /expand)` indicator by RPC-390. The 8-line
  `collapse_tool_body` path is for non-diff tools only.
- **Streaming.** Edit/Write tools are atomic (no `ToolProgress` stream), so the
  streaming-window path should not engage for diff cards. Verify `is_streaming` stays false
  for diff cards (a ToolResult settles them immediately).
- **TurnContentModal parity.** The Rust modal must show the FULL diff (uncollapsed). Ensure
  the full marker string is retained and the modal also decodes `[R]`/`[A]` markers (the TS
  `TurnContentModal.tsx:71-96` duplicates the decode). If the Rust modal renders plain
  `ChunkSource.text`, it will show markers literally — decode there too.
- **Non-diff tools unaffected.** Bash/Grep/etc. must render exactly as before (regression
  tests).

## Suggested File Touch-Points
- `codelet/fspec-tui/src/store/agent_view/chunk_processor.rs` — `handle_tool_call`,
  `handle_tool_result`.
- `codelet/fspec-tui/src/store/agent_view/chunk_wrap.rs` — marker decode in `wrap_tool_call`.
- `codelet/fspec-tui/src/store/agent_view/session_context.rs` (or wherever `SessionContext`
  lives) — add `pending_tool_diffs` map + `PendingToolDiff` struct (watch the 300-LoC
  ceiling; new module if needed, e.g. `pending_tool_diff.rs`).
- `codelet/fspec-tui/src/views/agent/*` — `ChunkKind::ToolCall { is_diff }` if chosen.
- Rust `TurnContentModal` equivalent — full-diff decode.

## Coding Standards
- No `unwrap()`/`expect()`/`todo!()`/`unimplemented!()` in production code.
- Zero `cargo clippy -p codelet-fspec-tui --all-targets` warnings; `cargo fmt --check` clean.
- Keep every touched file < 300 LoC.

## Acceptance Criteria (Example-Mapping seeds)

**Rules**
1. An Edit tool call's `old_string`/`new_string` are captured at tool-call time and consumed
   on the matching ToolResult to build a diff.
2. A Write tool call's `content` is rendered as an all-additions diff.
3. Removed diff lines render with a dark-red (`#8B0000`) background and white text.
4. Added diff lines render with a dark-green (`#006400`) background and white text.
5. Context lines render with a gray line-number gutter and default-white content.
6. The `[R]`/`[A]` marker characters are stripped before display (never shown literally).
7. Diff cards bypass the RPC-389 8-line tool-output collapse (they self-collapse at 25).
8. Non-Edit/Write tool results render unchanged (no regression).
9. The full (uncollapsed) diff is available to the turn-content modal and decoded there too.

**Examples (green cards)**
- Edit replacing one line → red old line + green new line visible with colored backgrounds.
- Write of a new 3-line file → three green-background lines.
- Edit with no captured pending entry (e.g. malformed input) → falls back to raw text, no panic.
- Bash tool result → still plain white, no diff coloring (regression).
- Edit producing > 25 display lines → inline shows 25 + `... +N lines` indicator; modal shows all.
- Context line `  250   foo` → `250` gutter gray, `foo` white.

## Verification
- `cargo test -p codelet-fspec-tui` — new tests green, full suite green (no regressions).
- `cargo clippy -p codelet-fspec-tui --all-targets` — zero warnings.
- `cargo fmt --check` — clean.
- Manual/automated assertion that a removed line's span carries `bg == Color::Rgb(139,0,0)`
  and an added line's span carries `bg == Color::Rgb(0,100,0)` (inspect produced `Line`/`Span`
  styles in tests).
