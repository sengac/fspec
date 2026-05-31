# Bug 3 — Model selector list is empty in Rust `fspec` binary

**Work unit:** RPC-073
**Date:** 2026-05-27
**Status:** Research only — no code changes

---

## TL;DR

The TS Ink frontend populates the model selector via NAPI `list_providers()` → `codelet_providers::custom::list_providers_info()`, which returns a fully-populated `Vec<ProviderInfo>` including built-ins + discovered custom providers from `~/.fspec/providers/*.json`, gated by credential detection against `~/.fspec/credentials/*.json` + env vars.

The Rust ratatui frontend's `ModelSelectorDialog` is wired end-to-end through 8 layers (compositor → tarpc → `SessionManagerHandle::list_providers`) but the terminal stub at **`codelet/sessions/src/handle_impl.rs:709-715` unconditionally returns `Vec::new()`** with a comment that punted the wiring to RPC-054. RPC-054 then wired *credentials* (a sibling method `list_provider_credentials`) but never came back to fix `list_providers`. The helper `codelet_providers::custom::list_providers_info()` is already called twice in the very same file (lines 798 and 944), and `codelet-providers` is already a Cargo dep of `codelet-sessions`.

Fix is a code-only change at one site: replace the stub body with a `list_providers_info()` call + a lossy adapter that maps the rich 9-field `codelet_providers::custom::ProviderInfo` down to the 3-field `codelet_rpc_types::ProviderInfo` (`name → key`, `display_name` unwrap, `models` → `Vec<ModelEntry>` with `supports_thinking → supports_reasoning`, `usize → u32` saturation, `is_custom` inherited from parent, `display_name` derived from `id`).

---

## 1. TS reference path (Ink frontend) — walk-through

### 1.1 Where the dialog is rendered

The model selector lives in two files under `src/tui/components/`:

- **`ModelSelectorScreen.tsx:39-268`** — orchestrator. Calls `useModelSelectorState()` at `:47` and renders `ModelSelectorView` at `:251-267` with overlays for `CustomModelFormView` and `DeleteCustomModelConfirmView`.
- **`ModelSelectorView.tsx:87-275`** — pure presentational. Renders the hierarchical list with `[R]` / `[V]` / `[C]` capability badges + context window.

State is owned by `useModelSelectorState` (`src/tui/hooks/useModelSelectorState.ts:129-345`), which calls `initializeModels()` from `modelInitializationService.ts` at `:183-185`.

### 1.2 What NAPI function it calls

`src/tui/services/customProviderSectionBuilder.ts:20` imports the NAPI binding:

```ts
import { listProviders } from '@sengac/codelet-napi';
```

The call site is `customProviderSectionBuilder.ts:88`:

```ts
const allProviders = await listProviders();
const customProviders = allProviders.filter(
  (p: JsProviderInfo) => p.isCustom && p.available
);
```

Filters on **`isCustom`** AND **`available`** — both fields exist on `JsProviderInfo` but NOT on the lossy `codelet_rpc_types::ProviderInfo` (see §3).

TS type declaration at `codelet/napi/index.d.ts:1104`:

```ts
export declare function listProviders(): Promise<Array<JsProviderInfo>>
```

JS export at `codelet/napi/index.js:655`.

### 1.3 NAPI binding shape

**`list_providers` impl** at `codelet/napi/src/session_bindings.rs:3466-3474`:

```rust
#[napi]
pub async fn list_providers() -> Result<Vec<JsProviderInfo>> {
    let _ = dotenvy::dotenv();
    codelet_providers::custom::list_providers_info()
        .map(|list| list.into_iter().map(Into::into).collect())
        .map_err(|e| Error::from_reason(format!("list_providers failed: {e}")))
}
```

**`JsProviderInfo`** at `codelet/napi/src/session_bindings.rs:3426-3440` (9 fields, near-1:1 with `codelet_providers::custom::ProviderInfo`):

```rust
#[napi(object)]
pub struct JsProviderInfo {
    pub name: String,
    pub display_name: Option<String>,
    pub available: bool,
    pub is_custom: bool,
    pub facade: Option<String>,
    pub base_url: Option<String>,
    pub api_key_env_var: Option<String>,
    pub models: Vec<JsProviderModelInfo>,
    pub api_style: Option<String>,
}
```

**`From<codelet_providers::custom::ProviderInfo> for JsProviderInfo`** at `session_bindings.rs:3442-3456` — straight field-by-field move (only `models` is mapped via nested `Into::into`). No data loss.

**`JsProviderModelInfo`** at `session_bindings.rs:3391-3407` — 7 fields, identical to `codelet_providers::custom::ProviderModelInfo` except `usize` → `u32` and `max_output_tokens` → `max_output` rename. Conversion at `:3409-3424` uses `u32::try_from(...).unwrap_or(u32::MAX)` saturation.

### 1.4 NAPI → providers crate resolution

Yes — confirmed at `session_bindings.rs:3471`: `codelet_providers::custom::list_providers_info()` is the resolver. The path is canonical and exported by the `codelet-providers` crate.

### 1.5 `list_providers_info()` — source of truth

`codelet/providers/src/custom/management.rs:95-158`. Behavior:

1. Snapshots credentials via `ProviderCredentials::detect()` (`:96`).
2. Emits **six** built-in entries unconditionally: `claude`, `openai`, `gemini`, `zai`, `codex`, `github-copilot` — each with `is_custom: false`, `models: vec![]`, and `available` set from the credential snapshot (`:99-118`).
3. Calls `discover_provider_configs()` to enumerate custom providers (`:120`).
4. For each custom config, builds `ProviderModelInfo` entries from `cfg.models` (`:127-139`).
5. Computes `api_style` as `"anthropic_messages"` or `"openai_chat"` (`:140-143`).

Returns `Result<Vec<ProviderInfo>, ProviderError>`.


### 1.6 `ProviderInfo` (providers crate) — full struct

`codelet/providers/src/custom/management.rs:53-77`:

```rust
#[derive(Debug, Clone)]
pub struct ProviderInfo {
    pub name: String,
    pub display_name: Option<String>,
    pub available: bool,
    pub is_custom: bool,
    pub facade: Option<String>,
    pub base_url: Option<String>,
    pub api_key_env_var: Option<String>,
    pub models: Vec<ProviderModelInfo>,
    pub api_style: Option<String>,
}
```

`ProviderModelInfo` at `:33-51`:

```rust
#[derive(Debug, Clone)]
pub struct ProviderModelInfo {
    pub id: String,
    pub context_window: usize,
    pub max_output_tokens: usize,
    pub supports_tools: bool,
    pub supports_streaming: bool,
    pub supports_thinking: bool,
    pub supports_vision: bool,
}
```

### 1.7 `ProviderCredentials::detect()` — what it scans

`codelet/providers/src/credentials.rs:33-53`:

```rust
pub fn detect() -> Self {
    Self {
        claude_available: std::env::var("ANTHROPIC_API_KEY").is_ok()
            || std::env::var("CLAUDE_CODE_OAUTH_TOKEN").is_ok()
            || has_claude_auth(),
        openai_available: std::env::var("OPENAI_API_KEY").is_ok(),
        codex_available: has_codex_auth(),
        gemini_available: std::env::var("GOOGLE_GENERATIVE_AI_API_KEY").is_ok(),
        zai_available: std::env::var("ZAI_PLAN_API_KEY").is_ok()
            || std::env::var("ZAI_API_KEY").is_ok(),
        github_copilot_available: has_github_copilot_auth(),
        custom_available: detect_custom_provider_availability(),
    }
}
```

Files / directories scanned:

| Source | File / env | Used by |
|---|---|---|
| `ANTHROPIC_API_KEY`, `CLAUDE_CODE_OAUTH_TOKEN` env | env vars | `claude_available` |
| `OPENAI_API_KEY` env | env var | `openai_available` |
| `GOOGLE_GENERATIVE_AI_API_KEY` env | env var | `gemini_available` |
| `ZAI_PLAN_API_KEY`, `ZAI_API_KEY` env | env vars | `zai_available` |
| `~/.codex/auth.json` | `has_codex_auth()` (`credentials.rs:128-141`) | `codex_available` |
| `~/.fspec/credentials/claude_auth.json` | `has_claude_auth()` (`credentials.rs:145-152`) | `claude_available` (fallback) |
| `~/.fspec/credentials/copilot_auth.json` | `has_github_copilot_auth()` (`credentials.rs:159-166`) | `github_copilot_available` |
| `~/.fspec/providers/*.json` (global) and `.fspec/providers/*.json` (project-local) | `discover_provider_configs()` (`custom/discovery.rs:26-36`) | `custom_available` + emitted custom entries |

`FSPEC_HOME` env var overrides the `~/.fspec` base path (`discovery.rs:43-63`, `claude_auth.rs:32`).

**Important:** `ProviderCredentials::detect()` does NOT scan `~/.fspec/fspec-config.json`. Searched `codelet/providers/src/` — no reference to `fspec-config.json` exists in the providers crate. The original bug description mentions `fspec-config.json` as a source, but the actual sources are `~/.fspec/credentials/*.json` + `~/.fspec/providers/*.json` + env vars.

### 1.8 `ProviderManager::new()` — for context

`codelet/providers/src/manager.rs:272-303`. Calls `ProviderCredentials::detect()` once, errors if no credentials, picks a default provider in the order **Claude API > Claude OAuth > Gemini > Codex > OpenAI** via `detect_default_provider()`. Not on the `list_providers` hot path — listed here for completeness because it shares the same `ProviderCredentials::detect()` snapshot.

### 1.9 End-to-end TS path

```
User opens model selector
   ↓
ModelSelectorScreen.tsx:47           useModelSelectorState()
   ↓
useModelSelectorState.ts:183         loadModels() → initializeModels()
   ↓
modelInitializationService.ts        composes provider sections
   ↓
customProviderSectionBuilder.ts:88   await listProviders()  ←─── NAPI call
   ↓
codelet/napi/index.js:655            NAPI export `listProviders`
   ↓
session_bindings.rs:3469             pub async fn list_providers() -> Result<Vec<JsProviderInfo>>
   ↓
session_bindings.rs:3471             codelet_providers::custom::list_providers_info()
   ↓
custom/management.rs:95              fn list_providers_info() -> Result<Vec<ProviderInfo>>
   ↓                                   ├─ ProviderCredentials::detect()  (credentials.rs:33)
   ↓                                   ├─ 6 built-in entries
   ↓                                   └─ discover_provider_configs()    (discovery.rs:26)
   ↓
session_bindings.rs:3442             From<ProviderInfo> for JsProviderInfo
   ↓
TS receives Array<JsProviderInfo>, filters isCustom && available
   ↓
ModelSelectorView renders hierarchical list
```

---

## 2. Rust reference path (ratatui, currently broken) — walk-through

### 2.1 `ModelSelectorDialog` rendering

`codelet/fspec-tui/src/components/model_selector_dialog.rs:35-44` — dialog state:

```rust
pub struct ModelSelectorDialog {
    id: String,
    session_id: SessionId,
    rows: Vec<ModelSelectorRow>,
    selected_index: usize,
    scroll_offset: usize,
    last_visible_rows: Cell<usize>,
    action_tx: Option<UnboundedSender<Action>>,
    pending_action: Option<Action>,
}
```

Imports at `:19`:

```rust
use codelet_rpc_types::{ProviderInfo, SessionId};
```

Constructor at `:47-63` converts `Vec<ProviderInfo>` to rows via `build_rows`. `set_providers` at `:70-78` replaces rows on each new payload. `update` at `:261-266` handles `Action::ListProvidersLoaded`:

```rust
fn update(&mut self, action: Action) -> Option<Action> {
    if let Action::ListProvidersLoaded(providers) = action {
        self.set_providers(providers);
    }
    None
}
```

### 2.2 `build_rows` — only consumes 3 fields

`build_rows` is extracted to a sibling module to keep the parent file under the 300-LoC budget. `codelet/fspec-tui/src/components/model_selector_dialog_rows.rs:12`:

```rust
use codelet_rpc_types::ProviderInfo;
```

The function at `:38-79` reads ONLY:

- `provider.display_name` (header label)
- `provider.models` (iterated)
- `provider.key` (copied to row's `provider_key`)
- per-model: `model.id`, `model.display_name`, `model.supports_reasoning`, `model.supports_vision`, `model.context_window`

It does NOT touch `.available`, `.is_custom`, `.facade`, `.api_key_env_var`, `.api_style`, `.base_url` — those fields don't exist on `codelet_rpc_types::ProviderInfo` anyway.

**Asymmetry vs. TS:** the TS frontend filters on `isCustom && available`; the Rust frontend doesn't (and can't, given the lossy rpc-type). This is an upstream design choice baked into RPC-022 — see §3 mismatch analysis.

### 2.3 `handle_open_model_dialog`

`codelet/fspec-tui/src/app/dispatch_rpc022.rs:30-51`:

```rust
pub(crate) fn handle_open_model_dialog(&mut self) {
    let session_id = match self.agent_view_store.current_session().cloned() {
        Some(sid) => sid,
        None => return,
    };
    if !self.compositor.contains(MODEL_SELECTOR_DIALOG_ID) {
        let dialog = ModelSelectorDialog::new(session_id, Vec::new())
            .with_action_tx(self.action_tx.clone());
        self.compositor.push(Box::new(dialog));
    }
    if tokio::runtime::Handle::try_current().is_err() {
        return;
    }
    let backend = self.backend.clone();
    let action_tx = self.action_tx.clone();
    let handle = tokio::spawn(async move {
        if let Ok(providers) = backend.list_providers().await {
            let _ = action_tx.send(Action::ListProvidersLoaded(providers));
        }
    });
    self.pending_tasks.push(handle);
}
```

Seeds dialog with empty vec, then spawns the backend call. `handle_list_providers_loaded` at `:80-83` is a deliberate no-op (compositor fan-out delivers the action directly to the dialog's `update`).

### 2.4 Action variant

`codelet/fspec-tui/src/components/mod.rs:354-357`:

```rust
/// RPC-022: a spawned `backend.list_providers()` task resolved.
ListProvidersLoaded(Vec<codelet_rpc_types::ProviderInfo>),
```

Carries `Vec<codelet_rpc_types::ProviderInfo>` — the lossy 3-field shape, end to end.

### 2.5 Embedded transport

`codelet/fspec-tui/src/transport/embedded.rs:192-195`:

```rust
async fn list_providers(&self) -> Result<Vec<ProviderInfo>> {
    Ok(self.client.list_providers(context::current()).await?)
}
```

### 2.6 WebSocket transport

`codelet/fspec-tui/src/transport/websocket.rs:420-427`:

```rust
async fn list_providers(&self) -> Result<Vec<ProviderInfo>> {
    let guard = self.client.read().await;
    let client = guard
        .as_ref()
        .ok_or(BackendError::Disconnected)?;
    Ok(client.client().list_providers(context::current()).await?)
}
```

### 2.7 RPC service impl

Trait method declaration at `codelet/rpc/src/lib.rs:162-168`:

```rust
async fn list_providers() -> Vec<ProviderInfo>;
```

Server impl at `:1000-1010`:

```rust
async fn list_providers(self, _ctx: Context) -> Vec<ProviderInfo> {
    match self.inner.session_manager() {
        Some(handle) => handle.list_providers(),
        None => Vec::new(),
    }
}
```

### 2.8 Trait default

`codelet/core/src/session_manager_handle.rs:130-139`:

```rust
fn list_providers(&self) -> Vec<ProviderInfo> {
    Vec::new()
}
```

### 2.9 Stub manager override (test-only)

`codelet/core/src/session_manager_handle.rs:1441-1446`:

```rust
fn list_providers(&self) -> Vec<ProviderInfo> {
    match self.providers.lock() {
        Ok(guard) => guard.clone(),
        Err(_) => Vec::new(),
    }
}
```

Backed by `Arc<Mutex<Vec<ProviderInfo>>>` (line 805), seedable via `set_providers` (line 1307-1310). Used by cross-transport parity tests to pre-seed. **Not on production path.**

### 2.10 The broken stub — production path

`codelet/sessions/src/handle_impl.rs:709-715`:

```rust
fn list_providers(&self) -> Vec<codelet_rpc_types::ProviderInfo> {
    // RPC-042 scope: providers registry wiring lives in RPC-054
    // (`/provider` ProviderSettingsView + credentials surface).
    // The trait default returns `Vec::new()`; keep that semantics
    // by returning an empty Vec here too.
    Vec::new()
}
```

**This is the bug.** RPC-054 wired `list_provider_credentials` (the `/provider` settings view) but never came back to fill in `list_providers` (the model picker dialog populated by RPC-022).

### 2.11 End-to-end Rust path

```
User presses /model
   ↓
handle_open_model_dialog()                                    (dispatch_rpc022.rs:30)
   ↓ pushes ModelSelectorDialog::new(sid, Vec::new())          ← seeded EMPTY
   ↓ spawns backend.list_providers().await
   ↓
EmbeddedBackend / WebsocketBackend::list_providers            (embedded.rs:192 / websocket.rs:420)
   ↓ forwards to tarpc
   ↓
client.list_providers(context::current())
   ↓
FspecServiceImpl::list_providers                              (rpc/src/lib.rs:1000)
   ↓ self.inner.session_manager() → Some(handle)
   ↓
handle.list_providers()                                       (SessionManagerHandle trait)
   ↓
codelet_sessions::SessionManager::list_providers              (sessions/src/handle_impl.rs:709)
   ↓
🚨 RETURNS Vec::new() 🚨
   ↓
Action::ListProvidersLoaded(vec![])
   ↓
compositor fan-out → dialog.set_providers(vec![]) → empty rows
   ↓
User sees empty dialog
```


---

## 3. Wire-type mismatch analysis

There are **three distinct `ProviderInfo`-named shapes** in play:

### 3.1 `codelet_providers::custom::ProviderInfo` (source of truth, 9 fields)

`codelet/providers/src/custom/management.rs:53-77` — full credential/config view.

### 3.2 `JsProviderInfo` (NAPI JS-facing, 9 fields)

`codelet/napi/src/session_bindings.rs:3426-3440` — near-1:1 mirror of the above, only `usize → u32` saturation on per-model fields.

### 3.3 `codelet_rpc_types::ProviderInfo` (tarpc wire, 3 fields)

`codelet/rpc-types/src/lib.rs:350-366`:

```rust
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub key: String,
    pub display_name: String,
    pub models: Vec<ModelEntry>,
}
```

`ModelEntry` at `:340-348`:

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelEntry {
    pub id: String,
    pub display_name: String,
    pub context_window: u32,
    pub supports_reasoning: bool,
    pub supports_vision: bool,
    pub is_custom: bool,
}
```

### 3.4 Field-by-field mapping table (providers → rpc-types)

| `providers::ProviderInfo` | `rpc_types::ProviderInfo` | Adapter action |
|---|---|---|
| `name: String` | `key: String` | rename: `key = name` |
| `display_name: Option<String>` | `display_name: String` | `unwrap_or_else(|| name.clone())` |
| `available: bool` | — | **dropped** (rpc-types has no slot; consider pre-filter) |
| `is_custom: bool` | — at provider level; pushed down to per-model `ModelEntry.is_custom` | propagate parent flag into every child `ModelEntry` |
| `facade: Option<String>` | — | dropped |
| `base_url: Option<String>` | — | dropped |
| `api_key_env_var: Option<String>` | — | dropped |
| `models: Vec<ProviderModelInfo>` | `models: Vec<ModelEntry>` | per-element adapter (below) |
| `api_style: Option<String>` | — | dropped |

### 3.5 Per-model mapping (`ProviderModelInfo` → `ModelEntry`)

| `ProviderModelInfo` | `ModelEntry` | Adapter action |
|---|---|---|
| `id: String` | `id: String` | direct |
| — | `display_name: String` | **no source** — set to `id.clone()` (line 954 in `handle_impl.rs` precedent uses `String::new()`; `id.clone()` is more user-friendly) |
| `context_window: usize` | `context_window: u32` | `u32::try_from(...).unwrap_or(u32::MAX)` (matches NAPI pattern at `session_bindings.rs:3419`) |
| `max_output_tokens: usize` | — | dropped |
| `supports_tools: bool` | — | dropped |
| `supports_streaming: bool` | — | dropped |
| `supports_thinking: bool` | `supports_reasoning: bool` | rename |
| `supports_vision: bool` | `supports_vision: bool` | direct |
| — | `is_custom: bool` | inherit parent `ProviderInfo.is_custom` |

### 3.6 Design observation — is the rpc-type "too lossy"?

The TS path filters `isCustom && available` (`customProviderSectionBuilder.ts:88`) and only renders custom+available providers in one section, with built-ins handled by a separate code path. The Rust ratatui dialog has no such filter — it would render **all 6 built-ins + all custom providers** with no filtering.

This is a design asymmetry, but **it does not block fixing the bug**. The cheapest correct fix is to:

- **Option A (recommended for this work unit):** Replace the stub body with the adapter and accept that ratatui shows all built-ins. Built-in entries have `models: vec![]` (per `management.rs:99-118`), so they render as headers with `(0 models)` and no selectable children — visible but inert. This matches `list_provider_credentials`'s behavior on the same crate output.
- **Option B (defer):** Pre-filter on `(p.is_custom && p.available) || (!p.is_custom && credential_for_builtin_is_present)` before the adapter. This requires deciding whether built-ins should appear at all, which is product-design and out of scope for a bug fix.

Going with **Option A** preserves API symmetry with the existing TS frontend's underlying data shape and defers any UX cleanup to a follow-up work unit.

### 3.7 Existing precedent in the same file

`handle_impl.rs:944-962` (inside the per-provider model lookup) already performs the exact field-level mapping for the per-model conversion:

```rust
let provider_info = codelet_providers::custom::list_providers_info()
    .map_err(|e| format!("list_providers_info failed: {e}"))?
    .into_iter()
    .find(|p| p.name == provider_id);
Ok(provider_info
    .map(|p| {
        p.models
            .into_iter()
            .map(|m| ModelEntry {
                id: m.id,
                display_name: String::new(),
                context_window: m.context_window as u32,
                supports_reasoning: m.supports_thinking,
                supports_vision: m.supports_vision,
                is_custom: true,
            })
            .collect()
    })
    .unwrap_or_default())
```

Note: this snippet hard-codes `is_custom: true` because it's called only for custom-provider lookups. The new adapter must use `is_custom: p.is_custom` (the parent's flag) to correctly tag built-in entries.

Also note: this snippet uses `m.context_window as u32` (silent truncation on overflow). The NAPI binding uses `u32::try_from(...).unwrap_or(u32::MAX)` (saturating). Recommend saturating — but matching existing code in the same file for consistency is also defensible.

---

## 4. Cargo dependency check

### 4.1 `codelet/sessions/Cargo.toml`

`codelet-providers` IS already a dependency at line 21:

```toml
[dependencies]
codelet-common = { workspace = true }
codelet-tools = { workspace = true }
codelet-providers = { workspace = true }    # ← line 21: ALREADY PRESENT
codelet-cli = { workspace = true }
codelet-git = { workspace = true }
codelet-core = { workspace = true }
codelet-rpc-types = { workspace = true }
```

### 4.2 `codelet/core/Cargo.toml`

`codelet-providers` IS already a dependency at line 12.

### 4.3 Conclusion

**No Cargo.toml change is required.** The crate is already linked and is already in active use within `codelet/sessions/src/handle_impl.rs` at lines 798 and 944 (both call `codelet_providers::custom::list_providers_info()` directly via fully-qualified path).

The top-of-file `use` block at `handle_impl.rs:29-48` does NOT currently include a `use codelet_providers::...` line. The fix can either:

- **Keep the fully-qualified path** (matches existing style at lines 798/944) — zero risk of import collisions
- **Add `use codelet_providers::custom::list_providers_info;`** for brevity — cosmetic preference only

Recommendation: keep the fully-qualified path for minimum diff size and consistency with surrounding code.

---

## 5. Concrete differences table — TS vs Rust vs required change

| Aspect | TS (Ink) | Rust (ratatui, current) | Rust (required) |
|---|---|---|---|
| **Dialog component** | `ModelSelectorScreen.tsx` + `ModelSelectorView.tsx` | `ModelSelectorDialog` (`model_selector_dialog.rs`) | no change |
| **Data source** | NAPI `listProviders()` | tarpc `list_providers()` | no change |
| **Wire format** | `Array<JsProviderInfo>` (9 fields) | `Vec<codelet_rpc_types::ProviderInfo>` (3 fields) | no change to wire format |
| **Backend resolver** | `codelet_providers::custom::list_providers_info()` | `Vec::new()` stub (`handle_impl.rs:709`) | **call `list_providers_info()` + adapt** |
| **Built-in providers** | Returned (6 of them, models empty) | Empty | Returned + lossy-mapped |
| **Custom providers** | Discovered from `~/.fspec/providers/*.json` | Empty | Discovered from `~/.fspec/providers/*.json` |
| **Credential gating** | `available` field; TS filters `available && isCustom` | n/a (always empty) | `available` not in rpc-type; either include all or pre-filter |
| **`is_custom` exposure** | Provider-level `JsProviderInfo.is_custom` | Per-model `ModelEntry.is_custom` only | Propagate parent flag down to every child `ModelEntry` |
| **Display name** | Optional → JS `display_name?` | Required `String` | `display_name.unwrap_or_else(|| name.clone())` |
| **Capability flag** | `supports_thinking` | `supports_reasoning` | rename in adapter |
| **Numeric width** | `usize` → `u32` (NAPI saturating) | `u32` in `ModelEntry` | `u32::try_from(...).unwrap_or(u32::MAX)` |
| **Cargo dep on `codelet-providers`** | n/a | already present (`Cargo.toml:21`) | no change |


---

## 6. Exact patch sites — before/after

### 6.1 Primary change site

**File:** `codelet/sessions/src/handle_impl.rs`
**Lines:** 709-715
**Risk:** Low. The helper is already called twice in the same file; the trait signature is unchanged.

**Before:**

```rust
fn list_providers(&self) -> Vec<codelet_rpc_types::ProviderInfo> {
    // RPC-042 scope: providers registry wiring lives in RPC-054
    // (`/provider` ProviderSettingsView + credentials surface).
    // The trait default returns `Vec::new()`; keep that semantics
    // by returning an empty Vec here too.
    Vec::new()
}
```

**After:**

```rust
fn list_providers(&self) -> Vec<codelet_rpc_types::ProviderInfo> {
    // RPC-073: wire the providers registry into the /model picker.
    // Mirrors the TS Ink path: NAPI `list_providers` →
    // `codelet_providers::custom::list_providers_info()`. The
    // tarpc wire format `codelet_rpc_types::ProviderInfo` is
    // lossy vs. the 9-field providers crate shape, so we map
    // field-by-field here (name→key, display_name unwrap,
    // is_custom propagated to children, supports_thinking
    // → supports_reasoning, usize → u32 saturating).
    match codelet_providers::custom::list_providers_info() {
        Ok(list) => list
            .into_iter()
            .map(|p| {
                let is_custom = p.is_custom;
                codelet_rpc_types::ProviderInfo {
                    key: p.name.clone(),
                    display_name: p.display_name.unwrap_or_else(|| p.name.clone()),
                    models: p
                        .models
                        .into_iter()
                        .map(|m| codelet_rpc_types::ModelEntry {
                            id: m.id.clone(),
                            display_name: m.id,
                            context_window: u32::try_from(m.context_window)
                                .unwrap_or(u32::MAX),
                            supports_reasoning: m.supports_thinking,
                            supports_vision: m.supports_vision,
                            is_custom,
                        })
                        .collect(),
                }
            })
            .collect(),
        Err(e) => {
            tracing::warn!(
                error = %e,
                "list_providers: codelet_providers::custom::list_providers_info failed"
            );
            Vec::new()
        }
    }
}
```

**Justification for each adapter decision:**

- `key: p.name.clone()` — provider slug is the canonical identifier passed back to `set_session_model` (see rpc-types comment at `lib.rs:357-358`).
- `display_name: p.display_name.unwrap_or_else(|| p.name.clone())` — mirrors existing precedent at `handle_impl.rs:803`.
- `models[*].display_name: m.id` — no source field; using `id` is more useful than empty string for human-readable picker labels. (Existing precedent at `:954` uses `String::new()` but only for custom-provider lookups where the caller may format separately.)
- `context_window: u32::try_from(...).unwrap_or(u32::MAX)` — matches NAPI's saturating pattern at `session_bindings.rs:3419`; safer than `as u32` cast which would silently truncate on overflow. Note: the sibling code at `:955` uses `as u32` — if you want absolute consistency with sibling code, use that, but saturating is preferred.
- `supports_reasoning: m.supports_thinking` — same rename used at `:956`.
- `is_custom: is_custom` (parent-inherited) — fixes a latent inconsistency vs. `:958` which hard-codes `true` (correct for that call site because it only handles custom providers).
- Error handling: `tracing::warn!` + empty fallback — mirrors `list_provider_credentials` style at `:816-819`. The trait method returns `Vec<...>` not `Result<...>`, so we cannot propagate errors.

### 6.2 No other code changes required

- ❌ `codelet/sessions/Cargo.toml` — already depends on `codelet-providers`
- ❌ `codelet/core/src/session_manager_handle.rs:130-139` — trait default of `Vec::new()` is correct (it's the fallback for handles without registry wiring; the production impl now overrides it)
- ❌ `codelet/core/src/session_manager_handle.rs:1441-1446` — stub manager override is test-only and intentionally backed by an in-memory `Vec`; leave it alone (it's seeded by parity tests via `set_providers`)
- ❌ `codelet/rpc-types/src/lib.rs` — wire shape unchanged
- ❌ `codelet/rpc/src/lib.rs` — service impl unchanged (delegates to the handle)
- ❌ `codelet/fspec-tui/src/components/model_selector_dialog.rs` — consumer unchanged
- ❌ `codelet/fspec-tui/src/transport/{embedded,websocket}.rs` — transport unchanged

### 6.3 Optional: add `use` shortcut at top of file

**Optional and stylistic only** — adds an import line:

`codelet/sessions/src/handle_impl.rs` (after line 47):

```rust
use crate::session_manager::SessionManager;
// optionally add:
// use codelet_providers::custom::list_providers_info;
```

Then the body becomes `match list_providers_info() { ... }` instead of `match codelet_providers::custom::list_providers_info() { ... }`. Either is acceptable; existing code at `:798, 819, 944, 945` uses the fully-qualified path, so keeping that is more consistent.

---

## 7. Test surface required

### 7.1 Existing source-shape tests (must not break)

`codelet/fspec-tui/tests/source_shape_rpc022.rs` performs substring assertions on source text. The following assertions touch `list_providers`:

| Line | Assertion |
|---|---|
| 85-86 | rpc trait file contains `async fn list_providers() -> Vec<ProviderInfo>` |
| 114-115 | core handle file contains `fn list_providers(&self) -> Vec<ProviderInfo>` |
| 132 | core handle trait default has `Vec::new()` for `list_providers` |
| 146-147 | `handle_impl.rs` contains `async fn list_providers` (NOTE: there is no `async` on the sessions impl — verify whether this is matching a different file or is a stale assertion) |
| 164-165 | `transport/embedded.rs` contains `async fn list_providers` |
| 174-175 | `transport/websocket.rs` contains `async fn list_providers` |

**Impact of the proposed fix:**

- Lines 85-86, 114-115, 164-165, 174-175 — unaffected (signatures unchanged).
- **Line 132** asserts the trait default contains `Vec::new()` for `list_providers`. The proposed fix keeps the trait default `Vec::new()` in `codelet/core/src/session_manager_handle.rs:137`; only the override in `codelet/sessions/src/handle_impl.rs:709` changes. **No breakage.**
- Line 146-147 — check whether this assertion is against `handle_impl.rs` (would still match since the method header `fn list_providers` is present) or against a different file. If it's against `handle_impl.rs` and asserts an `async fn` signature, it's a stale/incorrect assertion (the method is sync), but is a pre-existing issue, not caused by this fix.

**Action:** Re-read `codelet/fspec-tui/tests/source_shape_rpc022.rs` and update only if line 146-147 actually fails after the patch. Likely safe.

### 7.2 New test required — empty stub no longer acceptable

Add a behavioral test that the wired `list_providers` returns a non-empty `Vec` under realistic conditions.

**Location:** `codelet/sessions/tests/handle_impl.rs` (existing file)

**Pattern to follow:**

Existing tests in this file build a real `codelet_sessions::SessionManager` and exercise trait methods through `Arc<dyn SessionManagerHandle>`:

```rust
fn make_handle() -> Arc<dyn SessionManagerHandle> {
    Arc::new(SessionManager::new()) as Arc<dyn SessionManagerHandle>
}
```

**The challenge:** `list_providers_info()` reads `~/.fspec/providers/*.json` and env vars by default. Without `HOME` / `FSPEC_HOME` redirection, the test result is non-deterministic.

**Two options:**

#### Option A — Minimum-viable behavioral test (recommended)

Assert that `list_providers()` returns at least the 6 built-in providers (their existence is unconditional in `list_providers_info()`, regardless of credentials — see `management.rs:99-118`).

```rust
#[tokio::test(flavor = "multi_thread")]
async fn list_providers_returns_at_least_built_ins() {
    let handle = make_handle();
    let providers = handle.list_providers();

    // Six unconditional built-ins per
    // codelet/providers/src/custom/management.rs:99-118
    let built_in_keys = ["claude", "openai", "gemini", "zai", "codex", "github-copilot"];
    for key in built_in_keys {
        assert!(
            providers.iter().any(|p| p.key == key),
            "expected built-in provider {key} to be present; got {:?}",
            providers.iter().map(|p| &p.key).collect::<Vec<_>>()
        );
    }

    // Negative regression: never empty after RPC-073 wiring
    assert!(!providers.is_empty(), "list_providers must not return Vec::new()");
}
```

This test catches the bug (empty vec) without depending on test-environment credentials or filesystem layout, and runs with no fixture setup. It is robust across CI / dev machines.

#### Option B — Full custom-provider test with temp HOME (defer)

Borrow the `EnvGuard` + `TempDir` pattern from `codelet/providers/tests/custom_provider_manager_integration_test.rs`:

```rust
use tempfile::TempDir;
// EnvGuard RAII redirect of HOME + FSPEC_HOME
let home_tmp = TempDir::new().unwrap();
let _home = EnvGuard::set_path("HOME", home_tmp.path());
let _fspec = EnvGuard::set_path("FSPEC_HOME", &credentials_dir);
// write a fake `~/.fspec/providers/my-llm.json`
// ...
let handle = make_handle();
let providers = handle.list_providers();
assert!(providers.iter().any(|p| p.key == "my-llm"));
```

This requires:
- Lifting `EnvGuard` into `codelet/sessions/tests/` (or extracting it to a shared test helper crate).
- Constructing valid custom provider JSON config (consult `codelet/providers/src/custom/discovery.rs` and `config.rs` for the schema).
- Cross-test isolation: env-var mutation is process-global, so this test must run on a serial test harness (`#[serial_test::serial]` or `--test-threads=1`).

Option B exercises more of the path but introduces test infrastructure debt. **Recommend Option A for RPC-073** and a follow-up unit (e.g. RPC-074) to lift `EnvGuard` and add the custom-provider integration test.

### 7.3 Source-shape test update (optional)

If the team wants a static check that the stub is gone, add to `source_shape_rpc022.rs` or a new file `source_shape_rpc073.rs`:

```rust
#[test]
fn list_providers_in_handle_impl_calls_list_providers_info() {
    let src = std::fs::read_to_string("../sessions/src/handle_impl.rs").unwrap();
    assert!(
        src.contains("codelet_providers::custom::list_providers_info()"),
        "handle_impl.rs::list_providers must delegate to codelet_providers"
    );
    // Negative: ensure the old stub is gone
    let stub_marker = "// RPC-042 scope: providers registry wiring lives in RPC-054";
    assert!(
        !src.contains(stub_marker),
        "old RPC-042 stub comment must be removed"
    );
}
```

### 7.4 Quality gate

After applying the patch, run:

```bash
cargo test -p codelet-sessions
cargo test -p codelet-fspec-tui --test source_shape_rpc022
cargo build -p codelet-fspec-tui
```

Optional manual smoke: launch the Rust `fspec` binary, open the model selector, confirm the dialog now shows the 6 built-in provider rows (with `(0 models)` headers) plus any custom providers under `~/.fspec/providers/`.

---

## 8. Summary

| Item | Status |
|---|---|
| **Root cause** | `codelet/sessions/src/handle_impl.rs:709-715` returns `Vec::new()` — stub left over from RPC-042 that RPC-054 never came back to wire |
| **Cargo dep** | `codelet-providers` already in `codelet/sessions/Cargo.toml:21` — **no Cargo change needed** |
| **Wire-type adapter required** | Yes — 9-field `codelet_providers::custom::ProviderInfo` → 3-field `codelet_rpc_types::ProviderInfo`; per-model `ProviderModelInfo` → `ModelEntry` with rename `supports_thinking → supports_reasoning` and `usize → u32` saturation |
| **Code changes** | Exactly one site: replace the stub body at `handle_impl.rs:709-715` with the adapter (snippet in §6.1) |
| **Test changes** | Add a behavioral test asserting 6 built-in providers present; optional source-shape test to prevent regression |
| **Risk** | Low — helper already used twice in the same file; trait signature unchanged; transport / TUI / RPC layers unchanged |
| **Out of scope (defer)** | Pre-filtering on `available` for built-ins (UX decision); `EnvGuard`-based custom-provider integration test (infra debt) |

