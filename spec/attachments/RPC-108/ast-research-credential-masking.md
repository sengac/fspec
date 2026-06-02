# RPC-108 — AST research: credential masking + source-tag wiring

## Wire type (extension target)

`codelet/rpc-types/src/lib.rs:391-401`:

```rust
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCredentialInfo {
    pub provider_id: String,
    pub display_name: String,
    pub configured: bool,
    pub credential_type: String,
    pub model_count: u32,
}
```

Will add `pub masked_key: Option<String>` and `pub source: Option<String>`
(both default to `None`, matching existing `credential_type` string-based
discriminant convention because `napi(object)` does not support enums).

## Masking helper home

`codelet/providers/src/credentials.rs` is 229 LoC, contains
`ProviderCredentials::detect()` and the per-provider `has_*` probes. New
`pub fn mask_api_key(&str) -> String` lives at module scope alongside
`SOURCE_EXPLICIT`/`SOURCE_FILE`/`SOURCE_ENV`/`SOURCE_DOTENV` `pub const &str`
constants. This is the single source of truth — every transport reads
through it.

## Production list-provider-info path

`codelet/providers/src/custom/management.rs:95-180` —
`pub fn list_providers_info() -> Result<Vec<ProviderInfo>, ProviderError>`
loops `crate::catalog::CANONICAL_PROVIDERS` first (RPC-107) and appends
discovered customs. The `ProviderInfo` internal type at
`management.rs:57-77` does NOT carry a masked key today — we extend it
with two new fields:

```rust
pub struct ProviderInfo {
    // ... existing fields ...
    pub masked_key: Option<String>,
    pub source: Option<String>,
}
```

Population rules (mirror TS `getProviderConfig` L219-266):
- If env var declared and `std::env::var(env).map(|v| !v.is_empty()).unwrap_or(false)` →
  `masked_key = Some(mask_api_key(&raw_key))`, `source = Some("env".into())`.
- OAuth-only entries (anthropic/codex/github-copilot detected via auth file) →
  `masked_key = None`, `source = None` (TS renders 'OAuth' literal at view).
- Unconfigured → both `None`.

## Wire boundary mapper

`codelet/sessions/src/handle_impl.rs:869-898` —
`fn list_provider_credentials(&self) -> Vec<ProviderCredentialInfo>`
maps `ProviderInfo` → `ProviderCredentialInfo` row by row. Add
`masked_key: p.masked_key.clone()` and `source: p.source.clone()` to the
mapper.

## Transports (read-only consumers, no code change needed)

- `codelet/fspec-tui/src/transport/embedded.rs:579`
- `codelet/fspec-tui/src/transport/websocket.rs:940`
- `codelet/fspec-tui/src/transport/mod.rs:530`

Both transports already pass `Vec<ProviderCredentialInfo>` through
unchanged; the two new fields ride along via serde / napi automatically.

## Test surfaces

- Unit tests on `mask_api_key` covering 8 scenarios (5 prefix matches,
  no-prefix fallback, short-key fallback, prefix-order precedence).
- Integration tests on `list_providers_info` covering env-sourced
  population, OAuth-None semantics, and unconfigured-None semantics.
- Cross-transport parity in `codelet/fspec-tui/tests/rpc054_cross_transport_parity.rs`
  asserts identical `masked_key + source` through embedded + websocket.

## Out of scope

- TUI rendering of the masked key + `[source]` suffix lives under the
  RPC-104 visual-matrix card and is not re-implemented here.
- OAuth token byte masking — TS never carries OAuth tokens on the wire
  surface; the view layer substitutes literal `'OAuth'`.
- Custom provider `source: 'config'` tagging (PROV-067 scope, separate
  card).
