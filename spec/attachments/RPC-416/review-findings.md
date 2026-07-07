# Review: RPC-416 — Inline reconnect status in scrollback (replace-in-place + auto-dismiss)

**Date:** 2026-07-07
**Reviewer:** ACDD compliance reviewer (fspec review skill)
**Status:** PASS (0 critical, 2 warnings, 5 observations)

## 🔴 Critical Issues
None. 25 targeted tests green (8 RPC-416 + 5 slice2 + 5 slice1 + 5 rpc024 + 2 rpc049).
Both feature files validate. Coverage 8/8 with correct ranges. No unwrap/expect/todo!/
unimplemented!/panic!/HACK/FIXME in prod. Modal genuinely never pushed on the production
disconnect/reconnect path. Seq-keyed replace/remove re-anchors in-flight slots + selection.
Superseded-notice guard + timer-abort sound.

## 🟡 Warnings (Should Fix)
1. **Re-drop + originating-session-closed compound gap.** `handle_disconnected`
   (`dispatch_reconnect.rs:35-41`) handles only the "seq still present" branch. If the
   originating session was CLOSED (`session_context_mut_for(&sid)` → None), it silently
   does nothing AND returns early leaving `self.reconnect_notice = Some((sid, seq))`
   pointing at a dead session — so no reconnecting line is shown at all on the re-drop.
   Design edge-case 1 said "reuse same seq if present; otherwise push a fresh line." No
   scenario/test exercises re-drop + originating-session-closed. FIX: fall through to push
   a fresh notice into the currently-focused session (and reset tracking) when the original
   session is gone; add a scenario + test.
2. **`replace_notice_by_seq` silently no-ops when `chunk.source` is None**
   (`reconnect_notice.rs:43-48`). Safe in practice (Notification chunks always have Some
   source) but the guard is silent. Add a `debug_assert!`/comment documenting the invariant.

## 🟢 Observations
1. DISMISS_DELAY 1500ms vs test wait 2300ms — comfortable margin.
2. `handle_clear_reconnect_notice` clears handle without abort — correct (timer already
   fired); a one-line comment would aid readers.
3. DisconnectDialog severed, NOT deleted — CORRECT per design decision. Still referenced in
   production by `events.rs` (Stage-1 key-swallow guard) so NOT dead code; zero build
   warnings. Modal never auto-pushed on Disconnected. Satisfies "modal never appears."
4. `@RPC-416` tag unregistered project-wide (consistent with other RPC ids; pre-existing).
5. RPC-011 feature updates consistent; Given/When/Then/And ordering correct in both files.

## Fix Plan (ACDD)
- Warning 1: add a scenario "Re-drop after the originating session closed still shows a
  reconnecting line in the focused session" → failing test → implement fallback in
  `handle_disconnected`. (specifying → testing → implementing → validating → done)
- Warning 2: add debug_assert!/comment to `replace_notice_by_seq` (implementing).
