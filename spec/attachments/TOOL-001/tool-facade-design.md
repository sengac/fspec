# Provider-Specific Tool Facades - Design Document

## Problem Statement

Different LLM providers expect different tool schemas, names, and parameter formats:

| Our Tools | Gemini CLI Tools | Schema Difference |
|-----------|------------------|-------------------|
| `web_search` (complex action.type) | `google_web_search` (just `{query}`) | Gemini: simple params |
| Combined search/open/find | `web_fetch` (separate tool) | Gemini: separate tools |
| `Bash` | `run_shell_command` | Different name |
| `Grep` | `search_file_content` | Different name |
| `Read` | `read_file` | Different name |
| `Write` | `write_file` | Different name |
| `Edit` | `replace` | Different name |
| `Glob` | `glob` | Same name |
| `ls` | `list_directory` | Different name |

### Root Cause

1. **Schema incompatibility**: Gemini doesn't understand `oneOf` schemas with action types well
2. **Parameter format differences**: Gemini uses flat params, Claude uses nested action objects
3. **Tool naming conventions**: Different providers use different naming patterns
4. **Tool granularity**: What's one tool for Claude might be two tools for Gemini

### Evidence from Gemini CLI Repository

Source: https://github.com/google-gemini/gemini-cli

Gemini CLI's web search is much simpler:
- **Tool name**: `google_web_search`
- **Parameters**: Just `{ query: string }`
- **No action type**: Single-purpose tool

Gemini CLI has separate `web_fetch` tool:
- **Tool name**: `web_fetch`
- **Parameters**: `{ prompt: string }` (URL + instructions)
- **Purpose**: Fetch and process URL content

## Proposed Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Provider Layer                          │
├─────────────┬─────────────┬─────────────┬──────────────────┤
│   Claude    │   Gemini    │   OpenAI    │     Codex        │
│  Facade     │   Facade    │   Facade    │    Facade        │
├─────────────┴─────────────┴─────────────┴──────────────────┤
│                  Tool Adapter Layer                         │
│         (maps provider params → internal params)            │
├────────────────────────────────────────────────────────────┤
│                 Base Tool Implementation                    │
│              (Chrome browser, actual work)                  │
└────────────────────────────────────────────────────────────┘
```

## Design Details

### 1. ToolFacade Trait

```rust
/// Provider-specific tool facade
pub trait ToolFacade: Send + Sync {
    /// Provider this facade is for
    fn provider(&self) -> &'static str;

    /// Tool name as the provider expects it
    fn tool_name(&self) -> &'static str;

    /// Tool definition (schema) for this provider
    fn definition(&self) -> ToolDefinition;

    /// Map provider-specific params to internal params
    fn map_params(&self, input: serde_json::Value) -> Result<InternalParams, ToolError>;
}
```

### 2. Provider Tool Registry

```rust
pub struct ProviderToolRegistry {
    facades: HashMap<(&'static str, &'static str), Arc<dyn ToolFacade>>,
}

impl ProviderToolRegistry {
    /// Get tools for a specific provider
    pub fn tools_for_provider(&self, provider: &str) -> Vec<Arc<dyn ToolFacade>> {
        self.facades
            .iter()
            .filter(|((p, _), _)| *p == provider)
            .map(|(_, facade)| Arc::clone(facade))
            .collect()
    }
}
```

### 3. Web Search Facades Example

#### Claude Facade
```rust
pub struct ClaudeWebSearchFacade;

impl ToolFacade for ClaudeWebSearchFacade {
    fn provider(&self) -> &'static str { "claude" }
    fn tool_name(&self) -> &'static str { "web_search" }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "web_search".to_string(),
            description: "Perform web search, open pages, or find content".to_string(),
            parameters: json!({
                "properties": {
                    "action": {
                        "oneOf": [
                            { "type": "object", "properties": {
                                "type": { "const": "search" },
                                "query": { "type": "string" }
                            }},
                            { "type": "object", "properties": {
                                "type": { "const": "open_page" },
                                "url": { "type": "string" }
                            }},
                            // ... find_in_page
                        ]
                    }
                },
                "required": ["action"]
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalParams, ToolError> {
        // Claude sends: { action: { type: "search", query: "..." } }
        // Map to internal: WebSearchParams::Search { query: "..." }
        let action = input.get("action").ok_or(...)?;
        match action.get("type").and_then(|t| t.as_str()) {
            Some("search") => Ok(InternalParams::Search {
                query: action.get("query").and_then(|q| q.as_str()).unwrap_or("").to_string()
            }),
            Some("open_page") => Ok(InternalParams::OpenPage {
                url: action.get("url").and_then(|u| u.as_str()).unwrap_or("").to_string()
            }),
            // ...
        }
    }
}
```

#### Gemini Facades (Two separate tools)

```rust
pub struct GeminiGoogleWebSearchFacade;

impl ToolFacade for GeminiGoogleWebSearchFacade {
    fn provider(&self) -> &'static str { "gemini" }
    fn tool_name(&self) -> &'static str { "google_web_search" }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "google_web_search".to_string(),
            description: "Search the web using Google Search".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query"
                    }
                },
                "required": ["query"]
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalParams, ToolError> {
        // Gemini sends: { query: "..." }
        // Map to internal: WebSearchParams::Search { query: "..." }
        let query = input.get("query")
            .and_then(|q| q.as_str())
            .ok_or(ToolError::Validation { tool: "google_web_search", message: "query required" })?;
        Ok(InternalParams::Search { query: query.to_string() })
    }
}

pub struct GeminiWebFetchFacade;

impl ToolFacade for GeminiWebFetchFacade {
    fn provider(&self) -> &'static str { "gemini" }
    fn tool_name(&self) -> &'static str { "web_fetch" }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "web_fetch".to_string(),
            description: "Fetch and process content from URLs".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "prompt": {
                        "type": "string",
                        "description": "URL(s) and instructions for processing"
                    }
                },
                "required": ["prompt"]
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalParams, ToolError> {
        // Gemini sends: { prompt: "https://example.com summarize this" }
        // Parse URL from prompt and map to internal: WebSearchParams::OpenPage { url: "..." }
        let prompt = input.get("prompt").and_then(|p| p.as_str()).unwrap_or("");
        let url = extract_url_from_prompt(prompt)?;
        Ok(InternalParams::OpenPage { url })
    }
}
```

### 4. Base Tool Implementation (Unchanged)

```rust
pub struct WebSearchImpl;

impl WebSearchImpl {
    pub async fn execute(&self, params: InternalParams) -> Result<WebSearchResult, ToolError> {
        match params {
            InternalParams::Search { query } => perform_web_search(&query),
            InternalParams::OpenPage { url } => open_page(&url),
            InternalParams::FindInPage { url, pattern } => find_in_page(&url, &pattern),
        }
    }
}
```

### 5. Integration with Agent Builder

```rust
impl GeminiProvider {
    pub fn create_rig_agent(&self, preamble: Option<&str>) -> Agent<...> {
        let registry = ProviderToolRegistry::default();
        let tools = registry.tools_for_provider("gemini");

        let mut builder = self.client.agent(&self.model_name);

        for facade in tools {
            builder = builder.tool(FacadeToolWrapper::new(facade));
        }

        builder.build()
    }
}
```

## Tool Mapping Summary

### Web Search

| Provider | Tool Name(s) | Schema |
|----------|-------------|--------|
| Claude | `web_search` | `{action: {type, query/url/pattern}}` |
| Gemini | `google_web_search` | `{query}` |
| Gemini | `web_fetch` | `{prompt}` |
| OpenAI | TBD | TBD |

### File Operations

| Provider | Read | Write | Edit |
|----------|------|-------|------|
| Claude | `Read` | `Write` | `Edit` |
| Gemini | `read_file` | `write_file` | `replace` |

### Shell/Bash

| Provider | Tool Name | Schema |
|----------|-----------|--------|
| Claude | `Bash` | `{command, timeout?, ...}` |
| Gemini | `run_shell_command` | `{command}` |

### Search/Grep

| Provider | Tool Name | Schema |
|----------|-----------|--------|
| Claude | `Grep` | Complex with modes |
| Gemini | `search_file_content` | `{pattern, path}` |

## Implementation Plan

### Phase 1: Infrastructure
1. Create `ToolFacade` trait
2. Create `ProviderToolRegistry`
3. Create `FacadeToolWrapper` for rig integration

### Phase 2: Web Search Migration
1. Create `ClaudeWebSearchFacade`
2. Create `GeminiGoogleWebSearchFacade`
3. Create `GeminiWebFetchFacade`
4. Update provider agent builders

### Phase 3: Other Tools
1. File operations (Read, Write, Edit)
2. Shell/Bash
3. Search (Grep, Glob)
4. Directory listing (ls)
5. AST tools

### Phase 4: Testing
1. Unit tests for each facade's param mapping
2. Integration tests with mock providers
3. E2E tests with real providers

## Benefits

1. **Provider compatibility**: Each provider gets schemas they understand
2. **Shared implementation**: No code duplication for actual tool logic
3. **Easy extensibility**: Add new providers by adding facades
4. **Clear separation**: Provider concerns separate from tool logic
5. **Testability**: Facades can be tested independently

## References

- Gemini CLI Repository: https://github.com/google-gemini/gemini-cli
- Gemini CLI Tool Names: `/packages/core/src/tools/tool-names.ts`
- Gemini CLI Web Search: `/packages/core/src/tools/web-search.ts`
- Gemini CLI Web Fetch: `/packages/core/src/tools/web-fetch.ts`
