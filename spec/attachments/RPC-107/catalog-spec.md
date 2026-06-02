# RPC-107 — Provider catalog: canonical 17-provider ordered registry (Rust port)

**Parent:** RPC-054 · **Phase:** 7.1 follow-up · **Lane:** Provider Catalog (Agent B)

## Goal

Port the canonical 17-provider ordered registry from
[`src/utils/provider-registry.ts`](../../../src/utils/provider-registry.ts)
into Rust so the `ProviderSettingsView` in the Rust frontend shows the
**same 17 rows in the same order with the same display names** as the
TypeScript Ink reference. Today the Rust side (`codelet/sessions/src/handle_impl.rs:869-895` →
`codelet/providers/src/custom/management.rs:95-158`) hard-codes only 6
built-in providers (`claude`, `openai`, `gemini`, `zai`, `codex`,
`github-copilot`) with their **internal slug as the display name**
(`display_name: Some(name.to_string())` at `management.rs:109`). That
is a hard divergence from the TS canon: the TS frontend renders
`'OpenAI API'`, `'Anthropic'`, `'Cohere'`, etc., from
[`PROVIDER_REGISTRY`](../../../src/utils/provider-registry.ts#L43-L217).

## TS canonical surface (cite-locked)

`src/utils/provider-registry.ts:18-36` declares the ordering array:

```ts
export const SUPPORTED_PROVIDERS = [
  'openai', 'anthropic', 'cohere', 'gemini', 'mistral', 'xai',
  'together', 'huggingface', 'openrouter', 'groq', 'deepseek',
  'moonshot', 'galadriel', 'azure', 'zai', 'codex', 'github-copilot',
] as const;
```

`src/utils/provider-registry.ts:43-217` declares the per-provider
`PROVIDER_REGISTRY` entries with display name, base URL, env var,
`authMethod` (`'bearer' | 'x-api-key' | 'query_param' | 'none'`),
`authType` (`'api-key' | 'oauth'`), `requiresApiKey`, and description.

The full ordered list with display name + auth type:

| # | id                | name              | authType | TS line |
| - | ----------------- | ----------------- | -------- | ------- |
| 1 | `openai`          | OpenAI API        | api-key  | L45-54  |
| 2 | `anthropic`       | Anthropic         | oauth    | L55-64  |
| 3 | `cohere`          | Cohere            | api-key  | L65-74  |
| 4 | `gemini`          | Google Gemini     | api-key  | L75-84  |
| 5 | `mistral`         | Mistral AI        | api-key  | L85-94  |
| 6 | `xai`             | xAI               | api-key  | L95-104 |
| 7 | `together`        | Together AI       | api-key  | L105-114|
| 8 | `huggingface`     | Hugging Face      | api-key  | L115-124|
| 9 | `openrouter`      | OpenRouter        | api-key  | L125-134|
| 10| `groq`            | Groq              | api-key  | L135-144|
| 11| `deepseek`        | DeepSeek          | api-key  | L145-154|
| 12| `moonshot`        | Moonshot          | api-key  | L155-164|
| 13| `galadriel`       | Galadriel         | api-key  | L165-174|
| 14| `azure`           | Azure OpenAI      | api-key  | L175-184|
| 15| `zai`             | Z.AI              | api-key  | L185-195|
| 16| `codex`           | Codex (ChatGPT)   | oauth    | L196-205|
| 17| `github-copilot`  | GitHub Copilot    | oauth    | L206-216|

## Current Rust gap (cite-locked)

`codelet/providers/src/custom/management.rs:99-118` only iterates over
six built-ins (`claude`, `openai`, `gemini`, `zai`, `codex`,
`github-copilot`) when building the response list. The Rust slug
`claude` does NOT match the TS `anthropic`. The remaining 11 TS
providers (`cohere`, `mistral`, `xai`, `together`, `huggingface`,
`openrouter`, `groq`, `deepseek`, `moonshot`, `galadriel`, `azure`)
have no representation at all unless registered as custom providers via
`discover_provider_configs` (which requires user JSON files on disk).

`codelet/sessions/src/handle_impl.rs:873-887` then sets
`display_name: p.display_name.unwrap_or_else(|| p.name.clone())`, which
in practice is always the lowercase slug because
`management.rs:109` only ever sets `Some(name.to_string())` for
built-ins. That produces rows like `"openai"`, `"gemini"`,
`"github-copilot"` in the Rust TUI instead of `"OpenAI API"`,
`"Google Gemini"`, `"GitHub Copilot"`.

## Proposed Rust enum/struct

Introduce a new const slice in
`codelet/codelet-providers/src/catalog.rs` (or
`codelet/providers/src/catalog.rs`, whichever crate currently owns the
built-in registry — `codelet/providers/src/custom/management.rs` is the
de-facto site today):

```rust
/// Static, ordered list of TS-canonical providers. Matches
/// src/utils/provider-registry.ts SUPPORTED_PROVIDERS verbatim.
pub const CANONICAL_PROVIDER_ORDER: &[&str] = &[
    "openai", "anthropic", "cohere", "gemini", "mistral", "xai",
    "together", "huggingface", "openrouter", "groq", "deepseek",
    "moonshot", "galadriel", "azure", "zai", "codex", "github-copilot",
];

pub struct CanonicalProvider {
    pub id: &'static str,
    pub display_name: &'static str,
    pub env_var: &'static str,
    pub auth_type: AuthType,
    pub default_base_url: Option<&'static str>,
}

pub const CANONICAL_PROVIDERS: &[CanonicalProvider] = &[
    CanonicalProvider { id: "openai", display_name: "OpenAI API", env_var: "", auth_type: AuthType::ApiKey, default_base_url: Some("https://api.openai.com/v1") },
    CanonicalProvider { id: "anthropic", display_name: "Anthropic", env_var: "ANTHROPIC_API_KEY", auth_type: AuthType::OAuth, default_base_url: Some("https://api.anthropic.com/v1") },
    // ... all 17 entries
];

pub enum AuthType { ApiKey, OAuth }
```

The `list_provider_credentials` impl in
`codelet/sessions/src/handle_impl.rs` is rewritten to iterate over
`CANONICAL_PROVIDERS` first (preserving the canonical order), populate
`ProviderCredentialInfo { provider_id, display_name, configured,
credential_type, model_count }` from each entry + the runtime
`ProviderCredentials::detect()` result, and THEN append any custom
providers discovered on disk. The display_name field on
`ProviderCredentialInfo` (already exists at `rpc-types/src/lib.rs:395`)
becomes the source-of-truth display string for the TUI title row and
list rows.

The Rust slug `claude` is REMOVED — anthropic is now keyed as
`anthropic` matching the TS canon. (Existing `ProviderCredentials::has_claude()`
becomes `has_anthropic()` or stays as an internal alias; cross-card
coordination needed if any Rust callers depend on the `claude` slug.)

## Files to change

1. `codelet/providers/src/catalog.rs` — NEW: `CANONICAL_PROVIDERS` slice
   + `AuthType` enum + ordering helper.
2. `codelet/providers/src/lib.rs` — `pub mod catalog;` + re-export.
3. `codelet/providers/src/custom/management.rs:95-158` — rewrite
   `list_providers_info` to iterate `CANONICAL_PROVIDERS` for built-ins
   and append customs.
4. `codelet/sessions/src/handle_impl.rs:869-895` — no logic change but
   update tests; the `match p.name.as_str()` arm for credential_type
   stays.
5. `codelet/fspec-tui/src/views/provider_settings/list.rs` — no code
   change (already uses `info.display_name`); tests gain canonical-order
   assertions.
6. `codelet/fspec-tui/tests/provider_settings_view_rpc054.rs` — assert
   first 3 rows are `OpenAI API / Anthropic / Cohere` in that order.

## Test plan

1. **Unit:** `cargo test -p codelet-providers catalog::canonical_order`
   — asserts `CANONICAL_PROVIDERS.len() == 17`, ids match
   `SUPPORTED_PROVIDERS` exactly, display_names match the TS table
   above byte-for-byte.
2. **Integration (Rust):** new file
   `codelet/fspec-tui/tests/provider_settings_canonical_order_rpc107.rs`
   — drives `list_provider_credentials` against a stub session manager
   with no configured providers; asserts the response is exactly 17
   rows in canonical order with the canonical display names.
3. **Cross-transport parity:** extend
   `codelet/fspec-tui/tests/rpc054_cross_transport_parity.rs` with one
   new scenario — embedded and websocket both surface the same 17
   canonical providers in the same order.
4. **TUI rendering:** ratatui `TestBackend` snapshot — confirm the
   title row reads `Provider Settings (N configured)` (N = configured
   count, unchanged) and the body's first three rows render
   `OpenAI API`, `Anthropic`, `Cohere`.

## Out of scope (decomposed into sibling cards)

- Auth-method matrix (api_key / oauth-browser / oauth-headless / device
  / copilot / codex enum) — separate card.
- `get_provider_auth_methods(provider_id)` RPC — separate card.
- Configured-count semantics (unique-providers-with-creds vs row-count)
  — separate card.
- Default model per provider — separate card.

## Cross-card coordination

- **Agent E (backend RPC surface):** if `CANONICAL_PROVIDERS` ends up
  exposed via NAPI (so the TS frontend can stop carrying its own
  registry once the Rust side is canonical), the wire shape needs a
  new RPC method `list_canonical_providers()` returning
  `Vec<CanonicalProviderInfo>`. Flag in the new card.
- **Agent A (view layer):** the title row +configured count helper
  doesn't change shape; only the population data does.
