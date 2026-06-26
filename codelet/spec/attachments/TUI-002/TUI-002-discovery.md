# TUI-002 — Persist & re-apply default thinking level so `[T:High]` shows on idle

## Summary

In the TS Ink reference the chat header shows a yellow **`[T:High]`** thinking
badge on an **idle** session. The Rust ratatui session shows **no** thinking
badge. The badge renderer and the per-session `get_thinking_level` RPC wiring are
**already correct** in Rust — the gap is that the **default** thinking level is
neither persisted nor re-applied to new/resumed sessions, so Rust sessions start
at `Off`.

## Critical context: renderer + per-session RPC already work

Do **not** modify the badge rendering. These are already at parity:

- `codelet/fspec-tui/src/views/agent/header_build.rs::thinking_label` →
  `Off→None`, `Low→"Low"`, `Medium→"Med"`, `High→"High"`, painted yellow as
  `[T:<label>]` in `build_left_line` (mirrors `SessionHeader.tsx:81-94,175`).
- RPC path: `get_thinking_level` → `Action::ThinkingLevelLoaded` →
  store → widget. Spawned on `SessionCreated` in
  `codelet/fspec-tui/src/app/dispatch_session_chrome.rs`.
- The `/thinking` dialog already emits `Action::SetThinkingLevelDefault`, routed
  in `codelet/fspec-tui/src/app/dispatch_model_thinking_dialogs.rs::handle_set_thinking_level_default`
  → `backend.set_thinking_level_default(session_id, level)`.

## How the badge value is chosen (already implemented)

`SessionHeader.tsx:137`:
```ts
const displayLevel = isLoading && thinkingLevel !== null ? thinkingLevel : baseThinkingLevel;
```
i.e. **when idle it shows the base (default) level**. The Rust header reads the
session's base thinking level the same way. So if the base level is `Off`, no
badge — which is exactly the current Rust behaviour.

## Root cause — default level is in-memory only, never persisted/re-applied

`codelet/sessions/src/handle_impl.rs` (~line 844):

```rust
fn set_thinking_level_default(&self, session_id: &SessionId, level: ThinkingLevel) -> Result<(), String> {
    let uuid = uuid_from(session_id);
    match self.get_session(&uuid.to_string()) {
        Ok(session) => { session.set_base_thinking_level(level as u8); Ok(()) } // in-memory ONLY
        Err(_) => Err(format!("Session not found: {}", session_id.value.as_str())),
    }
}
```

Two missing behaviours vs TS:

1. **Persistence.** TS `defaultThinkingLevelConfig.ts` writes the chosen default
   to the user config key `tui.defaultThinkingLevel` (integer 0-3) via
   `writeConfig('user', …)` and reads it back with validation. Rust persists
   nothing.
2. **Re-application on session creation.** TS `useDefaultThinkingLevel` loads the
   persisted default on mount and pushes it into each new/resumed session via
   `setBaseThinkingLevel(sessionId, level)`. Rust applies nothing on
   `SessionCreated`, so every fresh session is `Off`.

## How TypeScript does it (behaviour to mirror)

| Concern | TS source | Behaviour |
|---|---|---|
| Persist default | `src/tui/config/defaultThinkingLevelConfig.ts` | `saveDefaultThinkingLevel(level)` → `tui.defaultThinkingLevel`; `loadDefaultThinkingLevel()` validates int 0-3, default `Off` |
| Apply per session | `src/tui/hooks/useDefaultThinkingLevel.ts` | on mount + on new/resumed session → `setBaseThinkingLevel(sid, level)` |
| Set from dialog | same hook (`setDefault`) | `saveDefaultThinkingLevel(level)` then apply to current session |

## Required behaviour (Rust)

1. **Persist** the default thinking level to a host config (e.g. under
   `~/.fspec`), value `0..=3`. Add load + save helpers in `codelet-sessions`
   (or the existing config module) with validation/clamping and an `Off`
   fallback for a missing/invalid file. Pure functions → unit-testable with a
   temp dir (no real `$HOME` writes; redirect via the existing test-helper
   pattern).
2. **`set_thinking_level_default`** must persist the level (in addition to the
   in-memory `set_base_thinking_level`). Persisting should not be gated on the
   session existing — match TS where the default is a user-level setting (decide
   ordering: persist always; apply to session when present).
3. **On `SessionCreated`**, load the persisted default and apply it to the new
   session's base thinking level **before/with** the `get_thinking_level` fetch
   in `codelet/fspec-tui/src/app/dispatch_session_chrome.rs` (or the
   server-side session-creation path), so the very first idle render shows
   `[T:High]` when a default is set.

## Open question for Example Mapping (resolve during ACDD)

- Where exactly should the default be applied — server-side at
  `create_session` (so ALL transports inherit it) or client-side in
  `dispatch_session_chrome`? Prefer the server/session-creation path so both
  embedded and websocket transports get it for free, mirroring TS applying it on
  every new/resumed session. Confirm via the existing `SessionCreated` wiring.

## Tests to add (ACDD)

- Config round-trip: save `High` → load returns `High`; missing file → `Off`;
  out-of-range value → clamped/`Off`.
- `set_thinking_level_default` persists (re-load reflects the new value).
- New session created with a persisted `High` default → its base thinking level
  is `High` → header renders `[T:High]` (yellow) when idle.
- Persisted `Off` → no badge.

## Files in scope

| Purpose | Path |
|---|---|
| Stub to extend (server) | `codelet/sessions/src/handle_impl.rs` (`set_thinking_level_default`) |
| New persistence helpers | `codelet/sessions/src/` (new module or existing config) |
| Apply on session create | `codelet/fspec-tui/src/app/dispatch_session_chrome.rs` and/or session-creation path |
| Renderer (reference only) | `codelet/fspec-tui/src/views/agent/header_build.rs` |
| TS reference | `src/tui/config/defaultThinkingLevelConfig.ts`, `src/tui/hooks/useDefaultThinkingLevel.ts` |

## Out of scope

- The model name / `[R]` / `[V]` / size badge (TUI-001).
- Changing the badge label mapping or colour (already correct).
- The per-prompt effective thinking level shown while streaming (already wired).
