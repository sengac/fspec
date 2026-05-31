# TS Ink → Rust ratatui Port Targets

Maps the TypeScript reference implementation files to the exact Rust
files that need to change for RPC-078.

## Read these TS files FIRST (reference, do not modify)

| Purpose | TS file |
|---------|---------|
| Single source of truth for chunk→message mapping (UserInput, Text, IncomingMessage, role colors) | `src/tui/utils/conversationUtils.ts` |
| Streaming Text "..." suffix logic + Done strip | `src/tui/utils/chunkProcessor.ts` |
| Thinking block accumulation + `[Thinking]\n` header | `src/tui/utils/thinkingBlockManager.ts` |
| Top-level chunk dispatcher: Error→modal+inline, UserNotification filter, replace-`⟳ Reconnecting...` semantics, isError red tint | `src/tui/components/AgentView.tsx` (see chunk handler around `case 'Error':` and `case 'Interrupted':`) |
| Inline status convention: `{ type: 'status', content }` rendered as `tool` role | `src/tui/utils/conversationUtils.ts` (line 31: `case 'status': return 'tool';`) |

## Rust files to modify

### Hot path

| File | Change |
|------|--------|
| `codelet/fspec-tui/src/views/agent/chunk_to_lines.rs` | Replace every wrong prefix with the table from `chunk-variant-matrix.md`. Each StreamChunk variant → exactly one mapping function returning `Vec<Line<'static>>`. |
| `codelet/fspec-tui/src/views/agent/scrollback.rs` | Pre-wrap every chunk's lines into one `Line` per visual row using a port of TS `wrapText`. `max_offset_for_viewport` must count Lines (visual rows), not chunks. |
| `codelet/fspec-tui/src/app/dispatch_rpc020.rs` (line ~251-253) | Delete the synchronous `scrollback.push("user> ...")` block. The chunks broadcast path is now the single emitter. |
| `codelet/fspec-tui/src/store/background_session.rs` (line ~1089) | Keep the existing UserInput broadcast — unchanged. |

### Word wrap utility (new)

Create `codelet/fspec-tui/src/views/agent/wrap.rs` mirroring TS
`wrapText`. Input: `&str`, `width: u16`. Output: `Vec<String>` where each
entry's rendered width is ≤ `width`. Respect existing newlines.

### Tests to rewrite (assert OLD wrong prefixes — must be flipped)

| Test file | Wrong literals it currently asserts |
|-----------|-------------------------------------|
| `codelet/fspec-tui/tests/chunk_rendering_parity_rpc071.rs` | `user>`, `assistant>`, `[error]`, `[done]` |
| `codelet/fspec-tui/tests/view_agent_unit_rpc018.rs` | `user>` |
| `codelet/fspec-tui/tests/view_agent_unit_rpc029.rs` | `user>`, `assistant>` |

All three must be updated to the new prefixes BEFORE the implementation
change so the testing→implementing phase has red-then-green parity.

### Tests to add (NEW)

- `tests/chunk_rendering_parity_rpc078.rs` — one scenario per
  `StreamChunk` variant from the matrix.
- `tests/scrollback_wrap_rpc078.rs` — 300-char `API Error:` body in an
  80-col viewport produces ≥4 Line entries with no truncation.
- `tests/scrollback_stick_to_bottom_rpc078.rs` — long wrapped chunk
  followed by short `You: hi`; the bottom row of the rendered buffer
  contains `You: hi`.
- `tests/no_duplicate_user_input_rpc078.rs` — `InputSubmitted` +
  `ChunkReceived(UserInput)` produces exactly one `You: …` line.
- `tests/e2e_scrollback_rpc078_tui_test.rs` — `@microsoft/tui-test`
  equivalent in Rust: real fspec binary, 220-col terminal, stub
  provider, user types, asserts substring counts.
