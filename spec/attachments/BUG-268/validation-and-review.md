# BUG-268 validation and direct review

## Acceptance, approvals and chronology

The immutable PROV-055/057 next-distinct-genuine-turn requirement remains intact.
`session-lineage-correction.md` supersedes only the earlier model-owned clearing
clause in `composition-contract.md`. Three actual native design answers approve
typed Chat/Responses composition, existing request-time refresh coordination and
the nonpersistent live ProviderSession extension. Synthetic models are protocol
fixtures, not a worker-model change or evidence of live Copilot model support.

Preserve all earlier causal failures: actual URI/body routing, nested-image
classification, two separate genuine `send_input` turns, refreshed Authorization,
deployment separation, send-time origin, fork/restore independence, SSE retry
metadata and nonstreaming terminal failures. Compiler errors and invalid fixture
attempts remain historical, not causal runtime RED or final GREEN.

The fourth matrix was **26 passed / 1 failed**, not final GREEN. Its remaining
Codex assertion expected `None` after `transition_generation(17)`. The isolated
`codex-initial-state-diagnostic.log` fails that expectation BEFORE registration
or any Copilot operation. The real Codex method intentionally creates `Some`
checkpoint. The repaired fixture installs a valid selected-model generation 17,
records its complete checkpoint/status before Copilot, and asserts exact
checkpoint, status and generation preservation afterward for both read-only
lookup and actual provider switches. No Codex reset or production behavior was
changed to satisfy this test. Failed evidence is retained.

## Runtime and shipping evidence

All Cargo commands use offline resolution, explicit packages/targets, `ci-test`,
jobs 2 and command-scoped `CARGO_PROFILE_CI_TEST_DEBUG=0`. Runtime settings and
debug assertions are unchanged. The strict 6 GiB free-space floor is retained.

Final source is validated by these logs (read their actual outcomes, not earlier
matrices, when evaluating completion):

- `sse-metadata-settled-green.log`: exact SSE capture unit test; other provider
  unit tests are filtered and are NOT claimed as passing.
- `issuer-isolation-settled-green.log`: all six isolation prerequisites, before
  full expired-token fixtures. Numeric-loopback destination, redirects, inherited
  proxy, canonical temporary credential root and returned inference origin.
- `settled-source-runtime-and-controls.log`: four genuine-session tests, five
  outbound/front-door tests, five lineage-boundary tests, three nonstreaming
  terminal tests, one send-time-origin test and three refresh tests; plus the
  existing 6 rig-agent, 17 middleware/routing, 7 initiator and 2 heuristic tests.
  Total expected scope is 21 acceptance tests plus 32 legacy controls in this
  log, and 28 distinct acceptance tests including the seven prerequisite/unit
  tests above. No filtered or failed test is counted as a pass.
- `settled-shipping-check.log`: default shipping-library compilation of providers,
  agent-loop, CLI, sessions and NAPI without the test-support feature.
- `settled-clippy.log`: the same scoped shipping consumers. The newly introduced
  large-enum warning is repaired with a boxed Responses payload, with no wire
  serialization or completion policy change. Unrelated NAPI dead-code and unused
  Result warnings, and shared fixture dead-code warnings, remain disclosed.

The genuine-turn matrix exercises nine history/provenance/provider changes,
ordinary tool identity and armed Done, plus independence of the original live
session after a restored fork. Refresh fixtures assert actual captured headers,
issuer coalescing, persisted synthetic tokens, endpoint refresh and zero inference
after rejected/cancelled exchange. A cancelled task is observed after issuer
admission rather than inferred from a startup timeout. No external issuer,
provider inference, installed user script or real credential is used.

## Traceability and native gates

The native transition initially rejected multiple test files per feature. This
was not bypassed. The same 28 scenarios and assertions were regrouped into eight
features, each mapped to its existing exact test file. No acceptance or test was
removed, and the original PROV-055/057 scenarios remain unchanged. The pre-split
native coverage reference is retained. Native coverage generation ran in a
disposable project containing ONLY these eight features, preserving existing
mappings; its generated sidecars were installed and linked with native commands.
No coverage JSON or work-unit status was hand-edited. A global preview found two
unrelated missing MCP sidecars; neither was created or modified.

`scoped-split-coverage-generation.log`, `scoped-split-coverage-links.log` and
`split-specifications-final-validation.log` retain generation, linking, scoped
formatting, Gherkin validation and coverage audits. All eight feature-level tag
checks pass through the native dispatcher `validate-tags` with an explicit file.
The standalone CLI rejects `--file` for this command; that failed invocation is
retained and not claimed as a tag pass. The known unrelated global tag failures
are not repaired or claimed green. `final-format.log` records focused formatting
and whitespace checks. Native validating transition succeeds after the split.

`native-review-context.log` reports no native findings, but its automatic summary
selects only one five-scenario feature. It is NOT evidence that the other 23
scenarios were automatically reviewed. Explicit coverage totals and direct source
review cover the complete scope instead.

## Direct source review

Reviewed actual typed routing and sanitization, both nonstreaming front doors,
stream event/tool/usage forwarding, successful terminal-only opaque capture,
exact preamble/history/tool-ID ancestry, logical-account/deployment/model binding,
actual send-time auth/origin, attempt epoch/sequence checks, cancellation and late
settlement, writer/checkpoint/serialization exclusion, real provider selection,
and construction-time fork owner detachment. Existing adapters, token coordinator
and public middleware remain the source of truth. Copilot detachment alone is
added; generic Clone and Codex policies are unchanged. New Copilot source files
remain below 300 lines. No inference retries, refusal bypass, durable opaque
adoption, new global/session-ID map, security-policy expansion or live-support
claim is introduced. Native Done is a separate final transition, not implied by
this document or a passing test log.
