# RPC-390 — Port Edit/Write Diff Generation & `[R]-`/`[A]+` Marker Encoding (Rust TUI)

## Problem Statement

The TypeScript reference (`src/tui/components/AgentView.tsx`, TUI-038) renders Edit/Write
tool results as **colored unified-style diffs** (green additions, red removals). The Rust
port (`codelet/fspec-tui`) renders Edit/Write tool results as **plain, uncolored text** —
the entire diff pipeline is unported.

This card ports **only the pure diff-generation logic** — the functions that turn an
`(old_string, new_string)` pair (Edit) or a `content` string (Write) into a
**marker-encoded display string**. Wire-up into the agent view and colored rendering are
**out of scope** and handled by the dependent card **RPC-391**.

Splitting this way keeps the logic independently testable (no terminal, no streaming state)
and mirrors how the TS code separates `src/git/diff-parser.ts` (the Myers diff) from the
`format*` helpers in `AgentView.tsx`.

## Scope

### In Scope (port these TS functions to Rust)

| TS symbol | TS location | Purpose |
|-----------|-------------|---------|
| `computeLineDiff(oldText, newText)` | `src/git/diff-parser.ts:119` | Myers line diff |
| `changesToDiffLines(changes)` | `src/git/diff-parser.ts:132` | `Change[]` → `DiffLine[]` with `+`/`-`/` ` prefixes |
| `formatEditDiff(oldString, newString)` | `AgentView.tsx:623` | Edit → `DiffOutputLine[]` with colors |
| `formatWriteDiff(content)` | `AgentView.tsx:644` | Write → all-additions `DiffOutputLine[]` |
| `formatDiffForDisplay(diffLines, visibleLines, startLine)` | `AgentView.tsx:670` | `DiffOutputLine[]` → marker-encoded display string |
| `calculateStartLine(filePath, oldString, newString)` | `AgentView.tsx:781` | 1-based line number of the edit within the file |

### Out of Scope (→ RPC-391)

- Capturing `old_string`/`new_string`/`content` at tool-call time (`pendingToolDiffsRef`).
- Replacing the raw ToolResult body with the diff in `chunk_processor.rs`.
- Decoding `[R]`/`[A]` markers into colored ratatui spans at the render layer.
- Any change to `ChunkKind` / `wrap_source` / `wrap_tool_call`.

## Reference Behaviour (must match exactly)

### Myers diff — `computeLineDiff`
```ts
export function computeLineDiff(oldText, newText): Change[] {
  return diffLines(oldText, newText, { newlineIsToken: true });
}
```
Rust: use the **`similar`** crate (v2, already a workspace dependency — see
`codelet/git/Cargo.toml:31`). Use `TextDiff::from_lines(old, new)` and iterate
`iter_all_changes()` producing tagged (Equal / Delete / Insert) line ops.

> **Parity note on newlines:** the jsdiff `{ newlineIsToken: true }` option and the later
> `change.value.split('\n').filter(line => line.length > 0)` in `changesToDiffLines` mean
> each output diff line corresponds to one source line, with empty fragments dropped.
> `similar::TextDiff::from_lines` already yields one change per line; trailing newline
> handling must produce the **same set of lines** as the TS pipeline. Cover this with a
> dedicated test (trailing-newline file, file without trailing newline).

### `changesToDiffLines` (prefix encoding)
- Context line → content `" {line}"` (leading space), `type: context`.
- Removed line → content `"-{line}"`, `type: removed`.
- Added line → content `"+{line}"`, `type: added`.
- `changeGroup` (replacement/addition/deletion) is computed in TS but is **only used by the
  git `FileDiffViewer`**, NOT by the AgentView format path. For RPC-390 the `changeGroup`
  field is **not required** — only `content` + `type` feed `formatEditDiff`/`formatDiffForDisplay`.
  Port a minimal `DiffLine { content: String, kind: DiffKind }` shape.

### `formatEditDiff` / `formatWriteDiff` → `DiffOutputLine`
```ts
interface DiffOutputLine { content: string; color: string | null; type: 'context'|'added'|'removed'; }
```
- `formatEditDiff`: map each `changesToDiffLines` entry; color = removed→`#8B0000`,
  added→`#006400`, context→`null`.
- `formatWriteDiff`: split `content` on `\n`; **every** line becomes `+{line}`,
  `type: added`, color `#006400`.

In Rust, model `DiffOutputLine` with an enum `DiffOutputKind { Context, Added, Removed }`.
Color can be carried as the enum itself (the concrete RGB is applied at render time in
RPC-391); but to keep parity of the `formatDiffForDisplay` branch logic, the encoder only
needs `kind`, so the color field may be omitted in Rust if the marker logic keys off `kind`.

### `formatDiffForDisplay` (the marker encoder) — THE CORE
Signature: `(diffLines, visibleLines = DIFF_COLLAPSED_LINES = 25, startLine = 1) -> String`

Algorithm (must replicate precisely — `AgentView.tsx:670-771`):

1. Collect `changedIndices` = indices where `type` is `added` or `removed`.
2. `maxLineNum = startLine + diffLines.length - 1`;
   `lineNumWidth = max(len(str(maxLineNum)), 3)`.
3. **No-changes branch** (`changedIndices` empty): take first `visibleLines` lines, each
   formatted `"{lineNum}   {content[1:]}"` (line number left-padded to `lineNumWidth`, then
   THREE spaces, then the content **with its first char (the prefix) stripped**). If
   `diffLines.length > visibleLines`, append
   `"... +{N} lines (select turn to /expand)"`. Wrap with tree connectors. Return.
4. **Changes branch**:
   - `CONTEXT_LINES = 3`. Build `indicesToShow` = each changed index, plus up to 3 indices
     before and 3 after (clamped to `[0, len-1]`).
   - Sort ascending. Walk them; when there is a gap (`idx > lastShownIdx + 1`), emit a gap
     marker line: `"{pad(lineNumWidth spaces)} ... ({skipped} lines)"` where
     `skipped = idx - lastShownIdx - 1`.
   - For each shown line:
     - `lineNum = pad(startLine + idx, lineNumWidth)`
     - `restOfLine = content[1:]` (strip the `+`/`-`/` ` prefix char)
     - removed → `"{lineNum} [R]- {restOfLine}"`
     - added → `"{lineNum} [A]+ {restOfLine}"`
     - context → `"{lineNum}   {restOfLine}"` (three spaces)
   - After the loop, if `lastShownIdx < len-1`, emit trailing
     `"{pad spaces} ... ({remaining} lines)"`, `remaining = len-1-lastShownIdx`.
   - **Collapse**: if `outputLines.length <= visibleLines` → join + tree connectors.
     Else take first `visibleLines`, append
     `"... +{N} lines (select turn to /expand)"`, then tree connectors.

### Tree connectors — `formatWithTreeConnectors` (`AgentView.tsx:551`)
- Empty/whitespace-only content → `""`.
- Else first line → `"L {line}"`, every subsequent line → `"  {line}"` (two spaces).

> **Rust note:** the existing Rust `wrap_tool_call` already applies a `● ` header prefix and
> its own collapse for non-diff tool output. RPC-390 produces the **same marker string** the
> TS stores in `toolResultContent`; RPC-391 decides where tree connectors / header live. For
> RPC-390, port `formatWithTreeConnectors` and apply it exactly as `formatDiffForDisplay`
> does so the produced string is byte-identical to TS for the same input.

### `calculateStartLine` (`AgentView.tsx:781`)
- If `filePath` is `None` → return `1`.
- Read file (UTF-8). On any IO error → return `1`.
- Search for `new_string` in the file content; if found, return
  `(count of '\n' before the match index) + 1`.
- Else search for `old_string`; same calculation.
- Else return `1`.

Rust: `fn calculate_start_line(file_path: Option<&str>, old: Option<&str>, new: Option<&str>) -> usize`.
Use `std::fs::read_to_string`; `str::find`; count `'\n'` in the prefix. Must NOT panic on
missing files (return 1). No `unwrap()` in production paths.

## Constants (port verbatim)
```rust
const DIFF_COLLAPSED_LINES: usize = 25; // AgentView.tsx:535
const CONTEXT_LINES: usize = 3;         // AgentView.tsx:703
// DIFF_COLORS (used by RPC-391 render): removed #8B0000, added #006400 (AgentView.tsx:608-611)
```

## Suggested File Layout
- New module: `codelet/fspec-tui/src/store/agent_view/diff_format.rs`
  (keep < 300 LoC; if it grows, split `myers.rs` + `display.rs`).
- Add `similar = "2"` to `codelet/fspec-tui/Cargo.toml` (workspace-pinned).
- Export the public functions for RPC-391 to consume.

## Coding Standards
- Rust: no `unwrap()`/`expect()`/`todo!()`/`unimplemented!()` in production code; `Result`
  for IO. `#![allow(clippy::unwrap_used, ...)]` permitted in `#[cfg(test)]` modules only.
- Must pass `cargo clippy -p codelet-fspec-tui --all-targets` with **zero** warnings.
- Must pass `cargo fmt --check`.

## Acceptance Criteria (Example-Mapping seeds)

**Rules**
1. An Edit diff is produced from `(old_string, new_string)` via a Myers line diff.
2. A Write diff treats the entire new file content as additions.
3. Removed lines are encoded `{lineNum} [R]- {content}`; added lines `{lineNum} [A]+ {content}`.
4. Context lines show the line number + three spaces + content, with no marker.
5. Only changed lines plus 3 lines of surrounding context are shown; skipped regions collapse
   to a `... (N lines)` gap marker.
6. Output longer than `DIFF_COLLAPSED_LINES` (25) is truncated with a
   `... +N lines (select turn to /expand)` indicator.
7. Line numbers are offset by `startLine` and left-padded to at least width 3.
8. `calculateStartLine` returns the 1-based line of the edit, or 1 when the file/string is
   unavailable (never panics).
9. Tree connectors: first line prefixed `L `, subsequent lines indented two spaces; empty
   content yields an empty string.

**Examples (green cards)**
- Single-line replacement → one `[R]-` line and one `[A]+` line, both within 3-line context.
- Pure addition (old empty) → only `[A]+` lines, no `[R]-`.
- Write of a 3-line file → three `[A]+` lines.
- 100-line edit with one change in the middle → leading/trailing `... (N lines)` gap markers.
- Diff exceeding 25 display lines → trailing `... +N lines (select turn to /expand)`.
- `calculateStartLine` on a missing file → 1.
- Edit at line 250 of a file → first marker line shows `250` (or offset), width-padded.

## Verification
- `cargo test -p codelet-fspec-tui` (new tests green).
- `cargo clippy -p codelet-fspec-tui --all-targets` (zero warnings).
- Add at least one **golden-string** test asserting the full `formatDiffForDisplay` output
  byte-for-byte against the TS output for a representative edit (compute the TS expectation
  by hand from the algorithm above).
