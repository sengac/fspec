# PROV-161: Shared provider settings and models.dev token limits

Date: 2026-10-21
Status: Read-only investigation and implementation proposal; implementation has not started.

## Requested outcome

Expose the same configuration system currently available for OpenAI API profiles
for all providers, particularly Codex and Anthropic, including custom providers
and facades. Include context-window overrides and default both context capacity
and maximum output tokens from the matching models.dev metadata.

The shared settings include:

- Streaming preference.
- Auto-continue enablement and budget.
- Thinking preservation.
- Maximum images.
- Loop detection enablement, window, repeat threshold, and retry budget.
- Compaction threshold.
- Context-window and maximum-output-token overrides.

Connection and authentication fields remain provider-specific. Configuration
parity must not imply that every API accepts identical request parameters.

## Executive summary

This is an end-to-end configuration change, not just a form-rendering change.
The existing profile form and numeric limit resolver are useful foundations,
but OpenAI-only assumptions exist in navigation, persistence, profile discovery,
session initialization, and environment bridging. Some existing profile defaults
are persisted without being consistently consumed at runtime.

Recommended approach: generalize the existing settings system, resolve it once
for the selected provider/profile/model, and pass a typed resolved configuration
to request builders, session behavior, tools, and budgeting. Preserve existing
OpenAI profiles and provider-sensitive defaults.

## Provider and facade inventory

The 17-entry canonical catalog is display/authentication metadata, not 17
independent runtime implementations. Runtime backends include Claude/Anthropic,
OpenAI-compatible Chat, Codex Responses, Gemini, ZAI, Copilot, and custom providers.

Custom providers can explicitly select a facade, derive an OpenAI or Anthropic
facade from their API style, or execute through Rhai when a script is present.
Settings must belong to the logical provider; facade choice determines protocol
behavior, not whose configuration is loaded.

References:

- `rust/providers/src/catalog.rs:49`: canonical provider catalog.
- `rust/providers/src/manager.rs:22`: runtime provider types.
- `rust/providers/src/custom/management.rs:441`: facade derivation.

## Confirmed current-state findings

### UI and persistence are OpenAI-only

- `rust/fspec-tui/src/views/provider_settings/profile_form.rs:39` defines the
  existing shared-looking form fields, including context and output overrides.
- `rust/fspec-tui/src/views/provider_settings/nav_item.rs:149` emits profile and
  add-profile rows only for OpenAI.
- `rust/fspec-tui/src/app/dispatch_provider_settings_profiles.rs:31` reloads only
  OpenAI profiles and their definitions.
- `rust/fspec-tui/src/views/provider_settings/nav_tree_ops.rs:63` stores form
  definitions by profile name alone. Generalization requires provider-qualified
  identity to avoid same-name collisions.
- `rust/fspec-tui/src/views/provider_settings/profile_form_parse.rs:135` requires
  a base URL, API key, and name. This is inappropriate for inherited OAuth auth.
- `rust/sessions/src/profile_persistence.rs:92` explicitly allows only OpenAI.
- `rust/sessions/src/profile_sections.rs:304` loads only OpenAI profiles; profile
  model sections also assume OpenAI-compatible model discovery.

The frontend currently reads configuration files locally. A generalized backend
read surface would keep remote and embedded clients consistent with writes and
avoid loading settings from a different machine or project directory.

### Runtime settings have different application paths

- Streaming is bridged through `OPENAI_STREAMING` and consumed by the OpenAI
  model's streaming flag. Other providers do not share that settings path.
  References: `rust/sessions/src/model_resolution.rs:389` and
  `rust/providers/src/openai.rs:211`.
- Auto-continue is seeded from OpenAI profile configuration before the first
  message. It is not a general provider-default resolver and is creation-time
  behavior. Reference: `rust/sessions/src/session_creation_helper.rs:264`.
- Thinking preservation is also initialized from profile configuration.
  Non-profile sessions intentionally preserve historical provider behavior,
  including signed Anthropic/Gemini reasoning. Reference:
  `rust/sessions/src/session_creation_helper.rs:193`.
- Built-in dispatch passes the preservation flag, but the Rhai dispatch needs
  parity review. References: `rust/agent-loop/src/dispatch.rs:96` and
  `rust/agent-loop/src/agent_loop.rs:1393`.
- Image and loop settings resolve through OpenAI-profile lookups by profile name
  rather than provider-qualified identity. References:
  `rust/sessions/src/model_resolution.rs:83` and
  `rust/sessions/src/model_resolution.rs:134`.

### Some persisted defaults are not consistently used

`rust/sessions/src/profile_model_limits.rs:33` reads compaction configuration
from the matching custom-model entry rather than falling back to the profile's
compaction setting. At `rust/sessions/src/profile_model_limits.rs:54`, maximum
output is likewise read only from the custom model. Context already has a
custom-model-to-profile fallback at line 43.

The generalized resolver should address these inconsistencies as part of making
the existing form's settings effective, not reproduce them for other providers.

### Catalog infrastructure already contains both token limits

- `rust/providers/src/models/types.rs:159` represents `limit.context` and
  `limit.output`.
- `rust/providers/src/models/cache.rs:129` fetches and caches models.dev data;
  existing cache and refresh behavior should be reused.
- `rust/providers/src/model_limits.rs:77` implements explicit override, registry
  value, then provider fallback, with optional provider ceilings.
- `rust/providers/src/manager.rs:494` records both catalog limits for ordinary
  registry selection.
- `rust/providers/src/manager.rs:617` clears catalog data during direct/profile
  selection. Unprofiled Codex subsequently looks up an exact model in the OpenAI
  catalog namespace; other direct/profile paths do not consistently do so.
- `rust/providers/src/models/registry.rs:88` tries exact keys, then prefix matches
  while iterating a HashMap. Metadata matching must avoid ambiguous matches when
  assigning authoritative limits.
- `rust/sessions/src/cloud_models.rs:41` contains provider aliases used for model
  listing. Default resolution must use consistent aliases too.
- `rust/rpc-types/src/lib.rs:486` defines model-list entries without an output
  limit field. Showing both catalog limits in the editor may require extending
  that metadata surface or adding a resolved-settings response.

### Budgeting and physical generation limits need alignment

`rust/cli/src/interactive/stream_loop.rs:254` passes manager-resolved context and
output values to admission and compaction. However,
`rust/core/src/selected_request.rs:13` creates its request template from the
agent builder, and `rust/providers/src/selected_request.rs:63` replaces history
without replacing the builder's output limit.

Provider builders can therefore retain a constant or environment-derived output
limit while admission uses a different resolved budget. Serializer tests alone
do not prove manager-to-builder propagation. The implementation must verify the
actual request bytes as well as the resolved configuration and UI display.

## Proposed configuration model

1. Reuse one optional settings schema for provider defaults and named profiles.
2. Add a default Settings entry for every provider and reuse the editor for
   named profiles.
3. Separate runtime options from connection/authentication configuration. Codex
   and Anthropic profiles should be able to inherit their existing credentials.
4. Resolve by provider, optional profile, and model through the common selection
   path used by creation, switching, and resume.
5. Pass resolved configuration explicitly rather than extending process-global
   environment variables as the transport for session settings.
6. Keep embedded, server, CLI, and remaining NAPI consumers on the same backend
   resolution and persistence behavior.
7. Preserve existing OpenAI configuration and unrelated keys during writes.

Proposed precedence for each token limit independently:

```text
Explicit session override
  > model-specific override
  > selected profile override
  > provider default override
  > matching models.dev limit.context / limit.output
  > backend fallback when metadata is unavailable
```

Apply genuine model/endpoint restrictions separately. Distinguish explicit
custom/script-supplied limits from defaults injected during deserialization so
fallback constants do not accidentally defeat catalog metadata.

### Editor behavior

- An empty context or output field means Automatic.
- Show the effective value and its source, such as models.dev or an override.
- Entering a number creates an override; clearing it restores automatic lookup.
- Do not persist automatic catalog values as explicit overrides.
- A provider-wide editor must not imply one catalog value applies to every
  model: effective catalog defaults are model-specific.
- Validate numeric values and explain unavailable metadata or real endpoint
  restrictions rather than silently ignoring an override.
- Keep the session header, accounting, compaction, admission, and physical
  request parameters consistent with the resolved settings.

## Provider-specific safeguards

### Anthropic

`rust/providers/src/claude.rs:180` and the manager's corresponding resolver impose
blanket context/output ceilings of 200000 and 8192. Exposing overrides alone
would still clamp larger models.dev values. Review these historical ceilings
against actual model/endpoint capabilities and update the corresponding specs
and tests, rather than treating fallback constants as universal API limits.

Preserve signed-reasoning requirements and provider-sensitive defaults when
exposing thinking preservation.

### Codex

`rust/providers/src/codex/mod.rs:136` declares that its backend rejects the
`max_output_tokens` API parameter. Resolve output capacity for admission and
compaction without introducing a forbidden wire parameter.

Keep catalog identity distinct from transport identity: current unprofiled
Codex selection uses OpenAI catalog metadata. Verify the profile/facade path
resolves equivalent model metadata without changing authentication or routing.

Streaming preference must be capability-aware. Where an endpoint requires
streaming, buffering display is different from disabling streaming transport;
do not present an unsupported transport toggle as functional.

### Custom providers and facades

Respect explicit facade selection, Rhai execution, real API model IDs, and
configured provider limits. Providers using the same facade must not share
mutable settings accidentally. Unknown/private models need usable explicit
overrides and an honest fallback or unavailable-metadata indication.

## Suggested implementation workstreams

1. Shared schema, provider defaults, profile persistence, qualified identity,
   backend reads, and compatibility with existing OpenAI configuration.
2. Unified catalog matching and independent context/output resolution, including
   custom/script limits and the Anthropic ceiling review.
3. Runtime propagation into request builders, streaming, auto-continue, thinking,
   image budgets, loop detection, admission, and compaction.
4. Shared provider-settings UI with provider-aware connection fields and
   automatic-value/source display.
5. Lifecycle and regression coverage across creation, switching, resume,
   subordinate inheritance, and actual HTTP request serialization.

Planning estimate: approximately one to two engineer-weeks for complete parity
and regression coverage. This is a rough estimate, not a delivery commitment;
adapter behavior and request-limit propagation are the largest uncertainties.

## Acceptance examples and validation plan

- A Codex user can configure auto-continue and context/output overrides without
  entering an API key or replacing OAuth credentials.
- Anthropic with no explicit limits resolves matching models.dev context and
  output metadata, subject only to genuine endpoint restrictions.
- Overriding context leaves output automatic, and vice versa; clearing either
  override restores its catalog default.
- Same-named profiles under Codex and Anthropic remain independent.
- Custom providers using OpenAI or Anthropic facades use their own settings.
- Switching providers does not leak the previous provider's settings; explicit
  session behavior overrides have documented seed-versus-live semantics.
- Resume and subordinate agents receive the intended resolved settings.
- Existing OpenAI profiles remain functional without manual migration.
- Missing/offline/unknown catalog data has deterministic fallback behavior;
  automatic values are never silently stored as permanent overrides.
- Physical requests use the configured output limit on supported APIs; Codex
  omits forbidden output-limit parameters while budgeting uses the resolved value.

Follow ACDD: feature scenarios first, corresponding failing tests second, then
implementation. Reuse shared filesystem fixtures and offline catalog/HTTP test
seams. Run package-and-target-scoped tests, not an unscoped workspace test run.

Useful regression foundations:

- `rust/providers/tests/cmpct071_builtin_output_limits.rs:1`.
- `rust/agent-loop/tests/bug215_effective_compaction_limits.rs:1`.
- `spec/features/provider-config-profile-persistence.feature:1`.
- `spec/features/rust-authoritative-context-window.feature:1`.
- `spec/features/modellimitsresolver-trait-provider-veto-authority.feature:1`.

## Decisions to finalize during specification

- Exact provider-default storage shape and profile inheritance semantics.
- Which settings reapply on switching versus only seed new sessions, and how
  explicit session overrides interact with those defaults.
- Whether unsupported streaming modes are disabled with an explanation or
  represented as a separate buffered-display option.
- Precise behavior when catalog metadata is missing and no override exists.
- Model/endpoint-aware replacements for historical Anthropic ceilings.

No production code changed and no tests ran during this investigation.
