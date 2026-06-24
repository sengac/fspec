# PROV-098 — Rich nav tree drops `masked_key` / `source`

## Symptom
Even when a provider's API key IS detected, the Rust Provider Settings screen
shows only the bare provider name — no `✓`, no masked key, no `[env]` tag, and no
`(not configured)` annotation for unconfigured providers.

## Root cause
The RPC-103 / RPC-349 rich nav-tree render replaced the old legacy flat list but
dropped the credential-display data on the way through.

1. **Display struct has no fields for it.**
   `ProviderDisplayInfo` (`codelet/fspec-tui/src/views/provider_settings/nav_item.rs:48`)
   has: `id, name, configured, credential_type, model_count, has_oauth_tokens,
   is_oauth_provider, requires_api_key, env_var, profiles, oauth_login_methods,
   oauth_status_label`. There is **no `masked_key` and no `source`**.

2. **Projection never copies them.**
   `projection.rs::project_one` (`fspec-tui/.../provider_settings/projection.rs:71`)
   maps `ProviderCredentialInfo` → `ProviderDisplayInfo` but ignores
   `info.masked_key` and `info.source` (the backend wire type DOES carry both —
   `codelet_rpc_types::ProviderCredentialInfo`).

3. **Labels are bare.**
   `list_nav_render.rs::row_kind_and_label` (`:109-131`):
   - Provider row label = `p.name` only (`:116`)
   - ApiKey row label = literal `"API Key"` (`:127`)
   No `✓`, mask, `[env]`, or `(not configured)`.

   (The legacy flat list at `list.rs:255-258` DID render `✓`/`·` + type, but the
   rich tree that replaced it does not.)

## TS reference render
`src/tui/components/ProviderSettingsPanel.tsx:594-604`:
```tsx
{status?.hasKey ? (
  <Text> ✓ {status.maskedKey}
    {status.source && <Text dimColor={!isSelected}> [{status.source}]</Text>}
  </Text>
) : ( <Text> (not configured)</Text> )}
```
- `maskApiKey` (`src/utils/credentials.ts:277-291`): `<prefix>••••••••<last4>`,
  prefix regex `/^(sk-ant-|sk-|gsk_|AIza|xai-)/` else first 6 chars; `••••••••`
  if key < 12 chars.
- `source` is one of `env` | `file` | `dotenv`.

## Fix direction
1. Add `masked_key: Option<String>` and `source: Option<String>` to
   `ProviderDisplayInfo`.
2. Copy them through in `project_one`.
3. In `list_nav_render.rs`:
   - Provider row: append ` ✓ {masked} [{source}]` when configured (api-key
     providers), or ` (not configured)` when not. For OAuth providers, append the
     OAuth status text (coordinate with PROV-099).
   - ApiKey child row: append ` ✓ {masked} [{source}]` / `(not configured)`.
4. The backend already supplies `masked_key`/`source` from
   `management.rs::list_providers_info` (`SOURCE_ENV`). Consider also emitting
   `SOURCE_FILE`/`SOURCE_DOTENV` (constants already exist in
   `providers/src/credentials.rs:18-35`) — at minimum render whatever `source`
   the backend provides.

## Files in play
- `codelet/fspec-tui/src/views/provider_settings/nav_item.rs`
- `codelet/fspec-tui/src/views/provider_settings/projection.rs`
- `codelet/fspec-tui/src/views/provider_settings/list_nav_render.rs`
- (backend, already populated) `codelet/providers/src/custom/management.rs:145-158`

## Acceptance pointers
- Configured api-key provider renders `✓ <masked> [env]`.
- Unconfigured provider renders `(not configured)`.
- Masking format matches TS (`prefix + •••••••• + last4`).
- Deterministic: build `ProviderDisplayInfo` fixtures directly; no network.
