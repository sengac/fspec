# RPC-403 Investigation: Bracketed paste never reaches agent input — compositor stub drops multi-line pastes

## Symptom

Pasting text (including multi-line text) into the Rust TUI agent-view input does nothing when no
modal is open. When a modal IS open, pasted text arrives as per-character synthetic key events
with `\n` mangled to `KeyCode::Char('\n')`. Multi-line paste into the agent input is impossible,
contributing to the "input only allows one line" report.

## Reference behavior (TS legacy TUI)

- TS TUI has **no bracketed-paste support at all**; Ink chunks flow through the printable filter
  at `src/tui/components/MultiLineInput.tsx:299-306` which strips `\n` (10) and `\r` (13), so TS
  concatenates pasted lines. The TS hook's `insertString` (`useMultiLineInput.ts:269-311`) fully
  supports multi-line strings but is unreachable from paste.
- The Rust TUI should EXCEED the TS behavior here: bracketed paste is already enabled
  (`EnableBracketedPaste`), crossterm delivers `Event::Paste(String)` with `\n` intact, and
  `MultiLineInput` already has a correct `Event::Paste` branch. Only routing is broken.

## Rust architecture (current)

```
crossterm EventStream
  → app/events.rs run loop
      Event::Paste(text) (:213-216)  ── special-cased ──► App::handle_paste (:157-167)
                                                            └─► compositor.handle_paste ONLY (no fallback)
      other events ────────────────────────────────────► handle_event → Navigator → AgentView
```

Key files:

| File | Role |
|---|---|
| `codelet/fspec-tui/src/app/events.rs` | Run loop special-cases `Event::Paste` (:213-216) → `App::handle_paste` (:157-167) which forwards **only** to `self.compositor.handle_paste(text)`. No Navigator/AgentView fallback. |
| `codelet/fspec-tui/src/compositor.rs` | `Compositor::handle_paste` (:188-209) — documented **stub** ("proper paste semantics… arrive in Slice 06 with tui-textarea"): explodes text into per-char synthetic `KeyCode::Char(c)` events dispatched through `Compositor::handle_event` (:132-161), which walks only **modal layers**. AgentView is NOT a compositor layer (rendered separately by Navigator, `app/events.rs:171-175`). |
| `codelet/fspec-tui/src/views/agent/multiline_input.rs` | `handle_event` has an `Event::Paste(s)` branch (:223-236) calling `textarea.insert_str(s)` — **preserves embedded newlines**, but is dead code in the live app (only exercised by unit tests). Gate: `block_edits` (RPC-095 compacting) must also suppress paste. |
| `codelet/fspec-tui/src/components/role_dialog.rs` | Another `Event::Paste` consumer (:155) — also unreachable via the char-splitting stub (receives synthetic char keys instead). |
| `codelet/fspec-tui/src/views/agent/dispatch.rs` | AgentView event entry; paste must flow here when no modal consumes it. |

## Root cause

`Event::Paste` is hijacked at `app/events.rs:214` and sent to the compositor's char-splitting stub
with **no fallback** to the Navigator/AgentView. Consequences:

1. **No modal open** → `Compositor::handle_event` finds no layer to consume the synthetic keys →
   paste is dropped entirely. The agent input receives nothing.
2. **Modal open** → the modal receives per-char synthetic keys; `\n` becomes `KeyCode::Char('\n')`
   which most key handlers ignore or mishandle; grapheme clusters are split; the modal's own
   `Event::Paste` branch (e.g. role_dialog.rs:155) never fires.
3. The correct, newline-preserving implementation in `multiline_input.rs:227-236` is dead code.

## Fix design

1. **Route real `Event::Paste` through the layer chain**: change `Compositor::handle_paste` to
   forward the *actual* `Event::Paste(String)` to the top modal layer's `handle_event` (layers
   already accept `&Event`) instead of exploding into synthetic chars. Delete the char-splitting
   stub behavior.
2. **Add Navigator/AgentView fallback**: in `App::handle_paste` (app/events.rs), if the compositor
   does not consume the paste (no modal open / modal ignored it), forward `Event::Paste` through
   the normal `handle_event` path → Navigator → AgentView → `MultiLineInput::handle_event`, whose
   existing branch inserts the text verbatim (newlines preserved, input auto-grows via
   `visible_rows()` up to 6 rows).
   - Simplest correct shape: make `handle_paste` return consumed/not-consumed from the
     compositor and fall back; or unify by simply routing `Event::Paste` through the SAME path
     as key events (compositor-first, then navigator) — mirror whatever `handle_event` does for
     `Event::Key`.
3. **Respect edit gates**: while `Compacting` (RPC-095 `block_edits`), paste into the agent input
   must be suppressed exactly like typed edits — verify `handle_key_gated`-equivalent gating
   covers the paste branch in `multiline_input.rs` (check whether the paste branch currently
   bypasses the gate; if so, gate it).
4. **Sanitization**: strip `\r` (normalize `\r\n` → `\n`) before insertion so CRLF pastes don't
   embed carriage returns into the buffer. Check what `textarea.insert_str` does with `\r`
   (tui-textarea treats `\n` as line break; raw `\r` would become literal text).

## Interaction with RPC-402

Independent but complementary: RPC-402 enables *typed* newlines (Shift+Enter / Alt+Enter);
this card enables *pasted* newlines. Both rely on the already-working multi-line buffer and
auto-grow rendering in `multiline_input.rs`. If both land, the agent input is fully multi-line.
No code-level dependency; can be implemented in either order. `relates-to` recorded.

## Risks / constraints

- Do not regress modal paste: any modal that previously "worked" via synthetic chars (text fields
  in dialogs that accept Char events) must now handle `Event::Paste` — audit compositor layers
  that accept text input (role_dialog already has a paste branch; check others, e.g. create
  session dialog / rename dialogs) and give each text-editing layer a paste branch or a shared
  helper.
- `compositor.rs` and `multiline_input.rs` (297 LoC) near the 300-LoC limit — may need extraction.
- Very large pastes: `insert_str` is O(n) — acceptable; but consider that `visible_rows()` caps
  at 6 while `cursor_position` math (agent.rs:133-143) does not account for textarea internal
  scroll when line_count > 6 — verify cursor stays sane after a 100-line paste (tui-textarea
  scrolls internally; the hardware-cursor row calc uses `textarea.cursor()` row which can exceed
  the viewport). If broken, clamp row to visible viewport using textarea's viewport offset —
  flag as question during Example Mapping if scope creep.
