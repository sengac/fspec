# Current core/tools completion evidence

Owner `1cd6ff6c`, coordinator `ad8aee9e`, 2026-10-18.
No board transitions, staging, commits, cross-owner production edits or bypasses.

## Current executed results

`/tmp/cmpct-core-current-status.log` records `regression exit=0`,
`blob-compat exit=0`, `tools exit=0`, `inject exit=0`, `check exit=0`,
`clippy-tools exit=0` and `clippy-lib-retry exit=0`.
Both first core Clippy attempts exit 101 during an in-flight provider edit: duplicate trait methods,
illegal trait-item `pub` qualifiers and `self.extensions` on the extension trait
in `providers/src/session.rs` (`E0428`, `E0449`, `E0609`). This is a dependency
compilation failure, not an owned lint or behavioral RED. The provider owner
subsequently moved the methods into the proper implementation; the fresh library
Clippy retry passes and the failed logs remain preserved. That passing retry
reports two existing provider warnings (unused `codex` import and dead `is_user`),
not owned lint errors. The test-target retry finds two forbidden `expect_err`
calls in `cmpct055_publication_ownership.rs`; both now propagate unexpected
success as a test error with `err().ok_or(...)?`, retaining the real error and
promoted-identity assertions. Final fourteen-target Clippy exits 0 in
`/tmp/cmpct-core-current-clippy-tests-final.log`; the focused publication rerun
passes both top-level tests, exit 0, in
`/tmp/cmpct-core-current-publication-final.log`. Its focused formatting check
also exits 0. The replacements preserve existing function line ranges and
scenario steps, so the strict mappings remain unchanged.

Exact successful quality commands, all from `rust/` with disk above 20 GiB:

```
cargo clippy --profile ci-test -p codelet-core -p codelet-tools --lib -j2
cargo clippy --profile ci-test -p codelet-tools --test compaction_generation_dispatch -j2
cargo clippy --profile ci-test -p codelet-core --test cmpct055_generation_commit --test cmpct055_publication_ownership --test cmpct059_blob_identity --test cmpct059_durable_boundary --test cmpct064_manifest_transactions --test cmpct064_lost_ack --test cmpct064_delete_ownership --test cmpct065_provider_usage --test cmpct069_durable_reasoning --test cmpct070_cold_boundary --test cmpct070_promotion_validation --test cmpct070_whole_dag --test cmpct072_durable_preservation --test cmpct082_fail_closed_hook -j2
cargo test --profile ci-test -p codelet-core --test cmpct055_publication_ownership -j2
rustfmt --edition 2021 --check core/tests/cmpct055_publication_ownership.rs
```

Library quality is recorded in `/tmp/cmpct-core-current-clippy-lib-retry.log`;
tools target quality is in `/tmp/cmpct-core-current-clippy-tools.log`.
The final integration Clippy also reports the existing unrelated agent-loop
unused `clear_history()` result warning. No warning suppressions or cross-owner
repairs were introduced. Final available disk is 24 GiB. All queued core quality
work has completed; no core validation command remains pending.

The regression command is `cargo test --profile ci-test -p codelet-core -j2
--no-fail-fast` with these explicit `--test` targets:

| Target | Top-level passed |
| --- | ---: |
| cmpct055_generation_commit | 4 |
| cmpct055_publication_ownership | 2 |
| cmpct059_blob_identity | 1 |
| cmpct059_durable_boundary | 4 |
| cmpct064_delete_ownership | 1 |
| cmpct064_lost_ack | 1 |
| cmpct064_manifest_transactions | 8 |
| cmpct065_provider_usage | 5 |
| cmpct069_durable_reasoning | 3 |
| cmpct070_cold_boundary | 2 |
| cmpct070_promotion_validation | 1 |
| cmpct070_whole_dag | 5 |
| cmpct072_durable_preservation | 8 |
| cmpct082_fail_closed_hook | 1 |

All 46 top-level tests pass in `/tmp/cmpct-core-current-regression.log`.
Successful child-process summaries are not counted again.
`cargo test --profile ci-test -p codelet-core --lib blob -j2` additionally
passes 18 tests in `/tmp/cmpct-core-current-blob-compat.log`.
Tools generation dispatch passes 4 tests in `/tmp/cmpct-core-current-tools.log`;
the existing inject-summary unit selection passes 7 tests in
`/tmp/cmpct-core-current-inject.log`. `cargo check --profile ci-test -p
codelet-core -p codelet-tools -j2` exits 0 in
`/tmp/cmpct-core-current-check.log`.
These are core seams, not a claim that CLI/provider/runtime gate acceptance passes.

The current blob-identity behavioral RED and GREEN are recorded in
`spec/attachments/CMPCT-059/core-structured-identity-current.md` with exact
commands, real assertion failure and real 1/1 passing execution. The repair
preserves original nested JSON while resolving blobs, retaining distinct item
and invocation identities rather than weakening fixtures to equal identities.
Existing reasoning and hook RED/GREEN provenance remains in the inherited
completion attachments; no compile-only failure is relabeled behavioral RED.

### Inspected behavioral RED provenance

Each log below contains actual test execution and an assertion failure, not
merely missing APIs or dependency compilation errors. The corresponding current
targets all pass in the 46-test execution listed above.

| Area | Actual RED log | Observed failure |
| --- | --- | --- |
| 055 captured prefix | /tmp/cmpct055-prefix-red.log | Appended suffix caused revision conflict; 0 passed / 1 failed |
| 055 publication ownership | /tmp/cmpct-core-ownership-red.log | Reentrant ownership and promoted-identity regressions both failed; 0 / 2 |
| 064 provider creation | /tmp/cmpct064-provider-red.log | Returned revision was zero; 7 / 1 |
| 070 whole DAG | /tmp/cmpct070-behavioral-red.log | Invalid whole candidates, provenance, source and overlap accepted; 1 / 4 |
| 070 promotion | /tmp/cmpct070-promotion-red.log | Malformed candidate reached request admission; 0 / 1 |
| 072 source authority | /tmp/cmpct072-source-red.log | Invalid persisted DAG reached capture; 0 / 1 |
| 072 warm-cache evidence | /tmp/cmpct-core-warm-evidence-red.log | Cached originals authorized capture after durable evidence disappeared; 0 / 1 |
| 069 reasoning | /tmp/cmpct-core-reasoning-red.log | Lost terminal reasoning and accepted overflow; 1 / 2 |
| 082 hook phases | /tmp/cmpct-core-hook-phases-red.log | Unavailable authority allowed request/finish phases; 0 / 1 |
| 059 structured identity | /tmp/cmpct-core-current-blob-red.log | Explicit nested invocation identity became Null; 0 / 1 |

## Strict scoped specification evidence

`python3 /tmp/cmpct_core_current_audit.py` exits 0. Exact commands and mappings
are in `/tmp/cmpct-core-current-strict.json`; compact results are in
`/tmp/cmpct-core-current-strict.log`. All 12 owned features / 37 scenarios have
exactly one complete real test function per scenario, exactly one test target
per feature, scenario-local matching `@step` documentation and nonempty bounded
implementation mappings. Installed `fspec link-coverage`, scoped syntax,
formatting comparison, file audit and registered-tag checks all pass against
disposable copied specifications with read-only source symlinks. No skip flags
or real board mutations are used. This is traceability, not measured line coverage.

Focused edition-2021 Rust formatting passes, recorded as `FORMAT_EXIT=0` in
`/tmp/cmpct-core-current-format-final.log`. Owned `git diff --check` also exits 0.
Formatting changes required refreshing complete test ranges in the publication
ownership and manifest transaction sidecars; obsolete duplicate implementation
segments were removed and derived line statistics refreshed.

## Complete owned scenario mappings

Each section names its single integration target. Ranges below are inclusive;
the sidecars and strict JSON retain the complete implementation line arrays.
Production paths are relative to `rust/core/src/`.

### versioned-dag-generation-commit

Target: `rust/core/tests/cmpct055_generation_commit.rs`.

| Scenario | Test lines | Implementation |
| --- | --- | --- |
| Appended work survives a captured prefix commit | 14-48 | persistence/compaction_commit.rs:144-186 |
| An older candidate cannot replace a newer generation | 100-135 | persistence/compaction_commit.rs:89-108 |
| Changed prefix or model identity rejects a candidate | 137-162 | persistence/compaction_commit.rs:98-117 |
| Obsolete publication cannot announce a replaced generation | 50-98 | persistence/compaction_commit.rs:221-260 |

### dag-publication-ownership

Target: `rust/core/tests/cmpct055_publication_ownership.rs`.

| Scenario | Test lines | Implementation |
| --- | --- | --- |
| Publication callbacks cannot reenter session ownership | 13-106 | persistence/manifest_updates.rs:7-52; persistence/manifest_commit.rs:87-134; persistence/manifest.rs:561-578; persistence/manifest_delete.rs:15-37 |
| Post-promotion durability failure retains the compaction identity | 108-188 | persistence/compaction_commit.rs:7-186; persistence/manifest_commit.rs:174-203 |

### durable-session-manifest-transactions

Target: `rust/core/tests/cmpct064_manifest_transactions.rs`.

| Scenario | Test lines | Implementation |
| --- | --- | --- |
| Concurrent field writers retain all committed updates | 71-117 | persistence/manifest_commit.rs:100-118 |
| Stale whole snapshots cannot overwrite a newer commit | 119-142 | persistence/manifest_commit.rs:136-152 |
| Provider session creation publishes the committed revision | 49-69 | persistence/manifest.rs:297-314 |
| Failed promotion cannot publish a changed cache | 144-171 | persistence/manifest_commit.rs:158-166 |
| Interrupted staging preserves the committed revision | 173-193 | persistence/manifest.rs:254-285 |
| Competing processes cannot mutate an owned session | 195-229 | persistence/manifest_commit.rs:50-85 |
| Unreadable committed manifests are not absent sessions | 231-253 | persistence/manifest.rs:237-278 |
| Missing committed message references block replay | 17-47 | persistence/session_replay.rs:10-14; persistence/compaction_boundary.rs:156-170 |

### session-manifest-lost-acknowledgement

Target: `rust/core/tests/cmpct064_lost_ack.rs`.

| Scenario | Test lines | Implementation |
| --- | --- | --- |
| Lost acknowledgement cannot roll back a promoted revision | 9-53 | persistence/manifest_commit.rs:174-203 |

### session-manifest-delete-ownership

Target: `rust/core/tests/cmpct064_delete_ownership.rs`.

| Scenario | Test lines | Implementation |
| --- | --- | --- |
| Competing processes cannot delete an owned session | 8-64 | persistence/manifest_commit.rs:120-134; persistence/manifest_delete.rs:1-37 |

### whole-dag-candidate-validation

Target: `rust/core/tests/cmpct070_whole_dag.rs`.

| Scenario | Test lines | Implementation |
| --- | --- | --- |
| Validate persisted coordinates after active history shrinks | 18-38 | compaction/dag_validation.rs:102-125 |
| Reject malformed candidates without salvaging valid nodes | 40-63 | compaction/dag_syntax.rs:10-130 |
| Reject invalid provenance and missing coverage | 65-79 | compaction/dag_validation.rs:102-125; compaction/dag_syntax.rs:46-62 |
| Preserve cross-depth overlap but reject same-depth overlap | 81-106 | compaction/dag_syntax.rs:10-93 |
| Reject invalid existing marked DAG as invalid source | 108-129 | compaction/dag_syntax.rs:11-130; compaction/dag_validation.rs:48-60 |

### dag-cold-boundary-validation

Target: `rust/core/tests/cmpct070_cold_boundary.rs`.

| Scenario | Test lines | Implementation |
| --- | --- | --- |
| Reject malformed committed memory during cold replay | 11-126 | persistence/compaction_boundary.rs:108-154 |
| Invalid promoted memory cannot be reconciled | 128-169 | persistence/compaction_commit.rs:189-219 |

### dag-promotion-revalidation

Target: `rust/core/tests/cmpct070_promotion_validation.rs`.

| Scenario | Test lines | Implementation |
| --- | --- | --- |
| Revalidate whole candidates inside durable promotion | 11-89 | persistence/compaction_commit.rs:118-133 |

### durable-dag-preservation

Target: `rust/core/tests/cmpct072_durable_preservation.rs`.

| Scenario | Test lines | Implementation |
| --- | --- | --- |
| Preserve durable payloads and file awareness on every route | 82-96 | compaction/dag_validation.rs:126-156 |
| Reject missing or rewritten durable content | 98-115 | compaction/dag_validation.rs:126-156 |
| Reject malformed or empty required file metadata | 117-136 | compaction/dag_syntax.rs:95-130; compaction/dag_validation.rs:140-147 |
| Reject inaccessible or invalid authoritative source | 138-170 | persistence/compaction_boundary.rs:51-89; compaction/dag_validation.rs:48-79 |
| Assemble missing file metadata deterministically before validation | 172-199 | compaction/dag_validation.rs:80-101; compaction/dag_syntax.rs:95-130 |
| Unavailable originals reject source capture and promotion | 201-263 | persistence/compaction_boundary.rs:51-170; persistence/compaction_commit.rs:85-88 |
| Empty authoritative reminders cannot authorize preservation | 265-279 | compaction/dag_validation.rs:39-47 |
| Cached originals cannot conceal missing durable evidence | 13-60 | persistence/compaction_boundary.rs:156-170; persistence/session_replay.rs:10-14; persistence/compaction_commit.rs:85-88 |

### core-admission-hook-cancellation

Target: `rust/core/tests/cmpct082_fail_closed_hook.rs`.

| Scenario | Test lines | Implementation |
| --- | --- | --- |
| Inaccessible admission authority cancels every hook phase | 13-60 | compaction_hook.rs:196-279 |

### durable-reasoning-settlement

Target: `rust/core/tests/cmpct069_durable_reasoning.rs`.

| Scenario | Test lines | Implementation |
| --- | --- | --- |
| Measured terminal reasoning survives a compacted cold resume | 44-117 | request_settlement_hook.rs:73-110; persistence/request_settlement.rs:31-79; persistence/compaction_commit.rs:144-186 |
| Reasoning overflow rejects the complete settlement atomically | 119-146 | persistence/request_settlement.rs:31-79; persistence/manifest_commit.rs:102-122 |
| Unknown reasoning does not erase earlier measured reasoning | 148-175 | persistence/request_settlement.rs:13-79 |

### durable-structured-tool-identity

Target: `rust/core/tests/cmpct059_blob_identity.rs`.

| Scenario | Test lines | Implementation |
| --- | --- | --- |
| Committed replay preserves distinct tool item and invocation identities | 10-80 | persistence/blob_processing.rs:144-183; persistence/session_replay.rs:10-65 |

## Shared handoff and readiness

Current traceability owner `bfd53429` has documentation-only permission for
`rust/core/tests/cmpct059_durable_boundary.rs` and `cmpct065_provider_usage.rs`.
The shared `inclusive-provider-usage` feature/sidecar transfer permits removing
only the two custom-provider rows already preserved by stable sibling features,
while retaining all five core regressions. All other core sidecars stay core-owned.
This permission is also durable in `core-resumed-completion.md`; repeated direct
messages returned busy and were not queued.

Wholly owned 055/064/070/072 have current behavioral and strict mapping GREEN;
current check, library/tools Clippy and all fourteen integration-target Clippy
checks pass. These four wholly owned cards are independently ready for the
coordinator's normal dependency-ordered closure. The core065 five-test seam is
also GREEN; shared traceability finalization belongs to its designated owner.
Core portions of 059/069/082 are
similarly verified at their seams, not full cross-layer card completion.
Only the coordinator may perform normal dependency-ordered board transitions
after reviewing the completed quality evidence and other owners' acceptance.

## Current completion-pass file inventory

The focused behavioral repair modifies
`rust/core/src/persistence/blob_processing.rs` and creates
`rust/core/tests/cmpct059_blob_identity.rs`,
`spec/features/durable-structured-tool-identity.feature`, its `.feature.coverage`
sidecar and `spec/attachments/CMPCT-059/core-structured-identity-current.md`.
The final lint-only repair modifies
`rust/core/tests/cmpct055_publication_ownership.rs` without changing assertions,
test identity, line counts or acceptance steps. Existing focused formatting also
normalizes inherited owned integration targets; corresponding mapping corrections
are in `dag-publication-ownership.feature.coverage` and
`durable-session-manifest-transactions.feature.coverage`.
This attachment is created and `core-resumed-completion.md` is updated as the
owned durable mailbox. No tools production repair is needed in this pass;
inherited tools implementation is retained and its actual tests/Clippy pass.
Shared core059/core065 documentation remains delegated, not claimed as core edits.
