# PROV-152 — Codex capability-aware request options

## Goal and evidence

Construct Codex requests from model capabilities without panics or silently losing supported knobs.
Audit: fspec `38a95522`; upstream https://github.com/openai/codex `bb64b89`.
The current shared AdditionalParameters conversion uses expect; effort supports none/minimal/low/
medium/high only. reasoning.context, text verbosity and summary-delivery stream options are absent.
Official `codex-rs/core/src/client.rs` resolves model-specific reasoning and optional settings;
`codex-rs/codex-api/src/common.rs` supports additional fields and custom numeric effort encoding.

## Codex-only boundary

Depends on PROV-147. Build typed options and validation in the Codex adapter; do not widen shared
OpenAI enums or change other providers' defaults to make Codex values fit. Narrow shared routing
must be explicitly selected by the Codex provider, including profiles and subordinate sessions.

## Acceptance criteria

1. Supported model effort levels, including xhigh or custom numeric levels where advertised, serialize correctly.
2. Invalid or unsupported user options return actionable typed errors, never panic or silently downgrade.
3. Reasoning context, summary, verbosity, service tier and sequential-cutoff delivery are capability-gated.
4. Missing or stale capability data has a documented conservative fallback and does not enable experimental options blindly.
5. Explicit user configuration and model defaults have deterministic precedence with snapshot-tested requests.
6. Non-Codex requests and configuration behavior are unchanged.

## Test examples and implementation notes

Test supported and unsupported effort values, a model without summaries, verbosity-only text
configuration, custom numeric serialization, and capability refresh. Preserve store:false and
required includes for the Codex endpoint. Do not hardcode speculative model availability.
Choose whether an unsupported optional default is omitted versus an explicit user request rejected,
and document that distinction in the feature before implementation.

PROV-150 consumes reasoning delivery options; cache keys are PROV-155. Native tool capabilities
are PROV-157/158. Header/auth credentials are not additional model parameters.

## Validation and exclusions

Define Gherkin, write failing tests and property-test option parsing with no panic for arbitrary
JSON. Use shared test fixtures and scoped Cargo targets. This is not a global model catalog rewrite.
