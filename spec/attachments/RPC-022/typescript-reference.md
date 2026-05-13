# RPC-022 — Modal dialogs: ModelSelector, ThinkingLevel, RoleBanner

## TypeScript reference

### ModelSelectorScreen
`src/tui/components/ModelSelectorScreen.tsx` (199 lines) +
`src/tui/components/ModelSelectorView.tsx` (210 lines) +
`src/tui/components/CustomModelFormView.tsx` (78 lines) +
`src/tui/components/DeleteCustomModelConfirmView.tsx` (35 lines)

A full-screen modal that lets the user pick from a list of providers
and models. Features:
- Tree-style display: provider → models → custom model creator.
- `Enter` selects; arrow keys navigate.
- Custom models can be added (form view) and deleted (confirm view).
- Per-session model persistence via NAPI.

NAPI calls:
- `sessionSetModel(sessionId, providerKey, modelId)` —
  `codelet/napi/src/session_manager.rs`.
- `sessionSetModelProfile(sessionId, profile)` —
  `codelet/napi/src/session_manager.rs`.
- Provider/model registry: `src/tui/store/modelStore.ts`.

### ThinkingLevelDialog
`src/tui/components/ThinkingLevelDialog.tsx` (138 lines)

Modal that picks one of `Off / Low / Medium / High` for the current
session.

NAPI calls:
- `getThinkingConfig(sessionId)` → current level.
- `JsThinkingLevel` enum import.

### RoleBanner
`src/tui/components/RoleBanner.tsx` (53 lines)

Visible at the top of the scrollback when a custom role overlay is
active. Shows the role text with `[role]` prefix and a `clear` hint.

NAPI calls:
- `sessionGetRole(sessionId)` → `Option<String>`.
- `sessionSetRole(sessionId, role)` — sets or clears.

## Current Rust state

None of these dialogs / banners exist in the Rust TUI. The model used
for chat is hardcoded by the backend; there is no way to switch from
the TUI.

The Compositor (`codelet/fspec-tui/src/compositor.rs`) already supports
modal layers — `HelpDialog` and `DisconnectDialog` use it. New dialogs
plug into the same pattern at `Priority::Foreground`.

## Target Rust behavior

### New shared types

`codelet/rpc-types/src/lib.rs`:
```rust
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub key: String,
    pub display_name: String,
    pub models: Vec<ModelEntry>,
}

#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEntry {
    pub id: String,
    pub display_name: String,
    pub context_window: u32,
    pub supports_reasoning: bool,
    pub supports_vision: bool,
    pub is_custom: bool,
}
```

### New FspecBackend methods

```rust
#[async_trait]
pub trait FspecBackend: Send + Sync {
    // existing methods...

    /// RPC-022: list providers and their models.
    async fn list_providers(&self) -> Result<Vec<ProviderInfo>>;

    /// RPC-022: set the model for a session.
    async fn set_session_model(
        &self,
        session: SessionId,
        provider_key: String,
        model_id: String,
    ) -> Result<()>;

    /// RPC-022: set the thinking level for a session.
    async fn set_thinking_level(
        &self,
        session: SessionId,
        level: ThinkingLevel,
    ) -> Result<()>;

    /// RPC-022: read the session's current role overlay text (None means
    /// no role active).
    async fn get_session_role(&self, session: SessionId) -> Result<Option<String>>;

    /// RPC-022: set or clear the session's role overlay.
    /// Passing `None` clears.
    async fn set_session_role(
        &self,
        session: SessionId,
        role: Option<String>,
    ) -> Result<()>;
}
```

### New RPC methods

Same surface lifted to `codelet/rpc/src/lib.rs`.

### Service impl

Delegates to:
- `codelet_core::models::list_providers()` (already exists, used by NAPI).
- `codelet_core::session_manager::set_model(sid, provider, model)`.
- `codelet_core::session_manager::set_thinking_level(sid, level)`.
- `codelet_core::session_manager::get_role(sid)` / `set_role(sid, role)`.

### Three new modal components

`codelet/fspec-tui/src/components/model_selector_dialog.rs`:
- Provider list pane (left) + model list pane (right) like the TS view.
- Enter on provider → expand models. Enter on model → emit
  `Action::ModelSelected(provider_key, model_id)`.
- Esc → emit `Action::CloseDialog(MODEL_SELECTOR_DIALOG_ID)`.
- Custom model creation is **out of scope** for this card — show a
  `Custom models: not yet supported (see RPC-NNN)` note.

`codelet/fspec-tui/src/components/thinking_level_dialog.rs`:
- 4 radio options: Off / Low / Medium / High.
- Arrow keys navigate; Enter selects; Esc cancels.

`codelet/fspec-tui/src/views/agent/role_banner.rs`:
- Inline component (not a Compositor layer) rendered above the
  scrollback when `agent_view_store.role_for(session).is_some()`.
- Single row, dim background, format: `[role] {text}` truncated to
  terminal width, with `Press /role clear to remove` hint on the right.

### AgentViewStore extension

```rust
pub struct AgentViewStore {
    // existing...
    role_by_session: HashMap<SessionId, String>,
}

impl AgentViewStore {
    pub fn role_for(&self, session: &SessionId) -> Option<&str> { ... }
    pub fn set_role(&mut self, session: SessionId, role: Option<String>) { ... }
}
```

App::bootstrap calls `backend.get_session_role(sid)` for each session
when it's first attached to populate the store.

### Slash command wiring (from RPC-020)

- `/model` → push `ModelSelectorDialog` into the Compositor.
- `/thinking` → push `ThinkingLevelDialog`.
- `/role <text>` → emit `Action::SetSessionRole(sid, Some(text))`.
- `/role clear` → emit `Action::SetSessionRole(sid, None)`.

## RPC/NAPI boundary contract

```
TS AgentView → napi.sessionSetModel / sessionSetModelProfile
              napi.getThinkingConfig
              napi.sessionGetRole / sessionSetRole
              → all already wired, unchanged

Rust TUI → FspecBackend::list_providers / set_session_model
                       / set_thinking_level
                       / get_session_role / set_session_role
       → FspecService::* [tarpc]
       → codelet_core::models / session_manager [shared impl]
```

## Existing TypeScript behavior preserved

- `src/tui/components/ModelSelectorScreen.tsx` — UNCHANGED.
- `src/tui/components/ModelSelectorView.tsx` — UNCHANGED.
- `src/tui/components/ThinkingLevelDialog.tsx` — UNCHANGED.
- `src/tui/components/RoleBanner.tsx` — UNCHANGED.
- `src/tui/store/modelStore.ts` — UNCHANGED.
- All TS NAPI calls — UNCHANGED.

## Acceptance criteria sketch

- `/model` in the Rust AgentView opens a modal showing providers + models.
- Selecting a model persists via the new RPC method and updates the
  SessionHeader from RPC-018.
- `/thinking` opens a modal with Off/Low/Medium/High.
- Selecting a level persists via the new RPC method and updates the
  `[T:Level]` badge in the SessionHeader.
- `/role <text>` sets a session role, which becomes visible as the
  RoleBanner above the scrollback.
- `/role clear` removes the role and the banner.
- Five new RPC methods are tested against both transports.
- All existing TS dialog behavior still works unchanged.

## Out of scope (explicitly)

- Custom model creation / deletion forms — defer to a future card.
- Provider settings panel (`/providers`) — defer.
- Model profile management — defer.
