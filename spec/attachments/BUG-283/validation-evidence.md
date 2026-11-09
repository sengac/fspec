# BUG-283 final validation evidence

## Result

The eleven specified scenarios pass, the scoped regression suite passes, the package checks and library build pass, strict Clippy passes, and native Fspec syntax/tag/coverage/review checks pass. The production change is confined to Navigator and its small auxiliary-routing helper. The normal native `done` transition and canonical readback are recorded separately in `done-verification.json` once completed.

Baseline HEAD: `626eee1ec96a8e3ec616b5e6411a5c1596815709`.

No commit or branch was created. No unrelated card was advanced. Concurrent BOARD-024 changes and prior work-unit edits were preserved. No native model/session changes were made.

## ACDD sequence

| Phase | Observed evidence |
| --- | --- |
| Specifying | Five rules, seven examples, user story, architecture notes and eleven scenarios; native syntax/tags valid before entering testing |
| Testing | Real App/MockBackend fixtures, exact per-test `@step` comments, native 1:1 mappings; production unchanged |
| RED | `red.log`: eleven compiled tests, ten genuine navigation failures and one passing context-preservation test; exit 101 |
| Implementing | Native transition only after RED; shared transient origin added to Navigator, with no backend/model/pane-layout rewrite |
| GREEN | `green.log`: eleven passed; final acceptance rerun after strengthening model commitment to Right/Down/Enter also passed, exit 0 |
| Regression reconciliation | Initial legacy provider/blocklist close fixtures failed because they never entered Agent; explicit Agent-entry preconditions added, original assertions retained |
| Validating | All commands below completed successfully; native review PASS, no critical issues or warnings, temporal ordering verified |

The RED source hashes are preserved separately from final source hashes. The later keyboard-commit strengthening changes only test fixtures; it does not replace the earlier observed behavioral RED with a compiler/setup failure.

## Executed Rust verification

All commands run from `rust/` through the archived isolated `run` launcher. Every Cargo command is offline, uses `--profile ci-test`, debug0 and `-j2`, with the fresh task cache `/tmp/fspec-bug283-target`. Test processes start in a synthetic project and all configured home/config/data/session roots are synthetic. No real provider credentials/history are used. No unscoped workspace tests are run.

| Check | Result |
| --- | --- |
| BUG-283 final acceptance target | 11 passed, 0 failed; `acceptance-final.log`, exit 0 |
| Thirteen scoped integration/regression targets | 108 passed, 0 failed; `regressions-final.log`, exit 0 |
| `cargo check --offline --profile ci-test -j2 -p codelet-fspec-tui` | PASS, exit 0 |
| `cargo build --offline --profile ci-test -j2 -p codelet-fspec-tui --lib` | PASS, exit 0 |
| `cargo clippy --offline --profile ci-test -j2 -p codelet-fspec-tui --lib --test bug283_auxiliary_view_return_origin --test rpc337_navigator_model_selector --test provider_settings_dispatch_rpc054 --test blocklist_view_rpc056 -- -D warnings` | PASS, exit 0; no lint suppressions added |
| Changed Rust files, `rustfmt --check`, `skip_children=true` | PASS, exit 0; no formatting of concurrent modules |
| Exact scenario/test/step/mapping integrity | Eleven scenarios map to eleven distinct tests, exact comments inside each mapped test, valid implementation lines; exit 0 |
| `git diff --check` | PASS |

The scoped regression targets and counts are:

- `blocklist_view_rpc056`: 14
- `bug175_mux_enabled_leak`: 4
- `bug183_mux_pane_esc_close`: 10
- `bug283_auxiliary_view_return_origin`: 11
- `menu008_menu_bar_reorg`: 17
- `model006_no_session_creates_session`: 3
- `prov118_no_session_default_model`: 2
- `provider_settings_dispatch_rpc054`: 12
- `provider_settings_tab_switch_to_models_rpc160`: 9
- `rpc337_navigator_model_selector`: 4
- `source_shape_rpc013`: 7
- `source_shape_rpc054`: 12
- `source_shape_stores_rpc012`: 3

No crate-wide runtime test suite or instrumented line-coverage percentage is claimed. The 100% figure is the verified acceptance-scenario and native traceability coverage: **11/11**.

## Native Fspec verification

- Canonical `validate` for `spec/features/auxiliary-view-return-origin.feature`: valid, exit 0.
- Canonical `validate-tags` for that file, including its final `@done`: one valid, zero invalid.
- Canonical `show-coverage`: eleven fully covered scenarios, 100%, one test file and two production implementation files; JSON report in the raw archive.
- Canonical `audit-coverage`: `All files found (22/22)` and `All mappings valid`, exit 0.
- Canonical `review BUG-283`: PASS; no critical issues, no warnings; five rules, seven examples, 100% coverage, temporal ordering verified.
- Native `check` has no file filter, so it ran against a scoped staging project containing a byte-identical copy of this feature and the canonical tag/work-unit registry. It checked **one** file with Gherkin/tag/format PASS. The final canonical and staged feature SHA-256 both equal `5fe16c6e34b87cbcb12d7c32c482531c6c63530e09c221547ffbcb12fab9c89c`.
- An earlier exploratory check used a symlink and reported zero files because the check's glob skips symlinks. That empty check is **not** counted as validation; the subsequent byte-identical regular-file staging check above is the qualifying result.

The native lifecycle reported no globally configured test/quality commands. Global configuration was not modified: the explicit scoped commands and their successful raw receipts supply the real verification, and all native lifecycle transitions use normal gates without skip flags.

## Focused code review

The shared helper is wired through `Navigator::apply_action`, the existing App dispatch connection. The origin is captured only from a live non-auxiliary surface; the finite set of possible origins is Board, Agent or Mux. Sibling/repeated opens cannot create a self-return loop, matching close consumes the origin, inactive close cannot navigate, and explicit non-auxiliary navigation clears stale state. Mux pane-local closes retain the existing focused-pane guard and do not consume a full-screen excursion origin. All mutations remain synchronous on the App task; no new shared state or asynchronous writer is introduced.

The Agent preservation test checks session identity, assignment, live and stored drafts, transcript and scroll state, and backend-write counters. Mux tests check configuration, pane list, focus, window identity and offset. Full-screen caller tests exercise real App actions and Escape. The local-cancellation fixture verifies provider filters, API-key/detail cascade, profile form, OAuth confirmation, delete confirmation and model filtering before final dismissal. Explicit model commitment uses actual keyboard navigation and retains the existing targeted model-write contract. PROV-118/MODEL-006 regressions verify no-session default-and-create semantics remain intact.

No correctness or concurrency issue remains in the new routing implementation. All new Rust files and modified production files are below 300 lines. The two already-oversized legacy provider/blocklist test files each receive only one explicit Agent-entry setup statement; no broad unrelated refactor is performed. Details and historical research are retained in the other attachments.

## Evidence files

- `raw-validation-receipts.tar.gz`: raw test/check/build/Clippy logs and exit codes, exact final command files, initial failure receipts, isolation/verification scripts, source hashes, and owned source/spec snapshots.
- `validation-hashes.sha256`: final owned source/spec and evidence hashes, independently verifiable with `sha256sum -c` from the repository root.
- `done-verification.json`: generated readback of the canonical BUG-283 record and state-list membership after native completion, not a hand-written status claim.

The final canonical status is verified after adding the final proof attachments as well. The session then returns idle with no outstanding implementation or verification work.
