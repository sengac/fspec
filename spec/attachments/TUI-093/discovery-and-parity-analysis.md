# TUI-093 — Default Thinking-Level Save/Restore TS Parity (Rust TUI)

## Problem statement

The Rust TUI port persists the user-selected **default thinking level** to the
shared `~/.fspec/fspec-config.json` under `tui.defaultThinkingLevel` (int `0..=3`),
which matches the TypeScript reference's storage. However the user reports they
"don't see it saving/restoring" the way the **default/current model** does.

Investigation (logging-only changes already applied, nothing committed) found the
storage works, but the **application** of the persisted value diverges from the
TypeScript reference in two reinforcing ways.

## Evidence — current Rust behavior

### SAVE path (works on disk, but no visible repaint)
- `D` key in `/thinking` dialog →
  `codelet/fspec-tui/src/app/dispatch_model_thinking_dialogs.rs:196-209`
  (`handle_set_thinking_level_default`) → spawns `backend.set_thinking_level_default(...)`
- RPC: `codelet/rpc/src/lib.rs:1575-1582`
- Handle: `codelet/sessions/src/handle_impl.rs:844-866`
  (`set_thinking_level_default`) → persists ALWAYS + applies in-memory when the
  session exists.
- Persistence: `codelet/sessions/src/default_thinking_level_persistence.rs:54-87`
  writes `tui.defaultThinkingLevel` via read-modify-write (preserves sibling keys).
- **Confirmed on disk:** `~/.fspec/fspec-config.json` contains
  `"tui": { "defaultThinkingLevel": 3 }`.
- **Gap:** `handle_set_thinking_level_default` persists but never re-fetches /
  emits an action to repaint the badge. Contrast with
  `handle_thinking_level_selected` (`dispatch_model_thinking_dialogs.rs:180-185`)
  which DOES drive a refresh. So after `D` the `[T:<level>]` badge can stay stale.

### RESTORE path (only at construction)
- Applied ONLY at session construction:
  - `codelet/sessions/src/session_manager.rs:574-580` (`create_session`)
  - `codelet/sessions/src/session_manager.rs:854-864` (`create_isolated_session`)
  - both call `session.set_base_thinking_level(load_default_thinking_level() as u8)`.
- Badge surfaces via `get_thinking_level` (`handle_impl.rs:919-929`) on
  `refresh_session_chrome`.
- **Gap A — no TUI bootstrap restore step.** The model path has
  `initialize_startup_model` in `codelet/fspec-tui/src/app/bootstrap.rs:67-85`.
  Thinking has no equivalent.
- **Gap B — no re-apply on session activation/resume.** When an existing session
  becomes active or is resumed via `/resume`, nothing re-applies the persisted
  default, so the badge falls back to `Off` despite the value on disk.

### Baseline logs
`set_thinking_level_default` appears **0 times** in
`~/.fspec/logs/fspec-combined.log.2026-06-26`; only tarpc envelope lines for
`set_thinking_level` / `get_thinking_level`. The default path was effectively
invisible (logging-only instrumentation has since been added to
`default_thinking_level_persistence.rs`, `session_manager.rs` restore sites, and
`handle_impl.rs:set_thinking_level_default`).

## The TypeScript reference (target behavior)

`src/tui/hooks/useDefaultThinkingLevel.ts`:

1. **Load on mount** (lines 48-55): `loadDefaultThinkingLevel()` -> `setDefaultLevel`.
2. **Apply to session when session changes OR default loads** (lines 57-71):
   ```ts
   if (defaultLevel !== null && sessionId &&
       appliedToSessionRef.current !== sessionId) {
     appliedToSessionRef.current = sessionId;
     getRustStateSource().setBaseThinkingLevel(sessionId, defaultLevel);
     refreshRustState();
   }
   ```
   This fires for **new AND resumed** sessions when they become active.
3. **`setDefault` applies immediately to the current session** (lines 74-85):
   `saveDefaultThinkingLevel(level)` -> `setDefaultLevel` -> if `sessionId`,
   `setBaseThinkingLevel(sessionId, level)` + `refreshRustState()`.

### Critical guard — `appliedToSessionRef`
TS uses a **per-session-id ref** so the default is applied **at most once per
session id**. This guarantees a manual `/thinking` change within a session is
**never clobbered** by a later re-apply of the default when that same session
regains focus. Any Rust fix MUST replicate this guard (e.g. a
`HashSet<SessionId>` of "already-applied" sessions), or it will stomp per-session
selections.

## TS vs Rust comparison

| Behavior | TypeScript (reference) | Rust (current) | Parity? |
|---|---|---|---|
| Persist on selecting default (D) | saveDefaultThinkingLevel | set_thinking_level_default -> persistence | YES |
| Storage location/format | tui.defaultThinkingLevel (0-3) in fspec-config.json | same | YES |
| Load default on startup | loadDefaultThinkingLevel() on mount | only inside create_session | NO bootstrap step |
| Apply default to NEW session | applied when session becomes active | applied at construction | YES (effectively) |
| Apply default to RESUMED/activated session | yes (effect on sessionId change) | no | NO |
| Repaint badge immediately after D | yes (refreshRustState) | no | NO |
| Once-per-session guard (no clobber) | appliedToSessionRef | n/a (no re-apply at all) | must add with fix |

## Proposed scope (~3 points)

1. **Repaint after `D`** — in `handle_set_thinking_level_default`
   (`dispatch_model_thinking_dialogs.rs:196-209`), after the persist call, also
   refresh thinking-level chrome (fetch via `get_thinking_level` and emit the
   existing `Action::ThinkingLevelLoaded`, mirroring
   `handle_thinking_level_selected`).
2. **Bootstrap restore** — add `initialize_default_thinking_level` paralleling
   `initialize_startup_model` (`bootstrap.rs`), so the active session reflects the
   persisted default at startup.
3. **Apply-on-activation/resume** — in the resume / `SessionCreated` /
   activation arm (`dispatch_session_chrome.rs`), apply the persisted default to
   the now-active session, **guarded by a per-session "already-applied" set**
   (Rust equivalent of `appliedToSessionRef`) so manual selections survive.

## Non-goals / invariants to preserve
- Do NOT change storage location or encoding (`tui.defaultThinkingLevel`, 0-3,
  invalid/missing -> `Off`).
- Persistence remains best-effort and non-fatal (logged warn on failure).
- Save still triggers on `D` and persists always (reference behavior).
- No per-session persisted level — only the global default is persisted.
- Keep the logging instrumentation already added.

## Key files
- `codelet/fspec-tui/src/app/dispatch_model_thinking_dialogs.rs`
- `codelet/fspec-tui/src/app/dispatch_session_chrome.rs`
- `codelet/fspec-tui/src/app/bootstrap.rs`
- `codelet/sessions/src/default_thinking_level_persistence.rs`
- `codelet/sessions/src/session_manager.rs`
- `codelet/sessions/src/handle_impl.rs`
- Reference: `src/tui/hooks/useDefaultThinkingLevel.ts`,
  `src/tui/config/defaultThinkingLevelConfig.ts`
