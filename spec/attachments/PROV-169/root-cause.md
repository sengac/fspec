# PROV-169 root cause: profile models report a guessed 128k context window

## Evidence

- `rust/sessions/src/profile_sections.rs:147` — `let context_window = profile.context_window.unwrap_or(128_000);`
- `rust/sessions/src/profile_sections.rs:95-130` — `merge_profile_models(discovered, custom, context_window: u32)`
  stamps it on every discovered model and on custom models without their own value
  (`c.context_window.unwrap_or(context_window)`).
- `rust/rpc-types/src/dialogs.rs:24-31` — `ModelEntry.context_window: u32` carries it over
  `list_providers` to the TUI model selector (`rows.rs:130-135`) and to external clients
  (fspec-paseo's `contextWindowMaxTokens`), which cannot tell it from a real value.
- `rust/sessions/src/cloud_models.rs:107-114` — catalog entries copy models.dev `limit.context`, whose 0
  means unknown (the manager already filters it: providers/src/manager_runtime.rs:83-87).

## Fix

`ModelEntry.context_window: Option<u32>`. `merge_profile_models(.., context_window: Option<u32>)`;
custom models use `c.context_window.or(context_window)`; `build_local_profile_sections` passes
`profile.context_window` unchanged. Catalog entries map 0 to `None`. Custom-provider definitions
(handle_impl_5/8) publish their configured value as `Some` (their serde defaults are a separate
guess, reported). The model selector renders no size badge for `None`.

## Alternatives rejected

- **Keep `u32` and use 0 for unknown** — the sentinel PROV-165/166 removed elsewhere.
- **Probe `/v1/models` for a context length** — OpenAI-compatible servers do not report one
  consistently; out of scope.

## Wire / API changes

`ModelEntry` fields, in order: `id: String`, `display_name: String`, `context_window: Option<u32>`,
`supports_reasoning: bool`, `supports_vision: bool`, `is_custom: bool`. `context_window` is JSON
`null` when unavailable. No backward compatibility (user rule).
