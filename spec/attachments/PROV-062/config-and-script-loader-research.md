# PROV-062: Provider Config Loader and Rhai Script Compiler — Research Document

**Work Unit:** PROV-062  
**Parent:** PROV-061 (Custom Scripted Providers)  
**Date:** 2026-04-17  
**Status:** Research Complete

---

## Table of Contents

1. [ProviderConfig JSON Schema](#1-providerconfig-json-schema)
2. [Config Loader — Discovery and Override Semantics](#2-config-loader--discovery-and-override-semantics)
3. [Rhai Script Compiler/Cacher](#3-rhai-script-compilercacher)
4. [Script Validation at Load Time](#4-script-validation-at-load-time)
5. [PROV-060 Engine Setup Reuse](#5-prov-060-engine-setup-reuse)
6. [Auth Type Resolution](#6-auth-type-resolution)

---

## 1. ProviderConfig JSON Schema

### 1.1 Design Rationale

The custom provider config must capture everything the system needs to instantiate a provider without hardcoded Rust types. The schema is derived from studying how each existing provider stores its configuration:

| Provider | Config Source | Key Fields |
|----------|-------------|------------|
| **Claude** (`claude.rs:154-160`) | Env vars + `claude_auth.json` | `model_name`, `auth_mode` (ApiKey/OAuth), beta headers |
| **OpenAI** (`openai.rs:60-70`) | Env vars + builder | `model_name`, `base_url`, `context_window`, `max_output_tokens` |
| **Copilot** (`copilot/provider.rs:43-51`) | `copilot_auth.json` + device flow | `deployment`, `access_token`, `model_name`, `base_url`, `auth` (two-token) |
| **Z.AI** (`zai.rs:37-43`) | Env vars | `model_name`, `is_plan_endpoint`, base URL constants |
| **Gemini** (`gemini.rs:27-31`) | Env vars | `model_name` |
| **Codex** (`codex/codex_auth.rs:21-31`) | `~/.codex/auth.json` + keychain | `openai_api_key`, `tokens` (OAuth) |
| **ScriptedOAuth** (`oauth/script_provider.rs:20-42`) | PROV-060 config struct | `name`, `display_name`, `script`, `auth_url`, `token_url`, `client_id`, etc. |

### 1.2 Proposed JSON Schema

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "ProviderConfig",
  "description": "Custom LLM provider configuration for fspec",
  "type": "object",
  "required": ["name", "display_name", "base_url", "script", "auth", "models"],
  "additionalProperties": false,
  "properties": {
    "name": {
      "type": "string",
      "pattern": "^[a-z][a-z0-9-]*$",
      "description": "Provider identifier (e.g., 'my-provider'). Must be unique across all providers. Used as the --provider CLI flag value.",
      "examples": ["my-llm", "internal-api"]
    },
    "display_name": {
      "type": "string",
      "description": "Human-readable provider name shown in TUI model selector.",
      "examples": ["My LLM Provider", "Internal API"]
    },
    "base_url": {
      "type": "string",
      "format": "uri",
      "description": "Base API URL. For OpenAI-compatible providers, this is the root (e.g., 'https://api.example.com'). The /v1 suffix is appended automatically if missing, matching OpenAI provider behavior (openai.rs:44-51).",
      "examples": ["https://api.example.com", "http://localhost:8888"]
    },
    "script": {
      "type": "string",
      "description": "Path to the .rhai script file, relative to the config file's directory. The script must define the 7 required functions.",
      "examples": ["./my-provider.rhai", "../scripts/custom-auth.rhai"]
    },
    "auth": {
      "$ref": "#/definitions/AuthConfig"
    },
    "models": {
      "type": "object",
      "description": "Map of model aliases to model definitions. Keys are short names used with --model flag.",
      "additionalProperties": {
        "$ref": "#/definitions/ModelDef"
      },
      "examples": [
        {
          "fast": { "id": "model-fast-v1", "context_window": 128000, "max_output_tokens": 4096 },
          "smart": { "id": "model-smart-v2", "context_window": 200000, "max_output_tokens": 8192 }
        }
      ]
    },
    "defaults": {
      "$ref": "#/definitions/Defaults"
    },
    "system_prompt": {
      "$ref": "#/definitions/SystemPromptConfig"
    },
    "tool_style": {
      "type": "string",
      "enum": ["openai", "anthropic"],
      "default": "openai",
      "description": "Tool calling convention. 'openai' uses OpenAI function-calling format. 'anthropic' uses Anthropic tool_use format. Determines how tool definitions and tool results are serialized in API requests."
    },
    "api_style": {
      "type": "string",
      "enum": ["openai_chat", "anthropic_messages"],
      "default": "openai_chat",
      "description": "API request/response format. 'openai_chat' uses /chat/completions shape. 'anthropic_messages' uses /v1/messages shape. Most OpenAI-compatible servers use 'openai_chat'."
    },
    "headers": {
      "type": "object",
      "additionalProperties": { "type": "string" },
      "description": "Additional static HTTP headers to include in every API request. Values can reference env vars with ${VAR_NAME} syntax.",
      "examples": [
        { "X-Custom-Header": "value", "X-Api-Version": "2024-01" }
      ]
    },
    "env_prefix": {
      "type": "string",
      "description": "Environment variable prefix for this provider. Used to auto-detect credentials. E.g., 'MY_PROVIDER' checks MY_PROVIDER_API_KEY.",
      "examples": ["MY_PROVIDER", "INTERNAL_API"]
    }
  },
  "definitions": {
    "AuthConfig": {
      "type": "object",
      "required": ["type"],
      "properties": {
        "type": {
          "type": "string",
          "enum": ["bearer", "api_key_header", "oauth_device_code", "oauth_pkce", "custom"],
          "description": "Authentication mechanism."
        }
      },
      "allOf": [
        {
          "if": { "properties": { "type": { "const": "bearer" } } },
          "then": {
            "properties": {
              "env_var": {
                "type": "string",
                "description": "Environment variable containing the bearer token.",
                "examples": ["MY_PROVIDER_API_KEY"]
              },
              "token_prefix": {
                "type": "string",
                "default": "Bearer",
                "description": "Authorization header prefix.",
                "examples": ["Bearer", "Token"]
              }
            },
            "required": ["env_var"]
          }
        },
        {
          "if": { "properties": { "type": { "const": "api_key_header" } } },
          "then": {
            "properties": {
              "env_var": {
                "type": "string",
                "description": "Environment variable containing the API key."
              },
              "header_name": {
                "type": "string",
                "default": "x-api-key",
                "description": "HTTP header name for the API key.",
                "examples": ["x-api-key", "api-key", "Authorization"]
              }
            },
            "required": ["env_var"]
          }
        },
        {
          "if": { "properties": { "type": { "const": "oauth_device_code" } } },
          "then": {
            "properties": {
              "client_id": { "type": "string" },
              "device_code_url": { "type": "string", "format": "uri" },
              "token_url": { "type": "string", "format": "uri" },
              "scopes": { "type": "string" },
              "credential_file": {
                "type": "string",
                "description": "Filename inside ~/.fspec/credentials/ for persisted tokens.",
                "examples": ["my_provider_auth.json"]
              }
            },
            "required": ["client_id", "device_code_url", "token_url", "credential_file"]
          }
        },
        {
          "if": { "properties": { "type": { "const": "oauth_pkce" } } },
          "then": {
            "properties": {
              "client_id": { "type": "string" },
              "authorize_url": { "type": "string", "format": "uri" },
              "token_url": { "type": "string", "format": "uri" },
              "redirect_uri": { "type": "string", "format": "uri" },
              "scopes": { "type": "string" },
              "credential_file": { "type": "string" }
            },
            "required": ["client_id", "authorize_url", "token_url", "credential_file"]
          }
        },
        {
          "if": { "properties": { "type": { "const": "custom" } } },
          "then": {
            "description": "Auth is fully handled by the Rhai script's get_auth_headers(config, credentials) function. The script reads credentials from its own credential_file and returns the required headers.",
            "properties": {
              "credential_file": {
                "type": "string",
                "description": "Filename for persisted auth state inside ~/.fspec/credentials/"
              }
            }
          }
        }
      ]
    },
    "ModelDef": {
      "type": "object",
      "required": ["id"],
      "properties": {
        "id": {
          "type": "string",
          "description": "Model identifier sent to the API (e.g., 'gpt-4o-2024-08-06')."
        },
        "context_window": {
          "type": "integer",
          "minimum": 1024,
          "default": 128000,
          "description": "Maximum context window in tokens."
        },
        "max_output_tokens": {
          "type": "integer",
          "minimum": 256,
          "default": 4096,
          "description": "Maximum output tokens per completion."
        },
        "supports_tools": {
          "type": "boolean",
          "default": true,
          "description": "Whether this model supports tool/function calling."
        },
        "supports_streaming": {
          "type": "boolean",
          "default": true,
          "description": "Whether this model supports SSE streaming."
        },
        "supports_thinking": {
          "type": "boolean",
          "default": false,
          "description": "Whether this model supports extended thinking."
        }
      }
    },
    "Defaults": {
      "type": "object",
      "properties": {
        "model": {
          "type": "string",
          "description": "Default model alias from the models map."
        },
        "temperature": {
          "type": "number",
          "minimum": 0,
          "maximum": 2
        },
        "max_output_tokens": {
          "type": "integer",
          "description": "Default max output tokens (overridden by model-specific values)."
        }
      }
    },
    "SystemPromptConfig": {
      "type": "object",
      "properties": {
        "prefix": {
          "type": "string",
          "description": "Text prepended to every system prompt (like Claude's CLAUDE_CODE_PROMPT_PREFIX for OAuth mode — see claude.rs:451-456)."
        },
        "cache_control": {
          "type": "boolean",
          "default": false,
          "description": "Whether to use Anthropic-style cache_control metadata on system prompt blocks."
        }
      }
    }
  }
}
```

### 1.3 Corresponding Rust Struct

Based on the existing `ScriptProviderConfig` pattern from `oauth/script_provider.rs:20-42`:

```rust
/// Full provider configuration loaded from JSON.
/// Extends ScriptProviderConfig (PROV-060) with model catalog,
/// system prompt config, and tool style.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    pub display_name: String,
    pub base_url: String,
    pub script: String,
    pub auth: AuthConfig,
    pub models: HashMap<String, ModelDef>,
    #[serde(default)]
    pub defaults: Option<Defaults>,
    #[serde(default)]
    pub system_prompt: Option<SystemPromptConfig>,
    #[serde(default = "default_tool_style")]
    pub tool_style: ToolStyle,
    #[serde(default = "default_api_style")]
    pub api_style: ApiStyle,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub env_prefix: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthConfig {
    Bearer { env_var: String, #[serde(default = "default_bearer_prefix")] token_prefix: String },
    ApiKeyHeader { env_var: String, #[serde(default = "default_api_key_header")] header_name: String },
    OauthDeviceCode { client_id: String, device_code_url: String, token_url: String, scopes: Option<String>, credential_file: String },
    OauthPkce { client_id: String, authorize_url: String, token_url: String, redirect_uri: Option<String>, scopes: Option<String>, credential_file: String },
    Custom { credential_file: Option<String> },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModelDef {
    pub id: String,
    #[serde(default = "default_context_window")]
    pub context_window: usize,
    #[serde(default = "default_max_output_tokens")]
    pub max_output_tokens: usize,
    #[serde(default = "default_true")]
    pub supports_tools: bool,
    #[serde(default = "default_true")]
    pub supports_streaming: bool,
    #[serde(default)]
    pub supports_thinking: bool,
}
```

---

## 2. Config Loader — Discovery and Override Semantics

### 2.1 Existing Credential Discovery Pattern

The existing `ProviderCredentials::detect()` in `codelet/providers/src/credentials.rs:20-36` uses a flat, hardcoded approach:

```rust
// From credentials.rs:20-36
impl ProviderCredentials {
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
        }
    }
}
```

Each `has_*_auth()` function reads a specific credential file:
- `has_claude_auth()` → `~/.fspec/credentials/claude_auth.json` (via `claude_auth.rs:42`)
- `has_codex_auth()` → `~/.codex/auth.json` (via `codex_auth.rs:60`)
- `has_github_copilot_auth()` → `~/.fspec/credentials/copilot_auth.json` (via `copilot/auth.rs:166`)

The `FSPEC_HOME` env var redirects the credentials directory (used by all `get_fspec_home()` implementations — see `claude_auth.rs:31-38`, `copilot/auth.rs:154-161`).

### 2.2 Config Discovery for Custom Providers

Custom provider configs are JSON files discovered from two directories:

```
~/.fspec/providers/*.json     # User-global configs
.fspec/providers/*.json       # Project-local configs (CWD-relative)
```

**Search Order and Override Semantics:**

```
1. Scan ~/.fspec/providers/*.json          → global configs
2. Scan .fspec/providers/*.json            → project-local configs
3. Project-local OVERRIDES global by `name` field match
```

This mirrors how fspec already uses `FSPEC_HOME` for credentials. The `providers/` directory is a sibling of `credentials/`.

**Implementation Pattern:**

```rust
use std::path::{Path, PathBuf};
use std::collections::HashMap;

/// Discover all custom provider configs from global and project-local dirs.
///
/// Project-local configs override global configs with the same `name`.
pub fn discover_provider_configs() -> anyhow::Result<Vec<ProviderConfig>> {
    let mut configs: HashMap<String, ProviderConfig> = HashMap::new();

    // 1. Global configs: ~/.fspec/providers/*.json
    let global_dir = get_fspec_home_base().join("providers");
    load_configs_from_dir(&global_dir, &mut configs)?;

    // 2. Project-local configs: .fspec/providers/*.json (override global)
    let local_dir = PathBuf::from(".fspec/providers");
    load_configs_from_dir(&local_dir, &mut configs)?;

    Ok(configs.into_values().collect())
}

fn get_fspec_home_base() -> PathBuf {
    if let Ok(fspec_home) = std::env::var("FSPEC_HOME") {
        // FSPEC_HOME points at the credentials dir; go up one level
        PathBuf::from(fspec_home).parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from(fspec_home))
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| String::from("/tmp"));
        PathBuf::from(home).join(".fspec")
    }
}

fn load_configs_from_dir(
    dir: &Path,
    configs: &mut HashMap<String, ProviderConfig>,
) -> anyhow::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "json") {
            let content = std::fs::read_to_string(&path)?;
            let config: ProviderConfig = serde_json::from_str(&content)
                .map_err(|e| anyhow::anyhow!(
                    "Failed to parse provider config {}: {e}", path.display()
                ))?;
            // Validate with schema here (see 2.3)
            validate_provider_config(&config, &path)?;
            configs.insert(config.name.clone(), config);
        }
    }
    Ok(())
}
```

### 2.3 Validation Strategy

Since the providers crate is Rust, validation should use **serde** for structural validation and manual checks for semantic validation:

```rust
fn validate_provider_config(config: &ProviderConfig, path: &Path) -> anyhow::Result<()> {
    // 1. Name must not collide with built-in providers
    const BUILTIN_NAMES: &[&str] = &[
        "claude", "openai", "codex", "gemini", "zai", "github-copilot", "copilot"
    ];
    if BUILTIN_NAMES.contains(&config.name.as_str()) {
        anyhow::bail!(
            "Provider name '{}' in {} conflicts with built-in provider",
            config.name, path.display()
        );
    }

    // 2. Script file must exist (relative to config file's directory)
    let config_dir = path.parent().unwrap_or(Path::new("."));
    let script_path = config_dir.join(&config.script);
    if !script_path.exists() {
        anyhow::bail!(
            "Script file '{}' not found (resolved to {})",
            config.script, script_path.display()
        );
    }

    // 3. Default model must be in the models map
    if let Some(ref defaults) = config.defaults {
        if let Some(ref default_model) = defaults.model {
            if !config.models.contains_key(default_model) {
                anyhow::bail!(
                    "Default model '{}' not found in models map",
                    default_model
                );
            }
        }
    }

    // 4. models map must not be empty
    if config.models.is_empty() {
        anyhow::bail!("Provider '{}' must define at least one model", config.name);
    }

    Ok(())
}
```

### 2.4 Integration with ProviderManager

The `ProviderManager` in `manager.rs` currently uses a `ProviderType` enum (`Claude`, `OpenAI`, `Codex`, `Gemini`, `ZAI`, `GitHubCopilot` — see `manager.rs:22-30`). Custom providers extend this:

```rust
// Existing enum (manager.rs:22-30):
pub enum ProviderType {
    Claude,
    OpenAI,
    Codex,
    Gemini,
    ZAI,
    GitHubCopilot,
}

// Extended for PROV-062:
pub enum ProviderType {
    Claude,
    OpenAI,
    Codex,
    Gemini,
    ZAI,
    GitHubCopilot,
    Custom(String),  // name from ProviderConfig
}
```

The `FromStr` impl at `manager.rs:33-48` needs extension:

```rust
impl FromStr for ProviderType {
    fn from_str(name: &str) -> Result<Self, ProviderError> {
        match name.to_lowercase().as_str() {
            "claude" => Ok(ProviderType::Claude),
            "openai" => Ok(ProviderType::OpenAI),
            // ... existing matches ...
            "github-copilot" | "copilot" => Ok(ProviderType::GitHubCopilot),
            other => {
                // Check if it's a custom provider
                let configs = discover_provider_configs()
                    .map_err(|e| ProviderError::config("manager", e.to_string()))?;
                if configs.iter().any(|c| c.name == other) {
                    Ok(ProviderType::Custom(other.to_string()))
                } else {
                    Err(ProviderError::config("manager", format!("Unknown provider: {name}")))
                }
            }
        }
    }
}
```

---

## 3. Rhai Script Compiler/Cacher

### 3.1 Rhai Compilation API — Exact Signatures from Source

All signatures verified against `/tmp/rhai/src/api/compile.rs` and `/tmp/rhai/src/api/files.rs`:

#### `Engine::compile`
**File:** `/tmp/rhai/src/api/compile.rs:29`

```rust
/// Compile a string into an AST, which can be used later for evaluation.
#[inline(always)]
pub fn compile(&self, script: impl AsRef<str>) -> ParseResult<AST>
```

Delegates to `compile_with_scope(&Scope::new(), script)`.

#### `Engine::compile_with_scope`
**File:** `/tmp/rhai/src/api/compile.rs:70`

```rust
/// Compile a string into an AST using own scope.
/// Constants defined in the scope are propagated throughout the script
/// including functions (for optimization).
#[inline(always)]
pub fn compile_with_scope(&self, scope: &Scope, script: impl AsRef<str>) -> ParseResult<AST>
```

Delegates to `compile_scripts_with_scope(scope, &[script])`.

#### `Engine::compile_file`
**File:** `/tmp/rhai/src/api/files.rs:71`  
**Availability:** Not available under `no_std` or WASM.

```rust
/// Compile a script file into an AST.
/// Not available under `no_std` or `WASM`.
#[inline(always)]
pub fn compile_file(&self, path: PathBuf) -> RhaiResultOf<AST>
```

Delegates to `compile_file_with_scope(&Scope::new(), path)`.

**Key difference:** Returns `RhaiResultOf<AST>` (not `ParseResult<AST>`) because file I/O can fail in addition to parse errors.

#### `Engine::compile_file_with_scope`
**File:** `/tmp/rhai/src/api/files.rs:109`

```rust
/// Compile a script file with own scope. Constants propagation applies.
#[inline]
pub fn compile_file_with_scope(&self, scope: &Scope, path: PathBuf) -> RhaiResultOf<AST> {
    Self::read_file(&path).and_then(|contents| {
        let mut ast = self.compile_with_scope(scope, contents)?;
        ast.set_source(path.to_string_lossy().as_ref());
        Ok(ast)
    })
}
```

**Note:** `read_file` also strips `#!` shebang lines (see `files.rs:36-44`).

#### `Engine::call_fn`
**File:** `/tmp/rhai/src/api/call_fn.rs:126`

```rust
/// Call a script function defined in an AST with multiple Dynamic arguments.
pub fn call_fn<T: Variant + Clone>(
    &self,
    scope: &mut Scope,
    ast: &AST,
    fn_name: impl AsRef<str>,
    args: impl FuncArgs,
) -> RhaiResultOf<T>
```

This is what `ScriptedOAuthProvider` uses to invoke script functions (see `script_provider.rs:116`).

### 3.2 Type Definitions

#### `ParseResult<T>` vs `RhaiResultOf<T>`

```
ParseResult<T> = Result<T, ParseError>       // Parse-only errors
RhaiResultOf<T> = Result<T, Box<EvalAltResult>>  // Parse + runtime + I/O errors
```

#### `AST` struct
**File:** `/tmp/rhai/src/ast/ast.rs:21-35`

```rust
#[derive(Clone)]
pub struct AST {
    source: Option<ImmutableString>,
    body: ThinVec<Stmt>,
    #[cfg(not(feature = "no_function"))]
    lib: crate::SharedModule,             // SharedModule = Shared<Module>
    #[cfg(not(feature = "no_module"))]
    pub(crate) resolver: Option<crate::Shared<crate::module::resolvers::StaticModuleResolver>>,
    #[cfg(feature = "metadata")]
    pub(crate) doc: crate::SmartString,
}
```

**`SharedModule` type alias** (from `/tmp/rhai/src/lib.rs:274`):
```rust
type SharedModule = Shared<Module>;
// where Shared<T> = Arc<T> (with `sync` feature) or Rc<T> (without)
```

### 3.3 How PROV-060 Currently Compiles Scripts

From `codelet/providers/src/oauth/script_provider.rs:53-67`:

```rust
impl ScriptedOAuthProvider {
    pub fn load(script_path: &Path, config: ScriptProviderConfig) -> Result<Self> {
        let engine = build_default_engine();
        let script_content = std::fs::read_to_string(script_path)
            .map_err(|e| anyhow!("Failed to read script {}: {e}", script_path.display()))?;
        let ast = engine.compile(&script_content).map_err(|e| {
            anyhow!("Failed to compile script {}: {e}", script_path.display())
        })?;
        Ok(Self {
            engine: Arc::new(engine),
            ast,
            config,
        })
    }
}
```

**Key observations:**
1. Uses `engine.compile()` (not `compile_file()`) — reads file manually, compiles the string.
2. The `Engine` is wrapped in `Arc<Engine>` for thread-safe sharing.
3. The `AST` is stored directly (not in `Arc`) — it is `Clone`able.
4. No caching mechanism exists yet.

### 3.4 ScriptLoader Design — Compile and Cache

The `ScriptLoader` should compile `.rhai` scripts once and cache the compiled `AST` keyed by file path. The `AST` is already `Clone` (cheap because `lib` is `SharedModule = Arc<Module>`), so caching behind `Arc<AST>` adds one more level of sharing.

```rust
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use anyhow::{anyhow, Result};
use rhai::{AST, Engine};

/// Cached compiled script entry.
struct CachedScript {
    ast: Arc<AST>,
    /// File modification time at compilation.
    mtime: SystemTime,
}

/// Compiles and caches Rhai scripts.
///
/// Thread-safe: the cache is behind a `parking_lot::RwLock` (or
/// `std::sync::RwLock`). The `Engine` is shared via `Arc`.
pub struct ScriptLoader {
    engine: Arc<Engine>,
    cache: parking_lot::RwLock<HashMap<PathBuf, CachedScript>>,
}

impl ScriptLoader {
    /// Create a new ScriptLoader with the given sandboxed engine.
    pub fn new(engine: Engine) -> Self {
        Self {
            engine: Arc::new(engine),
            cache: parking_lot::RwLock::new(HashMap::new()),
        }
    }

    /// Get the shared engine reference.
    pub fn engine(&self) -> &Arc<Engine> {
        &self.engine
    }

    /// Load and compile a script, returning a cached AST.
    ///
    /// If the file has been compiled before and hasn't changed (by mtime),
    /// returns the cached AST. Otherwise recompiles and updates the cache.
    pub fn load(&self, script_path: &Path) -> Result<Arc<AST>> {
        let canonical = std::fs::canonicalize(script_path)
            .map_err(|e| anyhow!("Cannot resolve script path {}: {e}", script_path.display()))?;

        let current_mtime = std::fs::metadata(&canonical)
            .and_then(|m| m.modified())
            .map_err(|e| anyhow!("Cannot stat {}: {e}", canonical.display()))?;

        // Fast path: check read cache
        {
            let cache = self.cache.read();
            if let Some(entry) = cache.get(&canonical) {
                if entry.mtime == current_mtime {
                    return Ok(entry.ast.clone());
                }
            }
        }

        // Slow path: compile and update cache
        let script_content = std::fs::read_to_string(&canonical)
            .map_err(|e| anyhow!("Failed to read script {}: {e}", canonical.display()))?;

        let ast = self.engine.compile(&script_content).map_err(|e| {
            anyhow!("Failed to compile script {}: {e}", canonical.display())
        })?;

        let arc_ast = Arc::new(ast);

        let mut cache = self.cache.write();
        cache.insert(canonical, CachedScript {
            ast: arc_ast.clone(),
            mtime: current_mtime,
        });

        Ok(arc_ast)
    }

    /// Load a script from a string (for testing). No caching.
    pub fn load_from_string(&self, script: &str) -> Result<Arc<AST>> {
        let ast = self.engine.compile(script).map_err(|e| {
            anyhow!("Failed to compile script: {e}")
        })?;
        Ok(Arc::new(ast))
    }

    /// Invalidate a cached script entry.
    pub fn invalidate(&self, script_path: &Path) {
        if let Ok(canonical) = std::fs::canonicalize(script_path) {
            self.cache.write().remove(&canonical);
        }
    }

    /// Clear the entire cache.
    pub fn clear_cache(&self) {
        self.cache.write().clear();
    }
}
```

### 3.5 Why `Arc<AST>` and Not Just `AST`

The `AST` struct is `Clone`, but cloning copies the `body: ThinVec<Stmt>` vector (the script-defined functions in `lib: SharedModule` are already shared via `Arc<Module>`). By wrapping in `Arc<AST>`, multiple `CustomProvider` instances sharing the same script avoid even cloning the statement body.

The PROV-060 `ScriptedOAuthProvider` stores `ast: AST` (not `Arc<AST>`) because each provider instance owns its own AST. With PROV-062's caching layer, multiple providers from the same script share the same `Arc<AST>`.

---

## 4. Script Validation at Load Time

### 4.1 Required Functions for Custom Providers

PROV-062 requires **7 functions** in each custom provider script. These extend the 5 OAuth functions from PROV-060 (`script_provider.rs:5-9`) with 2 additional provider-lifecycle functions:

| # | Function | Params | Returns | Origin |
|---|----------|--------|---------|--------|
| 1 | `build_authorization_request(config)` | 1 | Map | PROV-060 |
| 2 | `exchange_code(config, code, pkce_verifier)` | 3 | Map | PROV-060 |
| 3 | `refresh_token(config, current_tokens)` | 2 | Map | PROV-060 |
| 4 | `poll_for_token(config, device_data)` | 2 | Map | PROV-060 |
| 5 | `needs_refresh(tokens)` | 1 | bool | PROV-060 |
| 6 | `get_auth_headers(config, credentials)` | 2 | Map | PROV-062 (new) |
| 7 | `transform_request(config, request)` | 2 | Map | PROV-062 (new) |

**Note:** Not all auth types require all functions. The validator should check based on `auth.type`:
- `bearer` / `api_key_header` → only `get_auth_headers` required (others optional)
- `oauth_device_code` → requires `poll_for_token`, `needs_refresh`, `get_auth_headers`
- `oauth_pkce` → requires `build_authorization_request`, `exchange_code`, `refresh_token`, `needs_refresh`, `get_auth_headers`
- `custom` → requires all 7 functions

### 4.2 Rhai AST Introspection API

#### `AST::iter_functions()`
**File:** `/tmp/rhai/src/ast/ast.rs:672-677`

```rust
/// Iterate through all function definitions.
/// Not available under `no_function`.
#[cfg(not(feature = "no_function"))]
#[inline]
pub fn iter_functions(&self) -> impl Iterator<Item = super::ScriptFnMetadata<'_>> {
    self.lib
        .iter_script_fn()
        .map(|(.., fn_def)| fn_def.as_ref().into())
}
```

Returns `ScriptFnMetadata` items with:
- `name: &str` — function name
- `params: Vec<&str>` — parameter names
- `access: FnAccess` — `Public` or `Private`

**`ScriptFnMetadata` struct** (from `/tmp/rhai/src/ast/script_fn.rs:103-139`):
```rust
#[non_exhaustive]
pub struct ScriptFnMetadata<'a> {
    pub name: &'a str,
    pub params: Vec<&'a str>,
    pub access: FnAccess,
    #[cfg(not(feature = "no_object"))]
    pub this_type: Option<&'a str>,
    #[cfg(feature = "metadata")]
    pub comments: Vec<&'a str>,
}
```

#### `Module::get_script_fn()`
**File:** `/tmp/rhai/src/module/mod.rs:1333-1345`

```rust
/// Get a shared reference to the script-defined function based on name
/// and number of parameters.
#[cfg(not(feature = "no_function"))]
#[inline]
#[must_use]
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

Looks up by **name + arity** (parameter count). Returns `Option<&Shared<ScriptFuncDef>>`.

#### `AST::shared_lib()`
**File:** `/tmp/rhai/src/ast/ast.rs:212-214`

```rust
/// Get the internal shared Module containing all script-defined functions.
/// Exported under the `internals` feature only.
#[expose_under_internals]
#[cfg(not(feature = "no_function"))]
#[inline(always)]
#[must_use]
const fn shared_lib(&self) -> &crate::SharedModule {
    &self.lib
}
```

**Important:** This is gated behind the `internals` feature. For validation, prefer `iter_functions()` (public API) over `shared_lib()` + `get_script_fn()`.

### 4.3 Validation Implementation

```rust
use rhai::AST;
use anyhow::{anyhow, Result};

/// Required function signature: (name, param_count)
struct RequiredFn {
    name: &'static str,
    params: usize,
}

/// All 7 required functions for a fully custom provider script.
const ALL_REQUIRED_FNS: &[RequiredFn] = &[
    RequiredFn { name: "build_authorization_request", params: 1 },
    RequiredFn { name: "exchange_code", params: 3 },
    RequiredFn { name: "refresh_token", params: 2 },
    RequiredFn { name: "poll_for_token", params: 2 },
    RequiredFn { name: "needs_refresh", params: 1 },
    RequiredFn { name: "get_auth_headers", params: 2 },
    RequiredFn { name: "transform_request", params: 2 },
];

/// Functions required per auth type.
fn required_fns_for_auth_type(auth_type: &str) -> &'static [RequiredFn] {
    match auth_type {
        "bearer" | "api_key_header" => &[
            RequiredFn { name: "get_auth_headers", params: 2 },
        ],
        "oauth_device_code" => &[
            RequiredFn { name: "poll_for_token", params: 2 },
            RequiredFn { name: "needs_refresh", params: 1 },
            RequiredFn { name: "get_auth_headers", params: 2 },
        ],
        "oauth_pkce" => &[
            RequiredFn { name: "build_authorization_request", params: 1 },
            RequiredFn { name: "exchange_code", params: 3 },
            RequiredFn { name: "refresh_token", params: 2 },
            RequiredFn { name: "needs_refresh", params: 1 },
            RequiredFn { name: "get_auth_headers", params: 2 },
        ],
        "custom" => ALL_REQUIRED_FNS,
        _ => &[],
    }
}

/// Validate that a compiled AST contains all required functions for
/// the given auth type. Returns a list of missing function descriptions.
pub fn validate_script_functions(
    ast: &AST,
    auth_type: &str,
) -> Result<()> {
    let required = required_fns_for_auth_type(auth_type);

    // Build a set of (name, param_count) from the AST
    let defined: Vec<(&str, usize)> = ast
        .iter_functions()
        .map(|meta| (meta.name, meta.params.len()))
        .collect();

    let mut missing = Vec::new();

    for req in required {
        let found = defined.iter().any(|(name, count)| {
            *name == req.name && *count == req.params
        });
        if !found {
            missing.push(format!(
                "{}({} params)",
                req.name, req.params
            ));
        }
    }

    if missing.is_empty() {
        Ok(())
    } else {
        Err(anyhow!(
            "Script is missing required functions for auth type '{}': {}",
            auth_type,
            missing.join(", ")
        ))
    }
}
```

### 4.4 Integration with ScriptLoader

The `ScriptLoader::load()` method should validate after compilation:

```rust
impl ScriptLoader {
    /// Load, compile, validate, and cache a provider script.
    pub fn load_validated(
        &self,
        script_path: &Path,
        auth_type: &str,
    ) -> Result<Arc<AST>> {
        let ast = self.load(script_path)?;

        // Validate required functions exist
        validate_script_functions(&ast, auth_type)?;

        Ok(ast)
    }
}
```

### 4.5 Alternative: Using Module::get_script_fn for Validation

If the `internals` feature is enabled, you could use `Module::get_script_fn()` for individual lookups:

```rust
// Requires `internals` feature on rhai
let module: &Module = ast.as_ref();  // AST implements AsRef<Module>
if module.get_script_fn("build_authorization_request", 1).is_none() {
    // Missing!
}
```

However, **`iter_functions()` is preferred** because:
1. It's a public API (no feature gate needed)
2. Single pass over all functions is more efficient than N individual lookups
3. `ScriptFnMetadata` provides human-readable info for error messages

---

## 5. PROV-060 Engine Setup Reuse

### 5.1 Current Sandboxed Engine Architecture

From `codelet/providers/src/oauth/engine.rs:10-58`:

```rust
/// Safety limits
pub const MAX_OPERATIONS: u64 = 50_000;
const MAX_CALL_LEVELS: usize = 32;
const MAX_STRING_SIZE: usize = 1_048_576;   // 1 MB
const MAX_ARRAY_SIZE: usize = 10_000;
const MAX_MAP_SIZE: usize = 10_000;

/// A named Rhai module to be registered with the engine.
pub struct RhaiModule {
    pub name: String,
    pub module: Module,
}

/// Build a sandboxed Rhai engine with the given modules registered.
pub fn build_sandboxed_engine(modules: Vec<RhaiModule>) -> Engine {
    let mut engine = Engine::new_raw();

    // Safety limits
    engine.set_max_operations(MAX_OPERATIONS);
    engine.set_max_call_levels(MAX_CALL_LEVELS);
    engine.set_max_string_size(MAX_STRING_SIZE);
    engine.set_max_array_size(MAX_ARRAY_SIZE);
    engine.set_max_map_size(MAX_MAP_SIZE);

    // Register each module as a static namespace
    for rhai_module in modules {
        engine.register_static_module(&rhai_module.name, rhai_module.module.into());
    }

    engine
}

/// Build a sandboxed engine with the default PROV-060 modules.
pub fn build_default_engine() -> Engine {
    let modules = super::building_blocks::register_all_modules();
    build_sandboxed_engine(modules)
}
```

### 5.2 Current Modules (PROV-060)

From `codelet/providers/src/oauth/building_blocks.rs:14-21`:

```rust
pub fn register_all_modules() -> Vec<RhaiModule> {
    vec![
        build_http_module(),     // http::post, http::get
        build_crypto_module(),   // crypto::sha256, crypto::base64url_encode
        build_json_module(),     // json::parse, json::stringify
        build_oauth_module(),    // oauth::generate_pkce, oauth::generate_state, oauth::urlencoded
    ]
}
```

**Detailed function inventory:**

| Module | Function | Signature | Description |
|--------|----------|-----------|-------------|
| `http` | `post` | `(url: String, body: String, headers: Map) -> Map { status, body }` | Sync HTTP POST via ureq |
| `http` | `get` | `(url: String, headers: Map) -> Map { status, body }` | Sync HTTP GET via ureq |
| `crypto` | `sha256` | `(data: String) -> String` | SHA-256 hex digest |
| `crypto` | `base64url_encode` | `(data: String) -> String` | Base64url no-pad encoding |
| `json` | `parse` | `(s: String) -> Dynamic` | JSON string → Rhai map/array |
| `json` | `stringify` | `(value: Dynamic) -> String` | Rhai value → JSON string |
| `oauth` | `generate_pkce` | `() -> Map { verifier, challenge, challenge_method }` | PKCE code pair |
| `oauth` | `generate_state` | `() -> String` | Random 32-char alphanumeric |
| `oauth` | `urlencoded` | `(s: String) -> String` | Percent-encode string |

### 5.3 PROV-061 Extensions Needed

PROV-061 needs additional modules beyond PROV-060. The engine builder is already designed for extensibility:

```rust
// engine.rs:42 — accepts extensible module list
pub fn build_sandboxed_engine(modules: Vec<RhaiModule>) -> Engine
```

**New modules for PROV-061:**

#### `time` module
```rust
fn build_time_module() -> RhaiModule {
    let mut module = Module::new();

    // time::now() -> i64 (Unix timestamp seconds)
    module.set_native_fn("now", || -> Result<Dynamic, Box<rhai::EvalAltResult>> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| Box::new(rhai::EvalAltResult::ErrorRuntime(
                format!("time::now failed: {e}").into(),
                rhai::Position::NONE,
            )))?;
        Ok(Dynamic::from(now.as_secs() as i64))
    });

    // time::now_millis() -> i64 (Unix timestamp milliseconds)
    module.set_native_fn("now_millis", || -> Result<Dynamic, Box<rhai::EvalAltResult>> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| Box::new(rhai::EvalAltResult::ErrorRuntime(
                format!("time::now_millis failed: {e}").into(),
                rhai::Position::NONE,
            )))?;
        Ok(Dynamic::from(now.as_millis() as i64))
    });

    RhaiModule { name: "time".to_string(), module }
}
```

#### `env` module
```rust
fn build_env_module() -> RhaiModule {
    let mut module = Module::new();

    // env::get(name) -> String (or empty string if not set)
    module.set_native_fn("get",
        |name: String| -> Result<Dynamic, Box<rhai::EvalAltResult>> {
            Ok(Dynamic::from(std::env::var(&name).unwrap_or_default()))
        },
    );

    // env::has(name) -> bool
    module.set_native_fn("has",
        |name: String| -> Result<Dynamic, Box<rhai::EvalAltResult>> {
            Ok(Dynamic::from(std::env::var(&name).is_ok()))
        },
    );

    RhaiModule { name: "env".to_string(), module }
}
```

#### `cred` module (credential file I/O)
```rust
fn build_cred_module(credentials_dir: PathBuf) -> RhaiModule {
    let mut module = Module::new();
    let dir = credentials_dir.clone();

    // cred::read(filename) -> Map or () if missing
    module.set_native_fn("read",
        move |filename: String| -> Result<Dynamic, Box<rhai::EvalAltResult>> {
            let path = dir.join(&filename);
            if !path.exists() {
                return Ok(Dynamic::UNIT);
            }
            let content = std::fs::read_to_string(&path).map_err(|e| {
                Box::new(rhai::EvalAltResult::ErrorRuntime(
                    format!("cred::read failed: {e}").into(),
                    rhai::Position::NONE,
                ))
            })?;
            // Reuse json::parse logic
            let value: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
                Box::new(rhai::EvalAltResult::ErrorRuntime(
                    format!("cred::read JSON parse failed: {e}").into(),
                    rhai::Position::NONE,
                ))
            })?;
            Ok(json_value_to_dynamic(&value))
        },
    );

    let dir2 = credentials_dir;
    // cred::write(filename, data_map) -> ()
    module.set_native_fn("write",
        move |filename: String, data: Map| -> Result<Dynamic, Box<rhai::EvalAltResult>> {
            let path = dir2.join(&filename);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("cred::write mkdir failed: {e}").into(),
                        rhai::Position::NONE,
                    ))
                })?;
            }
            let json_val = dynamic_to_json_value(&Dynamic::from_map(data));
            let content = serde_json::to_string_pretty(&json_val).map_err(|e| {
                Box::new(rhai::EvalAltResult::ErrorRuntime(
                    format!("cred::write JSON failed: {e}").into(),
                    rhai::Position::NONE,
                ))
            })?;
            std::fs::write(&path, content).map_err(|e| {
                Box::new(rhai::EvalAltResult::ErrorRuntime(
                    format!("cred::write failed: {e}").into(),
                    rhai::Position::NONE,
                ))
            })?;
            Ok(Dynamic::UNIT)
        },
    );

    RhaiModule { name: "cred".to_string(), module }
}
```

### 5.4 Extended Engine Builder for PROV-062

```rust
/// Build a sandboxed engine with all modules for custom providers.
///
/// Extends PROV-060's default modules with PROV-061 additions.
pub fn build_custom_provider_engine(credentials_dir: PathBuf) -> Engine {
    let mut modules = super::building_blocks::register_all_modules();
    // PROV-061 additions
    modules.push(build_time_module());
    modules.push(build_env_module());
    modules.push(build_cred_module(credentials_dir));
    build_sandboxed_engine(modules)
}
```

---

## 6. Auth Type Resolution

### 6.1 How Each Auth Type Maps to Existing Infrastructure

The custom provider config's `auth.type` field determines which credential detection, storage, and refresh mechanisms are used. Each type maps to existing building blocks:

#### `bearer` — Simple Bearer Token

**Existing pattern:** Claude API key mode (`claude.rs:262-333`), OpenAI (`openai.rs:186-248`)

```
Credential source:  Environment variable (auth.env_var)
Header format:      Authorization: Bearer <token>
                    (or Authorization: <token_prefix> <token>)
Refresh:            None (static token)
Credential file:    None
Detection:          std::env::var(auth.env_var).is_ok()
```

**Code mapping:**
```rust
// Mirrors detect_credential_from_env() in adapter.rs
fn detect_bearer(config: &AuthConfig) -> Option<String> {
    if let AuthConfig::Bearer { env_var, .. } = config {
        std::env::var(env_var).ok()
    } else {
        None
    }
}
```

#### `api_key_header` — API Key in Custom Header

**Existing pattern:** Claude API key mode uses `x-api-key` header (non-OAuth mode at `claude.rs:321-333`), but via the rig library which handles it internally.

```
Credential source:  Environment variable (auth.env_var)
Header format:      <header_name>: <api_key>  (e.g., x-api-key: sk-...)
Refresh:            None (static key)
Credential file:    None
Detection:          std::env::var(auth.env_var).is_ok()
```

**Code mapping:**
```rust
fn build_api_key_headers(config: &AuthConfig, key: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    if let AuthConfig::ApiKeyHeader { header_name, .. } = config {
        headers.insert(
            HeaderName::from_str(header_name).unwrap(),
            HeaderValue::from_str(key).unwrap(),
        );
    }
    headers
}
```

#### `oauth_device_code` — RFC 8628 Device Authorization

**Existing pattern:** GitHub Copilot (`copilot/oauth.rs`), Codex (`codex/codex_device_auth.rs`)

```
Credential source:  Credential file (~/.fspec/credentials/<credential_file>)
Login flow:         POST device_code_url → display user_code → poll token_url
Header format:      Authorization: Bearer <access_token>
Refresh:            Via Rhai script's needs_refresh() + refresh_token()
Credential file:    ~/.fspec/credentials/<credential_file>
Detection:          CredentialStore::read_sync().is_ok()
```

**Existing infrastructure to reuse:**
- `oauth::DeviceCodeFlow` + `DeviceCodeProvider` trait (`oauth/device_flow.rs`)
- `oauth::CredentialStore<T>` for file I/O (`oauth/credential_store.rs:17-126`)
- `oauth::RefreshingHttpClient<S>` for auto-refresh middleware (`oauth/http_middleware.rs`)

**Copilot's two-token system** (`copilot/token_exchange.rs`) is a specialized variant where the long-lived GitHub token is exchanged for a short-lived Copilot token. Custom providers with `oauth_device_code` can implement similar multi-step token exchange in their Rhai script's `get_auth_headers()` function.

**Flow mapping:**
```
1. Check credential file via CredentialStore
   → Mirrors has_github_copilot_auth() in credentials.rs:136-143
2. If missing, invoke device flow:
   POST config.device_code_url (client_id, scopes)
   → Display user_code + verification_uri
   → Poll config.token_url
   → Persist via CredentialStore::write_secure()
3. For refresh:
   Script's needs_refresh(tokens) → bool
   Script's refresh_token(config, tokens) → new tokens
   → CredentialStore::write_secure()
```

#### `oauth_pkce` — Authorization Code with PKCE

**Existing pattern:** Claude OAuth (`claude_oauth.rs`, `claude_oauth_server.rs`)

```
Credential source:  Credential file (~/.fspec/credentials/<credential_file>)
Login flow:         Build auth URL → local callback server → exchange code
Header format:      Authorization: Bearer <access_token>
Refresh:            Via Rhai script's needs_refresh() + refresh_token()
Credential file:    ~/.fspec/credentials/<credential_file>
Detection:          CredentialStore::read_sync().is_ok()
```

**Existing infrastructure to reuse:**
- `oauth::OAuthCallbackServer` + `CodeExchangeHandler` (`oauth/callback_server.rs`)
- `oauth_crypto::generate_pkce()` → PKCE codes
- `oauth_crypto::urlencoded()` → URL encoding
- `oauth::CredentialStore<T>` for file I/O

**PKCE flow mapping:**
```
1. Script's build_authorization_request(config) → #{url, pkce_verifier, state}
   → Uses oauth::generate_pkce() and oauth::generate_state()
2. Open browser to URL
3. OAuthCallbackServer listens for redirect
4. Script's exchange_code(config, code, pkce_verifier) → #{access_token, refresh_token, ...}
5. Persist via CredentialStore::write_secure()
```

**Claude PKCE constants for reference** (from `claude_oauth.rs:30-46`):
```rust
pub const CLAUDE_CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
pub const CLAUDE_AUTHORIZE_URL: &str = "https://claude.ai/oauth/authorize";
pub const CLAUDE_TOKEN_ENDPOINT: &str = "https://console.anthropic.com/v1/oauth/token";
pub const CLAUDE_REDIRECT_URI: &str = "https://console.anthropic.com/oauth/code/callback";
pub const CLAUDE_SCOPE: &str = "org:create_api_key user:profile user:inference";
```

#### `custom` — Fully Scripted Auth

**No existing pattern** — this is entirely handled by the Rhai script.

```
Credential source:  Script's get_auth_headers() reads its own state
Login flow:         Entirely script-defined
Header format:      Whatever get_auth_headers() returns
Refresh:            Script's needs_refresh() + refresh_token()
Credential file:    Optional, script manages via cred:: module
Detection:          If credential_file is set, check CredentialStore::exists()
                    Otherwise, always "available" (script decides)
```

### 6.2 Credential Detection for Custom Providers

Extending `ProviderCredentials::detect()` (from `credentials.rs:20-36`):

```rust
impl ProviderCredentials {
    pub fn detect() -> Self {
        let mut creds = Self {
            claude_available: /* ... existing ... */,
            openai_available: /* ... existing ... */,
            // ... etc ...
            custom_available: HashMap::new(),
        };

        // Detect custom providers
        if let Ok(configs) = discover_provider_configs() {
            for config in &configs {
                let available = detect_custom_provider_credentials(config);
                creds.custom_available.insert(config.name.clone(), available);
            }
        }

        creds
    }
}

fn detect_custom_provider_credentials(config: &ProviderConfig) -> bool {
    match &config.auth {
        AuthConfig::Bearer { env_var, .. } |
        AuthConfig::ApiKeyHeader { env_var, .. } => {
            std::env::var(env_var).is_ok()
        }
        AuthConfig::OauthDeviceCode { credential_file, .. } |
        AuthConfig::OauthPkce { credential_file, .. } => {
            let path = get_fspec_home().join(credential_file);
            path.exists()
        }
        AuthConfig::Custom { credential_file } => {
            credential_file.as_ref().map_or(true, |f| {
                get_fspec_home().join(f).exists()
            })
        }
    }
}
```

### 6.3 Summary: Auth Type → Infrastructure Mapping

| Auth Type | Env Var | Cred File | Login Flow | Refresh | Rhai Functions Needed |
|-----------|---------|-----------|------------|---------|----------------------|
| `bearer` | ✅ Required | ❌ | None | None | `get_auth_headers` |
| `api_key_header` | ✅ Required | ❌ | None | None | `get_auth_headers` |
| `oauth_device_code` | ❌ | ✅ Required | `DeviceCodeFlow` | Script | `poll_for_token`, `needs_refresh`, `get_auth_headers` |
| `oauth_pkce` | ❌ | ✅ Required | `OAuthCallbackServer` | Script | `build_authorization_request`, `exchange_code`, `refresh_token`, `needs_refresh`, `get_auth_headers` |
| `custom` | ❌ | Optional | Script | Script | All 7 functions |

---

## Appendix A: File Reference Index

| File | Lines Referenced | Content |
|------|-----------------|---------|
| `codelet/providers/src/credentials.rs` | 1-143 | `ProviderCredentials::detect()`, per-provider auth checks |
| `codelet/providers/src/claude.rs` | 1-858 | `ClaudeProvider`, `AuthMode`, beta headers, `create_rig_agent` |
| `codelet/providers/src/openai.rs` | 1-613 | `OpenAIProvider`, base URL normalization, env var config |
| `codelet/providers/src/copilot/provider.rs` | 1-293 | `CopilotProvider`, two-token model, `from_auth()` |
| `codelet/providers/src/copilot/auth.rs` | 1-462 | `CopilotAuthJson`, legacy schema compat, secure file write |
| `codelet/providers/src/copilot/constants.rs` | 1-27 | Provider ID, npm key, user-agent constants |
| `codelet/providers/src/copilot/token_exchange.rs` | 1-202 | GitHub→Copilot token exchange, `exchange_copilot_token` |
| `codelet/providers/src/copilot/oauth_device_code.rs` | 1-123 | `request_device_code`, enterprise domain normalization |
| `codelet/providers/src/claude_auth.rs` | 1-87 | `ClaudeAuthJson`, `read_claude_auth_sync`, `FSPEC_HOME` |
| `codelet/providers/src/claude_oauth.rs` | 1-334 | Claude PKCE constants, token exchange, URL rewriting |
| `codelet/providers/src/codex/codex_auth.rs` | 1-226 | `CodexAuthJson`, keychain support, `CODEX_HOME` |
| `codelet/providers/src/zai.rs` | 1-442 | `ZAIProvider`, dual endpoint support |
| `codelet/providers/src/gemini.rs` | 1-379 | `GeminiProvider`, 1M context window |
| `codelet/providers/src/manager.rs` | 1-2141 | `ProviderManager`, `ProviderType` enum, `FromStr` |
| `codelet/providers/src/oauth/engine.rs` | 1-65 | `build_sandboxed_engine`, `RhaiModule`, safety limits |
| `codelet/providers/src/oauth/building_blocks.rs` | 1-271 | `http::`, `crypto::`, `json::`, `oauth::` modules |
| `codelet/providers/src/oauth/script_provider.rs` | 1-233 | `ScriptedOAuthProvider`, `ScriptProviderConfig`, `load()` |
| `codelet/providers/src/oauth/credential_store.rs` | 1-141 | `CredentialStore<T>`, secure writes, idempotent delete |
| `codelet/providers/src/oauth/mod.rs` | 1-33 | Module structure and re-exports |
| `codelet/providers/src/models/types.rs` | 1-283 | `ModelInfo`, `LimitInfo`, `ProviderInfo` from models.dev |
| `codelet/providers/src/lib.rs` | 1-119 | Crate root, `LlmProvider` trait, `CompletionResponse` |
| `/tmp/rhai/src/api/compile.rs` | 1-315 | `Engine::compile`, `compile_with_scope`, `compile_scripts_with_scope` |
| `/tmp/rhai/src/api/files.rs` | 1-260 | `Engine::compile_file`, `compile_file_with_scope`, `read_file` |
| `/tmp/rhai/src/api/call_fn.rs` | 120-200 | `Engine::call_fn`, `call_fn_with_options` |
| `/tmp/rhai/src/ast/ast.rs` | 1-958 | `AST` struct, `iter_functions`, `shared_lib`, `iter_fn_def` |
| `/tmp/rhai/src/ast/script_fn.rs` | 1-183 | `ScriptFuncDef`, `ScriptFnMetadata` structs |
| `/tmp/rhai/src/module/mod.rs` | 1328-1345 | `Module::get_script_fn(name, num_params)` |
| `/tmp/rhai/src/module/mod.rs` | 2200-2225 | `Module::iter_script_fn()` (pub(crate)) |
| `/tmp/rhai/src/lib.rs` | 274 | `type SharedModule = Shared<Module>` |

## Appendix B: Key Rhai API Quick Reference

| API | Signature | Return | Notes |
|-----|-----------|--------|-------|
| `Engine::new_raw()` | `fn new_raw() -> Self` | `Engine` | No standard library |
| `Engine::compile()` | `fn compile(&self, script: impl AsRef<str>) -> ParseResult<AST>` | Parse errors only |
| `Engine::compile_with_scope()` | `fn compile_with_scope(&self, scope: &Scope, script: impl AsRef<str>) -> ParseResult<AST>` | Constants folded |
| `Engine::compile_file()` | `fn compile_file(&self, path: PathBuf) -> RhaiResultOf<AST>` | Parse + I/O errors |
| `Engine::compile_file_with_scope()` | `fn compile_file_with_scope(&self, scope: &Scope, path: PathBuf) -> RhaiResultOf<AST>` | Sets AST source |
| `Engine::call_fn()` | `fn call_fn<T: Variant+Clone>(&self, scope: &mut Scope, ast: &AST, fn_name: impl AsRef<str>, args: impl FuncArgs) -> RhaiResultOf<T>` | Call script fn |
| `AST::iter_functions()` | `fn iter_functions(&self) -> impl Iterator<Item = ScriptFnMetadata<'_>>` | Public API |
| `AST::shared_lib()` | `const fn shared_lib(&self) -> &SharedModule` | `internals` feature only |
| `Module::get_script_fn()` | `fn get_script_fn(&self, name: impl AsRef<str>, num_params: usize) -> Option<&Shared<ScriptFuncDef>>` | Lookup by name+arity |
| `Engine::set_max_operations()` | `fn set_max_operations(&mut self, operations: u64)` | Safety limit |
| `Engine::register_static_module()` | `fn register_static_module(&mut self, name: &str, module: SharedModule)` | Register namespace |
