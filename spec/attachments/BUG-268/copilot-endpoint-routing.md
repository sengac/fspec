# BUG-268: Copilot endpoint helper is disconnected from runtime

## Finding and history

Source-confirmed, longstanding since `483f50b4`, not a proven October regression.
The selector advertises Responses routing for some models, but the native
provider and agent always construct Chat Completions.

## Evidence

- `rust/providers/src/copilot/endpoint.rs:75`: endpoint selector policy.
- `rust/providers/src/copilot/provider.rs:127`: CompletionsClient construction;
  line 141 selects the Chat model.
- `rust/providers/src/copilot/rig_agent.rs:61`: Chat-specialized agent type.
- `rust/providers/src/copilot/refreshing_client.rs:209`: middleware injects
  headers/cache state rather than converting endpoint/request dialect.
- `rust/providers/tests/copilot_http_middleware_routing_test.rs:97`: selector
  plus formatted-URL test, not actual provider HTTP evidence.

## Acceptance/test direction

First verify the supported Copilot model/API contract. Make actual requests
agree with the approved selector, including Chat exceptions/fallbacks. Capture
real provider HTTP and assert URI, request dialect, tools and opaque reasoning
round trips. Run an armed stop -> nudge -> Done flow for the selected route.
Test credential refresh and native tool-result pairing across follow-ups.

Use existing shared adapters rather than another business-logic implementation.
Related contracts: PROV-055 and the generic Responses terminal fix BUG-264.
See [full audit](../CONT-011/fspec-auto-continue-investigation.md).
