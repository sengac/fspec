# RPC-400 — Stderr line coloring parity (TS → Rust fspec-tui)

## Symptom
In the Rust ratatui TUI (`codelet/fspec-tui/`), stderr output from bash tool
calls is **not** rendered red the way the original TypeScript Ink TUI renders
it, and the literal `⚠stderr⚠` sentinel can survive to the screen. Only
**whole-result** errors (`is_error=true`, i.e. a non-zero exit) turn the card
body red today; per-line stderr on an otherwise-successful command is shown in
the normal body color, and the marker text is never stripped.

## The stderr marker mechanism (shared, Rust)
The bash tool marks stderr lines with a sentinel:

- `codelet/tools/src/bash_output.rs:13` — `pub const STDERR_MARKER: &str = "⚠stderr⚠";`
- `append_stderr_if_present` (`:88-101`) prefixes each stderr line with the
  marker when appending stderr to a **successful** result body.
- `combine_outputs` (`:106-131`) prefixes each stderr line with the marker in a
  **failed** result body.
- Live streaming: `bash_streams.rs::spawn_stderr_reader:114` calls
  `emit_tool_progress(session_id, &line, /*is_stderr=*/true)`, so each live
  stderr chunk carries `is_stderr=true` on `ToolProgressInfo`
  (`codelet/rpc-types/src/lib.rs:829-834`, field `is_stderr: bool`).

So the marker appears in two ways:
1. **Settle path** — `ToolResultInfo.content` already contains `⚠stderr⚠`-prefixed
   lines (bash produced them) with `is_error` = the whole-command error flag.
2. **Live path** — `ToolProgressInfo.is_stderr=true` for stderr chunks (the
   chunk text itself does NOT carry the marker; the flag does).

## TypeScript reference (the behavior we must match)
`src/tui/components/AgentView.tsx`:

- **Live path** (`:2482-2490`, also `:3489-3494`): when
  `chunk.toolProgress.isStderr`, each output line is prefixed with `⚠stderr⚠`
  before being folded into the tool card body. This converts the live
  `is_stderr` FLAG into the same in-band MARKER the settle path uses.
- **Render** (`:5393-5422`): `STDERR_MARKER = '⚠stderr⚠'`; TWO red paths:
  1. `if (line.isError)` → strip ALL markers, render `<Text color="red">`.
  2. `else if (content.includes(STDERR_MARKER))` → strip ALL markers, render
     `<Text color="red">` — **even on a successful command**.
- Test contract: `src/tui/utils/__tests__/stderr-rendering.test.ts` locks the
  marker value `⚠stderr⚠`, the per-line detection, marker stripping, and that
  non-stderr lines are untouched.

## Rust fspec-tui gap
Grep of `codelet/fspec-tui/src` for `stderr|STDERR_MARKER|is_stderr|⚠` → only an
unrelated `"⚠ Interrupted"` status line. Therefore:

- `handle_tool_progress` (`chunk_processor.rs:200-227`) **drops `info.is_stderr`**
  — it appends `info.output_chunk` verbatim, never adding the marker. So the
  live stderr FLAG is lost.
- `handle_tool_result` (`chunk_processor.rs:139-197`) appends `info.content`
  verbatim (the `sanitized` non-diff branch), so any `⚠stderr⚠` from bash is
  stored in `ChunkSource.text` **unstripped**.
- `wrap_source` (`chunk_wrap.rs:52-103`) colors the body red **only** when the
  whole card is `ChunkKind::ToolCall { is_error: true }` (`:55`). There is no
  per-line stderr detection, and the marker is never stripped → it would render
  verbatim.
- `turn_modal` / `diff_decode::style_modal_lines` (non-diff raw-span path) has
  the same gap: no per-line red, no strip.

## Decision / fix design
Mirror the TS two-stage design, keeping the marker as the single in-band signal:

1. **Live path parity** — in `handle_tool_progress`, when `info.is_stderr`,
   prefix each non-empty line of `output_chunk` with `STDERR_MARKER` before
   folding it into the card body (exactly as `AgentView.tsx:2485-2490`). This
   converts the flag into the same marker the settle path already carries. The
   marker constant lives in one Rust place shared by tui + tools; re-export from
   `codelet-tools` (`bash::STDERR_MARKER`) or define a fspec-tui local const
   equal to `"⚠stderr⚠"` locked by a parity test — decide in specifying.
2. **Settle path** — leave `handle_tool_result` storing the bash-produced marker
   in `text` (already happens); do NOT strip at store time (the modal + wrap
   both need to detect it). No change required beyond confirming.
3. **Render (scrollback)** — in `chunk_wrap.rs` non-diff body rendering, detect
   per hard body line: if the WHOLE card `is_error` OR the line contains
   `STDERR_MARKER`, strip every marker occurrence and style that line
   `Color::Red`; otherwise strip any stray marker (defensive) and keep the
   existing body style. Whole-card `is_error` red stays (parity path 1).
4. **Render (modal)** — in `style_modal_lines` non-diff branch (and/or
   `turn_modal`), apply the same per-line strip + red so the "Enter to view
   full" modal matches the scrollback.
5. **Marker never reaches screen** — strip in ALL render paths (scrollback +
   modal), matching TS which always `content.replace(STDERR_MARKER, '')`.

Diff cards (`is_diff: true`) bypass this entirely (they carry no stderr).

## Loci
| Concern | File:approx line |
|---|---|
| Marker const (source of truth) | codelet/tools/src/bash_output.rs:13 |
| Live is_stderr flag on progress | codelet/rpc-types/src/lib.rs:833 |
| Live fold (DROPS is_stderr — FIX) | codelet/fspec-tui/src/store/agent_view/chunk_processor.rs:200-227 |
| Settle fold (stores marker) | codelet/fspec-tui/src/store/agent_view/chunk_processor.rs:169-173 |
| Scrollback body render (no stderr — FIX) | codelet/fspec-tui/src/store/agent_view/chunk_wrap.rs:52-103,157-168 |
| Modal non-diff render (no stderr — FIX) | codelet/fspec-tui/src/store/agent_view/diff_decode.rs:80-99 |
| TS reference live fold | src/tui/components/AgentView.tsx:2482-2490,3489-3494 |
| TS reference render | src/tui/components/AgentView.tsx:5393-5422 |
| TS test contract | src/tui/utils/__tests__/stderr-rendering.test.ts |

## Out of scope
- Changing how bash produces the marker (already correct).
- Whole-result error coloring (already at parity).
- Diff-card styling.
