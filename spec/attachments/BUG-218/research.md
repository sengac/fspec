# BUG-218: RunPod model-discovery failures

## Scope and confidence

Investigated on 2026-10-21 at `7e60de8e`. Eight HTTP 404 discovery failures and the resulting unreachable classification are confirmed. The root cause is unresolved: endpoint configuration, proxy routing, API base-path composition or unsupported discovery remain possibilities. No live endpoint was contacted to retest.

## Observed evidence

The 02:48:19–04:48:19 UTC audit found eight warnings that the RunPod profile's `/v1/models` probe returned 404, including after the 14:40 local restart. One occurred at 03:11:38 UTC, near the second screenshot.

Evidence source: `~/.fspec/logs/fspec-combined.log.2026-10-21`, target `profile_sections`, message `probe_profile_models: /v1/models probe failed; marking profile unreachable`.

Do not infer from this probe alone that chat completion also failed, that the server was offline, or that a code change fixed the deployed endpoint. Avoid copying endpoint hostnames or credentials into fixtures.

## Source chain and existing contract

- `rust/providers/src/openai.rs:314`: `list_local_models_with_auth` performs authenticated model discovery. Inspect URL composition and response classification here.
- `rust/sessions/src/profile_sections.rs:613`: the profile probe calls that provider helper.
- Its error branch at line 629 returns an empty discovered list and `probe_failed = true`, marking the profile unreachable.
- `spec/features/local-profile-model-probe-resilience.feature` explicitly requires graceful unreachable degradation when probing fails or cannot run. A change that distinguishes unsupported discovery from unreachable inference needs a deliberate contract update.
- `rust/sessions/tests/prov131_local_profile_probe_resilience.rs` protects no-runtime safety; it is not proof that RunPod routing is correct.

## Reproduction and investigation plan

1. Establish the intended API base URL and whether the configured deployment supports model listing. Do not probe live infrastructure without an explicit need and suitable authorization.
2. Use a local HTTP fixture to capture composed URLs for bases with and without `/v1` and with proxy path prefixes.
3. Return discovery 404 with an independently usable configured chat model; compare the selector's state with the actual profile contract.
4. Separately test authentication errors, 5xx responses, transport failures, empty model lists and no-runtime invocation.

## Recommended outcome and open questions

If the configured base path is wrong, provide actionable validation/remediation rather than arbitrary fallback URLs. If discovery is optional for a known custom model, define whether it should remain selectable with a distinct discovery-unavailable status. Do not mark failed probes healthy or silently suppress errors.

Acceptance must preserve the no-panic behavior of PROV-131 and the profile selector's existing credential and reachability guarantees. This card is an investigation, not a confirmed claim that inference is currently broken.
