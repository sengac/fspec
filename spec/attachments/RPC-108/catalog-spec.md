# RPC-108 — Provider settings: credential masking + source-tag tagging (Rust port)

**Parent:** RPC-054 · **Phase:** 7.1 follow-up · **Lane:** Provider Catalog (Agent B)

## Goal

Port the TypeScript `maskApiKey` prefix-aware masking helper and the
`source: 'explicit' | 'file' | 'env' | 'dotenv'` provenance tag from
[`src/utils/credentials.ts`](../../../src/utils/credentials.ts) into
the Rust frontend so the `ProviderSettingsView` shows the TS-canonical
masked key + source tag on each configured row. Today the Rust
`ProviderCredentialInfo` wire type
(`codelet/rpc-types/src/lib.rs:393-401`) has NO `masked_key` field and
NO `source` field — only `configured: bool`. The TUI cannot show
`'sk-ant-••••mnop [env]'` because the wire surface doesn't carry the
masked key or the provenance tag.

## TS canonical surface (cite-locked)

### Masking helper

`src/utils/credentials.ts:277-291`:

```ts
export function maskApiKey(apiKey: string): string {
  if (!apiKey || apiKey.length < 12) {
    return '••••••••';
  }
  const prefixMatch = apiKey.match(/^(sk-ant-|sk-|gsk_|AIza|xai-)/);
  const prefix = prefixMatch ? prefixMatch[0] : apiKey.slice(0, 6);
  const suffix = apiKey.slice(-4);
  return `${prefix}••••••••${suffix}`;
}
```

Five recognised prefixes — `sk-ant-` (Anthropic), `sk-` (OpenAI / many
OpenAI-compatible), `gsk_` (Groq), `AIza` (Google Gemini), `xai-` (xAI).
Keys shorter than 12 chars fall back to a fully-masked `••••••••`.

### Source tag

`src/utils/credentials.ts:56-59`:

```ts
export interface ProviderConfigResult {
  apiKey?: string;
  source?: 'explicit' | 'file' | 'env' | 'dotenv';
}
```

Source tag is set by `getProviderConfig` (L219-266) based on which
layer the credential came from:

- `'explicit'` — passed in by the caller (rare; mostly tests).
- `'file'` — loaded from `~/.fspec/credentials/credentials.json`
  (`credentials.ts:232`).
- `'env'` — read from `process.env` for the provider's env var
  (`credentials.ts:243`).
- `'dotenv'` — parsed from `.env` in CWD (`credentials.ts:260`).

### Usage in TUI (where masked-key + source render)

`src/tui/hooks/useProviderSettingsState.ts:268-269`:

```ts
maskedKey: providerConfig.apiKey
  ? maskApiKey(providerConfig.apiKey)
  : ...
```

`src/tui/components/ProviderSettingsPanel.tsx:597, 737`:

```tsx
✓ {status.maskedKey}
```

The source tag is rendered alongside (RPC-104 visual matrix L594-604
covers the rendering — `' ✓ <masked-key> [<source>]'`).

## Current Rust gap (cite-locked)

`codelet/rpc-types/src/lib.rs:391-401` exposes
`ProviderCredentialInfo` with five fields — `provider_id`,
`display_name`, `configured`, `credential_type`, `model_count`. There
is NO `masked_key`, NO `source`. The TUI cannot show `[env]` or the
masked tail because the data simply isn't on the wire.

`codelet/providers/src/credentials.rs` (per the `ls` of
`codelet/providers/src/` — 8.7KB file) presumably already has
detection logic for env vs file but does NOT expose a masking helper.

## Proposed Rust surface

### Wire type extension

```rust
// codelet/rpc-types/src/lib.rs — extend ProviderCredentialInfo
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCredentialInfo {
    pub provider_id: String,
    pub display_name: String,
    pub configured: bool,
    pub credential_type: String,
    pub model_count: u32,
    // RPC-108 NEW: masked key for display, None when unconfigured.
    pub masked_key: Option<String>,
    // RPC-108 NEW: provenance — "explicit" | "file" | "env" | "dotenv".
    pub source: Option<String>,
}
```

`source` stays as `Option<String>` (not enum) because napi(object)
doesn't support enum discriminants — matches the existing
`credential_type` pattern at `lib.rs:397-399`.

### Masking helper

```rust
// codelet/providers/src/credentials.rs (new fn)
pub fn mask_api_key(api_key: &str) -> String {
    if api_key.len() < 12 {
        return "••••••••".to_string();
    }
    const PREFIXES: &[&str] = &["sk-ant-", "sk-", "gsk_", "AIza", "xai-"];
    let prefix = PREFIXES
        .iter()
        .find(|p| api_key.starts_with(*p))
        .map(|p| *p)
        .unwrap_or(&api_key[..6]);
    let suffix = &api_key[api_key.len() - 4..];
    format!("{prefix}••••••••{suffix}")
}
```

Prefix order matters — `sk-ant-` MUST be checked before `sk-` because
both match `sk-ant-XXXX`. The TS regex `/^(sk-ant-|sk-|gsk_|AIza|xai-)/`
greedy-matches in declaration order; the Rust port matches that.

## Files to change

1. `codelet/rpc-types/src/lib.rs:391-401` — add `masked_key` +
   `source` fields. Update doc comment to cite RPC-108.
2. `codelet/providers/src/credentials.rs` — add `mask_api_key` pub fn
   + `CredentialSource` constants (`SOURCE_EXPLICIT`, `SOURCE_FILE`,
   `SOURCE_ENV`, `SOURCE_DOTENV`).
3. `codelet/providers/src/custom/management.rs:99-118` — populate
   `masked_key` + `source` for each built-in by reading the env var,
   masking it via `mask_api_key`, and tagging the source.
4. `codelet/sessions/src/handle_impl.rs:872-887` — propagate
   `masked_key` + `source` from `ProviderInfo` to
   `ProviderCredentialInfo`.
5. `codelet/fspec-tui/src/views/provider_settings/list.rs:204-206` —
   render masked-key + source tag after the display name on configured
   rows.
6. `codelet/napi/index.d.ts` — regenerated automatically once the
   napi_derive picks up the new fields.

## Test plan

1. **Unit:** `cargo test -p codelet-providers credentials::mask_api_key`
   — table-driven, 6 scenarios:
   - `sk-ant-api03-abcdefghijklmnop` → `sk-ant-••••••••mnop`
   - `sk-test-1234567890abcdef` → `sk-••••••••cdef`
   - `gsk_test_1234567890abcdef` → `gsk_••••••••cdef`
   - `AIzaSyABCDEFGH1234IJKLmnop` → `AIza••••••••mnop`
   - `xai-test-1234567890abcdef` → `xai-••••••••cdef`
   - `short` (< 12 chars) → `••••••••`
2. **Integration:** `list_provider_credentials` against env-seeded
   provider — asserts `masked_key == Some("sk-ant-••••mnop")` and
   `source == Some("env".to_string())`.
3. **TUI render:** ratatui `TestBackend` — assert the buffer cell text
   for the anthropic row contains `'✓ sk-ant-••••mnop [env]'`.
4. **Cross-transport parity:** extend
   `rpc054_cross_transport_parity.rs` — embedded and websocket both
   surface the masked_key + source on the same configured row.

## Out of scope

- OAuth credentials masking (codex / anthropic / github-copilot show
  `'OAuth'` literal in TS at `useProviderSettingsState.ts:289`; that
  string lives in the view layer not the wire, so it's covered by the
  view-card RPC-104 visual matrix).
- Custom provider source tagging (those carry `is_custom: true` already
  via `ProviderInfo.is_custom`; the source tag for customs is
  `'config'` per TS — out of scope for this card).

## Cross-card coordination

- **Agent E (backend RPC surface):** the wire type gains two new
  optional fields. Backwards-compatible at the JSON level
  (`Option<String>` serialises to `null` when absent). napi_derive
  re-emits the index.d.ts.
- **Agent A (view layer):** the visual matrix in RPC-104 already
  reserves the `[<source>]` slot — RPC-108 fills the data side.
