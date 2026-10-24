# BUG-238: DeepSearch diagnostic severity

Research date: 2026-10-22. HITL-approved logging cleanup, not a DeepSearch execution
failure or provider-routing redesign. Source references describe the triage snapshot.

## Context and impact

Normal DeepSearch entry points unconditionally emit WARN records. These inflate error
inventories, make successful work look unhealthy, and obscure real failures. Both the
shared agent-loop implementation and NAPI counterpart contain the pattern.

## Incident evidence

- Log: `~/.fspec/logs/fspec-combined.log.2026-10-21:773` and `:774`.
- UTC time: 23:56:09.
- Messages: `[deep-search] execute_deep_search ENTER` and
  `[deep-search-dispatch] ENTER`.
- The messages report provider/model, ephemeral session, project, recursion, and graph
  availability; they are emitted without an error predicate.
- Normal entry does not establish whether the subsequent search ultimately succeeds.

## Code and parity surface

- `rust/agent-loop/src/deep_search_handler.rs:139` and
  `rust/agent-loop/src/deep_search_handler.rs:485`: unconditional entry warnings.
- `rust/napi/src/deep_search_handler.rs:139` and
  `rust/napi/src/deep_search_handler.rs:480`: corresponding NAPI entry warnings.
- `rust/agent-loop/src/deep_search_handler.rs:516` and
  `rust/agent-loop/src/deep_search_handler.rs:538`: routine custom-provider routing
  and successful construction also use WARN; inspect the NAPI counterparts nearby.
- `spec/features/deep-search.feature:1` supplies the existing behavior contract.

Related completed work: RLM-001, RLM-002, and AMGR-016. Their completed status does
not make unconditional WARN severity appropriate. BUG-239 owns payload/privacy policy;
this card should not duplicate that broader work.

## Confirmed cause

These records use `tracing::warn!` on normal control-flow paths. The severity mismatch
is source-confirmed; no provider outage, tool-admission failure, or recursion bug is
needed to explain these particular warnings.

## Expected outcomes

- Assign normal entry/routing/success diagnostics an appropriate DEBUG or INFO level,
  with a consistent rationale across shared and NAPI routes.
- Keep genuine failures, timeouts, unexpected fallbacks, and actionable anomalies at
  appropriate warning/error severity; do not globally suppress the module.
- Retain concise provider/model/session correlation useful during investigation,
  subject to the payload/privacy policy in BUG-239.
- Preserve execution, model selection, recursion limits, timeout policy, and tool
  attribution/admission exactly; logging changes must not alter behavior.
- Normal DeepSearch startup must no longer appear in a WARN/ERROR-only incident view.

## Regression evidence to produce

Capture logs from representative successful entry/dispatch paths and assert that
routine messages are not WARN/ERROR. Exercise genuine failure paths to show useful
warnings/errors remain visible. Check shared/NAPI parity without relying solely on
searching the source for macro names.

Implementation and executable regression evidence are recorded in
[regression-evidence.md](regression-evidence.md).

Use a scoped test subscriber or the repository's existing logging-test pattern;
avoid global subscriber changes that leak into other tests. Reuse existing DeepSearch
test fixtures and provider seams rather than requiring live external services.

## Scope limits and completion

Do not change provider selection, introduce new retries, silence real failures, or
rewrite DeepSearch. Completion requires scenario-linked log-level assertions, scoped
quality checks, and evidence that WARN/ERROR inventory now reflects abnormal events.
