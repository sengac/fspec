# BUG-283 superseding quality-review evidence: mandatory file sizes

## Supersession

This note supersedes the oversized-test-file exception stated in `implementation-findings.md` and `validation-evidence.md`. That exception did not satisfy the mandatory file-organization rule. Both edited legacy integration files are now mechanically split into focused modules, with every active Rust file changed by BUG-283 below 300 lines. There is no remaining size exception for these files.

The original research, causal RED, initial regression failures, successful first-pass verification, original production hashes, original Done observation and original receipt archive remain unchanged as historical evidence. The previous source manifest describes that earlier snapshot; use `quality-review-size-hashes.sha256` for the current split test sources and coverage sidecars. The original artifacts are not rewritten to disguise the review correction.

## Mechanical change

The existing integration target names remain `blocklist_view_rpc056` and `provider_settings_dispatch_rpc054`. Their shared fixtures remain in the target roots. Entire test functions move to ordinary child modules imported by the roots. Rust test output gains module qualifiers, but all original function names and test attributes remain intact.

| File under `rust/fspec-tui/tests/` | Lines |
| --- | ---: |
| `blocklist_view_rpc056.rs` | 105 |
| `blocklist_rpc056/dispatch.rs` | 127 |
| `blocklist_rpc056/interaction.rs` | 113 |
| `blocklist_rpc056/rendering.rs` | 146 |
| `provider_settings_dispatch_rpc054.rs` | 79 |
| `provider_dispatch_rpc054/lifecycle.rs` | 132 |
| `provider_dispatch_rpc054/credentials.rs` | 179 |
| `provider_dispatch_rpc054/status.rs` | 219 |

The independent preservation verifier compares every original test and fixture function body against the split sources byte-for-byte. It also compares all test attributes, test-name sets, and lint-allowance attributes. Result: **14 blocklist tests and 12 provider tests retained unchanged**, including all assertions, literal `@step` comments, Tokio runtime attributes and explicit Agent-entry corrections. No test, assertion or behavior is replaced with a placeholder or synthetic success. No lint allowance is added.

Per-test SHA-256 values and the full preservation result are in `receipts/preservation.json` inside the review archive. The before-split source snapshots are retained there for independent comparison.

## Coverage migration

Only the two affected legacy coverage sidecars are relinked, using native Fspec `link-coverage` followed by `unlink-coverage`; no shell fspec command or validation-skip flag is used. All 26 scenarios now point to the actual moved test file and its exact function range rather than stale coordinates in the former oversized root.

Every pre-existing implementation file/line binding is retained. The Down and Up scenarios previously each had two duplicate test-file mappings with different implementation-line subsets; their new single test mapping retains the union of both subsets. No implementation evidence is dropped. The archived before-sidecars and independent `verify-coverage.py` comparison verify these sets exactly. This mechanical review does not claim to repair or certify unrelated legacy implementation-coordinate drift; its scope is the moved test paths/ranges, retained literal steps, preserved bindings and file sizes.

Native coverage audits report all referenced files present: provider **24/24**, blocklist **28/28**. Both sidecars retain 100% scenario-mapping coverage. BUG-283's own eleven-scenario feature, test mapping and implementation bindings remain unchanged and independently pass the original exact per-test step/mapping verifier.

## Rerun results

All commands use the existing isolated `/tmp/fspec-bug283/run` launcher: synthetic home/config/data/session/search/runtime roots, offline Cargo, `ci-test`, debug0 and `-j2`. Cargo rebuilds the changed integration targets in the existing task-owned cache. No installation, daemon restart, commit, push, new agent/session or model change occurs.

- `cargo test --offline --profile ci-test -j2 -p codelet-fspec-tui --no-fail-fast --test blocklist_view_rpc056 --test provider_settings_dispatch_rpc054 --test bug283_auxiliary_view_return_origin --test rpc337_navigator_model_selector -- --test-threads=1`: **41 passed, 0 failed**, exit 0 (14 + 12 + 11 + 4).
- `cargo clippy --offline --profile ci-test -j2 -p codelet-fspec-tui --lib --test blocklist_view_rpc056 --test provider_settings_dispatch_rpc054 --test bug283_auxiliary_view_return_origin --test rpc337_navigator_model_selector -- -D warnings`: PASS, exit 0.
- `rustfmt --edition 2021 --config skip_children=true --check` on both roots and all six new modules: PASS, exit 0.
- Function-body/test-attribute/step/assertion preservation and under-300 line counts: PASS.
- All 26 moved scenario mappings, exact test ranges and original implementation-binding sets: PASS.
- Original BUG-283 eleven-scenario mapping/step verifier: PASS.
- `git diff --check`: PASS.
- Native `review BUG-283` after the correction: PASS, no critical issues or warnings, 100% scenario coverage and normal temporal history.

The earlier 108-test result and package check/library build receipts are retained, not silently represented as newly executed. This correction changes only test organization and scoped legacy test-coordinate mappings; it does not alter production code or acceptance semantics.

## Protected production and prior evidence

SHA-256 verification confirms these files remain byte-identical to the state at the start of this substantive review:

- `rust/fspec-tui/src/views/navigator.rs`: `bc53df4301fa8ba62c2b2a30ef21df4feb109b74acac228cfcdca8d7a10414bb`
- `rust/fspec-tui/src/views/navigator_auxiliary.rs`: `db3ebd963db206c8d6d348aa85858d74f52a14a547db7f21e320a431fded24c2`
- `rust/fspec-tui/src/app/dispatch_capability.rs`: `2e8c3c5303dd02f9748128492c2b9974e232e58643f1b63d473733e58fdba72d`

BOARD-024's dispatch branch and all its other work remain untouched. AUDIT-002 is not modified. Original receipt-archive and RED-log hashes also still match the original manifest. In particular, the initial regression receipts inside that unchanged archive are preserved.

## Lifecycle and proof attachments

BUG-283 is reopened through the supported native `done -> validating` transition for this substantive review, then returned through the normal native `validating -> done` gate only after the correction passes. No skip flags are used and no prior status history is discarded.

- `quality-review-size-receipts.tar.gz`: before-split snapshots, independent verification scripts, per-test hashes, coverage migration plan/results, raw test/Clippy/format receipts and current split source snapshots.
- `quality-review-size-done.json`: generated canonical readback after review closure, including the preserved original ACDD history and the normal review reopen/close transitions.
- `quality-review-size-hashes.sha256`: superseding current working-tree/evidence manifest, while retaining original historical manifests unchanged.
