# PROV-065: Custom Provider Rhai-Scriptable System Prompts — Research Document

## Table of Contents

1. [Existing SystemPromptFacade Trait](#1-existing-systempromptfacade-trait)
2. [How System Prompts Flow Through create_rig_agent](#2-how-system-prompts-flow-through-create_rig_agent)
3. [RhaiSystemPromptFacade Design](#3-rhaisystempromptfacade-design)
4. [Rhai Dynamic → serde_json::Value Conversion](#4-rhai-dynamic--serde_jsonvalue-conversion)
5. ['static Lifetime Handling](#5-static-lifetime-handling)

---

## 1. Existing SystemPromptFacade Trait

### 1.1 Trait Definition

**File:** `codelet/tools/src/facade/system_prompt.rs` (lines 215–237)

```rust
pub trait SystemPromptFacade: Send + Sync {
    /// Returns the provider this facade is for (e.g., "claude", "gemini", "openai")
    fn provider(&self) -> &'static str;

    /// Returns the identity prefix if required for this provider/auth mode.
    /// For Claude OAuth, returns Some("You are Claude Code...").
    /// For other providers/modes, returns None.
    fn identity_prefix(&self) -> Option<&'static str>;

    /// Transform the preamble according to provider requirements.
    /// This may prepend an identity prefix (for Claude OAuth) or
    /// pass through unchanged (for other providers).
    fn transform_preamble(&self, preamble: &str) -> String;

    /// Format the system prompt for the provider's API.
    /// Returns the properly formatted Value for the provider:
    /// - Claude: JSON array with cache_control blocks
    /// - Gemini/OpenAI: Plain string
    fn format_for_api(&self, preamble: &str) -> Value;
}
```

**Key observations:**
- `Send + Sync` bound — required because facades are stored in `BoxedSystemPromptFacade = Box<dyn SystemPromptFacade>` and shared across threads.
- `provider()` returns `&'static str` — a compile-time string identifying the provider.
- `identity_prefix()` returns `Option<&'static str>` — **this is the problematic method for Rhai** because Rhai scripts produce owned `String`, not `&'static str`.
- `transform_preamble()` returns owned `String` — no lifetime issues.
- `format_for_api()` returns `serde_json::Value` — either a JSON string or a JSON array with cache_control blocks.

### 1.2 fspec Workflow Guidance Injection

**File:** `codelet/tools/src/facade/system_prompt.rs` (lines 35–41)

```rust
pub fn prepend_fspec_guidance(preamble: &str) -> String {
    if preamble.trim().is_empty() {
        FSPEC_WORKFLOW_GUIDANCE.to_string()
    } else {
        format!("{FSPEC_WORKFLOW_GUIDANCE}\n\n{preamble}")
    }
}
```

**File:** `codelet/tools/src/fspec_workflow_guidance.rs` (line 12)

```rust
pub const FSPEC_WORKFLOW_GUIDANCE: &str = r#"<system-reminder>
<!-- type:fspecWorkflow -->
# fspec - Acceptance Criteria Driven Development (ACDD)
..."#;
```

The `FSPEC_WORKFLOW_GUIDANCE` constant is approximately 1,150 lines of ACDD workflow documentation. It is **always** prepended to system prompts across all providers. This is called by every facade's `transform_preamble()` and `format_for_api()`.

### 1.3 ClaudeOAuthSystemPromptFacade

**File:** `codelet/tools/src/facade/system_prompt.rs` (lines 248–304)

```rust
pub struct ClaudeOAuthSystemPromptFacade;

impl SystemPromptFacade for ClaudeOAuthSystemPromptFacade {
    fn provider(&self) -> &'static str { "claude" }

    fn identity_prefix(&self) -> Option<&'static str> {
        Some(CLAUDE_CODE_PROMPT_PREFIX) // "You are Claude Code, Anthropic's official CLI for Claude."
    }

    fn transform_preamble(&self, preamble: &str) -> String {
        let with_fspec = prepend_fspec_guidance(preamble);
        format!("{CLAUDE_CODE_PROMPT_PREFIX}\n\n{with_fspec}")
    }

    fn format_for_api(&self, preamble: &str) -> Value {
        let with_fspec = prepend_fspec_guidance(preamble);
        if preamble.trim().is_empty() {
            // 2 blocks: prefix (no cache_control) + fspec guidance (with cache_control)
            json!([
                { "type": "text", "text": CLAUDE_CODE_PROMPT_PREFIX },
                { "type": "text", "text": FSPEC_WORKFLOW_GUIDANCE,
                  "cache_control": { "type": "ephemeral" } }
            ])
        } else {
            // 2 blocks: prefix (no cache_control) + combined preamble (with cache_control)
            json!([
                { "type": "text", "text": CLAUDE_CODE_PROMPT_PREFIX },
                { "type": "text", "text": with_fspec,
                  "cache_control": { "type": "ephemeral" } }
            ])
        }
    }
}
```

**Key:** Returns a JSON **array** with `cache_control` metadata blocks. The first block is the identity prefix without caching; the second is the combined fspec guidance + user preamble with `cache_control: { type: "ephemeral" }`.

### 1.4 ClaudeApiKeySystemPromptFacade

**File:** `codelet/tools/src/facade/system_prompt.rs` (lines 315–344)

```rust
pub struct ClaudeApiKeySystemPromptFacade;

impl SystemPromptFacade for ClaudeApiKeySystemPromptFacade {
    fn provider(&self) -> &'static str { "claude" }
    fn identity_prefix(&self) -> Option<&'static str> { None }

    fn transform_preamble(&self, preamble: &str) -> String {
        prepend_fspec_guidance(preamble)
    }

    fn format_for_api(&self, preamble: &str) -> Value {
        let with_fspec = prepend_fspec_guidance(preamble);
        json!([{
            "type": "text",
            "text": with_fspec,
            "cache_control": { "type": "ephemeral" }
        }])
    }
}
```

**Key:** Single block with `cache_control`. No identity prefix. Still returns a JSON array (not a plain string).

### 1.5 GeminiSystemPromptFacade

**File:** `codelet/tools/src/facade/system_prompt.rs` (lines 356–377)

```rust
pub struct GeminiSystemPromptFacade;

impl SystemPromptFacade for GeminiSystemPromptFacade {
    fn provider(&self) -> &'static str { "gemini" }
    fn identity_prefix(&self) -> Option<&'static str> { None }

    fn transform_preamble(&self, preamble: &str) -> String {
        let with_fspec = prepend_fspec_guidance(preamble);
        format!("{with_fspec}\n{GEMINI_WEB_TOOL_GUIDANCE}")
    }

    fn format_for_api(&self, preamble: &str) -> Value {
        Value::String(self.transform_preamble(preamble))
    }
}
```

**Key:** Returns `Value::String(...)` — a plain string. Appends web tool guidance after fspec guidance.

### 1.6 OpenAISystemPromptFacade

**File:** `codelet/tools/src/facade/system_prompt.rs` (lines 390–408)

```rust
pub struct OpenAISystemPromptFacade;

impl SystemPromptFacade for OpenAISystemPromptFacade {
    fn provider(&self) -> &'static str { "openai" }
    fn identity_prefix(&self) -> Option<&'static str> { None }

    fn transform_preamble(&self, preamble: &str) -> String {
        prepend_fspec_guidance(preamble)
    }

    fn format_for_api(&self, preamble: &str) -> Value {
        Value::String(prepend_fspec_guidance(preamble))
    }
}
```

**Key:** Simplest facade. Returns `Value::String(...)`. No prefix, no extra guidance.

### 1.7 CopilotSystemPromptFacade

**File:** `codelet/providers/src/copilot/system_prompt_facade.rs` (lines 1–63)

```rust
/// Chat-completions: thin alias for OpenAISystemPromptFacade
pub type CopilotChatCompletionsSystemPromptFacade = OpenAISystemPromptFacade;

/// Responses endpoint: distinct identity but delegates to OpenAI
pub struct CopilotResponsesSystemPromptFacade;

impl SystemPromptFacade for CopilotResponsesSystemPromptFacade {
    fn provider(&self) -> &'static str { "copilot-responses" }
    fn identity_prefix(&self) -> Option<&'static str> { OpenAISystemPromptFacade.identity_prefix() }
    fn transform_preamble(&self, preamble: &str) -> String {
        OpenAISystemPromptFacade.transform_preamble(preamble)
    }
    fn format_for_api(&self, preamble: &str) -> Value {
        OpenAISystemPromptFacade.format_for_api(preamble)
    }
}

pub fn system_prompt_facade_for_endpoint(endpoint: CopilotEndpoint) -> BoxedSystemPromptFacade {
    match endpoint {
        CopilotEndpoint::ChatCompletions => Box::new(OpenAISystemPromptFacade),
        CopilotEndpoint::Responses => Box::new(CopilotResponsesSystemPromptFacade),
    }
}
```

**Key:** Both Copilot endpoints are OpenAI-wire-compatible and fully delegate to `OpenAISystemPromptFacade`. The `/responses` path gets its own `provider()` string for test distinguishability.

### 1.8 build_gemini_system_prompt()

**File:** `codelet/tools/src/facade/system_prompt.rs` (lines 163–190)

```rust
pub fn build_gemini_system_prompt(model_name: &str, user_preamble: Option<&str>) -> String {
    let is_gemini_3 = model_name.contains("gemini-3");
    let mut prompt = GEMINI_BASE_SYSTEM_PROMPT.to_string();  // ~140 lines of instructions

    if is_gemini_3 {
        // Insert Gemini 3 tool-silence instruction
        prompt = prompt.replace(
            "- **Do Not revert changes:**",
            &format!("{GEMINI_3_TOOL_INSTRUCTION}\n- **Do Not revert changes:**"),
        );
    }

    prompt.push_str("\n\n");
    prompt.push_str(FSPEC_WORKFLOW_GUIDANCE);

    if let Some(preamble) = user_preamble {
        if !preamble.trim().is_empty() {
            prompt.push_str("\n\n# Project-Specific Instructions\n\n");
            prompt.push_str(preamble);
        }
    }
    prompt
}
```

**Key:** Model-version-aware builder. Gemini gets a **completely different base system prompt** (`GEMINI_BASE_SYSTEM_PROMPT`, ~140 lines) rather than just prepending fspec guidance. The `GeminiSystemPromptFacade` does NOT use this function — it's used directly by `GeminiProvider::create_rig_agent()`.

### 1.9 Facade Selector

**File:** `codelet/tools/src/facade/system_prompt.rs` (lines 427–433)

```rust
pub fn select_claude_facade(is_oauth: bool) -> BoxedSystemPromptFacade {
    if is_oauth {
        Box::new(ClaudeOAuthSystemPromptFacade)
    } else {
        Box::new(ClaudeApiKeySystemPromptFacade)
    }
}
```

---

## 2. How System Prompts Flow Through create_rig_agent

Every provider implements `create_rig_agent(session_id, preamble, thinking_config)` which constructs a `rig::agent::Agent`. The system prompt flow differs significantly between Claude (array format with `additional_params` override) and all other providers (plain string via `preamble()`).

### 2.1 Claude: Array Format with additional_params Override

**File:** `codelet/providers/src/claude.rs` (lines 507–595)

```rust
pub fn create_rig_agent(
    &self,
    session_id: uuid::Uuid,
    preamble: Option<&str>,
    thinking_config: Option<serde_json::Value>,
) -> rig::agent::Agent<ClaudeCompletionModel> {
    // ... tool registration (lines 530-556) ...

    // Step 1: Select facade based on OAuth status
    let is_oauth = self.is_oauth_mode();
    let facade = select_claude_facade(is_oauth);
    let preamble_text = preamble.unwrap_or("");

    // Step 2: Transform preamble for rig's internal text handling
    let effective_preamble = facade.transform_preamble(preamble_text);
    agent_builder = agent_builder.preamble(&effective_preamble);

    // Step 3: Build structured array format with cache_control via facade
    let cached_system = facade.format_for_api(preamble_text);

    // Step 4: Override rig's system field via additional_params
    let mut additional = json!({ "system": cached_system });

    // Step 5: Merge thinking config if provided
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
    agent_builder.build()
}
```

**Critical flow for Claude:**
1. `facade.transform_preamble()` → sets rig's internal preamble as a plain string (rig uses this for context but it gets **overridden**)
2. `facade.format_for_api()` → produces the JSON array with `cache_control` blocks
3. The array is placed under `"system"` key in `additional_params`
4. rig's `additional_params` **overrides** the `system` field in the final API request body, replacing rig's default plain-string system prompt with the structured array format
5. Thinking config (e.g., `{"thinking": {"type": "adaptive"}}`) is merged into the same `additional_params` object

**This is the key insight for PROV-065:** The `format_for_api()` return value becomes the literal `"system"` field in the Anthropic API request. For Claude, this MUST be a JSON array of text blocks with `cache_control`.

### 2.2 OpenAI: Plain String via preamble()

**File:** `codelet/providers/src/openai.rs` (lines 410–464)

```rust
pub fn create_rig_agent(
    &self,
    session_id: uuid::Uuid,
    preamble: Option<&str>,
    _thinking_config: Option<serde_json::Value>,
) -> rig::agent::Agent<openai::completion::CompletionModel> {
    // ... tool registration (lines 428-451) ...

    // BUG-120: Always set system prompt with fspec guidance
    {
        use codelet_tools::facade::prepend_fspec_guidance;
        let preamble_text = preamble.unwrap_or("");
        let effective_preamble = prepend_fspec_guidance(preamble_text);
        agent_builder = agent_builder.preamble(&effective_preamble);
    }

    agent_builder.build()
}
```

**Key difference:** OpenAI does NOT use `additional_params` for the system prompt. It uses rig's `.preamble()` method directly, which produces a plain string system message. No `format_for_api()` call needed.

### 2.3 Gemini: build_gemini_system_prompt() + additional_params for generationConfig

**File:** `codelet/providers/src/gemini.rs` (lines 130–273)

```rust
pub fn create_rig_agent(
    &self,
    session_id: uuid::Uuid,
    preamble: Option<&str>,
    thinking_config: Option<serde_json::Value>,
) -> rig::agent::Agent<gemini::completion::CompletionModel> {
    // ... tool registration (lines 190-214) ...

    // Build complete system prompt using model-aware builder
    let system_prompt = build_gemini_system_prompt(&self.model_name, preamble);
    agent_builder = agent_builder.preamble(&system_prompt);

    // Apply generation config (temperature, topP, topK, thinkingConfig)
    let generation_config = serde_json::json!({
        "generationConfig": gen_config
    });
    agent_builder = agent_builder.additional_params(generation_config);

    agent_builder.build()
}
```

**Key:** Gemini uses `build_gemini_system_prompt()` instead of any facade's `format_for_api()`. The system prompt is a plain string via `.preamble()`. `additional_params` is used for `generationConfig` (thinking, temperature), NOT for the system prompt itself.

### 2.4 Z.AI: Same as OpenAI

**File:** `codelet/providers/src/zai.rs` (lines 218–311)

```rust
pub fn create_rig_agent(&self, session_id: uuid::Uuid, preamble: Option<&str>,
    _thinking_config: Option<serde_json::Value>,
) -> rig::agent::Agent<openai::completion::CompletionModel> {
    // ... tool registration ...

    // BUG-120: Same fspec guidance prepend as OpenAI
    {
        use codelet_tools::facade::prepend_fspec_guidance;
        let preamble_text = preamble.unwrap_or("");
        let effective_preamble = prepend_fspec_guidance(preamble_text);
        agent_builder = agent_builder.preamble(&effective_preamble);
    }

    // Generation config for GLM models
    let generation_config = serde_json::json!({
        "temperature": 1.0,
        "top_p": 0.95
    });
    agent_builder = agent_builder.additional_params(generation_config);

    agent_builder.build()
}
```

### 2.5 Codex: Custom CODEX_BASE_INSTRUCTIONS

**File:** `codelet/providers/src/codex/mod.rs` (lines 331–456)

```rust
pub fn create_rig_agent(&self, session_id: uuid::Uuid, preamble: Option<&str>,
    thinking_config: Option<serde_json::Value>,
) -> rig::agent::Agent<CodexResponsesModel> {
    // ... tool registration ...

    // Codex requires non-empty instructions. Uses its own base instructions.
    let effective_preamble = match preamble {
        Some(role) if !role.trim().is_empty() => {
            format!("{CODEX_BASE_INSTRUCTIONS}\n\n{role}")
        }
        _ => CODEX_BASE_INSTRUCTIONS.to_string(),
    };
    agent_builder = agent_builder.preamble(&effective_preamble);

    // Codex needs store:false + reasoning params
    let additional = Self::build_reasoning_params(thinking_config.as_ref());
    agent_builder = agent_builder.additional_params(additional);

    agent_builder.build()
}
```

**Key:** Codex uses its own `CODEX_BASE_INSTRUCTIONS` constant instead of `FSPEC_WORKFLOW_GUIDANCE`.

### 2.6 Copilot: Identical to OpenAI

**File:** `codelet/providers/src/copilot/rig_agent.rs` (lines 56–115)

```rust
pub fn create_rig_agent(&self, session_id: uuid::Uuid, preamble: Option<&str>,
    _thinking_config: Option<serde_json::Value>,
) -> rig::agent::Agent<openai::completion::CompletionModel<CopilotHttpClient>> {
    // ... identical tool set as OpenAI ...

    let preamble_text = preamble.unwrap_or("");
    let effective_preamble = prepend_fspec_guidance(preamble_text);
    agent_builder = agent_builder.preamble(&effective_preamble);

    agent_builder.build()
}
```

### 2.7 Summary: System Prompt Flow Patterns

| Provider | System Prompt Mechanism | format_for_api Used? | additional_params for system? |
|----------|------------------------|---------------------|------------------------------|
| **Claude OAuth** | Array with cache_control blocks | ✅ Yes | ✅ `{"system": [...]}` overrides rig |
| **Claude API Key** | Array with cache_control blocks | ✅ Yes | ✅ `{"system": [...]}` overrides rig |
| **OpenAI** | Plain string via `.preamble()` | ❌ No | ❌ No |
| **Gemini** | Plain string via `build_gemini_system_prompt()` | ❌ No | Only for `generationConfig` |
| **Z.AI** | Plain string via `.preamble()` | ❌ No | Only for temperature/top_p |
| **Codex** | Plain string via `.preamble()` | ❌ No | Only for reasoning/store |
| **Copilot** | Plain string via `.preamble()` | ❌ No | ❌ No |

**Critical implication for PROV-065:** The `RhaiSystemPromptFacade` must handle BOTH patterns:
1. **String format** — return `Value::String(...)` for OpenAI/Gemini/Z.AI/Copilot
2. **Array format** — return a JSON array with `cache_control` blocks for Claude

The Rhai script author decides which format via the return type of `format_system_prompt()`.

---

## 3. RhaiSystemPromptFacade Design

### 3.1 Struct Definition

```rust
use rhai::{AST, Dynamic, Engine, Map, Scope};
use serde_json::Value;
use std::sync::Arc;

/// Rhai-scriptable system prompt facade for custom providers (PROV-065).
///
/// Delegates SystemPromptFacade methods to optional Rhai script functions.
/// When a script function is not defined, falls back to sensible defaults.
///
/// # Script Functions (all optional)
///
/// - `provider()` → String — Provider identifier (default: "custom")
/// - `identity_prefix()` → String or () — Optional identity prefix
/// - `transform_preamble(preamble)` → String — Transform the preamble
/// - `format_system_prompt(preamble)` → String or Array of Maps — Format for API
pub struct RhaiSystemPromptFacade {
    engine: Arc<Engine>,
    ast: AST,
    /// Leaked provider name for 'static lifetime (see Section 5)
    provider_name: &'static str,
    /// Leaked identity prefix for 'static lifetime (see Section 5)
    identity_prefix_str: Option<&'static str>,
}
```

### 3.2 Checking if Optional Rhai Functions Exist

The Rhai `Module::get_script_fn()` method is the primary mechanism for checking function existence in a compiled AST.

**File:** `/tmp/rhai/src/module/mod.rs` (lines 1333–1345)

```rust
pub fn get_script_fn(
    &self,
    name: impl AsRef<str>,
    num_params: usize,
) -> Option<&Shared<crate::ast::ScriptFuncDef>> {
    self.functions.as_ref().and_then(|lib| {
        let name = name.as_ref();
        lib.values()
            .find(|(_, f)| f.num_params == num_params && f.name == name)
            .and_then(|(f, _)| f.get_script_fn_def())
    })
}
```

However, `Module::get_script_fn()` requires access to the AST's internal shared library module. The public API path is:

**File:** `/tmp/rhai/src/ast/ast.rs` (lines 204–214)

```rust
/// Get the internal shared Module containing all script-defined functions.
/// Exported under the `internals` feature only.
#[expose_under_internals]
#[cfg(not(feature = "no_function"))]
const fn shared_lib(&self) -> &crate::SharedModule { &self.lib }
```

**⚠️ Problem:** `shared_lib()` is gated behind `#[expose_under_internals]`, meaning it requires `feature = "internals"` to access.

**Solution: Use the public `iter_functions()` API instead:**

**File:** `/tmp/rhai/src/ast/ast.rs` (lines 673–677)

```rust
pub fn iter_functions(&self) -> impl Iterator<Item = super::ScriptFnMetadata<'_>> {
    self.lib
        .iter_script_fn()
        .map(|(.., fn_def)| fn_def.as_ref().into())
}
```

Where `ScriptFnMetadata` has:

**File:** `/tmp/rhai/src/ast/script_fn.rs` (lines 103–139)

```rust
pub struct ScriptFnMetadata<'a> {
    pub name: &'a str,
    pub params: Vec<&'a str>,
    pub access: FnAccess,
    #[cfg(not(feature = "no_object"))]
    pub this_type: Option<&'a str>,
    // ...
}
```

**Recommended check pattern:**

```rust
/// Check if a Rhai script defines a function with the given name and parameter count.
fn has_script_fn(ast: &AST, name: &str, num_params: usize) -> bool {
    ast.iter_functions()
        .any(|f| f.name == name && f.params.len() == num_params)
}
```

**Alternative (if `internals` feature is enabled):**

```rust
fn has_script_fn_internals(ast: &AST, name: &str, num_params: usize) -> bool {
    ast.shared_lib().get_script_fn(name, num_params).is_some()
}
```

### 3.3 Constructor with Eager Evaluation

Since `provider()` and `identity_prefix()` return `&'static str`, these values must be computed **once at construction time** and leaked (see Section 5). The constructor calls the Rhai functions eagerly:

```rust
impl RhaiSystemPromptFacade {
    pub fn new(engine: Arc<Engine>, ast: AST) -> Result<Self, anyhow::Error> {
        // Eagerly evaluate provider name
        let provider_name_string = if has_script_fn(&ast, "provider", 0) {
            let mut scope = Scope::new();
            let result: Dynamic = engine
                .call_fn(&mut scope, &ast, "provider", ())
                .map_err(|e| anyhow::anyhow!("provider() failed: {e}"))?;
            result.into_string()
                .map_err(|_| anyhow::anyhow!("provider() must return a string"))?
        } else {
            "custom".to_string()
        };

        // Eagerly evaluate identity_prefix
        let identity_prefix_string = if has_script_fn(&ast, "identity_prefix", 0) {
            let mut scope = Scope::new();
            let result: Dynamic = engine
                .call_fn(&mut scope, &ast, "identity_prefix", ())
                .map_err(|e| anyhow::anyhow!("identity_prefix() failed: {e}"))?;
            if result.is_unit() {
                None
            } else {
                Some(result.into_string()
                    .map_err(|_| anyhow::anyhow!("identity_prefix() must return a string or ()"))?
                )
            }
        } else {
            None
        };

        // Leak strings for 'static lifetime (see Section 5)
        let provider_name: &'static str = Box::leak(provider_name_string.into_boxed_str());
        let identity_prefix_str: Option<&'static str> = identity_prefix_string
            .map(|s| -> &'static str { Box::leak(s.into_boxed_str()) });

        Ok(Self {
            engine,
            ast,
            provider_name,
            identity_prefix_str,
        })
    }
}
```

### 3.4 Fallback Behavior When Functions Are Not Defined

| Rhai Function | Parameters | Default When Missing |
|---------------|-----------|---------------------|
| `provider()` | 0 | `"custom"` |
| `identity_prefix()` | 0 | `None` |
| `transform_preamble(preamble)` | 1 | `prepend_fspec_guidance(preamble)` |
| `format_system_prompt(preamble)` | 1 | `Value::String(prepend_fspec_guidance(preamble))` |

### 3.5 SystemPromptFacade Implementation

```rust
impl SystemPromptFacade for RhaiSystemPromptFacade {
    fn provider(&self) -> &'static str {
        self.provider_name
    }

    fn identity_prefix(&self) -> Option<&'static str> {
        self.identity_prefix_str
    }

    fn transform_preamble(&self, preamble: &str) -> String {
        if has_script_fn(&self.ast, "transform_preamble", 1) {
            let mut scope = Scope::new();
            match self.engine.call_fn::<String>(
                &mut scope, &self.ast, "transform_preamble",
                (preamble.to_string(),)
            ) {
                Ok(result) => result,
                Err(e) => {
                    tracing::warn!("Rhai transform_preamble() failed: {e}, using default");
                    prepend_fspec_guidance(preamble)
                }
            }
        } else {
            prepend_fspec_guidance(preamble)
        }
    }

    fn format_for_api(&self, preamble: &str) -> Value {
        if has_script_fn(&self.ast, "format_system_prompt", 1) {
            let mut scope = Scope::new();
            match self.engine.call_fn::<Dynamic>(
                &mut scope, &self.ast, "format_system_prompt",
                (preamble.to_string(),)
            ) {
                Ok(result) => dynamic_to_json_value(&result),
                Err(e) => {
                    tracing::warn!("Rhai format_system_prompt() failed: {e}, using default");
                    Value::String(prepend_fspec_guidance(preamble))
                }
            }
        } else {
            Value::String(prepend_fspec_guidance(preamble))
        }
    }
}
```

### 3.6 Detecting String vs. Structured Map Return Values

The `format_system_prompt()` Rhai function can return either:

1. **A string** — for OpenAI/Gemini-style plain string system prompts
2. **An array of maps** — for Claude-style structured cache_control blocks

The `dynamic_to_json_value()` function (already in the codebase) handles this transparently:

```rust
// If script returns: "Hello world"
// → serde_json::Value::String("Hello world")

// If script returns: [#{ type: "text", text: "Hello", cache_control: #{ type: "ephemeral" } }]
// → serde_json::Value::Array([{"type":"text","text":"Hello","cache_control":{"type":"ephemeral"}}])
```

**Example Rhai script for Claude-style:**

```rhai
fn format_system_prompt(preamble) {
    [
        #{
            type: "text",
            text: "You are a helpful assistant.",
        },
        #{
            type: "text",
            text: preamble,
            cache_control: #{ type: "ephemeral" },
        },
    ]
}
```

**Example Rhai script for OpenAI-style:**

```rhai
fn format_system_prompt(preamble) {
    "You are a helpful assistant.\n\n" + preamble
}
```

### 3.7 config.resolved_tools Population Timing

The `create_rig_agent` flow registers all tools BEFORE the system prompt facade is invoked. Looking at every provider's `create_rig_agent()`:

1. **Tool registration** happens first (`.tool(ReadTool::new(session_id))` etc.)
2. **System prompt** is set after all tools are wired

This means that by the time `format_for_api()` or `transform_preamble()` is called, the tool set is already determined. However, the facade itself does NOT receive a list of tool names — it only receives the `preamble` string.

If a Rhai script needs to know which tools are available (e.g., to include tool-specific instructions), the information must be passed via:
- The `preamble` parameter (tool names embedded in the preamble text)
- A custom scope variable set before calling the Rhai function
- A separate configuration map passed during `RhaiSystemPromptFacade` construction

**Recommended approach:** Add an optional `config` map to the constructor that gets injected into the Rhai scope before each function call:

```rust
pub struct RhaiSystemPromptFacade {
    engine: Arc<Engine>,
    ast: AST,
    provider_name: &'static str,
    identity_prefix_str: Option<&'static str>,
    /// Optional configuration map available to all script functions as `config`
    config: Option<Map>,
}
```

Then in each method:

```rust
fn format_for_api(&self, preamble: &str) -> Value {
    let mut scope = Scope::new();
    if let Some(ref config) = self.config {
        scope.push("config", Dynamic::from_map(config.clone()));
    }
    // ... call format_system_prompt ...
}
```

This aligns with the existing `ScriptedOAuthProvider` pattern where `config_map()` is built from the provider config and passed to every script function call.

---

## 4. Rhai Dynamic → serde_json::Value Conversion

### 4.1 Existing Conversion Functions in the Codebase

The codebase already contains battle-tested bidirectional conversion functions in the OAuth building blocks module.

**File:** `codelet/providers/src/oauth/building_blocks.rs` (lines 215–271)

#### 4.1.1 json_value_to_dynamic (serde_json → Rhai)

```rust
fn json_value_to_dynamic(value: &serde_json::Value) -> Dynamic {
    match value {
        serde_json::Value::Null => Dynamic::UNIT,
        serde_json::Value::Bool(b) => Dynamic::from(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Dynamic::from(i)
            } else if let Some(f) = n.as_f64() {
                Dynamic::from(f)
            } else {
                Dynamic::UNIT
            }
        }
        serde_json::Value::String(s) => Dynamic::from(s.clone()),
        serde_json::Value::Array(arr) => {
            let items: Vec<Dynamic> = arr.iter().map(json_value_to_dynamic).collect();
            Dynamic::from_array(items)
        }
        serde_json::Value::Object(obj) => {
            let mut map = Map::new();
            for (k, v) in obj {
                map.insert(k.clone().into(), json_value_to_dynamic(v));
            }
            Dynamic::from_map(map)
        }
    }
}
```

#### 4.1.2 dynamic_to_json_value (Rhai → serde_json)

```rust
fn dynamic_to_json_value(value: &Dynamic) -> serde_json::Value {
    if value.is_unit() {
        serde_json::Value::Null
    } else if let Ok(b) = value.as_bool() {
        serde_json::Value::Bool(b)
    } else if let Ok(i) = value.as_int() {
        serde_json::Value::Number(serde_json::Number::from(i))
    } else if let Ok(f) = value.as_float() {
        serde_json::Number::from_f64(f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null)
    } else if let Ok(s) = value.clone().into_string() {
        serde_json::Value::String(s)
    } else if value.is_array() {
        let arr = value.clone().into_typed_array::<Dynamic>().unwrap_or_default();
        serde_json::Value::Array(arr.iter().map(dynamic_to_json_value).collect())
    } else if value.is_map() {
        let map = value.clone().cast::<Map>();
        let mut obj = serde_json::Map::new();
        for (k, v) in &map {
            obj.insert(k.to_string(), dynamic_to_json_value(v));
        }
        serde_json::Value::Object(obj)
    } else {
        serde_json::Value::Null
    }
}
```

### 4.2 Type Mapping

| Rhai Type | serde_json::Value | Notes |
|-----------|------------------|-------|
| `()` (Unit) | `Value::Null` | |
| `bool` | `Value::Bool` | |
| `i64` (INT) | `Value::Number` | Rhai uses `i64` by default |
| `f64` (FLOAT) | `Value::Number` | NaN/Infinity → Null |
| `String` / `ImmutableString` | `Value::String` | `Dynamic::into_string()` handles both |
| `Array` (Vec<Dynamic>) | `Value::Array` | Recursive conversion |
| `Map` (BTreeMap<Identifier, Dynamic>) | `Value::Object` | Recursive conversion |
| Other variants | `Value::Null` | FnPtr, TimeStamp, custom types → Null |

### 4.3 Rhai's Built-in Serde Support (Alternative)

Rhai provides `rhai::serde::from_dynamic<T>()` and `rhai::serde::to_dynamic<T>()` for generic serde conversion.

**File:** `/tmp/rhai/src/serde/de.rs` (line 107)

```rust
pub fn from_dynamic<'de, T: Deserialize<'de>>(value: &'de Dynamic) -> RhaiResultOf<T> {
    T::deserialize(DynamicDeserializer::new(value))
}
```

**File:** `/tmp/rhai/src/serde/ser.rs` (line 83)

```rust
pub fn to_dynamic<T: Serialize>(value: T) -> RhaiResult {
    let mut s = DynamicSerializer::new(Dynamic::UNIT);
    value.serialize(&mut s)
}
```

**Using from_dynamic for serde_json::Value conversion:**

```rust
use rhai::serde::from_dynamic;

let result: Dynamic = engine.call_fn(&mut scope, &ast, "format_system_prompt", (preamble,))?;
let json_value: serde_json::Value = from_dynamic(&result)?;
```

This works because `serde_json::Value` implements `Deserialize`. The `DynamicDeserializer` dispatches based on the `Dynamic` variant type:
- `Union::Str` → `deserialize_str` → `Value::String`
- `Union::Map` → `deserialize_map` → `Value::Object`
- `Union::Array` → `deserialize_seq` → `Value::Array`

### 4.4 Recommended Approach: Use Existing Manual Conversion

**Recommendation: Use the existing `dynamic_to_json_value()` function rather than `rhai::serde::from_dynamic`.**

Reasons:
1. **Already tested** in the codebase (used by `json::stringify` building block)
2. **Predictable fallback** — unknown types become `Value::Null` instead of errors
3. **No borrow lifetime issues** — `from_dynamic` requires `&'de Dynamic` which can complicate ownership when the `Dynamic` is a local return value
4. **Better error handling** — manual conversion never panics; `from_dynamic` returns `RhaiResultOf` which needs unwrapping

The function should be **extracted to a shared module** (e.g., `codelet/providers/src/oauth/conversion.rs` or a new utility module) so both `building_blocks.rs` and `RhaiSystemPromptFacade` can use it without duplication.

### 4.5 Cache Control Block Example

A Rhai script producing Claude-compatible cache_control blocks:

```rhai
// Rhai script: format_system_prompt(preamble)
fn format_system_prompt(preamble) {
    [
        #{
            type: "text",
            text: "You are Claude Code, Anthropic's official CLI for Claude.",
        },
        #{
            type: "text",
            text: preamble,
            cache_control: #{
                type: "ephemeral",
            },
        },
    ]
}
```

After `dynamic_to_json_value()`, this becomes:

```json
[
    {
        "type": "text",
        "text": "You are Claude Code, Anthropic's official CLI for Claude."
    },
    {
        "type": "text",
        "text": "...",
        "cache_control": {
            "type": "ephemeral"
        }
    }
]
```

**This is exactly the format that Claude's API expects for the `system` field**, and it's what `ClaudeOAuthSystemPromptFacade::format_for_api()` currently produces via `serde_json::json!()`.

### 4.6 Rhai Map Syntax Reference

In Rhai, object maps use `#{ key: value }` syntax:

```rhai
let map = #{
    type: "text",
    text: "hello",
    cache_control: #{
        type: "ephemeral",
    },
};
```

The `#{}` syntax creates a `Map` (which is `BTreeMap<Identifier, Dynamic>` in Rust). When converted via `dynamic_to_json_value()`, map keys become JSON string keys and nested maps become nested JSON objects.

**File:** `/tmp/rhai/src/lib.rs` (line 304)

```rust
pub type Map = std::collections::BTreeMap<Identifier, Dynamic>;
```

**Note:** `Identifier` is Rhai's internal smart string type. `k.to_string()` converts it to a standard `String` during JSON conversion. Map keys are always strings in Rhai (no integer keys), which maps cleanly to JSON object keys.

---

## 5. 'static Lifetime Handling

### 5.1 The Problem

The `SystemPromptFacade` trait defines:

```rust
fn provider(&self) -> &'static str;
fn identity_prefix(&self) -> Option<&'static str>;
```

Both methods return `&'static str` — references to strings with the `'static` lifetime, meaning they must live for the entire duration of the program. The existing implementations satisfy this trivially because they return string literals or constants:

```rust
fn provider(&self) -> &'static str { "claude" }         // string literal → 'static
fn identity_prefix(&self) -> Option<&'static str> {
    Some(CLAUDE_CODE_PROMPT_PREFIX)                       // const → 'static
}
```

For `RhaiSystemPromptFacade`, the provider name and identity prefix come from **Rhai script execution**, which produces owned `String` values. An owned `String` has lifetime `'a` (scoped to where it's created), not `'static`.

### 5.2 Strategy A: Box::leak (Recommended)

`Box::leak()` converts a `Box<str>` into a `&'static str` by intentionally leaking the allocation:

```rust
let provider_string: String = /* from Rhai script */;
let provider_static: &'static str = Box::leak(provider_string.into_boxed_str());
```

**How it works:**
1. `String::into_boxed_str()` → `Box<str>` (heap-allocated, fixed-size string)
2. `Box::leak()` → `&'static str` (surrenders ownership to the static lifetime)

**The leaked memory is never freed.** For our use case, this is acceptable because:
- `RhaiSystemPromptFacade` is created **once per provider** during application startup
- The leaked strings are tiny (provider names like `"my-custom-provider"`)
- The process lifetime is the intended lifetime anyway
- At most 1-2 `RhaiSystemPromptFacade` instances exist per process

**Implementation in the constructor (from Section 3.3):**

```rust
impl RhaiSystemPromptFacade {
    pub fn new(engine: Arc<Engine>, ast: AST) -> Result<Self, anyhow::Error> {
        // Evaluate provider name from Rhai (or default to "custom")
        let provider_name_string = if has_script_fn(&ast, "provider", 0) {
            let mut scope = Scope::new();
            let result: Dynamic = engine.call_fn(&mut scope, &ast, "provider", ())?;
            result.into_string()
                .map_err(|_| anyhow::anyhow!("provider() must return a string"))?
        } else {
            "custom".to_string()
        };

        // SAFETY: Box::leak is intentional — provider name lives for the process lifetime
        let provider_name: &'static str = Box::leak(provider_name_string.into_boxed_str());

        // Same for identity_prefix
        let identity_prefix_str = if has_script_fn(&ast, "identity_prefix", 0) {
            let mut scope = Scope::new();
            let result: Dynamic = engine.call_fn(&mut scope, &ast, "identity_prefix", ())?;
            if result.is_unit() {
                None
            } else {
                let s = result.into_string()
                    .map_err(|_| anyhow::anyhow!("identity_prefix() must return a string or ()"))?;
                Some(Box::leak(s.into_boxed_str()) as &'static str)
            }
        } else {
            None
        };

        Ok(Self { engine, ast, provider_name, identity_prefix_str })
    }
}
```

### 5.3 Strategy B: Change the Trait to Return Owned String

An alternative is to modify the `SystemPromptFacade` trait to return owned types:

```rust
pub trait SystemPromptFacade: Send + Sync {
    fn provider(&self) -> String;                    // was &'static str
    fn identity_prefix(&self) -> Option<String>;     // was Option<&'static str>
    // ... rest unchanged ...
}
```

**Pros:**
- No memory leak
- Cleaner for dynamic values

**Cons:**
- **Breaking change** — all 6 existing implementations must be updated
- Unnecessary allocations for the 5 existing facades that return compile-time constants
- Every call site that uses `.provider()` must handle `String` instead of `&str`
- The `select_claude_facade()` function and test assertions would need updating

**Hybrid alternative — return `Cow<'static, str>`:**

```rust
use std::borrow::Cow;

pub trait SystemPromptFacade: Send + Sync {
    fn provider(&self) -> Cow<'static, str>;
    fn identity_prefix(&self) -> Option<Cow<'static, str>>;
    // ...
}
```

Existing implementations return `Cow::Borrowed("claude")` (zero-cost), while `RhaiSystemPromptFacade` returns `Cow::Owned(string_from_rhai)`.

**Pros:**
- No memory leak
- No unnecessary allocations for existing facades
- Ergonomic — `Cow<str>` derefs to `&str` for comparisons

**Cons:**
- Still a breaking change (though smaller — `&'static str` auto-converts to `Cow::Borrowed`)
- More complex type signature

### 5.4 Strategy C: Store Owned String, Cache &'static at Construction

This is a variation of Strategy A where the struct stores the owned `String` AND the leaked reference:

```rust
pub struct RhaiSystemPromptFacade {
    engine: Arc<Engine>,
    ast: AST,
    /// Owned copy (for Drop cleanup if desired)
    _provider_owned: String,
    /// Leaked reference returned by provider()
    provider_name: &'static str,
    _identity_owned: Option<String>,
    identity_prefix_str: Option<&'static str>,
}
```

This doesn't actually help with cleanup (you can't un-leak memory), but it documents intent.

### 5.5 Recommendation

**Use Strategy A (Box::leak)** for the initial implementation.

Rationale:
1. **Zero breaking changes** to the existing trait or its 6 implementations
2. **Memory cost is negligible** — a few bytes per provider, allocated once
3. **This pattern is common** in Rust for process-lifetime configuration
4. **The existing codebase has no precedent** for `Box::leak` but also no prohibition
5. If the trait is later refactored to `Cow<'static, str>` (e.g., during a broader refactor), the `Box::leak` calls can be trivially removed

### 5.6 When Box::leak Is NOT Acceptable

If the system were to support **hot-reloading** of provider scripts (creating new `RhaiSystemPromptFacade` instances with different provider names), each reload would leak the old name string. In that scenario, Strategy B (`Cow<'static, str>`) would be necessary.

For the current architecture (providers created once at startup), this is not a concern.

---

## Appendix A: Complete Example Rhai Script

```rhai
/// Custom system prompt script for a Claude-compatible provider
/// with OAuth-style cache_control blocks.

/// Provider identifier
fn provider() {
    "my-custom-claude"
}

/// Identity prefix (displayed to the user)
fn identity_prefix() {
    "You are MyBot, a helpful coding assistant."
}

/// Transform preamble for internal rig handling
fn transform_preamble(preamble) {
    "You are MyBot, a helpful coding assistant.\n\n" + preamble
}

/// Format system prompt for the API
/// Returns array of blocks with cache_control for Claude-style APIs
fn format_system_prompt(preamble) {
    [
        #{
            type: "text",
            text: "You are MyBot, a helpful coding assistant.",
        },
        #{
            type: "text",
            text: preamble,
            cache_control: #{
                type: "ephemeral",
            },
        },
    ]
}
```

## Appendix B: Minimal Rhai Script (All Defaults)

```rhai
/// Minimal script — only overrides the provider name.
/// All other functions use defaults:
/// - identity_prefix: None
/// - transform_preamble: prepend_fspec_guidance(preamble)
/// - format_system_prompt: Value::String(prepend_fspec_guidance(preamble))

fn provider() {
    "my-openai-compatible"
}
```

## Appendix C: Rhai Script with config Access

```rhai
/// Script that uses the config map for conditional logic.

fn provider() {
    "configurable-provider"
}

fn format_system_prompt(preamble) {
    // Access config map if available
    if config.contains("model_name") {
        let model = config.model_name;
        if model.contains("claude") {
            // Return array format for Claude models
            [
                #{
                    type: "text",
                    text: preamble,
                    cache_control: #{ type: "ephemeral" },
                },
            ]
        } else {
            // Return plain string for other models
            preamble
        }
    } else {
        preamble
    }
}
```

## Appendix D: Key File References

| File | Purpose |
|------|---------|
| `codelet/tools/src/facade/system_prompt.rs` | Trait definition + all built-in facades |
| `codelet/tools/src/fspec_workflow_guidance.rs` | `FSPEC_WORKFLOW_GUIDANCE` constant |
| `codelet/providers/src/claude.rs:507-595` | Claude's `create_rig_agent` — array format + additional_params |
| `codelet/providers/src/openai.rs:410-464` | OpenAI's `create_rig_agent` — plain string preamble |
| `codelet/providers/src/gemini.rs:130-273` | Gemini's `create_rig_agent` — build_gemini_system_prompt |
| `codelet/providers/src/zai.rs:218-311` | Z.AI's `create_rig_agent` — same as OpenAI |
| `codelet/providers/src/codex/mod.rs:331-456` | Codex's `create_rig_agent` — CODEX_BASE_INSTRUCTIONS |
| `codelet/providers/src/copilot/rig_agent.rs:56-115` | Copilot's `create_rig_agent` — same as OpenAI |
| `codelet/providers/src/copilot/system_prompt_facade.rs` | Copilot system prompt facade |
| `codelet/providers/src/oauth/building_blocks.rs:215-271` | `dynamic_to_json_value` + `json_value_to_dynamic` |
| `codelet/providers/src/oauth/engine.rs` | Sandboxed Rhai engine factory |
| `codelet/providers/src/oauth/script_provider.rs` | `ScriptedOAuthProvider` — existing Rhai-scriptable pattern |
| `/tmp/rhai/src/module/mod.rs:1333-1345` | `Module::get_script_fn()` — function existence check |
| `/tmp/rhai/src/ast/ast.rs:673-677` | `AST::iter_functions()` — public function enumeration |
| `/tmp/rhai/src/ast/script_fn.rs:103-139` | `ScriptFnMetadata` — function metadata struct |
| `/tmp/rhai/src/serde/de.rs:107` | `rhai::serde::from_dynamic()` — generic serde deserialization |
| `/tmp/rhai/src/serde/ser.rs:83` | `rhai::serde::to_dynamic()` — generic serde serialization |
| `/tmp/rhai/src/lib.rs:304` | `pub type Map = BTreeMap<Identifier, Dynamic>` |
