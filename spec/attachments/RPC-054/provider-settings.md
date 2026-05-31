# RPC-054 — `/provider` ProviderSettingsView + provider-credentials RPC surface

**Parent:** RPC-030 · **Phase:** 7.1 · **Estimate:** 8 pts · **Depends on:** RPC-053

## Goal

Port the TS `ProviderSettingsScreen` (`src/tui/components/ProviderSettingsScreen.tsx`) to Rust. New child view `ProviderSettingsView` in `codelet/fspec-tui/src/views/`. Add the supporting RPC surface (`list_provider_credentials`, `set_provider_credentials`, `test_provider_connection`, `refresh_models_cache`) on `SessionManagerHandle`, `FspecService`, and `FspecBackend`.

## TS reference

- `src/tui/components/ProviderSettingsScreen.tsx` — top-level view
- `src/tui/components/ProviderSettings/` — credential edit forms, OAuth flow, model picker
- `src/llm/provider-credentials.ts` — backend persistence

Capabilities (read the TS source):

1. List configured providers (anthropic, openai, copilot, custom, etc.).
2. For each provider: show display name, model count, configured status.
3. Edit credentials (API key, OAuth, custom endpoint URL).
4. Test connection (network round-trip to the provider).
5. Refresh model list (for providers that publish a dynamic catalog).
6. Add a new custom provider.
7. Delete a provider.

## Backend trait additions

Add to `SessionManagerHandle`:

```rust
fn list_provider_credentials(&self) -> Vec<ProviderCredentialInfo>;
fn get_provider_credential(&self, provider_id: &str) -> Option<ProviderCredentialInfo>;
fn set_provider_credentials(&self, provider_id: &str, creds: ProviderCredentialInput) -> Result<(), String>;
fn delete_provider_credentials(&self, provider_id: &str) -> Result<(), String>;
fn test_provider_connection(&self, provider_id: &str) -> Result<TestConnectionResult, String>;
fn refresh_models_cache(&self, provider_id: &str) -> Result<Vec<ModelEntry>, String>;
```

New wire types in `codelet/rpc-types/src/lib.rs`:

```rust
pub struct ProviderCredentialInfo {
    pub provider_id: String,
    pub display_name: String,
    pub configured: bool,
    pub credential_type: String, // "api_key" | "oauth" | "custom"
    pub model_count: usize,
}

pub enum ProviderCredentialInput {
    ApiKey { key: String },
    OAuth { token: String, refresh_token: Option<String> },
    Custom { endpoint: String, api_key: Option<String> },
}

pub struct TestConnectionResult {
    pub success: bool,
    pub error: Option<String>,
    pub latency_ms: u32,
}
```

(These belong in RPC-036, but if missed there, add in this card.)

## Implementation — `codelet/sessions/src/lib.rs`

Delegate to `codelet-providers` (already NAPI-free). The methods correspond to:

- `list_provider_credentials` → `codelet_providers::CredentialStore::list_all()`
- `set_provider_credentials` → `codelet_providers::CredentialStore::set(...)`
- `test_provider_connection` → `codelet_providers::ProviderManager::test_connection(...)` (existing `#[napi]` at line 7975 of original `session_manager.rs`)
- `refresh_models_cache` → `codelet_providers::ProviderManager::refresh_models(...)`

## Frontend — `ProviderSettingsView`

`codelet/fspec-tui/src/views/provider_settings/mod.rs`:

```rust
pub struct ProviderSettingsView {
    providers: Vec<ProviderCredentialInfo>,
    selected_index: usize,
    edit_mode: Option<EditMode>,
    test_status: HashMap<String, TestStatus>,
}

pub enum EditMode {
    ApiKey { provider_id: String, draft_key: String },
    OAuth { provider_id: String, flow_state: OAuthFlowState },
    Custom { provider_id: String, draft_endpoint: String, draft_key: String },
}
```

Layout: left pane list of providers, right pane detail/edit form. Match TS aesthetics where reasonable.

## Slash command wiring

`SlashCommandAction::Provider` (currently notice-fallback) opens the view:

```rust
SlashCommandAction::Provider => {
    self.emit_action(Action::OpenProviderSettingsView);
}

Action::OpenProviderSettingsView => {
    let backend = self.backend.clone();
    let sender = self.dispatch_sender.clone();
    tokio::spawn(async move {
        let providers = backend.list_provider_credentials().await.unwrap_or_default();
        let _ = sender.send(Action::ProviderSettingsLoaded { providers });
    });
    self.navigator.go_to_provider_settings();
}
```

## Acceptance criteria

1. New trait methods exist on `SessionManagerHandle`, `FspecService`, `FspecBackend`, with stubs for `StubSessionManagerHandle`.
2. `codelet/sessions` implementation delegates to `codelet-providers`.
3. `/provider` opens `ProviderSettingsView` showing all configured providers.
4. User can edit an API key in the dialog → save → verify with `test_provider_connection`.
5. Adding a custom provider works.
6. Deleting a provider works.
7. `refresh_models_cache` updates the displayed model count.
8. Integration test in `codelet/fspec-tui/tests/provider_settings.rs` drives the happy path.

## Risks

- OAuth flow is complex (PKCE, browser handoff). For this card, scope OAuth to "stub flow that opens a URL"; the actual OAuth port is large. Document as follow-up if needed.
- Credential storage: `codelet-providers::CredentialStore` writes to OS keychain on macOS / Win, file on Linux. Confirm behaviour matches TS.

## Out of scope

- Custom provider type detection beyond OpenAI-compatible API.
