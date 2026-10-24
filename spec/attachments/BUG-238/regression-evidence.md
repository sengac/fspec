# BUG-238 ACDD evidence

Example Mapping captured before writing
`spec/features/deep-search-diagnostic-severity.feature`.
Feature formatted and validated before tests.

## RED (2026-10-22)

`cargo test --profile ci-test -p codelet-agent-loop --test bug238_deep_search_severity scenario_ -- --test-threads=1`
Full output: `/tmp/bug238-runtime-red.log`.

Two successful real handler searches failed specifically because their startup
diagnostics were WARN rather than DEBUG. Both shared and NAPI paths returned
`local answer` and each sent exactly one HTTP request to a local wiremock server
before the log-level assertions failed. The two genuine HTTP-400 failure scenarios
already passed and observed actionable prompt-failure warnings.

The target compiles the real NAPI handler source into the agent-loop integration
test rather than requiring a Node runtime. Its existing test submodule receives
an explicit path (test-only declaration), matching the shared implementation.
The provider configuration source is included so crate-private configuration
helpers remain accessible without changing their production visibility.

Tests use a scoped subscriber on each future, isolated FSPEC_HOME provider
files, serial_test, and restored environment state. No global subscriber changes,
external credentials, network egress, or source-string macro assertions.

## GREEN and traceability

Five routine call sites per handler now emit DEBUG: execute entry, dispatch
entry, custom-provider routing, successful construction, and completion. Build
and prompt failure WARN records are unchanged, as are timeout and streaming
failure warnings. No request selection, attribution/admission, recursion or
timeout behavior changes.

`/tmp/bug238-green.log`: all 22 tests passed, including the four new scenario
tests and 18 existing NAPI/configuration regressions (stream collection, errors,
stall/deadline behavior, and provider request configuration).

| Scenario | Canonical test in rust/agent-loop/tests/bug238_deep_search_severity.rs |
| --- | --- |
| Shared DeepSearch success stays out of incident logs | scenario_shared_deep_search_success_stays_out_of_incident_logs |
| NAPI DeepSearch success stays out of incident logs | scenario_napi_deep_search_success_stays_out_of_incident_logs |
| Shared DeepSearch failures remain actionable | scenario_shared_deep_search_failures_remain_actionable |
| NAPI DeepSearch failures remain actionable | scenario_napi_deep_search_failures_remain_actionable |

The feature has 100% scenario coverage (4/4), one canonical test file, exact
@step comments, and shared/NAPI implementation links. `fspec audit-coverage`
reports all mappings valid.

`/tmp/bug238-check.log`: cargo check passed for both codelet-agent-loop and
codelet-napi (existing NAPI warnings remain).

`/tmp/bug238-clippy-scoped-final.log`: cargo clippy passes for both selected
libraries with `--no-deps`; the seven existing NAPI warnings remain warnings.
The run is scoped to the two owned production surfaces, not unrelated workspace
dependency lint changes. Earlier attempts caught temporary compaction_builder
edits from the parallel compaction worker and the privacy worker's dependency
lint failures; no unrelated files were changed to bypass those errors.

Targeted rustfmt checks pass for both handlers and both new test files. Every
new Rust file is below 300 lines. No commits or branches were created.

## Specification validation

`/tmp/bug237-238-scoped-spec-check.log`: fspec check passes syntax, registered
tags, required categories and formatting for all three BUG-237/238 features,
using the repository's unchanged tag registry in an isolated scoped project.

`/tmp/bug237-238-global-validate.log`: all 2062 feature files pass Gherkin syntax.
Global validate-tags/check remains blocked by 527 unrelated existing feature
tag violations, including unregistered @fanout and @wire-format. These features
are outside BUG-237/238 and were not changed.

## Completion

BUG-238 is **done** through the full fspec lifecycle. The final rebuilt suite
`/tmp/bug238-final-green.log` again passes all 22 tests. The @done-tagged features
pass `/tmp/bug237-238-final-scoped-spec-check.log`; coverage audits pass.
