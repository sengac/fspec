# BUG-226 — Failed model selection leaves optimistic identity caches

## Summary and confidence
- Audit baseline: HEAD `7e60de8e`, 2026-10-21; source-only audit.
- **Source-confirmed conditional defect; high confidence:** current TUI selection caches the requested model before backend acceptance and does not reconcile that cache on failure. Header refresh updates a different cache.
- Legacy NAPI setters have a distinct ordering defect: they change display labels before fallible provider selection and do not restore labels on error. Do not conflate those setters with the current tarpc resolver.
- No reproduction, tests, builds, or live calls were executed; these findings do not establish any incident's actual runtime model.

## Source chain — current TUI and tarpc
1. `rust/fspec-tui/src/app/dispatch_model_thinking_dialogs.rs:76` stores the requested model id using `set_selected_model_id` before starting the backend call at `:98`.
2. `rust/fspec-tui/src/app/dispatch_model_thinking_dialogs.rs:106` only logs backend failure. `:114` fetches model info after either outcome, but sends ModelInfoLoaded rather than an accepted-selection update.
3. `rust/fspec-tui/src/app/dispatch.rs:184` handles ModelInfoLoaded by updating only `model_info`; it never corrects `selected_model_id`.
4. `rust/fspec-tui/src/app/dispatch_model_selector.rs:21` reopens the selector using the separate last-selected id at `:25`, so its current marker/cursor can disagree with the backend and refreshed header.
5. `rust/fspec-tui/src/store/agent_view/chrome_state.rs:40` and `:47` confirm these are separate maps.
6. `rust/rpc/src/lib.rs:1531` delegates current model switching to `SessionManagerHandle::set_model` at `:1542`, not to the NAPI setters below.
7. `rust/sessions/src/handle_impl.rs:1248` rejects a busy session; `:1250` validates selection; `:1258` resolves a replacement provider manager; `:1262` persists identity before `:1264` replaces the live manager. Only afterward does `:1283` publish display labels and `:1284` cache limits.

## Source chain — legacy direct NAPI setters
1. `rust/napi/src/session_bindings.rs:1951` defines `session_set_model`; `:1967` updates labels before registry/direct selection at `:2015`. The error branch at `:2085` returns an error without restoring those labels.
2. `rust/napi/src/session_bindings.rs:2114` defines `session_set_model_profile`; `:2132` similarly updates labels before `set_model_direct_with_profile` at `:2177`. Its error branch at `:2275` also returns without restoring labels.
3. Both legacy paths also set compaction overrides before fallible model selection (`rust/napi/src/session_bindings.rs:2002`, `:2159`), so transactional failure coverage should include associated caches/overrides, not labels alone.
4. The profile setter deliberately bypasses registry validation; its exact failure conditions differ from `session_set_model`. A failure reproduction must use an actually rejected direct selection, not assume registry rejection on that surface.

## Impact and boundaries
- On current TUI failure, the selector can falsely mark the rejected model as current while the backend correctly retains its previous selection. A refreshed header need not be stale: it comes from the other map.
- Busy-session rejection is a concrete current-backend failure condition; invalid selection and persistence failure are also covered by backend contracts.
- Legacy label mutation can misreport backend identity after a failed setter. That does not prove that the current tarpc path switched to a bad provider or shared the legacy ordering bug.
- No claim is made that a UI label proves which endpoint or model handled an incident request.

## Reproduction outline — proposed, not executed
1. Initialize current TUI and backend with accepted model A; attempt B with a deterministic backend error, such as busy-session rejection.
2. Await the setter and model-info refresh; reopen the selector. Compare requested-id marker B against authoritative model A and the header's ModelInfo.
3. Repeat with success and two rapid, differently resolved requests to detect stale async results.
4. Separately invoke each legacy setter with an actually failing selection and compare labels, provider-manager selection, limits, and overrides before/after failure.

## Existing specifications/tests and missing regressions
- `spec/features/rpc022-app-dispatch.feature:48` and `rust/fspec-tui/tests/app_dispatch_rpc022.rs:53` cover setter dispatch and chrome refresh, not rejected selection reconciliation.
- `spec/features/model-selector-open-on-current-model.feature:36` requires current-model cursor placement; `rust/fspec-tui/src/views/model_selector/tests_current_model.rs:1` covers the view given a current id, not backend acceptance.
- `spec/features/model-switch-durable-identity.feature:26` requires rejection to preserve identity; `:31` covers busy sessions, and `:53` durable-write failure.
- `rust/sessions/tests/model_switch_durable_identity.rs:202`, `:251`, and `:270` cover these backend conditions, not the optimistic TUI map or legacy setter label ordering.
- Missing: failed-switch selector/header/backend agreement, visible originating-session failure notice, out-of-order completion protection, and legacy failure atomicity across labels/limits/overrides.

## Recommended direction
- Publish current selection from accepted backend identity, or keep pending selection separate and reconcile on both success and failure; route visible failure feedback to the originating session.
- Unify selector identity and model-info hydration without treating requested ids as authoritative. Protect against late results from an older request.
- For legacy NAPI paths, stage validation/resolution before publishing labels and dependent caches, preferably through shared transactional selection behavior while preserving legacy API semantics.

## Uncertainties and related cards
- Legacy runtime reachability depends on the caller/frontend. Current Rust TUI transport routing is source-confirmed; this audit does not claim legacy direct setters are its implementation.
- RPC-022/RPC-337/RPC-341 cover selection/chrome; RPC-343 and BUG-137 concern resolver/profile identity. BUG-225 is the related optimistic continue-cache gap.
- CONT-010 separately owns profile-switch lifecycle semantics; changing model must not silently introduce continue reseeding. BUG-221/BUG-222 are exhaustion/retry issues, not evidence that a rejected switch changed the live provider.
