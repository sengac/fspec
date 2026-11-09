# BUG-331 root cause: FspecBackend::submit_input defaults to dropping the input mode

## Problem

- `FspecBackend::submit_input` has a default body
  (`rust/fspec-tui/src/transport/backend/subscriptions.rs:155-166`):
  `let _ = mode; self.send_input(id, text).await`. Its doc says it exists "so
  mock/test backends compile unchanged".
- The TUI routes steer / queue / interrupt submissions through it
  (`rust/fspec-tui/src/app/dispatch_input_modes.rs:20-27`,
  `dispatch_slash_submit.rs:155`). Any implementor that does not override it silently
  turns a steer or interrupt into a plain `send_input` turn: the delivery mode the user
  chose is discarded with no error — a fallback path.
- Both real transports override it with the `submit_input` RPC
  (`transport/embedded/subscriptions.rs:146-156`,
  `transport/websocket/scheduling_loops.rs:162-175`).
- Test backends that rely on the default:
  - `ModelBackend` (`tests/common/model_backend.rs:57-…`) and `ContinueBackend`
    (`tests/common/continue_backend.rs:66-…`) forward a fixed method list to an inner
    `MockBackend`; `submit_input` is not in the list, so the default sends a plain turn
    to the inner mock and the mode never reaches `MockBackend::submitted_inputs`.
  - `HistoryMockBackend` (`tests/app_dispatch_history_rpc025.rs:86`) and
    `DefaultsOnlyBackend` (`tests/provider_settings_oauth_disconnect_prov112.rs:546`).
  - `MockBackend` (`tests/common/mock/mock_backend_1.rs:85-96`) already overrides it and
    records `(id, text, mode)`.

## Fix

- Remove the default: `submit_input` becomes a required trait method, so every backend
  must state how it delivers each mode (the compiler enforces it).
- Transports keep their RPC implementations.
- Test backends implement it explicitly and record the mode:
  `ModelBackend` / `ContinueBackend` forward it to the inner `MockBackend` (which records
  the mode); `HistoryMockBackend` and `DefaultsOnlyBackend` record `(id, text, mode)`.
- `app_dispatch_history_rpc025.rs` (655 lines) and
  `provider_settings_oauth_disconnect_prov112.rs` (737 lines) must change, so in this
  card their tests move into child modules (`tests/rpc025_history/`, `tests/prov112/`)
  that import the root with `use super::*`, keeping every file under 300 lines; their
  coverage mappings are relocated to the test fns (the stale RPC-025 ranges, which
  pointed 30-40 lines off, are corrected on the way).

## Alternatives rejected

- Keeping the default but logging a warning: still a silent fallback for the user (their
  steer becomes a queued turn).
- A default that returns an error: compiles for implementors that forgot the method and
  only fails at runtime; a required method fails at compile time.

## Other fallbacks reviewed for this card

The two fallbacks kept by BUG-326 and BUG-327 are handled separately:
BUG-327's replay fallbacks for rows without a stored envelope get their own card (they
change the persistence write and replay contract, not the TUI trait); BUG-326's
`#[serde(default)]` on `current_context_output_tokens` is reviewed in that card's
document.
