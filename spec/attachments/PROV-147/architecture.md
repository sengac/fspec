# PROV-147 architecture decision

## Ownership and activation

Use composition: `codex::CodexResponsesModel` implements rig's `CompletionModel`
and owns an existing Responses model. Codex constructors and `create_rig_agent`
both use this wrapper. The existing provider-resolution path selects the provider;
there is no model-name test, global switch, or modification to OpenAI defaults.
The approved mechanical split keeps Codex construction, agent tools, completions,
and existing unit tests in separate small modules.

## Baseline and future seams

For this card completion and stream calls delegate unchanged, including raw
response types, errors and token usage. The existing refreshing HTTP client still
owns OAuth refresh and endpoint rewriting. Generic rig history, retry and frontend
paths are untouched. This is an ownership boundary, not a claim of protocol parity.

Later cards may replace Codex request/event types inside this wrapper. Typed
Codex history and events should be translated at explicitly Codex-selected
integration seams into existing agent/frontend interfaces; if new shared plumbing
is necessary, it must be default-off and carry a non-Codex regression test.
Do not copy the generic agent or change its global defaults.

## Verification

Production-agent type checks cover identical names and non-Codex model spelling.
Local HTTP fixtures compare requests and stream/direct results against the existing
Responses model using deterministic payloads, without credentials or external
network access. Shared history/retry code is unchanged, so its execution remains
the same; later behavioral changes require dedicated regression controls.
