# RPC-097 Re-review: First Shift+Right from BoardView Still Broken

**Date:** 2026-05-31
**Reviewer:** Claude Code (@spec/skills/review-skill.md)
**Card under review:** RPC-097 (originally marked `done`)
**User report:** "doesn't create a new session properly the first time you go shift+right from the board"

---

## Status: 🔴 FAIL — Regression escape from original RPC-097 fix

The original RPC-097 fix correctly patched the **AgentView**-side dispatch path
(`dispatch_rpc024.rs::handle_session_cycle` NavTarget::CreateDialog branch) to
delegate to the proven RPC-060 helper `handle_open_create_session_dialog(None)`.

**However**, a SECOND code path exists for "first Shift+Right press while still
on BoardView" that was not touched by the original fix:

```
BoardView Shift+Right
   └─> views/board.rs:117-122  emits Action::OpenAgentView(target)
        └─> app/dispatch.rs:99-110  Action::OpenAgentView(None) arm
             ├── Some(sid)  → set_navigation_target(Some(sid)) ✅ correct
             └── None       → request_create_session_dialog()  ❌ DEAD FLAG
                              (no render-pipeline subscriber consumes this)
        └─> navigator.active_view = ViewMode::Agent
```

`AgentViewStore::request_create_session_dialog()` (store/agent_view.rs:234-237)
only flips `show_create_session_dialog = true` and
`should_auto_create_session = true`. A grep across the entire `fspec-tui`
crate confirms **no render-pipeline subscriber reads these flags** — they are
orphan state. The same root cause this card already documented for
NavTarget::CreateDialog reappears here in the BoardView arm.

---

## Trace Comparison: Buggy First Press vs Working Second Press

### First Shift+Right (from BoardView, work unit has no attached session)

1. `views/board.rs:117` matches Shift+Right
2. `selected_session(store)` returns `None`
3. Emits `Action::OpenAgentView(None)`
4. `app/dispatch.rs:105-107` matches `None` arm
5. Calls `agent_view_store.request_create_session_dialog()` (DEAD FLAG)
6. Sets `navigator.active_view = ViewMode::Agent`
7. **No `CreateSessionDialog` is ever pushed onto the Compositor**
8. User sees an empty AgentView. The dialog never appeared.

### Second Shift+Right (now in AgentView, no open sessions yet)

1. `navigator::handle_event` routes to `views/agent/dispatch.rs` because `active_view == Agent`
2. `shift_arrow_to_action(KeyCode::Right)` → `Some(Action::SessionNext)`
3. `app/dispatch.rs::Action::SessionNext` → `handle_session_cycle(1)`
4. `agent_view_store.navigate_next()` on empty list returns `NavTarget::CreateDialog`
5. `dispatch_rpc024.rs:55` calls `handle_open_create_session_dialog(None)`
6. ✅ Dialog is pushed via RPC-060 helper

This is why the user sees a 2-press requirement instead of TS Ink's 1-press behavior.

---

## TS Ink Behavior (Canonical Source)

`src/tui/components/BoardView.tsx:347-356` + `src/tui/hooks/useSessionNavigation.ts:48-62`:

- BoardView's `handleShiftRight` calls `navigateRight()` (NAPI session-list lookup)
- If sessions exist → jump to first session (`onNavigate(sessionId)`)
- If sessions DON'T exist → `openCreateSessionDialog()` flips Zustand
  `showCreateSessionDialog = true`
- **BoardView itself observes the flag and renders `<CreateSessionDialog />`
  inline** (BoardView.tsx:619-637)

The Rust port emulates this by routing through `Action::OpenAgentView` and
expecting the Compositor to host the dialog. The defect is purely that the
`None` arm forgot to actually push the dialog.

**TS-vs-Rust semantic note:** TS BoardView selects "first session" from a
global session list (regardless of which work unit is focused). Rust BoardView
selects the focused work unit's *attached* session via `session_for(&unit.id)`.
This is by design (Rust ties sessions to work units more tightly) and is NOT
part of the bug — only the `None` arm is broken.

---

## Fix (Minimal, Surgical, 4-Line Change)

`codelet/fspec-tui/src/app/dispatch.rs:99-110` becomes:

```rust
Action::OpenAgentView(target) => {
    match target {
        Some(sid) => {
            self.agent_view_store
                .set_navigation_target(Some(sid.clone()));
            self.navigator.active_view = ViewMode::Agent;
        }
        None => {
            // RPC-097 (BoardView first-press fix): mirror the proven
            // RPC-060 mount path from dispatch_rpc024.rs so the dialog
            // ACTUALLY appears on the first Shift+Right from BoardView.
            // The earlier request_create_session_dialog() store-flag
            // setter is orphan state — no render-pipeline subscriber
            // consumes it.
            self.agent_view_store
                .request_create_session_dialog_no_auto();
            self.navigator.active_view = ViewMode::Agent;
            self.handle_open_create_session_dialog(None);
        }
    }
}
```

### Why `request_create_session_dialog_no_auto` (not the `_no_auto` variant)?

Looking at the AgentView path in `dispatch_rpc024.rs:55`, the proven helper is
`request_create_session_dialog_no_auto()` — it sets only
`show_create_session_dialog = true` and explicitly does NOT set
`should_auto_create_session`. The user is being given the explicit Yes / Yes
- Isolated / Cancel choice via the dialog; we don't want to also kick off an
auto-create. This matches RPC-097's existing rule [2].

### Why active_view BEFORE handle_open_create_session_dialog?

`handle_open_create_session_dialog` itself doesn't require `active_view ==
Agent` (Compositor is global), but ordering it after the view switch keeps
the state machine internally consistent — if any future render code checks
both flags, they're set in dependency order.

---

## Test Plan

Add 3 new scenarios + 1 regression-guard scenario to
`spec/features/agentview-shift-right-create-session-dialog.feature`:

1. **BoardView first Shift+Right with unattached work unit mounts dialog**
   — Setup: BoardView selected, focused unit has no session.
   — Act: feed `KeyEvent { code: Right, modifiers: SHIFT }` to
     `App::handle_event` once.
   — Assert: `compositor.contains(CREATE_SESSION_DIALOG_ID) == true`,
     `navigator.active_view == ViewMode::Agent`.

2. **BoardView first Shift+Right with attached session does NOT mount dialog**
   — Setup: BoardView selected, focused unit has session `sid-1` attached.
   — Act: one Shift+Right.
   — Assert: `compositor.contains(CREATE_SESSION_DIALOG_ID) == false`,
     `agent_view_store.navigation_target == Some(sid-1)`,
     `active_view == ViewMode::Agent`.

3. **Two Shift+Rights from BoardView are idempotent**
   — Setup: BoardView, no attached session.
   — Act: Shift+Right, Shift+Right (second goes through AgentView path now).
   — Assert: exactly ONE CreateSessionDialog in compositor (the second press's
     `handle_session_cycle` calls `handle_open_create_session_dialog` again,
     which is idempotent on CREATE_SESSION_DIALOG_ID — line 42-44 guard).

4. **Regression-guard: orphan store flag is not relied on**
   — Setup: same as scenario 1, but assert that `handle_open_create_session_dialog`
     was the code path that mounted (i.e. compositor has the dialog with its
     `with_action_tx` configured so `CreateSessionSubmitted` round-trips).

---

## Files To Modify

| File | Change | LoC impact |
|---|---|---|
| `codelet/fspec-tui/src/app/dispatch.rs` | 4-line surgical patch to `OpenAgentView(None)` arm | +3 |
| `spec/features/agentview-shift-right-create-session-dialog.feature` | +4 scenarios, +1 rule reference | +~50 lines |
| `codelet/fspec-tui/tests/shift_right_create_session_dialog_rpc097.rs` | +4 test functions | +~120 lines |

---

## Risk Assessment

- **Blast radius:** 1 arm of 1 match expression in `app/dispatch.rs`.
- **Backward compatibility:** `request_create_session_dialog()` is still set
  (just renamed to `_no_auto` variant) — any future code that observes
  `show_create_session_dialog` continues to read the same flag.
- **Test coverage:** 17 existing RPC-097 tests still pass; 4 new tests guard
  the new entry point.
- **No new actions, no new public API.**
