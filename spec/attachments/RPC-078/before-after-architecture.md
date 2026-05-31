# Duplicate UserInput — Root Cause and Fix

## The bug (before)

```
┌─ user presses Enter on "is this card done?" ─────────────────┐
│                                                              │
│  dispatch_rpc020.rs:251-253                                  │
│  ┌──────────────────────────────────────────┐               │
│  │ scrollback.push(Line::from(              │               │ ← (A) sync push
│  │   format!("user> {}", text)              │               │     wrong prefix
│  │ ));                                       │               │     duplicate
│  └──────────────────────────────────────────┘               │
│                                                              │
│  background_session.rs:1089                                  │
│  ┌──────────────────────────────────────────┐               │
│  │ self.chunks_tx.send(                      │               │
│  │   StreamChunk::UserInput { text }         │               │ ← (B) broadcast
│  │ )?;                                       │               │     correct path
│  └──────────────────────────────────────────┘               │
│                                                              │
│  chunks subscriber                                           │
│    → AgentView::on_chunk_received                            │
│    → scrollback.push(chunk_to_lines(UserInput))              │
│                                                              │
│  Result: TWO lines in scrollback                             │
│    user> is this card done?    ← from (A)                    │
│    You: is this card done?     ← from (B), after RPC-078 fix │
│                                                              │
│  Bug today: BOTH lines are "user> ..." because chunk_to_lines│
│  also emits the wrong prefix.                                │
└──────────────────────────────────────────────────────────────┘
```

## The fix (after)

1. **Delete (A)** — `dispatch_rpc020.rs:251-253`. The synchronous push
   was a stub before the chunks pipeline existed. It now duplicates.
2. **Rewrite chunk_to_lines** — every variant maps to the prefixes from
   `chunk-variant-matrix.md`. `UserInput { text }` → green `You: <text>`.
3. **Keep (B) unchanged** — the broadcast remains the single source of
   truth for "user said X" lines.

```
┌─ user presses Enter on "is this card done?" ─────────────────┐
│                                                              │
│  dispatch_rpc020.rs                                          │
│  ┌──────────────────────────────────────────┐               │
│  │ // sync push removed                      │               │
│  └──────────────────────────────────────────┘               │
│                                                              │
│  background_session.rs:1089                                  │
│  ┌──────────────────────────────────────────┐               │
│  │ chunks_tx.send(UserInput { text })         │               │
│  └──────────────────────────────────────────┘               │
│                                                              │
│  chunk_to_lines (rewritten)                                  │
│    UserInput { text } → vec![                                │
│      Line::from(Span::styled(                                │
│        format!("You: {}", text),                             │
│        Style::default().fg(Color::Green),                    │
│      ))                                                      │
│    ]                                                         │
│                                                              │
│  Result: ONE line                                            │
│    You: is this card done?                                   │
└──────────────────────────────────────────────────────────────┘
```

## Stub session caveat (the only exception)

For the `rpc-no-session-manager` stub session there is no background
session, so (B) never fires. In that single case, `dispatch_rpc020`
must still emit the green `You: <text>` line synchronously — but using
the SAME chunk_to_lines mapping function, never an inline hand-built
`Line::from("user> ...")`.

```rust
// Pseudocode for the stub branch
if session_kind == SessionKind::StubNoManager {
    let lines = chunk_to_lines(&StreamChunk::UserInput { text: text.clone() });
    scrollback.extend(lines);
}
```

This keeps the "one source of truth for prefixes" invariant intact.

## Wrapping bug (independent, same work unit)

`scrollback.rs` today calls `Paragraph::new(lines).render(area, buf)`
with no `.wrap(Wrap { trim: false })`. ratatui silently clips long
lines at `area.width`. Stick-to-bottom math
(`max_offset_for_viewport`) counts the *number of chunks*, not visual
rows.

### Fix

1. Port TS `wrapText(text, width)` to Rust as
   `crate::views::agent::wrap::wrap_to_width(s: &str, width: u16) -> Vec<String>`.
2. In `chunk_to_lines`, pre-wrap every body string into one `Line` per
   visual row using the **current viewport width** (passed in by the
   caller). Each Line is already a visual row.
3. In `scrollback.rs`, render the pre-wrapped Lines with NO wrap (each
   Line is already one row). `max_offset_for_viewport` sums
   `lines.len()` across all chunks instead of chunks themselves.
4. Stick-to-bottom: target offset = `total_lines - viewport_height`.

This matches the TS contract: `messageProcessor` produces an array of
*display lines*, not logical messages.
