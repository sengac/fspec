# CLA-001 validation record

## Scope and status

Complete for the supported native compiler/lint matrix: warning-denied workspace
checks, separate native NAPI and noop modes, and independent checks of the patched
rig-core workspace all pass. Dependency compilation alone is not counted as a
rig-core lint result. This is not an all-platform or all-workspace-runtime pass.

Host: aarch64-apple-darwin; rustc 1.95.0 (59807616e, 2026-04-14).
No cross-platform result is implied by the native checks.
No older-toolchain or MSRV result is claimed.

## Verified workspace checks

The following checks exited zero with no compiler-message diagnostics:

- Workspace default, all targets.
- Workspace full native runtime, all targets, including gated native tests.
- Workspace noop, all targets, checked separately from native runtime.
- Workspace shipping libraries/binaries in ci-test and release-slim profiles.
- Standalone fspec, all targets, without workspace feature unification.
- Standalone rpc-types, all targets, without default features.

Commands run from `rust/`, using `CARGO_PROFILE_CI_TEST_DEBUG=0`,
`CARGO_BUILD_JOBS=1`, and `CARGO_INCREMENTAL=0`:

```sh
cargo clippy --offline --profile ci-test --keep-going -j1 --workspace --all-targets -- -D warnings
cargo clippy --offline --profile ci-test --keep-going -j1 --workspace --all-targets --features codelet-agent-loop/test-support,codelet-cli/test-support,codelet-providers/integration-tests,codelet-providers/test-support,codelet-rpc/test-fixture,codelet-fspec/test-stub-provider,codelet-rpc-types/napi,codelet-napi/__full_runtime -- -D warnings
cargo clippy --offline --profile ci-test --keep-going -j1 --workspace --all-targets --features codelet-agent-loop/test-support,codelet-cli/test-support,codelet-providers/integration-tests,codelet-providers/test-support,codelet-rpc/test-fixture,codelet-fspec/test-stub-provider,codelet-rpc-types/napi,codelet-napi/noop -- -D warnings
cargo clippy --offline --profile ci-test --keep-going -j1 --workspace --lib --bins -- -D warnings
cargo clippy --offline --profile release-slim --keep-going -j1 --workspace --lib --bins -- -D warnings
cargo clippy --offline --profile ci-test --keep-going -j1 -p codelet-fspec --all-targets -- -D warnings
cargo clippy --offline --profile ci-test --keep-going -j1 -p codelet-rpc-types --all-targets --no-default-features -- -D warnings
```

Machine-readable receipts and complete command output are local at
`/Users/rquast/Documents/Codex/2026-10-23/task-3/validation/clippy-oct27/`.
The final matrix receipt suffix is `cla001-final`. Default workspace validation
was subsequently repeated with suffix `cla001-pinned`, with exit zero, zero
diagnostics and unchanged inputs. The final native-runtime confirmation is
`/tmp/cla001-native-final.receipt.json`: exit zero, zero diagnostics and unchanged
inputs. The interrupted earlier duplicate native confirmation is not counted as
a passing result.

The initial default, native and noop receipts independently recorded identical
before/after hashes for 315 changed Rust build inputs. Files were subsequently
staged externally, which exposes a limitation in the existing snapshot helper:
`git ls-files --modified` excludes staged-only modifications. A separate audit
compared all 315 recorded source hashes against the actual working files and
found no content changes. The release check's differing snapshot counts must
not be mistaken for a source modification or an unchanged-source proof.

The final audit instead hashes all 3,930 tracked and untracked Rust build inputs,
including staged-only files. The inventory is byte-identical before and after
the source-pinned confirmations. Its SHA-256 digest is
`4433d132cb072b91659fc66aa4bcb1deae2b3d3470768315af355443592ada77`.
Inventories: `/tmp/cla001-pinned-source-before.json` and
`/tmp/cla001-pinned-source-after.json`. Changed Rust files pass rustfmt checks;
`git diff HEAD --check` passes across both staged and unstaged changes.

A conservative six-GiB disk-reserve guard interrupted one duplicate native
confirmation without compiler diagnostics. After checking available memory and
disk headroom, the remaining metadata-only checks ran at one job with a four-GiB
reserve guard. Both completed successfully without hitting that guard. No cache
deletion was used to recover space. The final native/all-features receipts record
this reserve explicitly; stopped attempts remain distinguishable from passes.

## Focused runtime evidence already reviewed

- Patched rig-core: all 13 library/integration test targets pass, totaling
  306 passed, zero failed and three pre-existing ignored tests. The successful
  complete run is `/tmp/cla001-rig-regressions-complete.log`. Fixture repairs
  preserve every assertion: reasoning/cache subsets now fit within measured
  totals, successful streams carry explicit finish reasons and usage, and
  invalid-usage fixtures reach the intended validation failure rather than
  failing earlier on missing terminal evidence. Production usage validation
  is unchanged.
- Required Codex history and reload targets pass: two provider cases and three
  agent-loop cases. Logs: `/tmp/cla001-prov149-history.log` and
  `/tmp/cla001-prov149-reload.log`.
- Cold compaction: 11 distinct cold-process cases and eight failure cases pass.
  Child-process repetitions are not counted as extra distinct cases.
- Typed builder context: five cases pass after a demonstrated generation-bound
  authority failure and repair.
- Replay lifecycle: ten cases pass after a demonstrated shifted-reminder
  failure and repair; missing, ambiguous and rewritten anchors fail closed.
- Replay transport projection: three cases pass, including genuine signature
  and tool-input preservation and the known Gemini transport placeholder.
- Native uncaptured injection parity: three integration cases and eight
  handler unit cases pass against the canonical rejecting engine.
- Compactor pinning: eight cases pass; convergence watchdog: nine cases pass.
- Provider-waiting noop wire codec: one case passes after extracting the same
  serializer for both build modes.
- Additional existing receipts cover persistence diagnostics, MCP command
  coverage, summary validation, subordinate relay and search responsiveness.

These are focused checks, not a claim that every workspace runtime test passed.
No unscoped workspace test invocation is required or claimed.

Complete patched rig-core runtime reproduction:

```sh
cargo test --manifest-path patches/rig-core/Cargo.toml --offline --locked --profile ci-test -j1 -p rig-core --features derive --lib --tests --no-fail-fast --target-dir target --config 'profile.ci-test.inherits="test"' --config profile.ci-test.debug=0
```

## Patched rig-core checks

The independent workspace passes all-targets Clippy with no default features,
default features, the host-optional feature selection, and all features. Final
no-default/default/host-optional receipts use the `cla001-final` suffix in the
matrix directory. The final all-features receipt is
`/tmp/cla001-rig-all-final.receipt.json`, with corresponding `.jsonl` and `.log`
files: exit zero, zero diagnostics, no disk interruption, and all 3,930 build
inputs unchanged. The JSON stream ends in `build-finished: success=true`.

The original metadata declared 108 targets. The repaired metadata still
declares exactly 108 targets. The all-features check emits rig-core artifacts
for every one of them, including derive, Discord, audio, image, PDF and RMCP
examples and the derive integration test. No target was removed. The 19
feature-dependent targets now declare their actual prerequisites, rather than
attempting to import APIs disabled in default builds.

The missing rig-derive dependency is restored at the upstream-compatible
version 0.1.10, with its dependency closure in the standalone lockfile. Fixture
initializers are brought up to date without dropping assertions. Schema test
variant renaming preserves the exact serialized names. Iterator simplification,
let chains and descending sort_by_key retain the previous behavior without
adding lint suppressions.

Reproduction from `rust/` (same low-resource environment as the root gates):

```sh
cargo clippy --manifest-path patches/rig-core/Cargo.toml --offline --locked --profile ci-test -j1 --all-targets --no-default-features --target-dir target --config 'profile.ci-test.inherits="test"' --config profile.ci-test.debug=0 -- -D warnings
cargo clippy --manifest-path patches/rig-core/Cargo.toml --offline --locked --profile ci-test -j1 --all-targets --target-dir target --config 'profile.ci-test.inherits="test"' --config profile.ci-test.debug=0 -- -D warnings
cargo clippy --manifest-path patches/rig-core/Cargo.toml --offline --locked --profile ci-test -j1 --all-targets --all-features --target-dir target --config 'profile.ci-test.inherits="test"' --config profile.ci-test.debug=0 -- -D warnings
```

Dependencies were fetched when restoring the missing optional proc macro;
subsequent checks can use the recorded lockfile and offline cache.

## Independent limitations

- Non-native targets are not validated. Installed Rust standard libraries alone
  do not provide Linux/Windows C toolchains, and no corresponding cross C
  compiler was found. Native all-features compilation enables the wasm feature
  but does not exercise wasm32-specific implementations; it is not a wasm32
  build result.
- Some legacy native injection integration tests still require uncaptured raw
  injection success. Their expectations predate the canonical rejecting
  compaction contract; compilation is not evidence that these runtime tests
  pass. They remain present and have not been excluded from Clippy.
- The independent-provider persistence lifecycle test fails at the real writer
  with `incompatible provider state: provider identity`. The strict Codex
  identity implementation is unchanged from HEAD. This pre-existing protocol
  mismatch is not repaired by weakening storage authority validation.
- Repository-wide Gherkin syntax validation passes. Repository-wide tag
  validation reports 1,638 valid and 522 invalid feature files. The updated
  structured-replay and typed-summary feature files individually pass tag
  validation; unrelated missing tags are not silently repaired.

## Safety and lint policy

Workspace lint levels are unchanged. Three statement-scoped result_large_err
exceptions accommodate tungstenite's fixed owned callback error signature;
there is no blanket warning suppression. Unpublished files and externally
staged changes are preserved. This follow-up performs no commit, deployment,
daemon restart, index manipulation or project-cache deletion.
