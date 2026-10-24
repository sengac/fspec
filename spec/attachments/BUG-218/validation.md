# BUG-218 implementation and validation

Card status: **done**, after the scoped quality gates below passed. Parent owns
staging and committing this work; no commit or branch was created by this worker.

## Outcome

- Model listing HTTP 404/405 now returns the typed, nonretryable
  `ProviderError::ModelDiscoveryUnsupported { provider, status }`.
- A nonblank configured `customModels` ID makes only that optional listing
  failure nonfatal. A warning explicitly states that inference is unverified.
- Authentication, server, transport, malformed-response and skipped-runtime
  failures remain unreachable even with configured models. Custom definitions
  remain visible; blank IDs are not offered.
- Successful empty discovery stays a successful empty list, not a fabricated
  configured model or a transport failure.
- Existing URL composition already preserves proxy prefixes and normalizes
  a single trailing `/v1`; it is protected by authenticated local HTTP tests.
  No speculative fallback URLs or production inference probes were added.

## ACDD evidence

Read `AGENTS.md`, `docs/TESTING.md`, the attached research, FOUNDATION, TAGS,
PROV-131 and RPC-338 contracts before editing. Recorded Example Mapping on
BUG-218, wrote features first, then integration tests with exact `@step`
comments. Ran behavioral red tests before changing production code.

The legacy custom-model override was deliberately narrowed in both
`local-profile-model-probe-resilience.feature` architecture notes and the
RPC-338 custom-model example. No-runtime safety remains protected by the
existing PROV-131 integration target and new configured-model cases.

All cargo commands below ran from `rust/`, prefixed with:

```sh
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
```

Output was captured with `2>&1 | tee <log>` and `set -o pipefail`.

| Phase | Command | Result and log |
| --- | --- | --- |
| Red | `cargo test -p codelet-providers --test bug218_model_discovery_errors -j 2` | 1 failed, 2 passed: unsupported 404 was generic `Api`; `/tmp/bug218-providers-red.log` |
| Red | `cargo test -p codelet-sessions --test bug218_optional_model_discovery -j 2` | 5 failed, 2 passed: blanket custom-model override hid hard/runtime failures and blank IDs; `/tmp/bug218-sessions-red.log` |
| Green | `cargo test -p codelet-providers --test bug218_model_discovery_errors -j 2` | 3 passed; `/tmp/bug218-providers-final-green.log` |
| Green | `cargo test -p codelet-sessions --test bug218_optional_model_discovery -j 2` | 7 passed; `/tmp/bug218-sessions-final-green.log` |
| Regression | `cargo test -p codelet-sessions --test prov131_local_profile_probe_resilience -j 2` | 3 passed; `/tmp/bug218-prov131-regression.log` |
| Check | `cargo check -p codelet-providers -p codelet-sessions -j 2` | Passed; `/tmp/bug218-check.log` |
| Clippy | `cargo clippy -p codelet-providers --test bug218_model_discovery_errors -j 2 -- -D warnings` | Passed; `/tmp/bug218-providers-final-clippy.log` |
| Clippy | `cargo clippy -p codelet-sessions --test bug218_optional_model_discovery -j 2 -- -D warnings` | Passed; `/tmp/bug218-sessions-final-clippy.log` |

Initial strict test-target clippy runs rejected test `expect` calls. The final
tests propagate fixture errors instead; no lint suppressions were introduced.

## Specifications and coverage

- `model-discovery-http-errors.feature`: 3 scenarios, 3 integration tests,
  one test file, implementation mappings to `openai.rs` and `error.rs`.
- `optional-profile-model-discovery.feature`: 7 scenarios, 7 integration tests,
  one test file, implementation mappings to `profile_sections.rs`.
- Both unique coverage files have exactly one test mapping per scenario and
  100% scenario coverage. `fspec audit-coverage` passes for each feature.
- Scoped `fspec validate` and `fspec validate-tags` pass for both new and both
  deliberately updated legacy features: `/tmp/bug218-spec-quality.log`.
- `rustfmt --edition 2021 --check` passes for all five changed Rust files.
  Scoped `git diff --check` passes: `/tmp/bug218-fmt-scoped.log`.
- Every new file is below 300 lines. Existing large source files were changed
  narrowly without structural refactoring.
- All mutating fspec commands were serialized with the shared
  `/tmp/fspec-acdd-board.lock` mutex. No other cards or tag registry were edited.

## Exact owned file list

1. `rust/providers/src/error.rs`
2. `rust/providers/src/openai.rs`
3. `rust/providers/tests/bug218_model_discovery_errors.rs`
4. `rust/sessions/src/profile_sections.rs`
5. `rust/sessions/tests/bug218_optional_model_discovery.rs`
6. `spec/features/model-discovery-http-errors.feature`
7. `spec/features/model-discovery-http-errors.feature.coverage`
8. `spec/features/optional-profile-model-discovery.feature`
9. `spec/features/optional-profile-model-discovery.feature.coverage`
10. `spec/features/local-profile-model-probe-resilience.feature`
11. `spec/features/model-selector-profile-server.feature`
12. `spec/attachments/BUG-218/validation.md`
13. `spec/work-units.json` (BUG-218 workflow, Example Mapping, attachment only;
    shared file also contains other agents' work)

The pre-existing attached `research.md` is unchanged. Nothing was staged or
committed. A small adjacent import-wrap change in `openai.rs` is rustfmt-only.

## Remaining limits

- No live endpoint was contacted. Original RunPod deployment routing and
  inference availability remain unverified, as required by the research.
- A 404/405 can still indicate a misconfigured deployment path. Keeping an
  explicitly configured custom model selectable is not evidence of health;
  the typed error and warning retain that distinction. Profiles without such
  a model remain unavailable.
- The existing wire schema exposes only `is_unreachable`, not a separate
  discovery-capability status. No RPC schema, TUI cache or model mutation was
  changed in this card.
- A global tag-validation attempt encountered unrelated pre-existing feature
  tag violations. A global whitespace check encountered another worker's
  trailing blank line in `cont009_completion_contract_sync.rs`. Scoped BUG-218
  checks pass; unrelated files were left untouched.
- `fspec validate-spec-alignment BUG-218` reports no scenario-level tags because
  its scan does not inherit feature-level tags. Feature-level work-unit tags
  are required by the tag validator; direct coverage audits are authoritative
  for these ten scenarios. The tool itself was not changed.
