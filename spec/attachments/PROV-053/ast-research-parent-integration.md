# AST Research: PROV-053 Parent Integration Work

## Goal

Wire up `CopilotProvider` (built by PROV-054, PROV-055, PROV-056) into the
runtime provider dispatch layer so the rest of fspec/codelet can actually
USE the GitHub Copilot provider end-to-end via the existing `ProviderManager`
abstraction.

## Files / Symbols Touched

### `codelet/providers/src/manager.rs`

Current `ProviderType` enum (lines 19-26):

```rust
pub enum ProviderType {
    Claude,
    OpenAI,
    Codex,
    Gemini,
    ZAI,
}
```

All 5 existing variants are wired through 11 spots that any new variant
MUST also be added to (verified by `grep -n "ProviderType::" manager.rs`):

| # | Location           | Lines     | What it does                                                |
|---|--------------------|-----------|-------------------------------------------------------------|
| 1 | enum definition    | 19-26     | The variant itself                                          |
| 2 | `FromStr::from_str`| 31-43     | String → variant ("claude", "openai", …)                    |
| 3 | `as_str`           | 48-56     | Variant → string                                            |
| 4 | `has_credentials`  | 61-69     | Variant → `credentials.has_X()`                             |
| 5 | `map_provider_id_to_type` | 334-348 | models.dev provider id → variant ("anthropic", "openai", …) |
| 6 | `detect_default_provider` | 351-375 | Priority cascade for default selection                       |
| 7 | `get_X()` method    | one per   | Per-provider constructor (`get_claude`, `get_openai`, …)   |
| 8 | `context_window`    | 537-544   | Variant → `X::CONTEXT_WINDOW`                              |
| 9 | `max_output_tokens` | 552-565   | Variant → `X::MAX_OUTPUT_TOKENS`                           |
|10 | `list_available_providers` | 491-509 | Display string per available provider                |
|11 | `for_testing` constructor | 571-584 | `credentials.X_available: false` initialiser           |

### `codelet/providers/src/credentials.rs`

Current struct (lines 9-14):

```rust
pub struct ProviderCredentials {
    pub claude_available: bool,
    pub openai_available: bool,
    pub codex_available: bool,
    pub gemini_available: bool,
    pub zai_available: bool,
}
```

Detection cascade (lines 17-30) checks env vars + auth files. For GitHub
Copilot, detection should look for the auth file at
`~/.fspec/credentials/copilot_auth.json` (created by PROV-054
`copilot::auth::write_copilot_auth`) — analogous to `has_codex_auth()`
and `read_claude_auth_sync()`.

Required additions:
- `github_copilot_available: bool` field
- `has_github_copilot()` method (mirrors `has_claude()`)
- Detection logic in `detect()` calling existing
  `crate::copilot::auth::has_copilot_credential()` (or equivalent sync check)
- `has_any()` and `available_providers()` updates
- `for_testing` initialiser updates in manager.rs and any tests that
  construct `ProviderCredentials` literally

### `codelet/providers/src/copilot/mod.rs` and `codelet/providers/src/lib.rs`

`CopilotProvider` already exists in `copilot::provider::CopilotProvider`
(PROV-055). It needs:
- to be re-exported from `lib.rs` alongside `ClaudeProvider`, `OpenAIProvider`, …
  (currently NOT in the public list)
- a `get_github_copilot(&self) -> Result<CopilotProvider, ProviderError>`
  accessor on `ProviderManager` mirroring `get_codex` (no per-call API key —
  the token comes from the auth file via `copilot::auth::read_copilot_auth_sync`)

### `codelet/providers/src/copilot/mod.rs` constants

Other providers expose `CONTEXT_WINDOW` and `MAX_OUTPUT_TOKENS` constants
at the module root (e.g. `claude::CONTEXT_WINDOW`, `openai::MAX_OUTPUT_TOKENS`).
For Copilot these are model-driven and the values come from the live
`/models` endpoint payload (PROV-056). The pragmatic compromise that does
NOT violate the "zero model details in code" rule is:
- Use neutral fallbacks for `context_window()` and `max_output_tokens()` —
  e.g. the smallest value the runtime can safely budget against (200_000 ctx,
  4_096 max-output) — until the runtime is fully model-aware.
- These constants live in `copilot::mod.rs` as
  `pub const CONTEXT_WINDOW: usize = 200_000;` etc., NOT keyed to any
  specific model.

## Test Files Already Covering This Behavior

The 8 parent scenarios in `add-github-copilot-provider.feature` are
end-to-end integration scenarios. Each one is already exercised at the
unit/integration level by the children's test files:

| Parent scenario | Covered by child test file |
|---|---|
| Login to github.com Copilot deployment via OAuth device flow | `tests/copilot_oauth_device_flow_test.rs::test_login_to_github_com_copilot_deployment_via_oauth_device_flow` |
| Login to GitHub Enterprise Copilot deployment with enterprise URL | `tests/copilot_oauth_device_flow_test.rs::test_login_to_github_enterprise_with_normalized_enterprise_url` |
| Chat completion request to gpt-4o-copilot uses /chat/completions endpoint with Copilot headers | `tests/copilot_http_middleware_routing_test.rs::scenario_chat_completion_gpt_4o_copilot_uses_chat_completions_with_required_headers` |
| gpt-5 model is routed to the /responses endpoint with reasoning_opaque round-trip | `tests/copilot_http_middleware_routing_test.rs::scenario_gpt_5_routed_to_responses_with_reasoning_opaque_roundtrip` |
| gpt-5-mini is excluded from the Responses API rule and uses /chat/completions | `tests/copilot_http_middleware_routing_test.rs::scenario_gpt_5_mini_excluded_from_responses_api_rule` |
| Image attachment triggers Copilot-Vision-Request header on claude-sonnet-4.5 | `tests/copilot_http_middleware_routing_test.rs::scenario_image_attachment_triggers_vision_request_header_on_claude_sonnet` |
| Logout deletes the github-copilot credential file | `tests/copilot_oauth_device_flow_test.rs::test_logout_deletes_the_github_copilot_credential_file` |
| Model picker fetches the live catalog from /models with no static merge | `tests/copilot_models_catalog_test.rs::scenario_each_fetch_fully_replaces_catalog_with_no_merging` + `scenario_models_flagged_picker_disabled_are_filtered_out` |

For the parent's "manager-level integration" piece, a NEW test file
`tests/copilot_provider_manager_integration_test.rs` will exercise:
- `ProviderType::from_str("github-copilot")` returns `GitHubCopilot`
- `ProviderType::GitHubCopilot.as_str() == "github-copilot"`
- `ProviderManager::with_provider("github-copilot")` succeeds when the
  copilot auth file exists
- `ProviderManager::get_github_copilot()` round-trips
- `ProviderCredentials::detect()` sets `github_copilot_available = true`
  when the auth file exists at the temp HOME

## Sequence of Implementation

1. Add `has_copilot_credential()` sync helper to `copilot::auth`.
2. Add `github_copilot_available` to `ProviderCredentials` and wire detection.
3. Add `GitHubCopilot` variant + all 11 manager.rs spots.
4. Add `get_github_copilot()` accessor.
5. Add `pub const CONTEXT_WINDOW` / `MAX_OUTPUT_TOKENS` to `copilot::mod`.
6. Re-export `CopilotProvider` from `providers/src/lib.rs`.
7. Write the 8 parent-level integration tests (link to scenarios).
8. Run `cargo test -p codelet-providers`, fix any compile errors.
9. Run clippy.
10. Move PROV-053 → validating → done.
