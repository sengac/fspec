# AST Research: Provider Analysis for PROV-006

## Purpose
Analyze existing provider implementations to understand the pattern for adding OpenAI-compatible local model support (vLLM, Ollama).

## Research Method
Used `ast-grep` to analyze Rust code patterns in the codelet providers crate.

## Key Findings

### 1. Provider Structure Pattern

All providers follow a consistent pattern:
- Struct with `completion_model`, `rig_client`, and `model_name` fields
- `new()` constructor that detects credentials from environment
- `from_api_key()` constructor for explicit initialization
- `client()` getter for the underlying rig client
- `create_rig_agent()` to create a rig Agent with tools configured
- `LlmProvider` trait implementation

### 2. ZAI Provider Pattern (Custom Base URL)

The ZAI provider demonstrates the pattern for custom base URLs:

```rust
// ZAI constants
const ZAI_API_BASE_URL: &str = "https://api.z.ai/api/paas/v4";
const ZAI_PLAN_API_BASE_URL: &str = "https://api.z.ai/api/coding/paas/v4";

// Build client with custom base URL
let rig_client = openai::CompletionsClient::builder()
    .api_key(api_key)
    .base_url(base_url)  // <-- Custom base URL
    .build()
```

### 3. OpenAI Provider Current Implementation

Current OpenAI provider (openai.rs):
- Uses hardcoded CONTEXT_WINDOW (128,000)
- Uses hardcoded MAX_OUTPUT_TOKENS (4,096)
- Does NOT support custom base_url
- Requires OPENAI_MODEL env var

```rust
let rig_client = openai::CompletionsClient::builder()
    .api_key(api_key)
    .build()  // <-- No base_url support
```

### 4. Model Registry Pattern

Current flow:
1. `ModelCache` fetches from https://models.dev/api.json
2. `ModelRegistry` provides lookup and validation
3. Manager validates credentials and model capabilities

For local providers (vLLM/Ollama), need an alternative path that:
- Fetches model list from `{base_url}/models` endpoint
- Bypasses models.dev validation for local providers

### 5. vLLM API Compatibility

Based on vLLM source code analysis:
- Endpoint: `GET /v1/models`
- Returns: `ModelList` with `ModelCard` entries
- Each model has: `id`, `max_model_len`, `root`, `permission`
- Tool calling supported via standard OpenAI format

### 6. Ollama API Compatibility

Based on Ollama source code analysis:
- Endpoint: `GET /v1/models` (OpenAI-compatible wrapper)
- Returns: OpenAI-format `ListCompletion` with `Model` entries
- Each model has: `id`, `object`, `created`, `owned_by`
- Tool calling supported for compatible models
- Finish reasons: `tool_calls`, `stop`, `length` (same as OpenAI)

## Required Changes

### OpenAI Provider Modifications

1. **Add base_url support:**
   - Check `OPENAI_BASE_URL` environment variable
   - If set, use as base_url for rig client builder
   - If not set, use default OpenAI endpoint (current behavior)

2. **Add configurable context/output tokens:**
   - Check `OPENAI_CONTEXT_WINDOW` env var
   - Check `OPENAI_MAX_OUTPUT_TOKENS` env var
   - Fall back to defaults if not set

3. **Model list fetching:**
   - When `OPENAI_BASE_URL` is set, fetch from `{base_url}/models`
   - Parse OpenAI-format model list response
   - Skip models.dev validation for local providers

### API Key Handling

For local servers without auth:
- `OPENAI_API_KEY` can be any non-empty value (e.g., "local", "dummy")
- vLLM and Ollama typically don't require authentication

## Files to Modify

1. `codelet/providers/src/openai.rs` - Add base_url and configurable token limits
2. `codelet/providers/src/models/cache.rs` - Add local model list fetching
3. `codelet/providers/src/manager.rs` - Update get_openai() for local provider mode
4. `codelet/providers/src/credentials.rs` - Allow "local" mode for OpenAI

## Searched Patterns

```bash
# Find all provider struct implementations
ast-grep --pattern 'impl $TYPE { $$$BODY }' --lang rust codelet/providers/src

# Find all public functions
ast-grep --pattern 'pub fn $NAME($$$ARGS) -> $RET { $$$BODY }' --lang rust codelet/providers/src

# Find struct definitions
ast-grep --pattern 'struct $NAME { $$$FIELDS }' --lang rust codelet/providers/src
```

## Conclusion

The ZAI provider provides the exact pattern needed for OpenAI-compatible local model support.
Key differences for local mode:
1. Base URL from `OPENAI_BASE_URL` instead of hardcoded
2. Model list from local `/v1/models` endpoint instead of models.dev
3. Configurable context window and max output tokens via env vars
4. No authentication required (any non-empty API key works)
