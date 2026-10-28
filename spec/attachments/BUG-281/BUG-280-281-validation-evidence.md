# BUG-280 / BUG-281 final source-validation evidence

Date: 2026-10-28. Status: BLOCKED at validation, not DONE.
This shared record is attached independently to both cards.

## Source changes and card ownership

- BUG-280: Board Enter resolves the earliest open session with the selected
  story's authoritative focus; otherwise it offers explicit story creation.
  No unrelated agent, including a resumed agent, is reassigned. Disk-only
  sessions are not implicitly resumed. Mux retains its grid.
- BUG-281: New Agent, right-edge Shift Right and generic initialization never
  infer a story from ambient selection. Dialog title and submission share a
  captured optional intent. Normal and isolated creation follow the same policy.
- Shared completion targets the returned session ID explicitly, handles
  observation-before-acknowledgement, and ignores duplicate completion after a
  newer focus clear. Cancellation consumes intent and clears the mux request.
- Existing authoritative focus/revision, authorization, resume and history
  behavior remains exercised by regression tests. No production session backend
  or persistence implementation was changed.

The installed daemon is NOT fixed by these source edits. No install, deployment,
restart, commit, push, model change, real credential edit or real history edit was
performed. HEAD remains 042d3974. Original research files remain unchanged, both
SHA-256 5e43e550f4e63cbb577f595a3693cfdeea81a491d4aba4623a8d59cefed652ce.

## ACDD and causal RED

Rules/examples -> acceptance specifications -> executable tests -> production
implementation -> validation were followed. Native gates and their corrections
are recorded in the attached archive's native-validation.md. Ten scenarios map
one-to-one to ten executable acceptance tests across three focused features:
session-creation-intent (3), session-creation-completion (4), generic-agent-creation (3).

Before any production edits, the compiled acceptance suite ran on the original
production source and failed all ten tests behaviorally (red.log):
- navigation chose unrelated instead of earliest associated session;
- unassociated/resumed entry failed to offer the dialog;
- New Agent inherited AUTH-001 instead of None;
- creation races produced extra assignment calls;
- duplicate completion produced another assignment;
- generic initialization attached ambient selection;
- cancellation/failure and Shift Right violated their creation contract.

The first build attempt had a fixture field-name compiler error; that was fixed
and is NOT counted as RED. Subsequent fixture fixes corrected error-modal dismissal
and a counter assumption, without relaxing any behavioral acceptance assertion.
A further causal cancellation RED (cancellation-red.log) showed pending_new_agent
remaining true after Esc, before its production fix.

Final acceptance output (regressions-final.log):
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out.

## Final executed validation

All Cargo commands ran from rust/ with --offline --locked --profile ci-test,
-j 2, CARGO_PROFILE_CI_TEST_DEBUG=0, CARGO_PROFILE_DEV_DEBUG=0 and
CARGO_PROFILE_TEST_DEBUG=0. Full exact final argv, environment roots and stage
output are preserved in the attached validation-records archive. The initial
RED predates the reusable runner; its resolved temporary HOME name was not
logged, but its tool command and per-test guard used the same isolation policy.

| Stage | Result |
| --- | --- |
| Ten acceptance scenarios | PASS, 10/10 |
| TUI acceptance plus fourteen regression targets | 160 passed, 1 failed; exit 101 |
| Session focus/status/resume regressions | PASS, 12/12; exit 0 |
| cargo check, codelet-fspec-tui and codelet-fspec | PASS; exit 0 |
| cargo build, codelet-fspec-tui --lib | PASS; exit 0 |
| Standalone codelet-fspec --bin fspec build | DENIED before execution |
| Clippy affected library and eight affected test targets, -D warnings | PASS; exit 0 |
| rustfmt --check on every changed/new Rust file | PASS; exit 0 |
| cargo fmt -p codelet-fspec-tui --check | FAIL on unchanged bug207 formatting; exit 1 |
| git diff --check | PASS |
| Nine scoped Gherkin syntax validations | PASS |
| New acceptance feature tags | PASS, 3/3 |
| Legacy rust-mux-mode tags | FAIL: five pre-existing scenario work-unit tags |
| New acceptance coverage/audits | PASS, 10/10 with test and implementation links |
| Full workspace tests | NOT RUN, prohibited/unnecessary for this scoped change |
| Installed-daemon/native interactive behavior | NOT RUN; no restart/deployment |

Exact final test commands:

```text
cargo test --offline --locked --profile ci-test -p codelet-fspec-tui --test bug280281_session_intent --test bug230_focus_projection --test bug230_focus_recovery --test bug230_focus_transports --test board_agent_navigation_rpc012 --test work_unit_binding_rpc050 --test isolated_session_dialog_rpc060 --test shift_arrow_end_of_list_rpc096 --test shift_right_create_session_dialog_rpc097 --test prov101_session_creation_decline --test bug175_mux_enabled_leak --test mux001 --test mux002 --test bug164_mux_retained_on_session_exit --test bug174_mux_empty_agent_grid_transient_board_pane -j 2 --no-fail-fast -- --test-threads=1
cargo test --offline --locked --profile ci-test -p codelet-sessions --test sess003_work_unit_focus --test bug231_status_focus -j 2 --no-fail-fast -- --test-threads=1
cargo check --offline --locked --profile ci-test -p codelet-fspec-tui -p codelet-fspec -j 2
cargo build --offline --locked --profile ci-test -p codelet-fspec-tui --lib -j 2
cargo clippy --offline --locked --profile ci-test -p codelet-fspec-tui --lib --test bug280281_session_intent --test bug230_focus_projection --test bug230_focus_recovery --test bug230_focus_transports --test board_agent_navigation_rpc012 --test bug175_mux_enabled_leak --test mux001 --test shift_right_create_session_dialog_rpc097 -j 2 -- -D warnings
```

## Isolation and provenance

Each Cargo stage received a new synthetic HOME, FSPEC_USER_DIR, FSPEC_HOME,
CODEX_HOME, XDG_CONFIG_HOME and XDG_DATA_HOME. New tests additionally use the
shared temp-directory helper, serial environment guards, and a synthetic global
data/session root. Existing real-store tests redirect their own data/project
roots. No test was run against the actual user configuration or session root.

CARGO_HOME and RUSTUP_HOME identify the existing toolchain caches only. There
were no source copies. A fresh dedicated CARGO_TARGET_DIR,
/tmp/fspec-bug280281-target, was used for this source tree, with normal Cargo
fingerprinting across the documented RED-to-GREEN edits. No pre-fix copied tree
shared this cache. All final stages' full Rust source-manifest hashes match:

```text
final Rust manifest: b124450408e1e50c89b0ddc99dbb660b71ddef08088c7cd8b5128faf8de4f9a1
RED Rust manifest:   5dbb5a8edd9697a6c668c967e2f4e4dfc82b06e2efbdc9c4d78c8145a67cea88
final specs manifest:5286aa3e47b0601f323b4b8829a894d2348c6e3ccbecaa087ca9393b6de5900d
```

regressions-final-source.sha256, cargo-check-source.sha256, clippy-source.sha256,
library-build-source.sha256 and focus-resume-regressions-source.sha256 all have
the final Rust manifest hash. sha256sum --check --status against final source
passed. The archive contains every per-file hash and the exact test output.

Principal production file hashes, relative to rust/fspec-tui/src/:

```text
2e8c3c5303dd02f9748128492c2b9974e232e58643f1b63d473733e58fdba72d app/dispatch_capability.rs
0d98432d772ca96eda3c68eb599b3f3c905af5e9909d10b5d0dc98c56a53ee1a app/dispatch_create_session_dialog.rs
6562db2938728b822f36bbab1aa1353d955114e1142146b978ed5d963c533868 app/dispatch_focus.rs
febaca6a69201d8d8ce4b0aad63239e70fc902ee78b28771eff1558f50c54d89 app/dispatch_mux.rs
1cbb8638bec8a6cde7020b3ca4388168d5099774b0d7c02564fd5bf5b2874070 app/dispatch_work_unit_binding.rs
10e18d7de9f046f454fbf36547152df74871f4f9371d9b99747ab9897981d8d0 app/dispatch_work_unit_entry.rs
8662d5167d74abd6317ef4a3741a491bebc6224785cb5b4209e4d1ac807a1776 app/state/state_types.rs
6e4b1593b249e9a07fe5759d49b0f7ed5ecbd7d4371e09363769723fe8404c2a views/multiplex/mod.rs
```

## Exact remaining blockers and limitations

1. Standalone binary build command was DENIED by the tool-only policy, before
   execution. Exact call/target/reason and not-pending state are recorded in
   native-validation.md. It was not retried or re-expressed.
2. chunks_subscriber_filter_follows_current_session_via_watch_channel fails
   expecting ChunkReceived for text. Unchanged bootstrap.rs routes text through
   Transcript(Live). The unchanged test function hash at HEAD and final source
   is bc198647b7c7bfa6fba7a6acc09ad0d0b87caf5b1b6d936a4c0e6a3d71dc7b5f;
   bootstrap.rs is also identical to HEAD. No assertion was weakened or skipped.
3. Package fmt check flags only the unchanged
   model_selector_refresh_forces_catalog_refetch_bug207.rs. Its HEAD/final hash
   is 7502614b61e89aafc0f230b72b51bcc4b9da6db219c5881a830e9baacdb2f2c1.
   Incidental formatter changes to that unrelated file were restored, not kept.
4. rust-mux-mode retains five pre-existing @BUG-164/@BUG-165 scenario tags.
   Those unrelated card/tag cleanups were not performed.
5. The separately denied legacy architecture-note patch and temporary issue
   note were not retried; original research remains attached and unchanged.

No synthetic all-green result or completion is claimed. Both cards reached
validating through supported gates, then were marked blocked with these reasons.
Unblocking requires resolving the denied binary validation through an authorized
native mechanism and an explicit decision on the out-of-scope baseline failures.
There is no unanswered approval request or pending retry owned by this session.

## Initial partial-patch outcome and follow-up verification

The ORIGINAL denied call was functions.apply_patch, work_unit_id BUG-280,
with these ordered operations and absolute targets:

| Operation | Target | Read-only verified initial outcome |
| --- | --- | --- |
| Add | /home/rquast/projects/fspec/spec/features/session-creation-intent.feature | APPLIED before denial; initially ten scenarios |
| Update | /home/rquast/projects/fspec/spec/features/session-work-unit-focus-projection.feature | APPLIED before denial; board-entry scenario/step changed to explicit assignment |
| Update | /home/rquast/projects/fspec/spec/features/enter-key-on-board-bypasses-create-session-dialog-no-isolated-option.feature | NOT APPLIED; unchanged from HEAD |
| Add | /tmp/bug280-281-issue.md | NOT APPLIED; absent |

Exact tool response:

```text
Blocked: RLCD security: P(risk)=0.35367945 >= blockThreshold 0.35 — the engine judged this operation risky
```

The response did not identify the individually triggering target and did not
roll back the first two operations. Thus the initial statement that nothing
applied was incorrect; read-only inspection corrected it before further work.
The first two applied targets subsequently received the separately authorized
independent specification/test/implementation work. The remaining two targets
are DENIED, not pending, with the precise triggering target UNCERTAIN. The renewed
two-target call and its separate denial are recorded in native-validation.md.
Neither remaining operation has been retried or re-expressed since that denial;
the latest user instruction explicitly retains it. This audit section records
the denial only; it does not supply the denied architecture update or issue note.

A follow-up isolated execution of the same acceptance target passed 10/10,
exit 0, without any further production or test changes. The command was:

```text
bash /tmp/fspec-bug280281-run.sh follow-up-acceptance cargo test --offline --locked --profile ci-test -p codelet-fspec-tui --test bug280281_session_intent -j 2 -- --test-threads=1
```

The existing archive remains an immutable record of the preceding validation.
The additional follow-up-acceptance.log is attached separately. The Rust source
manifest is unchanged. Both cards remain blocked; no done transition is claimed.

## Verification of the reported intermediate feature corruption

The reported missing first scenario and merged New Agent header describe a real
earlier intermediate edit, not the current files. That intermediate version was
repaired before the causal behavioral RED run. Later, the native implementing
gate required one feature per test file, so the unchanged ten acceptance blocks
were split into board entry (3), completion (4) and generic creation (3).

On the controller's subsequent verification request, the actual current files
were read first. The board-navigation scenario was present; New Agent had its
own Scenario and Given in generic-agent-creation.feature; no scenario-level tags
remained in any of these three files. A single allowed native patch added only
cross-references to session-creation-intent.feature's architecture doc string.
No assertion, scenario, test, implementation or denied target was changed.

The patch outcome was checked from actual file contents, not its acknowledgement:
a read-only integrity check compared all ten scenario names and every original
Given/When/Then/And step exactly, checked absence of scenario tags, and verified
one test mapping with implementation coverage and matching @step comments for
each scenario. Native format did not change the checked file hashes. Native
validate and validate-tags then passed independently for all three files.
Only AFTER those specification checks was the isolated acceptance target run:
10 passed, 0 failed, exit 0 (feature-integrity-acceptance.log).
The integrity/setup check is NOT counted as behavioral RED.

Current specification file hashes:

```text
4bac18215e03ba9834d0a65a137408ee046b126505fadcc4f885e7ca8fcff68b session-creation-intent.feature
a18c10d9f65d64bce039082f4c8b6dc27205c63437badc49ef83d9e0ed1b8bbf session-creation-completion.feature
226912b7e022fc651ff8e1c982e9296858144c1b27bf6f4f906e37bfbc2ffbbf generic-agent-creation.feature
```

The additional specification manifest has SHA-256
b48ca8529b4122a0427ef30ed7df192b6dd10e244592adde052b33cfe97218c2;
it supersedes the earlier specification manifest only for the cross-reference
doc-string addition. The original archive is preserved as a historical snapshot.
Rust source still matches b124450408e1e50c89b0ddc99dbb660b71ddef08088c7cd8b5128faf8de4f9a1.
Read-only checks again confirmed the denied legacy feature unchanged and
/tmp/bug280-281-issue.md absent. No denial retry or done transition occurred.
