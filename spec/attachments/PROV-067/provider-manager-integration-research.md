# PROV-067: Custom Provider — ProviderManager Integration & create_rig_agent

> **Research Document** — Generated from codelet source code analysis
> **Date**: 2026-04-17
> **Sources**: `codelet/providers/src/`, `codelet/napi/src/`, `codelet/cli/src/interactive/`

---

## Table of Contents

1. [ProviderManager Architecture](#1-providermanager-architecture)
2. [create_rig_agent Integration](#2-create_rig_agent-integration)
3. [Agent Loop Integration](#3-agent-loop-integration)
4. [Auto-Discovery Design](#4-auto-discovery-design)
5. [CLI Commands Design](#5-cli-commands-design)
6. [Model Selection](#6-model-selection)
7. [Implementation Checklist](#7-implementation-checklist)

---

## 1. ProviderManager Architecture

**Source**: `codelet/providers/src/manager.rs`

### 1.1 ProviderType Enum (lines 20–30)

The central dispatch discriminant. Every match arm in the codebase switches on this enum.

```rust
// codelet/providers/src/manager.rs:20-30
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderType {
    Claude,
    OpenAI,
    Codex,
    Gemini,
    ZAI,
    /// PROV-053: GitHub Copilot via OAuth device flow
    GitHubCopilot,
}
```

**PROV-067 change**: Add `Custom(String)` variant. Because `ProviderType` currently derives `Copy`, and `String` is not `Copy`, we must either:
- **Option A**: Change to `Clone`-only (remove `Copy` derive), update all call sites.
- **Option B**: Use a newtype like `Custom(u32)` with a lookup table mapping IDs to names.
- **Option C**: Use `Custom(&'static str)` with leaked/interned strings.

**Recommendation**: Option A (remove `Copy`). The enum is not used in hot loops — it's used at session-creation time only. Grep shows `ProviderType` is `Copy`-used in `has_credentials`, `as_str`, `detect_default_provider`, and match arms. All are low-frequency paths.

### 1.2 FromStr Implementation (lines 32–49)

Parses CLI `--provider` flag values:

```rust
// codelet/providers/src/manager.rs:32-49
impl FromStr for ProviderType {
    type Err = ProviderError;

    fn from_str(name: &str) -> Result<Self, ProviderError> {
        match name.to_lowercase().as_str() {
            "claude" => Ok(ProviderType::Claude),
            "openai" => Ok(ProviderType::OpenAI),
            "codex" => Ok(ProviderType::Codex),
            "gemini" => Ok(ProviderType::Gemini),
            "zai" => Ok(ProviderType::ZAI),
            "github-copilot" | "copilot" => Ok(ProviderType::GitHubCopilot),
            _ => Err(ProviderError::config(
                "manager",
                format!("Unknown provider: {name}"),
            )),
        }
    }
}
```

**PROV-067 change**: The `_` catch-all arm must check the custom provider registry before returning an error. If `name` matches a discovered custom provider slug, return `ProviderType::Custom(name.to_string())`.

### 1.3 as_str Method (lines 51–62)

```rust
// codelet/providers/src/manager.rs:51-62
impl ProviderType {
    pub fn as_str(self) -> &'static str {
        match self {
            ProviderType::Claude => "claude",
            ProviderType::OpenAI => "openai",
            ProviderType::Codex => "codex",
            ProviderType::Gemini => "gemini",
            ProviderType::ZAI => "zai",
            ProviderType::GitHubCopilot => "github-copilot",
        }
    }
```

**PROV-067 change**: If `Custom(String)`, this can't return `&'static str`. Options:
- Return a `Cow<'static, str>` (breaking change to signature).
- Leak the string with `Box::leak` (acceptable for a small number of custom providers).
- Change return type to `&str` and have `Custom` hold the string. `&self` → borrow from inner string.

**Recommendation**: Change signature to `pub fn as_str(&self) -> &str` (taking `&self` instead of `self`). The `Custom` variant borrows from its inner `String`. All existing arms return string literals which coerce to `&str`.

### 1.4 has_credentials Method (lines 64–77)

```rust
// codelet/providers/src/manager.rs:64-77
pub fn has_credentials(self, credentials: &ProviderCredentials) -> bool {
    match self {
        ProviderType::Claude => credentials.has_claude(),
        ProviderType::OpenAI => credentials.has_openai(),
        ProviderType::Codex => credentials.has_codex(),
        ProviderType::Gemini => credentials.has_gemini(),
        ProviderType::ZAI => credentials.has_zai(),
        ProviderType::GitHubCopilot => credentials.has_github_copilot(),
    }
}
```

**PROV-067 change**: Add `ProviderType::Custom(ref name) => credentials.has_custom(name)`. This requires extending `ProviderCredentials` (see §1.8).

### 1.5 map_provider_id_to_type (lines 464–479)

Maps models.dev provider IDs to internal `ProviderType`:

```rust
// codelet/providers/src/manager.rs:464-479
fn map_provider_id_to_type(provider_id: &str) -> Result<ProviderType, ProviderError> {
    match provider_id {
        "anthropic" => Ok(ProviderType::Claude),
        "openai" => Ok(ProviderType::OpenAI),
        "google" => Ok(ProviderType::Gemini),
        "zai" | "z-ai" => Ok(ProviderType::ZAI),
        "codex" => Ok(ProviderType::Codex),
        "github-copilot" | "copilot" => Ok(ProviderType::GitHubCopilot),
        _ => Err(ProviderError::config(
            "manager",
            format!(
                "Provider '{provider_id}' is not supported. Supported providers: ..."
            ),
        )),
    }
}
```

**PROV-067 change**: The `_` arm must check the custom provider registry before erroring. Custom providers use their slug as the provider_id (e.g., `my-llm/my-llm-large`).

### 1.6 detect_default_provider (lines 482–509)

```rust
// codelet/providers/src/manager.rs:482-509
fn detect_default_provider(
    credentials: &ProviderCredentials,
) -> Result<ProviderType, ProviderError> {
    // Priority: Claude > Gemini > ZAI > Codex > GitHubCopilot > OpenAI
    if credentials.has_claude()         { return Ok(ProviderType::Claude); }
    if credentials.has_gemini()         { return Ok(ProviderType::Gemini); }
    if credentials.has_zai()            { return Ok(ProviderType::ZAI); }
    if credentials.has_codex()          { return Ok(ProviderType::Codex); }
    if credentials.has_github_copilot() { return Ok(ProviderType::GitHubCopilot); }
    if credentials.has_openai()         { return Ok(ProviderType::OpenAI); }

    Err(ProviderError::auth("manager", "No provider credentials available"))
}
```

**PROV-067 change**: Custom providers should NOT participate in auto-detection priority. They are explicitly selected via `--provider my-llm` or `codelet model my-llm/model-name`. No changes needed here.

### 1.7 provider_limits_resolver (lines 739–789)

Returns a lightweight `ModelLimitsResolver` for the current provider without requiring credentials:

```rust
// codelet/providers/src/manager.rs:739-789
fn provider_limits_resolver(&self) -> Box<dyn ModelLimitsResolver> {
    match self.current_provider {
        ProviderType::Claude => Box::new(ConstantResolver {
            max_ctx: Some(claude::CONTEXT_WINDOW),
            default_ctx: claude::CONTEXT_WINDOW,
            max_out: Some(claude::MAX_OUTPUT_TOKENS),
            default_out: claude::MAX_OUTPUT_TOKENS,
        }),
        ProviderType::OpenAI => { /* reads env vars for defaults */ },
        ProviderType::Gemini => { /* ... */ },
        ProviderType::Codex => { /* ... */ },
        ProviderType::ZAI => { /* ... */ },
        ProviderType::GitHubCopilot => Box::new(ConstantResolver {
            max_ctx: None,
            default_ctx: copilot::CONTEXT_WINDOW,
            max_out: None,
            default_out: copilot::MAX_OUTPUT_TOKENS,
        }),
    }
}
```

**PROV-067 change**: Add `ProviderType::Custom(_)` arm. Custom providers trust registry/user-override values (no hard ceiling), with defaults from the provider definition JSON:

```rust
ProviderType::Custom(ref _name) => Box::new(ConstantResolver {
    max_ctx: None,  // Trust user/registry values
    default_ctx: self.user_context_window.unwrap_or(128_000),
    max_out: None,
    default_out: self.user_max_output_tokens.unwrap_or(4096),
}),
```

### 1.8 ProviderCredentials (credentials.rs)

**Source**: `codelet/providers/src/credentials.rs`

```rust
// codelet/providers/src/credentials.rs:8-16
pub struct ProviderCredentials {
    pub claude_available: bool,
    pub openai_available: bool,
    pub codex_available: bool,
    pub gemini_available: bool,
    pub zai_available: bool,
    pub github_copilot_available: bool,
}
```

Detection happens in `detect()` (line 20) by checking env vars and auth files:
- Claude: `ANTHROPIC_API_KEY` | `CLAUDE_CODE_OAUTH_TOKEN` | `~/.fspec/credentials/claude_auth.json`
- OpenAI: `OPENAI_API_KEY`
- Codex: `~/.codex/auth.json`
- Gemini: `GOOGLE_GENERATIVE_AI_API_KEY`
- ZAI: `ZAI_PLAN_API_KEY` | `ZAI_API_KEY`
- Copilot: `~/.fspec/credentials/copilot_auth.json`

**PROV-067 change**: Add `custom_available: HashMap<String, bool>` field. The `detect()` method scans `~/.fspec/providers/*.json` and `.fspec/providers/*.json` for custom provider definitions, checking each one's credential requirements (env var or auth file).

```rust
// New method needed
pub fn has_custom(&self, name: &str) -> bool {
    self.custom_available.get(name).copied().unwrap_or(false)
}
```

### 1.9 ProviderManager Struct (lines 112–143)

```rust
// codelet/providers/src/manager.rs:112-143
pub struct ProviderManager {
    credentials: ProviderCredentials,
    current_provider: ProviderType,
    model_registry: Option<ModelRegistry>,
    selected_model: Option<String>,
    registry_context_window: Option<usize>,
    registry_max_output_tokens: Option<usize>,
    user_context_window: Option<usize>,
    user_max_output_tokens: Option<usize>,
    facade_override: Option<String>,
    compaction_threshold_override: Option<(String, u64)>,
}
```

### 1.10 Constructors

| Constructor | Line | Purpose |
|---|---|---|
| `new()` | 159 | Auto-detect credentials, select default provider |
| `with_provider(name)` | 187 | Explicit provider selection by name |
| `with_provider_and_model(provider, model, ctx, max_out)` | 227 | For internal operations (compaction) |
| `with_model_support()` | 267 | Async, initializes ModelCache + ModelRegistry |
| `for_testing(provider, ctx, max_out)` | 900 | Test-only, no credentials |

### 1.11 Factory Methods (get_*())

Each factory method follows this pattern:
1. Guard: `if self.current_provider != ProviderType::X { return Err(...) }`
2. Get model ID: `self.selected_model_id().ok_or_else(|| ...)?`
3. Get credentials from env/file
4. Construct provider: `XProvider::from_api_key(...)` or equivalent

| Method | Line | Returns | Notes |
|---|---|---|---|
| `get_claude()` | 521 | `ClaudeProvider` | Checks OAuth first, falls back to env var |
| `get_openai(session_id)` | 554 | `OpenAIProvider` | Takes session_id for cache optimization |
| `get_codex()` | 580 | `CodexProvider` | Sets CODEX_MODEL env var from selected model |
| `get_gemini()` | 599 | `GeminiProvider` | Reads GOOGLE_GENERATIVE_AI_API_KEY |
| `get_zai()` | 930 | `ZAIProvider` | Checks ZAI_PLAN_API_KEY first, then ZAI_API_KEY |
| `get_github_copilot()` | 638 | `CopilotProvider` | Reads copilot_auth.json, determines deployment type |

**PROV-067 change**: Add `get_custom(&self) -> Result<CustomProvider, ProviderError>`. This method reads the custom provider JSON definition, extracts credentials from the configured env var or auth file, and constructs a `CustomProvider` that wraps an OpenAI-compatible client.

### 1.12 selected_model / facade_override (MODEL-004)

```rust
// codelet/providers/src/manager.rs:384-413
pub fn set_model_direct(
    &mut self,
    provider_id: &str,
    model_id: &str,
    context_window: Option<usize>,
    max_output_tokens: Option<usize>,
    facade_override: Option<String>,
) -> Result<(), ProviderError> {
    let provider_type = Self::map_provider_id_to_type(provider_id)?;
    self.current_provider = provider_type;
    self.selected_model = Some(model_id.to_string());
    self.user_context_window = context_window;
    self.user_max_output_tokens = max_output_tokens;
    self.registry_context_window = None;
    self.registry_max_output_tokens = None;
    self.facade_override = facade_override;
    Ok(())
}
```

The `facade_override` field (line 136) controls agent loop dispatch. When set, the agent loop dispatches to the facade provider instead of `current_provider`. This is critical for PROV-067 — a custom provider `my-llm` with `facade: "openai"` will use the OpenAI tool schema and agent construction, but route API calls to the custom base URL.

```rust
// codelet/napi/src/session_manager.rs:4845-4851
let provider = inner.provider_manager()
    .facade_override()
    .map(|s| s.to_string())
    .unwrap_or_else(|| inner.current_provider_name().to_string());
```

---

## 2. create_rig_agent Integration

Every provider implements `create_rig_agent` with the **same signature**:

```rust
pub fn create_rig_agent(
    &self,
    session_id: uuid::Uuid,
    preamble: Option<&str>,
    thinking_config: Option<serde_json::Value>,
) -> rig::agent::Agent<COMPLETION_MODEL_TYPE>
```

The return type varies per provider because each uses a different `CompletionModel` generic parameter. The `run_with_provider!` macro handles this by being generic over the return type.

### 2.1 ClaudeProvider::create_rig_agent

**Source**: `codelet/providers/src/claude.rs:507-595`

**Tools registered** (19 tools):

| # | Tool | Constructor |
|---|---|---|
| 1 | `ReadTool` | `ReadTool::new(session_id)` |
| 2 | `WriteTool` | `WriteTool::new(session_id)` |
| 3 | `EditTool` | `EditTool::new(session_id)` |
| 4 | `BashTool` | `BashTool::new(session_id)` |
| 5 | `GrepTool` | `GrepTool::new(session_id)` |
| 6 | `GlobTool` | `GlobTool::new(session_id)` |
| 7 | `LsTool` | `LsTool::new(session_id)` |
| 8 | `AstGrepTool` | `AstGrepTool::new(session_id)` |
| 9 | `AstGrepRefactorTool` | `AstGrepRefactorTool::new(session_id)` |
| 10 | FspecTool | `claude_fspec_tool(session_id)` |
| 11 | BridgeTool | `claude_bridge_tool(session_id)` |
| 12 | WebSearchTool | `FacadeToolWrapper::new(Arc::new(ClaudeWebSearchFacade), session_id)` |
| 13 | `ConnectMcpTool` | `ConnectMcpTool::new(session_id)` |
| 14 | `SessionSearchTool` | `SessionSearchTool::new(session_id)` |
| 15 | `GraphSearchTool` | `GraphSearchTool::new(session_id)` |
| 16 | `InjectSummaryTool` | `InjectSummaryTool::new(session_id)` |
| 17 | `DeepSearchTool` | `DeepSearchTool::new(session_id)` |
| 18 | `AgentManagerTool` | `AgentManagerTool::new(session_id)` |
| 19 | `RequestUserInputTool` | `RequestUserInputTool::new(session_id)` |
| 20 | `ScheduleTool` | `ScheduleTool::new(session_id)` |

**System prompt**: Uses `select_claude_facade(is_oauth)` for cache_control formatting. OAuth mode adds `CLAUDE_CODE_PROMPT_PREFIX`.

**Thinking config**: Merged into `additional_params` alongside `system` field:
```rust
// claude.rs:577-592
let mut additional = json!({ "system": cached_system });
if let Some(thinking) = thinking_config {
    if let Some(obj) = additional.as_object_mut() {
        if let Some(thinking_obj) = thinking.as_object() {
            for (key, value) in thinking_obj {
                obj.insert(key.clone(), value.clone());
            }
        }
    }
}
agent_builder = agent_builder.additional_params(additional);
```

### 2.2 OpenAIProvider::create_rig_agent

**Source**: `codelet/providers/src/openai.rs:410-464`

**Tools registered**: Same 20 tools as Claude, EXCEPT:
- Uses `openai_fspec_tool(session_id)` instead of `claude_fspec_tool`
- Uses `openai_bridge_tool(session_id)` instead of `claude_bridge_tool`
- Uses `WebSearchTool::new(session_id)` directly (no facade wrapper)

**System prompt**: Uses `prepend_fspec_guidance(preamble_text)` (BUG-120).

**Thinking config**: `_thinking_config` parameter is IGNORED (prefixed with `_`).

**No additional_params** — no generation config set.

### 2.3 GeminiProvider::create_rig_agent

**Source**: `codelet/providers/src/gemini.rs:130-273`

**Tools registered**: Uses Gemini-specific facade wrappers for ALL file/search/bash/web tools:

| # | Tool Name | Facade |
|---|---|---|
| 1 | `read_file` | `GeminiReadFileFacade` via `FileToolFacadeWrapper` |
| 2 | `write_file` | `GeminiWriteFileFacade` via `FileToolFacadeWrapper` |
| 3 | `replace` | `GeminiReplaceFacade` via `FileToolFacadeWrapper` |
| 4 | `run_shell_command` | `GeminiRunShellCommandFacade` via `BashToolFacadeWrapper` |
| 5 | `search_file_content` | `GeminiSearchFileContentFacade` via `SearchToolFacadeWrapper` |
| 6 | `find_files` | `GeminiGlobFacade` via `SearchToolFacadeWrapper` |
| 7 | `list_directory` | `GeminiListDirectoryFacade` via `LsToolFacadeWrapper` |
| 8 | `AstGrepTool` | Direct |
| 9 | `AstGrepRefactorTool` | Direct |
| 10 | FspecTool | `gemini_fspec_tool(session_id)` |
| 11 | BridgeTool | `gemini_bridge_tool(session_id)` |
| 12 | `google_web_search` | `GeminiGoogleWebSearchFacade` via `FacadeToolWrapper` |
| 13 | `web_fetch` | `GeminiWebFetchFacade` via `FacadeToolWrapper` |
| 14–20 | ConnectMcp, SessionSearch, GraphSearch, InjectSummary, DeepSearch, AgentManager, RequestUserInput, Schedule | Direct |

**System prompt**: Uses `build_gemini_system_prompt(&self.model_name, preamble)` — model-aware (Gemini 3 vs 2.5).

**Thinking config**: Extracts `thinkingConfig` from the JSON, applies defaults for Gemini 3 models (`thinkingLevel: "high"`).

**additional_params**: Always set with generation config:
```rust
// gemini.rs:255-268
let mut gen_config = json!({
    "temperature": 1.0,
    "topP": 0.95,
    "topK": 64
});
// Add thinking config if present
let generation_config = json!({ "generationConfig": gen_config });
agent_builder = agent_builder.additional_params(generation_config);
```

### 2.4 ZAIProvider::create_rig_agent

**Source**: `codelet/providers/src/zai.rs:218-311`

**Tools**: Uses Z.AI-specific facade wrappers (similar pattern to Gemini):
- `ZAIReadFileFacade`, `ZAIWriteFileFacade`, `ZAIEditFileFacade` via `FileToolFacadeWrapper`
- `ZAIRunCommandFacade` via `BashToolFacadeWrapper`
- `ZAIGrepFilesFacade`, `ZAIFindFilesFacade` via `SearchToolFacadeWrapper`
- `ZAIListDirFacade` via `LsToolFacadeWrapper`
- Direct: AstGrep, AstGrepRefactor, WebSearch, etc.
- Uses `zai_fspec_tool` / `zai_bridge_tool`

**System prompt**: Uses `prepend_fspec_guidance(preamble_text)` (same as OpenAI — BUG-120).

**Thinking config**: `_thinking_config` IGNORED.

**additional_params**: Generation config with temperature and top_p:
```rust
// zai.rs:304-308
let generation_config = json!({ "temperature": 1.0, "top_p": 0.95 });
agent_builder = agent_builder.additional_params(generation_config);
```

### 2.5 CodexProvider::create_rig_agent

**Source**: `codelet/providers/src/codex/mod.rs:331-456`

**Tools**: Uses Codex-native facades (most different tool set):

| # | Tool Name | Facade |
|---|---|---|
| 1 | `shell_command` | `CodexShellCommandFacade` via `BashToolFacadeWrapper` |
| 2 | `read_file` | `CodexReadFileFacade` via `FileToolFacadeWrapper` |
| 3 | `view_image` | `CodexViewImageFacade` via `FileToolFacadeWrapper` |
| 4 | `list_dir` | `CodexListDirFacade` via `LsToolFacadeWrapper` |
| 5 | `grep_files` | `CodexGrepFilesFacade` via `SearchToolFacadeWrapper` |
| 6 | `shell` | `CodexShellFacade` via `ExecToolFacadeWrapper` |
| 7 | `exec_command` | `CodexExecCommandFacade` via `ExecToolFacadeWrapper` |
| 8 | `write_stdin` | `CodexWriteStdinFacade` via `ExecToolFacadeWrapper` |
| 9 | `apply_patch` | `ApplyPatchTool::new(session_id)` — replaces Write+Edit |
| 10 | `AstGrepTool` | Direct |
| 11 | `AstGrepRefactorTool` | Direct |
| 12 | `WebSearchTool` | Direct |
| 13 | FspecTool | `codex_fspec_tool(session_id)` |
| 14 | BridgeTool | `codex_bridge_tool(session_id)` |
| 15 | `request_user_input` | `CodexRequestUserInputFacade` via `HitlToolFacadeWrapper` |
| 16–20 | ConnectMcp, SessionSearch, GraphSearch, InjectSummary, DeepSearch, AgentManager, Schedule | Direct |

**Key differences**: No `WriteTool`, `EditTool`, `GlobTool` — uses `apply_patch` instead. Does NOT set `.max_tokens()` — Codex API rejects `max_output_tokens`.

**System prompt**: Always includes `CODEX_BASE_INSTRUCTIONS` (required by backend API). Role appended after.

**Thinking config**: CONSUMED via `build_reasoning_params()`:
```rust
// codex/mod.rs:287-314
fn build_reasoning_params(thinking_config: Option<&serde_json::Value>) -> serde_json::Value {
    let mut params = json!({
        "store": false,
        "include": ["reasoning.encrypted_content"],
    });
    // Merge reasoning config or default to effort: "high", summary: "auto"
    // ...
}
```

### 2.6 CopilotProvider::create_rig_agent

**Source**: `codelet/providers/src/copilot/rig_agent.rs:56-115`

**Tools**: Mirrors OpenAI exactly — same 20 tools with same constructors. Uses `openai_fspec_tool` / `openai_bridge_tool`.

**System prompt**: Uses `prepend_fspec_guidance(preamble_text)` (same as OpenAI/ZAI).

**Thinking config**: `_thinking_config` IGNORED.

**No additional_params**.

**Return type**: `rig::agent::Agent<openai::completion::CompletionModel<CopilotHttpClient>>` — note the `CopilotHttpClient` type parameter, which ensures requests go through the Copilot middleware.

### 2.7 Summary: Tool Registration Patterns

| Provider | Tool Schema Style | Fspec/Bridge Facade | WebSearch | System Prompt | Thinking | additional_params |
|---|---|---|---|---|---|---|
| Claude | Standard names | `claude_*` | ClaudeWebSearchFacade | `select_claude_facade()` | Merged into params | `system` + thinking |
| OpenAI | Standard names | `openai_*` | Direct WebSearchTool | `prepend_fspec_guidance()` | Ignored | None |
| Gemini | Gemini-native names | `gemini_*` | Google facades | `build_gemini_system_prompt()` | `generationConfig.thinkingConfig` | `generationConfig` |
| ZAI | ZAI-native names | `zai_*` | Direct WebSearchTool | `prepend_fspec_guidance()` | Ignored | `temperature`, `top_p` |
| Codex | Codex-native names | `codex_*` | Direct WebSearchTool | `CODEX_BASE_INSTRUCTIONS` | `reasoning` params | `store`, `include`, `reasoning` |
| Copilot | Standard names (=OpenAI) | `openai_*` | Direct WebSearchTool | `prepend_fspec_guidance()` | Ignored | None |

**PROV-067 Implication**: Custom providers should use `facade_override` to select which tool schema style to use. A custom provider with `"facade": "openai"` reuses the OpenAI tool set; `"facade": "claude"` reuses the Claude tool set, etc.

---

## 3. Agent Loop Integration

### 3.1 NAPI Agent Loop (Primary Path)

**Source**: `codelet/napi/src/session_manager.rs`

The NAPI agent loop is the primary dispatch path used by the TUI. It uses the `run_with_provider!` macro.

#### The run_with_provider! Macro (lines 4270–4335)

```rust
// codelet/napi/src/session_manager.rs:4270-4335
macro_rules! run_with_provider {
    ($inner:expr, $getter:ident, $input:expr, $images:expr, $session:expr, $output:expr, $thinking:expr) => {
        match $inner.provider_manager_mut().$getter() {
            Ok(provider) => {
                tracing::debug!(
                    "[run_with_provider] Creating agent - session={}, getter={}",
                    $session.id, stringify!($getter)
                );
                let mcp_wrappers = codelet_tools::gather_mcp_tool_wrappers($session.id);
                let role_preamble = $session.get_role();
                let agent = provider.create_rig_agent(
                    $session.id,
                    role_preamble.as_deref(),
                    $thinking.clone()
                );
                // MCP-001: Add dynamic MCP tools post-build
                if !mcp_wrappers.is_empty() {
                    for wrapper in mcp_wrappers {
                        if let Err(e) = agent.tool_server_handle.add_tool(wrapper).await {
                            tracing::warn!("[MCP] Failed to add MCP tool: {}", e);
                        }
                    }
                }
                // MCP-002: Store handle for mid-turn registration
                codelet_tools::set_mcp_tool_server_handle($session.id, agent.tool_server_handle.clone());
                
                let agent = codelet_core::RigAgent::with_default_depth(agent);
                codelet_cli::interactive::run_agent_stream_with_images(
                    agent, $input, $images, $inner,
                    $session.is_interrupted.clone(),
                    $session.compaction_in_progress.clone(),
                    $session.interrupt_notify.clone(),
                    $output,
                ).await
            }
            Err(e) => {
                tracing::warn!("[run_with_provider] Failed to get provider: {}", e);
                Err(anyhow::anyhow!("Failed to get provider: {}", e))
            }
        }
    };
}
```

#### The Dispatch Match (lines 5336–5425)

```rust
// codelet/napi/src/session_manager.rs:5336-5425
let result = match current_provider.as_str() {
    "claude" => run_with_provider!(&mut inner_session, get_claude, input, ...),
    "openai" => {
        // Special case: get_openai requires session_id parameter
        match inner_session.provider_manager_mut().get_openai(session.id) {
            Ok(provider) => {
                let agent = provider.create_rig_agent(session.id, role_preamble.as_deref(), thinking_config_value.clone());
                // ... same MCP + stream logic
            }
            Err(e) => { ... }
        }
    },
    "gemini" => run_with_provider!(&mut inner_session, get_gemini, input, ...),
    "zai" => run_with_provider!(&mut inner_session, get_zai, input, ...),
    "codex" => run_with_provider!(&mut inner_session, get_codex, input, ...),
    "github-copilot" | "copilot" => run_with_provider!(
        &mut inner_session, get_github_copilot, input, ...
    ),
    _ => {
        tracing::error!("Unsupported provider: {}", current_provider);
        Err(anyhow::anyhow!("Unsupported provider: {}", current_provider))
    }
};
```

#### facade_override Resolution (lines 4845–4855)

Before entering the match, the agent loop resolves `facade_override`:

```rust
// codelet/napi/src/session_manager.rs:4845-4855
let (current_provider, current_model) = {
    let inner = session.inner.lock().await;
    let provider = inner.provider_manager()
        .facade_override()
        .map(|s| s.to_string())
        .unwrap_or_else(|| inner.current_provider_name().to_string());
    let model = inner.current_model_id().map(|s| s.to_string());
    (provider, model)
};
```

**This is the critical insight for PROV-067**: The `facade_override` mechanism already exists and works. A custom provider `my-llm` with `facade: "openai"` will:
1. Set `current_provider = ProviderType::Custom("my-llm")`
2. Set `facade_override = Some("openai")`
3. At dispatch time, `current_provider` string resolves to `"openai"` (from facade_override)
4. The `"openai"` match arm fires
5. `get_openai(session_id)` constructs an OpenAI provider — but it reads `OPENAI_BASE_URL` and `OPENAI_API_KEY` from env, which were set by `set_model_direct()` from the profile

**This means custom providers using OpenAI-compatible APIs (vLLM, Ollama, LM Studio) already work via `set_model_direct()` with facade_override!** PROV-067's job is to provide the JSON definition format and auto-discovery, not to add a new agent loop arm.

### 3.2 CLI Agent Loop (Secondary Path)

**Source**: `codelet/cli/src/interactive/agent_runner.rs`

Simpler version for the CLI REPL, with the same pattern but fewer match arms:

```rust
// codelet/cli/src/interactive/agent_runner.rs:62-83
match provider_name.as_str() {
    "claude" => run_with_provider!(get_claude, None),
    "openai" => {
        let provider = manager.get_openai(session_id)?;
        // ... manual expansion
    },
    "codex" => run_with_provider!(get_codex, None),
    "gemini" => run_with_provider!(get_gemini, None),
    _ => Err(anyhow::anyhow!("Unknown provider")),
}
```

**Note**: The CLI agent_runner is missing ZAI and GitHubCopilot arms (the `_` catch-all returns error). PROV-067 changes for the CLI should add custom provider support through the facade_override path, but the CLI is a secondary concern — the NAPI path is primary.

### 3.3 What Needs to Change for Custom(String)

Given that `facade_override` already routes custom providers through existing match arms, the changes for PROV-067 are minimal:

1. **No new match arm needed** in the dispatch — facade_override handles routing.
2. **ProviderType::Custom** is needed for `has_credentials`, `as_str`, `provider_limits_resolver`, `list_available_providers`.
3. **`set_model_direct()`** already accepts `facade_override` — custom provider JSON specifies which facade to use.
4. **`session_set_model_profile()`** in NAPI already passes facade_override through.

The only scenario requiring a new match arm is if a custom provider has `facade: null` (no facade — uses its own tool schema). This would require a new `"custom"` arm that constructs a generic OpenAI-compatible agent. For MVP, we can require `facade` to be one of the existing providers.

---

## 4. Auto-Discovery Design

### 4.1 Provider Definition JSON Schema

Custom providers are defined in JSON files at two locations, with project-local overriding user-global:

```
~/.fspec/providers/*.json          # User-global
.fspec/providers/*.json            # Project-local (overrides user-global)
```

Proposed schema for a custom provider definition:

```json
{
  "$schema": "https://fspec.dev/schemas/custom-provider.json",
  "name": "my-llm",
  "displayName": "My Local LLM",
  "description": "vLLM server running Llama 3.1 70B",
  "facade": "openai",
  "baseUrl": "http://localhost:8888/v1",
  "apiKeyEnvVar": "MY_LLM_API_KEY",
  "models": [
    {
      "id": "llama-3.1-70b",
      "name": "Llama 3.1 70B",
      "contextWindow": 131072,
      "maxOutputTokens": 4096,
      "capabilities": {
        "toolCall": true,
        "reasoning": false,
        "vision": false
      }
    },
    {
      "id": "qwen-2.5-coder-32b",
      "name": "Qwen 2.5 Coder 32B",
      "contextWindow": 32768,
      "maxOutputTokens": 8192,
      "capabilities": {
        "toolCall": true,
        "reasoning": false,
        "vision": false
      }
    }
  ],
  "defaults": {
    "temperature": 0.7,
    "topP": 0.95
  }
}
```

### 4.2 Discovery Mechanism

The discovery scan should happen at two points:

1. **`ProviderCredentials::detect()`** — scan for provider definitions and check their `apiKeyEnvVar` is set.
2. **`ProviderManager::new()` / `with_model_support()`** — load definitions and make them available for model selection.

```rust
// Proposed: codelet/providers/src/custom/discovery.rs

use std::path::PathBuf;
use std::collections::HashMap;

pub struct CustomProviderDefinition {
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub facade: String,                // "openai" | "claude" | "gemini" | etc.
    pub base_url: String,
    pub api_key_env_var: String,
    pub models: Vec<CustomModelDef>,
    pub defaults: Option<serde_json::Value>,
}

pub struct CustomModelDef {
    pub id: String,
    pub name: String,
    pub context_window: usize,
    pub max_output_tokens: usize,
    pub capabilities: CustomCapabilities,
}

pub struct CustomCapabilities {
    pub tool_call: bool,
    pub reasoning: bool,
    pub vision: bool,
}

/// Discover custom provider definitions from filesystem.
/// Project-local (.fspec/providers/) overrides user-global (~/.fspec/providers/).
pub fn discover_custom_providers(
    project_root: Option<&std::path::Path>,
) -> HashMap<String, CustomProviderDefinition> {
    let mut providers = HashMap::new();
    
    // 1. Scan user-global first
    if let Some(home) = dirs::home_dir() {
        let global_dir = home.join(".fspec").join("providers");
        scan_provider_dir(&global_dir, &mut providers);
    }
    
    // 2. Scan project-local (overrides user-global)
    if let Some(root) = project_root {
        let local_dir = root.join(".fspec").join("providers");
        scan_provider_dir(&local_dir, &mut providers);
    }
    
    providers
}

fn scan_provider_dir(
    dir: &std::path::Path,
    providers: &mut HashMap<String, CustomProviderDefinition>,
) {
    if !dir.is_dir() { return; }
    
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(def) = serde_json::from_str::<CustomProviderDefinition>(&content) {
                        // Project-local overrides user-global (insert replaces)
                        providers.insert(def.name.clone(), def);
                    }
                }
            }
        }
    }
}
```

### 4.3 ProviderCredentials Integration

```rust
// In ProviderCredentials::detect():
pub fn detect() -> Self {
    let custom_providers = discover_custom_providers(/* project root */);
    let mut custom_available = HashMap::new();
    
    for (name, def) in &custom_providers {
        // Check if the required env var is set
        let available = std::env::var(&def.api_key_env_var)
            .map(|v| !v.is_empty())
            .unwrap_or(false);
        custom_available.insert(name.clone(), available);
    }
    
    Self {
        claude_available: /* ... */,
        // ...
        custom_available,
    }
}
```

### 4.4 How `codelet providers list` Shows Them

Custom providers appear alongside built-in providers:

```
Available Providers:
  ✓ Claude (/claude)           [ANTHROPIC_API_KEY]
  ✓ Gemini (/gemini)           [GOOGLE_GENERATIVE_AI_API_KEY]
  ✗ OpenAI (/openai)           [OPENAI_API_KEY not set]
  ✓ My LLM (/my-llm)          [MY_LLM_API_KEY] (custom)
    Models: llama-3.1-70b, qwen-2.5-coder-32b
    Base URL: http://localhost:8888/v1
    Facade: openai
```

---

## 5. CLI Commands Design

### 5.1 Existing Command Patterns

NAPI bindings are exposed via `#[napi]` annotated async functions in `codelet/napi/src/session_manager.rs`. The TypeScript TUI calls these bindings. There is no separate CLI subcommand system for provider management — the CLI REPL uses the same `ProviderManager` API.

For model selection, the NAPI layer exposes:
- `session_set_model_profile()` — set provider/model directly (for profiles)
- `session_select_model()` — select from registry (for models.dev models)
- Model listing via `codelet/napi/src/models/napi_bindings.rs`

### 5.2 Proposed Provider Management Commands

These would be implemented as NAPI bindings callable from TypeScript:

#### `providers list`

```typescript
// TypeScript side (calls NAPI)
const providers = await listProviders();
// Returns: Array<{ name, displayName, available, isCustom, models?, baseUrl?, facade? }>
```

NAPI binding:
```rust
#[napi]
pub async fn list_providers() -> Result<Vec<NapiProviderInfo>> {
    // Combine built-in providers from ProviderCredentials
    // with custom providers from discover_custom_providers()
}
```

#### `providers show <name>`

```typescript
const info = await showProvider("my-llm");
// Returns: { name, displayName, description, facade, baseUrl, apiKeyEnvVar, models, defaults }
```

#### `providers validate <name>`

Validates a custom provider definition:
1. JSON schema validation
2. Checks `apiKeyEnvVar` is set
3. Checks `baseUrl` is reachable (HTTP GET to health endpoint)
4. Checks `facade` is a valid provider name

#### `providers test <name>`

Tests connectivity:
1. Calls the `/v1/models` endpoint at `baseUrl`
2. Verifies at least one model from the definition is served
3. Sends a simple completion request to verify tool calling works

#### `providers init <name> --template <template>`

Creates a new provider definition:

```bash
codelet providers init my-llm --template vllm
# Creates: .fspec/providers/my-llm.json
```

Templates:
- `vllm` — vLLM server (OpenAI-compatible)
- `ollama` — Ollama server
- `lmstudio` — LM Studio
- `openai-compatible` — Generic OpenAI-compatible API
- `blank` — Empty template

---

## 6. Model Selection

### 6.1 How `codelet model my-llm/my-llm-large` Should Work

The model selection string format is `provider/model-id`. For custom providers:

```
codelet model my-llm/llama-3.1-70b
```

#### Flow through the codebase:

1. **TypeScript TUI** calls `session_set_model_profile()` NAPI binding (line 6828):

```rust
// codelet/napi/src/session_manager.rs:6828
pub async fn session_set_model_profile(
    session_id: String,
    provider_id: String,     // "my-llm"
    model_id: String,        // "llama-3.1-70b"
    context_window: Option<u32>,     // from provider JSON
    max_output_tokens: Option<u32>,  // from provider JSON
    facade_override: Option<String>, // "openai" from provider JSON
    compaction_threshold_type: Option<String>,
    compaction_threshold_value: Option<u32>,
) -> Result<()>
```

2. **`set_model_direct()`** is called (manager.rs line 384):

```rust
// This already handles the custom provider case via facade_override
inner.provider_manager_mut().set_model_direct(
    &provider_id,       // "my-llm"  (or "openai" if facade handles routing)
    &model_id,          // "llama-3.1-70b"
    context_window,
    max_output_tokens,
    facade_override,    // Some("openai")
)?;
```

3. **`map_provider_id_to_type()`** needs to recognize "my-llm" → `ProviderType::Custom("my-llm")`.

4. **At dispatch time**, facade_override resolves "my-llm" → "openai", the "openai" arm fires, and `get_openai()` constructs an OpenAI provider reading `OPENAI_BASE_URL` (set to the custom provider's `baseUrl`) and `OPENAI_API_KEY` (set from the custom provider's `apiKeyEnvVar`).

### 6.2 Existing select_model() Path (Registry Models)

For models.dev registry models, the flow is different:

```rust
// codelet/providers/src/manager.rs:308-360
pub fn select_model(&mut self, model_string: &str) -> Result<&ModelInfo, ProviderError> {
    self.credentials = ProviderCredentials::detect(); // Re-detect credentials
    let registry = self.model_registry.as_ref()?;
    let (provider_id, model_id) = registry.parse_model_string(model_string)?;
    let provider_type = Self::map_provider_id_to_type(&provider_id)?;
    
    if !provider_type.has_credentials(&self.credentials) { return Err(...); }
    
    let model_info = registry.validate_model_for_use(&provider_id, &model_id)?;
    
    self.registry_context_window = Some(model_info.limit.context as usize);
    self.registry_max_output_tokens = Some(model_info.limit.output as usize);
    self.current_provider = provider_type;
    self.selected_model = Some(model_string.to_string());
    
    Ok(model_info)
}
```

Custom providers bypass this path entirely — they use `set_model_direct()` because they don't appear in the models.dev registry.

### 6.3 Environment Variable Setup

When a custom provider is selected, the TypeScript layer (or the Rust provider manager) must set the appropriate env vars before `get_openai()` (or other facade provider) is called:

```rust
// For a custom provider with facade: "openai"
std::env::set_var("OPENAI_API_KEY", &custom_api_key);
std::env::set_var("OPENAI_BASE_URL", &custom_base_url);
std::env::set_var("OPENAI_MODEL", &model_id);
std::env::set_var("OPENAI_CONTEXT_WINDOW", &context_window.to_string());
std::env::set_var("OPENAI_MAX_OUTPUT_TOKENS", &max_output_tokens.to_string());
```

This pattern is already used by profile-based models (see `session_set_model_profile()`).

---

## 7. Implementation Checklist

### Phase 1: Core Types & Discovery

- [ ] Create `codelet/providers/src/custom/mod.rs` with `CustomProviderDefinition` struct
- [ ] Create `codelet/providers/src/custom/discovery.rs` with `discover_custom_providers()`
- [ ] Add JSON schema validation for provider definitions
- [ ] Add `Custom(String)` variant to `ProviderType` (remove `Copy` derive)
- [ ] Update all `ProviderType` match arms:
  - [ ] `FromStr::from_str()` — check custom registry on `_` arm
  - [ ] `as_str()` — change to `&self` return, borrow from inner String
  - [ ] `has_credentials()` — delegate to `credentials.has_custom(name)`
  - [ ] `map_provider_id_to_type()` — check custom registry on `_` arm
  - [ ] `provider_limits_resolver()` — add Custom arm with defaults from definition
- [ ] Extend `ProviderCredentials` with `custom_available: HashMap<String, bool>`
- [ ] Add `has_custom(&self, name: &str) -> bool` method

### Phase 2: ProviderManager Integration

- [ ] Update `list_available_providers()` to include custom providers
- [ ] Ensure `set_model_direct()` works with custom provider names
- [ ] Ensure facade_override routes custom providers through existing match arms
- [ ] Add `get_custom(&self) -> Result<CustomProvider, ProviderError>` factory (for facade:null case)

### Phase 3: NAPI Bindings

- [ ] Add `list_providers()` NAPI binding
- [ ] Add `show_provider(name)` NAPI binding
- [ ] Add `validate_provider(name)` NAPI binding
- [ ] Add `test_provider(name)` NAPI binding
- [ ] Add `init_provider(name, template)` NAPI binding
- [ ] Update `session_set_model_profile()` to handle custom providers

### Phase 4: Agent Loop

- [ ] Verify facade_override dispatch works end-to-end with custom providers
- [ ] Add `register_deep_search_handler` support for custom providers
- [ ] Add `register_agent_manager_handler` support for custom providers
- [ ] Update CLI agent_runner.rs if needed

### Phase 5: Testing

- [ ] Unit tests for `discover_custom_providers()`
- [ ] Unit tests for ProviderType::Custom in all match arms
- [ ] Integration test: custom provider → facade_override → OpenAI dispatch
- [ ] Integration test: `codelet model my-llm/model-name` end-to-end
