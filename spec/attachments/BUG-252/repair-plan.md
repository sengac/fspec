# BUG-252: Distinguish discovery capability from inference availability

## Priority and evidence

**P2 — observed misleading discovery classification; historical runtime details are incomplete.**
The reviewed overnight log contains nine Runpod `/v1/models` HTTP 404 warnings marked
unreachable, and four separate Spark connection/send failures. The latter are genuine
availability observations from that process, but the log does not establish server-down
versus network/routing cause. Neither cluster is evidence of conversation-store corruption.

`rust/providers/src/openai.rs:356` classifies discovery 404 as unsupported discovery.
`rust/sessions/src/profile_sections.rs:634` uses a generic unreachable branch.
**Current code already exempts unsupported discovery when a nonblank custom model ID is
configured at line 620.** Do not implement a duplicate exemption or claim that behavior is absent.
Historical custom-model configuration and exact running executable identity were not established.

## Required repair

1. Reproduce classification using deterministic local endpoints and inspect current profile/model
   configuration. Record runtime/version/configuration provenance without exporting credentials.
2. Separate discovery capability/status from inference availability: supported discovery, unsupported
   discovery, failed authentication, failed transport, and unknown inference availability are not the same.
3. Preserve usable explicitly configured models under the existing agreed unsupported-discovery contract.
   A discovery 404 alone must not assert that actual inference is unavailable or invent discovered models.
4. Keep genuine connection/auth/server failures visible with concise, sanitized reasons and deliberate
   retry behavior. Do not turn every failure into an optimistic available state.
5. Apply the resolved classification consistently to provider enumeration, settings, selection and logs.
   Existing serial discovery probes and their latency may be measured, but async/concurrency refactoring
   is not required to close this semantic card unless shown necessary.
6. Clarify model-list-empty versus discovery-unsupported behavior and preserve custom-model persistence
   and selection across startup/model switching.

## Regression matrix

| Endpoint/profile condition | Expected contract |
| --- | --- |
| `/v1/models` 404, nonblank custom model | Existing configured-model exemption remains usable; discovery explicitly unsupported |
| `/v1/models` 404, no configured models | No fabricated models; inference availability unknown rather than proven unreachable |
| `/v1/models` 200, valid model list | Normal discovery and selection remain intact |
| `/v1/models` 200, empty list | Explicit empty-discovery result, not a transport failure |
| Connection refusal/timeout | Real transport failure remains visible and scoped to the endpoint |
| 401/403 or malformed response | Correct authentication/protocol failure; no silent custom-model bypass |

Add a cold-start/switch test using a local endpoint whose inference succeeds despite unsupported
discovery. Verify persisted custom model IDs are not lost. Exercise settings and diagnostic consumers,
not only the lower-level HTTP parser, and ensure secrets/provider payloads do not appear in warnings.

## Non-goals and constraints

- Do not claim the fix repairs Spark connectivity; endpoint reachability may require operational work.
- Do not suppress errors, implement transparent transport downgrades, or infer arbitrary model capacities.
- Session-count filtering diagnostics are a separate misleading-log observation, not evidence that
  manifests were deleted and not necessary scope for this card.
- Validate current behavior before production changes; this card may require contract clarification
  plus a focused classifier fix rather than a large provider-settings redesign.

## Completion evidence

- [ ] Current exemption is regression-tested; historical uncertainty is not presented as proven regression.
- [ ] Discovery, availability, authentication and empty-model states are explicit across consumers.
- [ ] Configured-model selection works when inference is valid but discovery unsupported.
- [ ] Feature-first tests, scoped quality checks, and payload-free diagnostics are documented.

Related: existing provider-profile/discovery specifications and BUG-239 privacy requirements.
Investigation: `docs/investigations/2026-10-23-compaction-and-resume.md`.
