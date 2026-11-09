# BUG-283 implementation findings

## Scope authorization and ownership

The user expanded the research-only task to complete ACDD in the existing native session on 2026-10-28. The original research is retained as historical evidence, not overwritten with a later success claim. No agents or sessions were spawned. No model switch, installation, daemon restart, commit, push, session-log read, or unrelated cleanup was performed.

The concurrent BOARD-024 work owns the board Enter chooser and full-card details. BUG-283 does not modify those cards, specifications, fixtures, or implementations. The unrelated AUDIT-002 work is untouched. `spec/work-units.json` remains managed by the native Fspec tool; concurrent edits are not reset.

## Root cause and correction

Navigator originally discarded the calling surface on auxiliary entry. Providers and Blocklist always closed to Agent; Models closed/committed to Agent; full-screen Files and Checkpoints always closed to Board. An earlier Agent visit was not necessary to trigger the bug: it merely determined which retained agent state was rendered at the incorrect destination.

The correction adds a transient `Option<ViewMode>` excursion origin to Navigator, with open/close routing in its focused `navigator_auxiliary.rs` child module. The existing App dispatch tail remains the single connection to Navigator. No backend logic, session assignment, model update, menu mapping, or pane layout code is duplicated or replaced.

- Entering an auxiliary screen from a live non-auxiliary surface captures that surface.
- Repeated opens and Providers/Models sibling switches retain the excursion origin.
- A matching close consumes the origin; missing origin uses the specified safe Board landing.
- Mismatched and duplicate closes do not change the visible surface.
- Explicit navigation to a non-auxiliary surface clears the old origin; a later excursion captures the new live caller.
- Mux Files/Checkpoints close actions still reach the existing focused-pane removal path.
- Neither the persisted mux enabled preference nor `pre_mux_view` is used as auxiliary caller evidence.

## Model and legacy contract reconciliation

RPC-054/RPC-056 agent-origin Escape and RPC-337 agent-origin model close/commit still return to Agent. Their regression fixtures must actually enter Agent before opening the auxiliary view. The original return-to-Agent assertions are retained; they are not changed to Board to obtain a pass.

The first broader regression run found exactly two legacy fixture failures: the provider-close and blocklist-close fixtures seeded a session but remained on Board before invoking an Agent slash action. Those fixtures now explicitly enter the seeded Agent before opening their screens. The two model navigator fixtures have the equivalent explicit Agent precondition. No assertions are suppressed or weakened.

The provider and blocklist integration files were already above 300 lines at baseline. Only one Agent-entry setup statement is added to each; they are not broadly refactored because the user expressly restricted unrelated restructuring and preservation of prior Done evidence. All new Rust files and the modified production Navigator remain below 300 lines.

Explicit model selection is not the same operation as restoring navigation. The existing PROV-118/MODEL-006 no-session selection path persists a default and retries session creation. BUG-283 does not remove that behavior. With an existing session, only that session's explicit model update is allowed. The new commit acceptance test drives Right/Down/Enter through the real App model selector with synthetic provider data. Closing screens without commitment must not create/resume a session, assign work, or write a model.

## ACDD evidence

1. Native Example Mapping recorded five rules, seven examples, and the user story while `specifying`.
2. Native feature generation and scenario/step commands created eleven explicit scenarios. Scoped syntax and tag checks passed before native transition to `testing`.
3. A real App/MockBackend acceptance target and small fixtures were written and linked 1:1 to the scenarios with exact `@step` comments before production edits.
4. The original production code compiled and ran: **1 passed, 10 failed**, exit 101. Every failure was an actual navigation assertion, including Agent versus Board/Mux and an inactive close pulling Providers to Agent. Setup/compiler errors are not counted as RED.
5. Only after that observed RED and native transition to `implementing` was the production origin handling added.
6. Initial GREEN ran all eleven scenarios successfully. Final verification receipts record the subsequent keyboard-commit strengthening, scoped regressions, check/build, formatting, strict Clippy, and native review results separately.

`red.log` is attached independently. The raw evidence archive contains the exact invocations, exit codes, source hashes, and final verification outputs. Initial regression failures remain in the archive instead of being hidden.

## Isolation and scoped generation

All Cargo invocations use `/tmp/fspec-bug283/run`, an `env -i` launcher with synthetic HOME, FSPEC_HOME, FSPEC_USER_DIR, CODEX_HOME, XDG config/data/cache/state/runtime/search roots and TMPDIR. Cargo's target cache is newly built at `/tmp/fspec-bug283-target`; no unverified prebuilt binary is reused. The synthetic Cargo home exposes only the existing registry/git source caches, not the real Cargo configuration or credentials. Offline mode, ci-test debug0, and `-j2` are explicit. A Cargo target runner starts each test process in a synthetic project directory; individual BUG-283 tests use shared temp-directory helpers and serial execution for the process-global data-directory setting.

`generate-scenarios` creates no coverage sidecar, and native `generate-coverage` has no file-scoping argument. To avoid rewriting any other card's coverage, generation ran natively against a staging project containing only this feature. Its generated sidecar was then moved unchanged to the canonical feature path; all test and implementation mappings were populated by native `link-coverage`. No broad coverage regeneration or hand-authored coverage statistics were used.
