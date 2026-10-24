# PROV-161 ACDD validation

## Specification

- Example Mapping: four rules, twenty concrete examples, and explicit assumptions
  recorded before tests.
- Twenty frozen scenarios linked 1:1 to five Rust integration targets with exact `@step`
  comments before production changes.
- Before implementation, the workflow gate required one test file per feature.
  The unchanged scenarios were split by editor, backend, resolution, lifecycle,
  and physical request ownership into five `@PROV-161` feature files. Existing
  test mappings and exact step comments were retained.
- Feature formatting, syntax validation, and scoped tag validation passed.
- Card transitioned from specifying to testing before test creation.

## RED: lifecycle integration

Command, from `rust/`:

```sh
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 \
  cargo test -p codelet-sessions --test prov161_runtime_lifecycle -j 2
```

Result: **0 passed, 2 failed**, exercising real session creation, manifest resume,
subordinate registration, and backend model switching with a temporary offline
catalog and filesystem configuration.

- Creation/resume/subordinate scenario: continuation state was `(false, 10)`;
  expected saved provider default `(true, 7)` for the original session.
- Switching scenario: inherited continuation remained `(false, 10)`;
  expected new provider default `(true, 3)`.
- Full original output: `/tmp/prov161-runtime-red.log`.

## Build environment incident

After lifecycle RED completed, `rust/target/debug` artifacts disappeared during
the next build. Cargo reported missing fingerprint directories and missing
dependency metadata. Free disk increased from approximately 9 GiB to 37 GiB.
No cleanup command was issued by the coordinating agent or authorized for workers.
These infrastructure failures are **not** accepted as RED behavior evidence;
affected targets must be rebuilt and rerun before implementation authorization.

## RED: missing integration APIs

- Provider resolution and serde targets were rebuilt successfully through their
  dependencies. Compilation fails on the planned missing `ProviderRuntimeOptions`,
  `resolve_runtime_options`, `runtime_options`, and output wire-policy APIs.
  Logs: `/tmp/prov161-prov161_runtime_resolution-red-retry.log` and
  `/tmp/prov161-prov161_runtime_options_serde-red-retry.log`.
- TUI target fails on the missing provider-settings read types, Settings row,
  settings editor mode, and settings save action. Full output:
  `/tmp/prov161-provider_settings_parity_prov161-red.log`.
- Physical-request target fails on the missing `RigAgent::with_output_limit`
  integration point. Full output: `/tmp/prov161-prov161_physical_runtime-red.log`.

These are API-level RED results, not passing executable behavior tests. Each
target must subsequently compile and exercise its assertions during GREEN.

## Remaining validation

Backend persistence rebuilt through its dependencies and fails on the missing
`save_provider_settings`, `list_provider_settings`, and inherited profile section
APIs (`/tmp/prov161-prov161_provider_settings_backend-red-retry.log`).

The legacy `profile_rpc_surface` target executes **7 passed, 1 failed**: saving
an Anthropic profile still returns the obsolete OpenAI-only error. Full output:
`/tmp/prov161-profile_rpc_surface-red-retry.log`.

Historical TUI targets are being rerun after their specifications and tests were
updated. Their RED build fails on the intentionally provider-qualified profile
map API (`/tmp/prov161-tui-legacy-red.log`).

## Implementation authorization

After the RED checks and feature split, PROV-161 entered `implementing`.
Production work was then authorized for the four worker slices and session
lifecycle integration. GREEN behavior and quality checks remain pending.

## GREEN: acceptance scenarios

All twenty frozen acceptance scenarios now compile and execute successfully:

| Target | Passed | Full output |
| --- | ---: | --- |
| `prov161_runtime_resolution` | 5 | `/tmp/prov161-prov161_runtime_resolution-green-retry.log` |
| `prov161_provider_settings_backend` | 4 | `/tmp/prov161-sessions-green.log` |
| `prov161_runtime_lifecycle` | 2 | `/tmp/prov161-sessions-green.log` |
| `prov161_physical_runtime` | 4 | `/tmp/prov161-physical-green.log` |
| `provider_settings_parity_prov161` | 5 | `/tmp/prov161-tui-new-green.log` |

Supplemental runtime-option serde test passes (1/1), and the historical
`profile_rpc_surface` target passes (8/8). The physical tests capture real local
HTTP requests, including output budgets, Codex parameter omission, signed
reasoning, and gated progressive output. Lifecycle tests exercise saved settings
on creation, resume, subordinate registration, and provider/profile switching.

The first historical TUI batch found a missing local fixture binding in the
failed-delete test. The binding is restored without removing its error-path
assertions; the historical targets and broader quality checks remain pending.

## Review regressions and quality

- Historical TUI regression batches pass: 72 tests across nine navigation/editor
  targets and 74 across ten authentication/form/dispatch targets. Logs:
  `/tmp/prov161-tui-legacy-green-retry.log` and
  `/tmp/prov161-tui-auth-regression.log`.
- Cross-crate `cargo check` and production-library `cargo clippy` pass for
  providers, CLI, sessions, TUI, NAPI, and agent-loop (two build jobs, debug info
  and incremental compilation disabled). Existing warnings remain. Full output:
  `/tmp/prov161-cargo-check.log` and `/tmp/prov161-clippy.log`.
- Changed Rust files pass targeted rustfmt checking and `git diff --check`.
  Package-wide formatting reports existing differences in untouched files;
  those files are deliberately not reformatted.
- All five PROV-161 features pass syntax and registered-tag checks. Coverage
  audits pass for all five features. Historical profile-CRUD scenario-level
  work-unit tags still violate current tag rules; they predate this work.
- Review tests demonstrate three additional resolution failures before fixes:
  standalone `FSPEC_USER_DIR` settings were ignored, a native Rhai provider was
  incorrectly assigned a facade, and script-provided limits lost precedence.
  RED: 0/3 passed, `/tmp/prov161-provider-review-red.log`.
- An enhanced backend round-trip assertion demonstrates a legacy alias bug:
  writing canonical Anthropic settings creates a shadow provider instead of
  preserving the existing `claude` node. RED: 0/1 passed,
  `/tmp/prov161-alias-red.log`. The storage fix is authorized only after this RED.

Broader historical library runs also reveal pre-existing expectation drift:
four Codex tests expect a 272000 fallback although the unchanged resolver
returns unknown (zero); two CLI compaction tests expect 68000 for missing output
metadata although the unchanged implementation fails closed with zero. These
are not changed or disabled in source. Focused regression reruns exclude only
these known unrelated tests and retain all affected behavior assertions.

## Final regression results

- Provider integration targets pass: review regressions 3/3, resolution 5/5,
  serde 1/1, Rhai model limits 10/10, built-in physical output limits 3/3, and
  custom-provider model limits 4/4. Log:
  `/tmp/prov161-providers-targets-green-retry.log`.
- Provider library passes 351/351 executed tests; the four unrelated historical
  Codex fallback tests named above are explicitly filtered, not edited.
  Log: `/tmp/prov161-providers-lib-green.log`.
- CLI compaction module passes 33/33 executed tests; only the two unrelated
  missing-output fallback tests are filtered (the other filtered tests are
  outside this module). Log: `/tmp/prov161-compaction-green.log`.
- The scripted-provider fixture uses an absolute script path, matching the
  existing runtime hook tests; its first retry exposed the separate existing
  relative-script loader issue. This changes fixture input only and retains
  native identity, script token-limit, and compaction assertions.

## Continuation review

Reviewed the coordinating session `86e15383-927d-462e-917b-94e668435e11`
and its backend worker `0506a9aa-90df-4eb4-bcb8-ae9afbe47610`. The backend
worker completed the final coverage refresh after the coordinator stopped.
The card was still implementing, not done.

- Reran all twenty acceptance scenarios, supplemental resolution/serde checks,
  and the eight historical profile RPC scenarios: **32 passed**. Full output:
  `/tmp/prov161-continuation-acceptance.log`.
- Cross-crate check and production-library Clippy pass for providers, CLI,
  sessions, TUI, NAPI, and agent-loop. Configured Codex history/reload checks
  pass **5/5**. Log: `/tmp/prov161-continuation-quality.log`.
- Credential bridging, continuation persistence, thinking persistence and
  thinking session seeding pass **12/12**. Log:
  `/tmp/prov161-continuation-session-regressions.log`.
- All **2040** existing feature files pass syntax validation. Repository-wide
  tag validation reports **526** pre-existing feature violations; all five
  original PROV-161 features pass individually. Logs:
  `/tmp/prov161-continuation-features.log` and
  `/tmp/prov161-continuation-tags.log`.

Review discovered two in-scope integration gaps: upstream scripted model IDs
were forwarded to a factory requiring configuration aliases, and custom
profile selections lost the underlying model's vision capability. Moved the
card from validating back to specifying, captured concrete examples, added
separate acceptance features, then moved to testing before creating tests.
No production changes were made during this review or specification phase.

### RED: custom profile capabilities

`cargo test -p codelet-sessions --test prov161_custom_profile_capabilities -j 2`
with debug information and incremental compilation disabled executes a real
session against temporary custom-provider and profile configuration.
Result: **0 passed, 1 failed** on the vision assertion: selecting the runtime
profile resolves vision as false despite `supports_vision: true`. The test also
checks the actual tool capability registry, inherited image budget, and a
non-vision control model. Log: `/tmp/prov161-custom-capabilities-red.log`.

### RED: scripted dispatch identity

`cargo test -p codelet-providers --test prov161_scripted_dispatch -j 2`
executes five scenarios using the real custom-agent factory and local HTTP.
Result: **3 passed, 2 failed**. Ordinary and named-profile selections fail with
`model alias 'private/script-model' not found in config.models` before sending
their requests. Exact alias priority and unknown/ambiguous identifier rejection
already pass and must remain intact. Log:
`/tmp/prov161-scripted-dispatch-red.log`.

The proposed fix resolves only unique upstream identifiers at the scripted
provider construction boundary, preserving both the manager's API ID and the
script's original alias. It does not require refactoring the oversized agent
factory or changing the public selected-model identity contract.

### GREEN: continuation integration fixes

- Scripted dispatch passes **5/5**, including real local HTTP requests for
  ordinary and named-profile selections, exact alias precedence, and rejection
  of unknown/ambiguous upstream IDs. The constructor normalizes only unique
  upstream IDs; the manager's public selected identity remains unchanged.
- Custom runtime-profile capabilities pass **1/1** after resolving the custom
  provider through its logical provider name rather than its profile-qualified
  composite prefix. Tool-layer vision and inherited image budgets are checked
  against real sessions, including a text-only control.
- The first wider session rerun exposed two obsolete RPC-343 baseline assertions
  expecting the removed universal Anthropic output ceiling of 8192. The actual
  catalog result is 64000. Returned to specifying, captured the catalog-derived
  switch/rejection example, updated the historical feature, then entered testing
  before updating its linked assertions. No production ceiling was restored.
- RPC-348 failed **0/5** because its fixture discarded the data directory before
  model switching and reused process-global persistence/configuration state.
  Repair is confined to test fixture lifetime and isolation; its five existing
  behavioral scenarios remain unchanged. Prior failure output is in
  `/tmp/prov161-custom-capabilities-green-retry.log`.

### Final continuation validation

- Provider resolution/serde/scripted dispatch, physical CLI requests, and TUI
  parity pass **23/23** after both integration fixes. Log:
  `/tmp/prov161-final-acceptance-frontdoors.log`.
- Cross-crate `cargo check` and production-library `cargo clippy` pass for
  providers, CLI, sessions, TUI, NAPI, and agent-loop after both fixes. Existing
  warnings remain, including NAPI unused results; no lint suppression was added.
  Log: `/tmp/prov161-final-quality.log`.
- All **2042** feature files pass syntax validation. The two new capability and
  scripted-dispatch features and both historical mid-session features pass
  individual tag validation. Both new features have **100% scenario coverage**
  and pass coverage audits.
- Session backend, lifecycle, historical profile RPC, custom capabilities,
  vision, and image-budget targets pass **27/27**. Log:
  `/tmp/prov161-final-session-acceptance.log`. Its initial RPC-343 execution
  still found one unchanged obsolete baseline assertion; the final retry below
  verifies the corrected test without changing production behavior.
- Both historical mid-session targets pass **10/10** after fixture repair and
  complete catalog-baseline reconciliation. Log:
  `/tmp/prov161-final-mid-session-retry.log`. A preceding compilation attempt
  caught incomplete test-helper migration; all five scenarios now use the
  retained fixture and propagate helper errors.
- Final focused total: **60 passed**, none ignored or filtered in these targets.
  Both historical coverage sidecars now reference current test/implementation
  ranges and pass audits. Targeted Rust formatting and staged/unstaged whitespace
  checks pass. No commit, branch creation, or unrelated production fix was made.
