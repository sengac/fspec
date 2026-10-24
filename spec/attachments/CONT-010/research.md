# CONT-010: Define auto-continue defaults across session lifecycle changes

## Scope and confidence

Discovery/documentation task, researched on 2026-10-21 at `7e60de8e`. The behavior below is source-confirmed. Model-switch reseeding and runtime-setting persistence are not promised by the current creation-time contract; do not label those choices fixed bugs or silently implement a new precedence policy.

## Existing contract

`spec/features/profile-auto-continue-session-seeding.feature:9` promises a seed before the first user dispatch for a newly created profile session. `autoContinue >= 1` enables its budget; zero/absent disables it with the normal fallback budget. Runtime `/continue` is an override, not a profile lock.

`rust/sessions/src/session_creation_helper.rs:264` implements this seed through shared initialization. PROV-142's research explicitly excludes persistence of runtime overrides (`spec/attachments/PROV-142/PROV-142-research.md:427`). Auto-continue is profile-scoped, not a universal default for every model of an underlying provider.

## Lifecycle findings

| Operation | Current behavior | Source |
| --- | --- | --- |
| New ordinary profile session | Apply profile seed | `rust/sessions/src/session_manager.rs:810` |
| Cold resume from disk | Reconstruct using manifest model and current profile default | `rust/sessions/src/handle_impl.rs:203`; `rust/sessions/src/session_manager.rs:1011` |
| Resume already-live session | Preserve in-memory settings | `rust/sessions/src/handle_impl.rs:194` |
| Restore transcript/token state | Does not restore runtime continue override/counter | `rust/sessions/src/session_restore.rs:26` |
| Isolated/worktree session | Use shared initialization and profile seed | `rust/sessions/src/session_manager.rs:1174` |
| Fork, later cold resume | Manifest carries model identity, not runtime continue settings; reconstruction seeds again | `rust/core/src/persistence/manifest.rs:408` |
| Subordinate or scheduled session | Ordinary shared creation applies inherited/selected model seed | `rust/napi/src/agent_manager_handler.rs:138`; `rust/sessions/src/session_manager.rs:1289` |
| Switch an existing session's model/profile | Keep existing continue enabled/budget unchanged | `rust/sessions/src/handle_impl.rs:1218` |
| Select model with no active session | Create a session, so initialization applies | `rust/fspec-tui/src/app/dispatch_model_thinking_dialogs.rs:138` |

Example: create a cloud/Codex session with continuation off, then switch to Strada with `autoContinue: 300`. The session can remain genuinely off, not merely display the wrong indicator. That is consistent with a creation-only seed.

## Inner engine versus chrome state

`rust/sessions/src/background_session.rs:1424` updates background atomics. `sync_completion_contract_for_user_turn` at line 1525 copies them into the inner engine on the next dispatched message, resets nudges and arms the completion contract before agent construction. A change during an active stream is not a live update to that stream.

`rust/agent-loop/src/agent_loop.rs:576` calls this shared synchronization. NAPI now uses the same engine (`rust/napi/src/agent_loop.rs:6`); the old claim that its independent loop never arms completion is not true at this HEAD. BUG-228 tracks the obsolete parity assertion.

## Questions to resolve before changing behavior

1. Should a model/profile switch always reapply the destination default, never do so, or do so only if there is no explicit runtime override?
2. If overrides survive a switch, does an explicit budget remain meaningful across providers with different defaults?
3. Should cold resume and fork preserve runtime settings, use the latest profile default, or offer a selectable policy?
4. Should an edit to a profile affect existing live sessions at all?
5. Should `/continue` changes take effect immediately in an active stream, or be visibly labelled as next-turn settings?
6. What wins when switching, attaching, sending the first input and asynchronously loading chrome state race?

## Follow-up specification and regressions

Document the chosen precedence and lifecycle behavior before implementation. Add scenario-backed coverage for cloud-to-profile and profile-to-profile switches, explicit on/off overrides, changed budgets, live/cold resume, forks, isolated sessions and mid-stream changes. Reuse real session/temp-directory helpers.

`rust/sessions/tests/prov142_session_seed.rs` covers basic backend seeding; `rust/sessions/tests/wt012_isolated_session_shared_creation.rs:322` covers isolated profile creation. They do not decide the open switch policy.

BUG-225 is a separate confirmed defect: the TUI currently fails to hydrate even correctly seeded state. Fixing that cache must not accidentally invent a new lifecycle policy.
