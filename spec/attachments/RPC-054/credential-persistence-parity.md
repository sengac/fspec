# RPC-054 reopened — Credential persistence parity gap

**Date:** 2026-06-19
**Reason for reopen:** The provider screen does not actually persist credentials.
RPC-054 v1 explicitly deferred "Real credential persistence" as out-of-scope
(`set_provider_credentials` / `delete_provider_credentials` are no-op success).
In the Rust-native TUI there is no TypeScript frontend to own the write path,
so entering an API key in `/provider` silently does nothing.

## TypeScript reference (canonical write path)

`src/utils/credentials.ts`:

- `getCredentialsPath()` → `<getFspecUserDir()>/credentials/credentials.json`
  (`getFspecUserDir()` = `FSPEC_USER_DIR` env or `~/.fspec`).
- File shape `CredentialsFile { version: number, providers: Record<id, { apiKey, lastUpdated }> }`.
- `saveCredential(providerId, apiKey)`:
  1. `mkdir(credDir, { recursive: true })` then `chmod(credDir, 0o700)`.
  2. `loadCredentials()` (missing/empty file → `{ version: 1, providers: {} }`).
  3. `providers[providerId] = { apiKey, lastUpdated: new Date().toISOString() }`.
  4. `writeFile(credPath, JSON.stringify(creds, null, 2))`.
  5. `chmod(credPath, 0o600)`.
  6. `credentialsReload()` (NAPI) to notify Rust to reload cache + env vars.
- `deleteCredential(providerId)`:
  1. `loadCredentials()`, `delete providers[providerId]`.
  2. `writeFile(...)`, `chmod(credPath, 0o600)`.
  3. `credentialsReload()`.
- chmod errors are swallowed (tests may have torn down the dir).
- NEVER logs the raw key.

## Rust current state

- `codelet/sessions/src/credentials/store.rs` — READ ONLY:
  `get_api_key`, `reload_if_changed` (mtime), `force_reload`, `load_from_disk`,
  `credentials_reload()` (re-reads on mtime change + `update_all_provider_env_vars`),
  `get_stored_api_key[_with_dir]`. NO save/write/delete.
- `codelet/sessions/src/credentials/types.rs` —
  `CredentialsFile { version, providers: HashMap<String, ProviderCredential> }`,
  `ProviderCredential { api_key, last_updated }`, `#[serde(rename_all = "camelCase")]`.
  Already matches the TS JSON shape (`apiKey`, `lastUpdated`).
- `codelet/sessions/src/handle_impl.rs:1187` `set_provider_credentials` — validates only.
- `codelet/sessions/src/handle_impl.rs:1222` `delete_provider_credentials` — no-op success.
- Path: `fspec_user_dir()` (profile_sections.rs:183) = `FSPEC_USER_DIR` env or `$HOME/.fspec`.

## Required change (the port)

1. New write functions in the credentials module (e.g. `credentials/store.rs` or a
   new `credentials/writer.rs`, keeping files < 300 LoC):
   - `save_credential_with_dir(data_dir, provider_id, api_key)` and a
     `save_credential(provider_id, api_key)` convenience that resolves
     `fspec_user_dir()`.
   - `delete_credential_with_dir(data_dir, provider_id)` + convenience.
   - Behaviour mirrors TS: create dir (0700 on unix), read-modify-write
     `credentials.json` (default `{version:1,providers:{}}`), set ISO-8601
     `last_updated`, write pretty JSON, set 0600 on unix, then call
     `credentials_reload()` so the in-memory cache + env vars refresh.
   - Swallow chmod errors; never log the key.
2. Wire `set_provider_credentials` (api_key kind) → `save_credential`.
   Wire `delete_provider_credentials` → `delete_credential`.
   Keep existing input validation (empty api_key → Err).
3. `oauth` / `custom` credential kinds remain out of scope here (OAuth is a
   separate follow-up; document as assumption) — only the `api_key` write path
   is ported in this card. Validation for those kinds stays.

## Testability (offline)

- Use `FSPEC_USER_DIR` pointed at a temp dir (no `$HOME` mutation, no network).
- Assert file created with correct JSON, `apiKey`/`lastUpdated` present, second
  save updates in place, delete removes the key, delete of last key leaves
  `{version:1,providers:{}}` (or empty providers map), unix perms 0600/0700.
- After save, `get_stored_api_key_with_dir(provider_id, dir)` returns the key
  (proves the read+write round-trip and reload contract).

## Unix-perms note

`chmod` is unix-only (`std::os::unix::fs::PermissionsExt`). Guard perm-setting
with `#[cfg(unix)]`; on non-unix the write still succeeds (matches TS swallow).
