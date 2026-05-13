# RPC-018 — SessionHeader + SessionFooter widgets

## TypeScript reference

### SessionHeader
`src/tui/components/SessionHeader.tsx` (228 lines)

Visible in `~/Desktop/typescript-agent-view.png` top-left:
```
#1: Claude Opus 4.7 [R] [V] [192k] [T:High]                tokens: 0↓ 0↑ [0%]
```

Layout:
- **Left:**
  - `#N:` — 1-based index of the current session among all open sessions.
  - Model display name (e.g. `Claude Opus 4.7`).
  - `[R]` — Reasoning-capable indicator (badge color depends on model).
  - `[V]` — Vision-capable indicator.
  - `[192k]` — Context window size (compact form).
  - `[T:High]` — Thinking level (Off / Low / Medium / High).
- **Right:**
  - `tokens: <input>↓ <output>↑` — token deltas for the current turn.
  - `[N%]` — context fill percentage (input tokens / context window).

Data sources:
- Session index: `useSessionStreamManager.sessions` array position.
- Model name: `useModelStore.getModel(sessionId)`.
- R/V/context: `useModelStore.getCapabilities(modelId)` — loaded from
  `models/provider_models.json`.
- Thinking level: `getThinkingConfig(sessionId)` NAPI call.
- Token counts: parsed from `StreamChunk` events via `extractTokenStateFromChunks`
  (`src/tui/utils/tokenStateUtils.ts`).

### SessionFooter
`src/tui/components/SessionFooter.tsx` (74 lines)

Visible bottom-right of `~/Desktop/typescript-agent-view.png`:
```
~/projects/fspec [⌥ codelet-integration]
```

Layout:
- **Left:** input hints (`'Shift+↑/↓' history | 'Shift+←/→' sessions | 'Tab' select turn`).
- **Right:**
  - cwd (shortened with `~` for home).
  - `[⌥ <git-branch>]` — current git branch with a unicode `⌥` glyph.

Data sources:
- cwd: `process.cwd()` (TS) → would be passed in as a config.
- Git branch: read from `.git/HEAD` and resolved to a ref name.

## Current Rust state

`codelet/fspec-tui/src/views/agent.rs:191-194` renders a single-line
title:
```rust
let title = match store.current_session() {
    Some(sid) => format!(" Agent — {} ", sid.value),
    None => " Agent ".to_string(),
};
```

No model info, no token counts, no thinking level, no cwd, no git branch.

## Target Rust behavior

### New AgentViewStore fields

```rust
pub struct AgentViewStore {
    // existing fields...
    sessions: Vec<SessionMeta>,            // for #N:M display
    model_info_by_session: HashMap<SessionId, ModelInfo>,
    token_state_by_session: HashMap<SessionId, TokenState>,
    thinking_level_by_session: HashMap<SessionId, ThinkingLevel>,
    cwd: PathBuf,
    git_branch: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub display_name: String,
    pub supports_reasoning: bool,
    pub supports_vision: bool,
    pub context_window: u32,
}

#[derive(Debug, Clone, Default)]
pub struct TokenState {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub context_fill_pct: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThinkingLevel { Off, Low, Medium, High }
```

### New FspecBackend methods

```rust
#[async_trait]
pub trait FspecBackend: Send + Sync {
    // existing methods...

    /// RPC-018: list model capabilities so the SessionHeader can paint
    /// the [R] / [V] / [Nk] badges. Mirrors `useModelStore.getCapabilities`.
    async fn get_model_info(&self, session_id: SessionId) -> Result<ModelInfo>;

    /// RPC-018: return the current thinking level for the session.
    /// Mirrors `getThinkingConfig` NAPI call.
    async fn get_thinking_level(&self, session_id: SessionId) -> Result<ThinkingLevel>;

    /// RPC-018: return the current working directory + git branch
    /// (or None if not a git repo). Mirrors the SessionFooter cwd+branch.
    async fn get_workspace_info(&self) -> Result<WorkspaceInfo>;
}

#[derive(Debug, Clone)]
pub struct WorkspaceInfo {
    pub cwd: String,
    pub git_branch: Option<String>,
}
```

### New shared types

`codelet/rpc-types/src/lib.rs`:
```rust
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub display_name: String,
    pub supports_reasoning: bool,
    pub supports_vision: bool,
    pub context_window: u32,
}

#[cfg_attr(feature = "napi", napi_derive::napi(string_enum))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThinkingLevel { Off, Low, Medium, High }

#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub cwd: String,
    pub git_branch: Option<String>,
}
```

### Token-state derivation from chunks

Mirror `src/tui/utils/tokenStateUtils.ts::extractTokenStateFromChunks`.
The Rust App already receives `StreamChunk` events in
`Action::ChunkReceived` (`codelet/fspec-tui/src/app/dispatch.rs:34-39`).
Extend `record_chunk` in `codelet/fspec-tui/src/views/agent.rs` to also
update `AgentViewStore.token_state_by_session` when the chunk carries
token-usage metadata (e.g. `StreamChunk::Usage` or `StreamChunk::Done`
with embedded counts — confirm exact variants in specifying phase).

### New SessionHeader + SessionFooter widgets

`codelet/fspec-tui/src/views/agent/header.rs`:
```rust
pub struct SessionHeader<'a> {
    pub session_index: usize,    // 1-based
    pub session_count: usize,
    pub model: &'a ModelInfo,
    pub thinking: ThinkingLevel,
    pub tokens: TokenState,
}

impl<'a> Widget for SessionHeader<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) { ... }
}
```

`codelet/fspec-tui/src/views/agent/footer.rs`:
```rust
pub struct SessionFooter<'a> {
    pub workspace: &'a WorkspaceInfo,
}

impl<'a> Widget for SessionFooter<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) { ... }
}
```

### AgentView render layout

Reorganize `AgentView::render_with_store`:
```
┌─────────────────────────────────────────────────────────────┐
│ Header (1 row): #1: Claude... [R] [V] [192k] [T:High]  tokens: ... │
├─────────────────────────────────────────────────────────────┤
│ Scrollback (flex)                                           │
│                                                             │
├─────────────────────────────────────────────────────────────┤
│ Input (3 rows)                                              │
├─────────────────────────────────────────────────────────────┤
│ Footer (1 row): hints                ~/projects/fspec [⌥ branch] │
└─────────────────────────────────────────────────────────────┘
```

The current single-line input box from RPC-012 lands inside this layout
as-is — the input widget overhaul ships in RPC-019.

## RPC/NAPI boundary contract

```
TS AgentView.tsx → useModelStore.getCapabilities + getThinkingConfig + readGitBranch
                  → already wired via existing NAPI exports

Rust TUI → FspecBackend::get_model_info / get_thinking_level / get_workspace_info
       → FspecService::get_model_info / get_thinking_level / get_workspace_info [tarpc]
       → codelet_core::models::get_capabilities(session_id) [shared impl]
       → codelet_core::session::get_thinking_level(session_id)
       → codelet_git::get_branch(cwd)
```

NAPI exports stay additive:
- `napi::get_model_info(sessionId)` — wraps `codelet_core::models::get_capabilities`.
- `napi::get_workspace_info(cwd)` — wraps `codelet_git::get_branch` + cwd.
- `napi::get_thinking_level` — already exists (`thinking_config.rs`).

## Existing TypeScript behavior preserved

- `src/tui/components/SessionHeader.tsx` — UNCHANGED.
- `src/tui/components/SessionFooter.tsx` — UNCHANGED.
- `src/tui/utils/tokenStateUtils.ts` — UNCHANGED.
- `src/tui/store/modelStore.ts` — UNCHANGED.

## Acceptance criteria sketch

- A 1-row header is visible at the top of the Rust AgentView with the
  format `#N: <model display name> [R] [V] [Nk] [T:<level>]` on the left.
- The header right side shows `tokens: <in>↓ <out>↑ [<fill>%]`.
- A 1-row footer is visible at the bottom of the Rust AgentView with
  input hints on the left and `<cwd> [⌥ <branch>]` on the right (or
  just `<cwd>` if not a git repo).
- `cwd` is shortened with `~` when it lives inside `$HOME`.
- Three new RPC methods exist (`get_model_info`, `get_thinking_level`,
  `get_workspace_info`) and are tested against both transports.
- Token counts update in real time as `StreamChunk::Done` (or equivalent)
  events arrive.
- Existing TS AgentView still renders unchanged.
