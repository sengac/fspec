# PROV-157 — Codex custom tools and namespaces

## Goal and evidence

Support the optional Codex custom/freeform tool dialect and structured namespaces without breaking
ordinary JSON function calls. Audit: fspec `38a95522`, upstream https://github.com/openai/codex `bb64b89`.
Upstream tools/src/tool_spec.rs and tools/src/responses_api.rs define function/custom/namespace
specifications. protocol/src/models.rs distinguishes custom raw input from JSON function arguments.
Our Responses tool conversion always emits function and the output enum has no custom-call variant.

## Codex-only boundary

Depends on PROV-147, PROV-149, PROV-151 and PROV-152. Put dialect negotiation, definitions and
dispatch adaptation in Codex-specific modules/facades. Shared tool implementations keep their
existing interface; any alternate adapter must be selected only for Codex.

## Acceptance criteria

1. Capability-enabled Codex tools can advertise supported custom raw-input and grammar definitions.
2. Custom input remains raw text and is not mistakenly parsed as ordinary JSON function arguments.
3. Structured namespace qualification survives request registration, streaming, dispatch and result matching.
4. Custom input deltas use the Codex stream bridge and incomplete inputs cannot execute.
5. Existing permissions, isolation and approval hooks apply equally to custom and function calls.
6. Ordinary function tools remain available and all non-Codex tool definitions and dispatch remain unchanged.

## Test examples and implementation notes

Test a raw patch-like input, Unicode and chunk boundaries, a grammar definition, two tools with
the same local name in different namespaces, invalid namespace and matched custom result IDs.
Do not advertise a custom dialect without an implemented safe executor; decide the initial supported
tool allowlist during specifying. Native hosted tools are PROV-158, not local custom executors.

## Validation and exclusions

Write Gherkin and failing local fixtures before code, including authorization-denied scenarios.
Use shared helpers, scoped Cargo tests and wire snapshots for ordinary functions as controls.
No automatic conversion of every fspec tool, no sandbox bypass, no global tool registry rewrite.
