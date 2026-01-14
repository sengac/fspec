# Z.AI GLM Provider Research

## Overview

This document contains research findings for integrating Z.AI (Zhipu AI) GLM models into fspec's codelet infrastructure.

## Z.AI API Details

### Base URL
```
https://api.z.ai/api/paas/v4
```

### Environment Variable
```
ZAI_API_KEY
```

### API Compatibility
**Z.AI is fully OpenAI-compatible**, meaning it uses the standard OpenAI chat completions format:
- Endpoint: `{base_url}/chat/completions`
- Authentication: `Authorization: Bearer {api_key}`
- Request/Response format: OpenAI-compatible JSON

### Supported Models

From vtcode's implementation (`vtcode-config/src/constants.rs`):

| Model ID | Description | Reasoning Support |
|----------|-------------|-------------------|
| `glm-4-plus` | Top flagship model | ✅ Yes |
| `glm-4.7` | Latest flagship (default) | ✅ Yes |
| `glm-4.6` | Previous flagship | ✅ Yes |
| `glm-4.6v` | Vision model | ❌ No |
| `glm-4.6v-flash` | Fast vision model | ❌ No |
| `glm-4.6v-flashx` | Extra fast vision | ❌ No |
| `glm-4.5` | Stable flagship | ✅ Yes |
| `glm-4.5-air` | Lightweight variant | ✅ Yes |
| `glm-4.5-x` | Extended variant | ✅ Yes |
| `glm-4.5-airx` | Lightweight extended | ✅ Yes |
| `glm-4.5-flash` | Fast variant | ✅ Yes |
| `glm-4.5v` | Vision variant | ❌ No |
| `glm-4-32b-0414-128k` | Large context model | ❌ No |

### Default Model
```
glm-4.7
```

## API Features

### Thinking Mode (Reasoning)
Z.AI supports "thinking mode" via the `thinking` parameter in requests:

```json
{
  "model": "glm-4.7",
  "messages": [...],
  "thinking": {
    "type": "enabled"  // or "disabled"
  }
}
```

Thinking mode is supported on reasoning models (GLM-4.7, GLM-4.6, GLM-4.5, etc.)

### Tool Calling
Z.AI supports OpenAI-compatible function/tool calling:
- Uses `tools` array with function definitions
- Supports `tool_choice` parameter (`auto`, `none`, `required`)
- For streaming with tools, set `tool_stream: true`

### Response Format
Supports JSON mode via:
```json
{
  "response_format": { "type": "json_object" }
}
```

### Reasoning Content
Z.AI returns reasoning in the `reasoning_content` field of delta/message:
- Can be a string or array of strings
- Present when thinking mode is enabled

### Usage Tracking
Z.AI returns token usage with special handling for cached tokens:
```json
{
  "usage": {
    "prompt_tokens": 100,
    "completion_tokens": 50,
    "total_tokens": 150,
    "prompt_tokens_details": {
      "cached_tokens": 20
    }
  }
}
```

## Error Handling

### Error Codes (from vtcode research)

| Code Range | Category | Description |
|------------|----------|-------------|
| 1000-1004 | Authentication | Token/API key issues |
| 1110-1121 | Account | Account-related issues |
| 1113 | Balance/Quota | Insufficient balance |
| 1302-1309 | Rate Limit | Rate limiting |

### Error Response Format
```json
{
  "error": {
    "code": "1302",
    "message": "High concurrency usage of this API"
  }
}
```

## Implementation Reference: vtcode

vtcode has a fully functional Z.AI provider at:
- `vtcode-core/src/llm/providers/zai.rs` (958 lines)
- `vtcode-config/src/constants.rs` (model definitions)
- `vtcode-config/src/api_keys.rs` (credential handling)

Key implementation details from vtcode:
1. Uses standard HTTP client with bearer auth
2. Converts requests to OpenAI-compatible format
3. Custom handling for `reasoning_content` field
4. Supports both streaming and non-streaming
5. Detailed error classification for rate limits, auth, etc.

## Implementation Reference: opencode

opencode uses the Vercel AI SDK ecosystem (`@ai-sdk/openai-compatible`). Z.AI would be accessed through:
```typescript
import { createOpenAICompatible } from '@ai-sdk/openai-compatible';

const client = createOpenAICompatible({
  baseURL: 'https://api.z.ai/api/paas/v4',
  apiKey: process.env.ZAI_API_KEY,
});
```

## rig Library Status

**rig does NOT have native Z.AI support** as of the latest version. The supported providers are:
- OpenAI, Anthropic, Cohere, Gemini, Mistral, xAI
- Together, HuggingFace, OpenRouter, Groq, Ollama
- DeepSeek, Perplexity, Moonshot, Hyperbolic
- Mira, Galadriel, Azure, VoyageAI

However, since Z.AI is OpenAI-compatible, we can:
1. Use rig's OpenAI provider with custom base_url
2. Or create a dedicated ZAI provider module

## Recommended Implementation for fspec

### Option 1: Extend rig with Custom Provider (Recommended)

Create a new Z.AI provider in `codelet/providers/src/zai.rs` following the pattern of `claude.rs` and `gemini.rs`:

```rust
// codelet/providers/src/zai.rs
use rig::providers::openai; // Use OpenAI as base

const ZAI_BASE_URL: &str = "https://api.z.ai/api/paas/v4";
const DEFAULT_MODEL: &str = "glm-4.7";

pub struct ZAIProvider {
    client: openai::Client, // Use OpenAI client with custom base_url
    model_name: String,
}

impl ZAIProvider {
    pub fn new() -> Result<Self, ProviderError> {
        let api_key = std::env::var("ZAI_API_KEY")
            .map_err(|_| ProviderError::auth("zai", "ZAI_API_KEY not set"))?;
        
        let client = openai::ClientBuilder::default()
            .api_key(&api_key)
            .base_url(ZAI_BASE_URL)
            .build()?;
        
        Ok(Self {
            client,
            model_name: DEFAULT_MODEL.to_string(),
        })
    }
}
```

### Option 2: Add to Provider Registry

Update `src/utils/provider-config.ts`:

```typescript
{
  id: 'zai',
  name: 'Z.AI',
  baseUrl: 'https://api.z.ai/api/paas/v4',
  envVar: 'ZAI_API_KEY',
  authMethod: 'bearer',
  requiresApiKey: true,
  description: 'Z.AI GLM models',
}
```

### Files to Modify

1. **Provider Config** (`src/utils/provider-config.ts`)
   - Add 'zai' to SUPPORTED_PROVIDERS
   - Add ZAI registry entry

2. **Credentials** (`src/utils/credentials.ts`)
   - Add ZAI_API_KEY handling

3. **Rust Provider** (`codelet/providers/src/`)
   - Create `zai.rs` provider implementation
   - Update `lib.rs` to export ZAIProvider
   - Update `manager.rs` to include ZAI

4. **Facades** (`codelet/tools/src/facade/`)
   - Create ZAI-specific facades if needed (likely can use OpenAI facades)

5. **Models** (`codelet/providers/src/models/`)
   - Add Z.AI models to registry
   - Add model info for GLM-4.7, GLM-4.6, etc.

6. **TUI** (`src/tui/components/AgentView.tsx`)
   - Add provider ID mapping for 'zai'

## Testing Considerations

1. **Authentication**: Test with ZAI_API_KEY env var
2. **Streaming**: Test streaming responses with tools
3. **Reasoning**: Test thinking mode on supported models
4. **Error Handling**: Test rate limit and auth errors
5. **Tool Calling**: Test function calling compatibility

## Z.AI Documentation References

- API Docs: https://docs.z.ai/guides/develop/openai/python
- OpenAI Compatibility: Z.AI provides OpenAI-compatible interfaces
- Model List: https://docs.z.ai/api-reference/introduction
