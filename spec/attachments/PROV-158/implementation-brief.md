# PROV-158 — Codex native output and compaction compatibility

## Goal and evidence

Handle negotiated native Codex output items explicitly instead of losing a response because an
unsupported item cannot deserialize. Audit: fspec `38a95522`, upstream https://github.com/openai/codex `bb64b89`.
Upstream protocol/src/models.rs includes hosted web search, tool search, image generation,
compaction/context-compaction and additional item families. Our rig Output enum has only message,
function call and reasoning. This card is capability-gated, not a promise to enable every upstream tool.

## Codex-only boundary

Depends on PROV-147, PROV-148, PROV-149 and PROV-152. Extend only the Codex-owned item model and
adapter. Existing fspec local tools and generic compaction remain unchanged. Any Codex-specific
compaction handoff must be opt-in and must not alter other providers' history management.

## Acceptance criteria

1. Each advertised native capability has a defined item parser, presentation policy and continuation policy.
2. Hosted execution items are not dispatched again as local tools and their results/status remain attributable.
3. Supported compaction items preserve opaque protocol state and replace history only under explicit lifecycle rules.
4. Unsupported known capability items produce actionable compatibility diagnostics rather than silent success.
5. Unknown optional metadata does not invalidate otherwise usable text, and unknown executable items never execute.
6. Capability-disabled sessions and every non-Codex provider retain existing behavior.

## Test examples and implementation notes

During specifying, produce an explicit supported/deferred matrix for native item families; split
additional executor/product work into child cards if large. Test hosted search progress/completion,
an opaque compaction result followed by continuation, an unsupported item inside response output,
and session reload after compaction. Use synthetic payloads and no external service credentials.
Native tool availability, pricing, permissions and output rendering require capability checks;
do not infer support from model name or expose encrypted fields in the UI.

## Validation and exclusions

Gherkin first, failing parse/roundtrip/local-server tests next. Use shared helpers, scoped Cargo
targets and non-Codex compaction controls. Existing local WebSearch and image tooling are not
replaced. Global compaction redesign, enabling remote tools by default and unrelated audio/realtime
features are outside scope. Revalidate upstream wire types before implementation.
