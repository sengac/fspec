# RPC-107 AST Research — Canonical Provider Catalog (Rust Port)

## Goal of the research
Identify the precise call sites in the Rust workspace that must change to introduce a static `CANONICAL_PROVIDERS` slice and rewrite `list_providers_info` to iterate it (canonical order first, customs appended).

## Search 1 — Existing `list_providers_info()` definition (Rust)

Pattern (AstGrep, language=rust):
```
pub fn list_providers_info() -> Result<Vec<ProviderInfo>, ProviderError> { $$$BODY }
```

Result:
- `codelet/providers/src/custom/management.rs:95` — sole definition; iterates a hard-coded 6-tuple `("claude", "openai", "gemini", "zai", "codex", "github-copilot")` and builds `ProviderInfo { name, display_name: Some(name.to_string()), available, is_custom: false, ... }` then appends `discover_provider_configs()` entries.

Gap vs TS canon:
- Only 6 built-ins vs 17.
- `display_name` is the slug, not the TS-canonical human string.
- Rust uses slug `"claude"`, TS canon uses `"anthropic"`.

## Search 2 — Wire-side consumer

Pattern (AstGrep, language=rust):
```
fn list_provider_credentials(&self) -> Vec<ProviderCredentialInfo> { $$$BODY }
```

Result:
- `codelet/sessions/src/handle_impl.rs:869` — only impl; maps `ProviderInfo` → `ProviderCredentialInfo { provider_id, display_name, configured, credential_type, model_count }` and decides `credential_type` via `match p.name.as_str() { "codex" | "github-copilot" => "oauth", _ => "api_key" }`.

Implication for RPC-107: after the rewrite, this `match` arm must recognise `"anthropic"` as another `oauth` provider (matrix is the responsibility of a sibling card; RPC-107 only needs the catalog order/display).

## Search 3 — `ProviderInfo` struct shape

Pattern (AstGrep, language=rust):
```
pub struct ProviderInfo { $$$FIELDS }
```

Result:
- `codelet/providers/src/models/types.rs:17` — model-level info (provider id + model list); unrelated.
- `codelet/providers/src/custom/management.rs:57` — the `ProviderInfo` consumed by `list_providers_info`. Fields: `name`, `display_name: Option<String>`, `available`, `is_custom`, `facade`, `base_url`, `api_key_env_var`, `models`, `api_style`.

The struct surface is sufficient — no field changes required. We will populate `display_name: Some(catalog.display_name.to_string())` for built-ins.

## Search 4 — Detection of currently-supported provider env vars

Pattern (Grep, content):
```
has_claude\(|"claude"
```

Hits inside `codelet/providers/`:
- `credentials.rs` — `ProviderCredentials { claude_available, openai_available, codex_available, gemini_available, zai_available, github_copilot_available }`. Only 6 fields → cannot drive 17-provider configured status without extension. **However**, RPC-107 only requires the catalog + order + display name. The `configured` flag for the 11 net-new providers can default to `false` until a sibling card extends `ProviderCredentials::detect()` to probe their env vars (out of scope per attachment §“Out of scope”).

## Search 5 — Cross-transport test surface

Pattern (Grep, content):
```
list_provider_credentials
```

Result (relevant):
- `codelet/fspec-tui/tests/rpc054_cross_transport_parity.rs` — already exercises `set_provider_credentials` / `delete_provider_credentials` parity. Will extend with a `list_provider_credentials` parity test for RPC-107.

## Files to change (locked from research)

1. **NEW** `codelet/providers/src/catalog.rs` — `pub enum AuthType { ApiKey, OAuth }` + `pub struct CanonicalProvider { id, display_name, env_var, auth_type, default_base_url }` + `pub const CANONICAL_PROVIDERS: &[CanonicalProvider]` (17 entries).
2. **MODIFY** `codelet/providers/src/lib.rs` — `pub mod catalog;` + re-exports.
3. **MODIFY** `codelet/providers/src/custom/management.rs:95-118` — replace hard-coded 6-tuple with iteration over `CANONICAL_PROVIDERS`; set `display_name: Some(p.display_name.to_string())`; configured for built-ins not in the legacy 6-field `ProviderCredentials` defaults to `false`.
4. **NEW** `codelet/fspec-tui/tests/provider_settings_canonical_order_rpc107.rs` — drives `list_provider_credentials` against a controlled env; asserts 17 canonical rows in canonical order with canonical display names.
5. **MODIFY** `codelet/fspec-tui/tests/rpc054_cross_transport_parity.rs` — add one parity test for `list_provider_credentials` returning equal results across embedded + websocket transports.

## Out of scope (per work unit description)

- Extending `ProviderCredentials::detect()` to probe the 11 net-new env vars (sibling card).
- `credential_type` matrix beyond the existing `codex` / `github-copilot` OAuth arms (sibling card).
- TS frontend convergence onto a NAPI `list_canonical_providers()` (sibling card, Agent E).
