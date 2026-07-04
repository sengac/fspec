# PROV-133 — Pressing 'd' in provider settings does not actually remove credentials

## Summary
In the Rust TUI Provider Settings view, pressing `d` to delete a provider's credentials
appears to succeed (confirm dialog fires, `credentials.json` is mutated) but the provider
still reports as **configured** afterwards. For OAuth providers it is a **pure no-op**;
for API-key providers the file is cleared but the process env var keeps it "configured".

## Evidence (root cause)

### The delete flow is wired end-to-end (this part works)
```
d key
  -> provider_settings/nav_tree_ops.rs::delete_on_nav_item
  -> open_delete_confirm (ConfirmDialog)
  -> confirm Primary -> Action::ConfirmDeleteProviderCredentials(id)
  -> app/dispatch_provider_settings.rs
  -> backend.delete_provider_credentials
  -> sessions/handle_impl.rs::delete_provider_credentials
  -> credentials::delete_credential
  -> writer.rs:115-116  (mutates credentials.json)
```

### But "configured" is not derived from credentials.json
- `configured` / availability is projected from **env vars + OAuth auth files**, NOT
  from `credentials.json`:
  - `credentials.rs:105-236`
  - `management.rs:137-142`

### Consequence 1 — OAuth providers: pure no-op
OAuth providers (`anthropic`, `codex`, `github-copilot`) store the credential in
dedicated auth files:
- `claude_auth.json`
- `codex auth.json`
- `copilot_auth.json`

`delete_credential` never touches these files, so delete does nothing observable.

### Consequence 2 — API-key providers: env var survives
- `credentials.json` is cleared, but the process env var stays set.
- `update_all_provider_env_vars` only ever calls `set_var`, never `remove_var`
  (`resolver.rs:240-249`).
- The next `detect()` / `list_provider_credentials` re-reads the still-set env var and
  re-reports `configured = true`.

## Fix direction (choose one, prefer A)

### Option A — Make delete authoritative (recommended)
Extend `delete_provider_credentials` so it removes the credential from *every* source
that feeds the availability projection:
1. Delete from `credentials.json` (already done).
2. For OAuth providers, delete/clear the matching auth file
   (`claude_auth.json` / `codex auth.json` / `copilot_auth.json`).
3. Unset the process env var for that provider (`std::env::remove_var` via a new
   `remove` path in the resolver, complementing `update_all_provider_env_vars`).

### Option B — Split the projection
Separate "stored in credentials.json" from "present via env/auth-file" so the UI can
show an accurate configured state and delete only affects the credentials.json layer,
with a clear message that env/auth-file sources remain.

**Prefer A** because the user's mental model of `d` is "remove this credential," which
should be authoritative across all sources.

## Safety / scope notes
- `remove_var` mutates process-global env state — must be done carefully and only for the
  specific provider's key(s).
- OAuth auth-file deletion must not delete unrelated data in shared auth files; delete
  only the provider's entry.
- Do not regress the existing `credentials.json` write path or the confirm-dialog wiring.

## Acceptance criteria (for Example Mapping)
- **Rule:** After confirming delete for an API-key provider, the provider reports
  `configured = false` — the env var is unset AND credentials.json is cleared.
- **Rule:** After confirming delete for an OAuth provider (anthropic/codex/github-copilot),
  the provider's auth file entry is removed and it reports `configured = false`.
- **Rule:** Delete only affects the targeted provider; other providers' credentials,
  env vars, and auth-file entries are untouched.
- **Rule:** The confirm dialog and existing dispatch wiring behaviour is unchanged.
- **Example:** OpenAI configured via `OPENAI_API_KEY` env var → press `d`, confirm →
  next list shows OpenAI unconfigured.
- **Example:** Anthropic configured via OAuth (`claude_auth.json`) → press `d`, confirm →
  auth entry removed, Anthropic unconfigured.
- **Example:** Delete Anthropic while Codex is also configured → Codex remains configured.

## Test strategy
Integration tests using a redirected `$HOME` / temp fspec home (per project testing
philosophy — real filesystem, no mocks). Seed a fake `credentials.json`, an env var, and a
fake OAuth auth file. Invoke the delete path. Assert the availability projection returns
`configured = false` for the targeted provider and unchanged for others. For env-var
unset, set the var in-test, run delete, assert `std::env::var(...)` is now `Err`.

## Files
- Fix: `codelet/sessions/src/handle_impl.rs` (delete_provider_credentials),
  credentials/writer + credentials.rs, resolver.rs (env unset), management.rs (projection)
- Reference/trace: `provider_settings/nav_tree_ops.rs`,
  `app/dispatch_provider_settings.rs`
