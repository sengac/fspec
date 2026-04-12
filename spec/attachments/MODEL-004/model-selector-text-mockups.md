
# Model Selector — Text Mockups

## 1. CURRENT VIEW (before MODEL-004/005)

```
┌──────────────────────────────────────────────────────────────────────┐
│  Select Model                              (14 models)              │
│                                                                      │
│  ▼ Anthropic (3 models)                                              │
│    > claude-sonnet-4              [R]      [200k]  (current)         │
│      claude-opus-4                [R]      [200k]                    │
│      claude-haiku-4                        [200k]                    │
│  ▶ Codex (ChatGPT) (4 models)                                       │
│  ▶ Google (3 models)                                                 │
│  ▼ 📁 openai: work-vllm (2 models)                                  │
│      Qwen/Qwen3-80B                       [128k]                    │
│      meta-llama/Meta-Llama-3.1-405B       [128k]                    │
│  ▼ 📁 openai: ollama-local (1 model)                                │
│      deepseek-coder-v2                     [128k]                    │
│  ▶ Z.AI (1 model)                                                   │
│                                                                      │
│                                                                      │
│                                                                      │
│  Enter: select | ←→: collapse/expand | r: refresh | Tab: providers  │
│  [R] Reasoning | [V] Vision | 📁 Profile (local server)             │
└──────────────────────────────────────────────────────────────────────┘
```

**Problems visible here:**
- vLLM models show `[128k]` — this is the profile default, not the actual model context
- No way to add a model that isn't returned by `/v1/models`
- All profile models use OpenAI tool schemas — no facade control



## 2. AFTER MODEL-005 (per-model context windows)

```
┌──────────────────────────────────────────────────────────────────────┐
│  Select Model                              (14 models)              │
│                                                                      │
│  ▼ Anthropic (3 models)                                              │
│    > claude-sonnet-4              [R]      [200k]  (current)         │
│      claude-opus-4                [R]      [200k]                    │
│      claude-haiku-4                        [200k]                    │
│  ▼ Codex (ChatGPT) (4 models)                                       │
│      o3                           [R]      [200k]                    │
│      o3-pro                       [R]      [200k]                    │
│      gpt-4.1                               [1M]                     │
│      codex-mini                   [R]      [272k]                    │
│  ▶ Google (3 models)                                                 │
│  ▼ 📁 openai: work-vllm (2 models)                                  │
│      Qwen/Qwen3-80B                       [32k]                     │
│      meta-llama/Meta-Llama-3.1-405B       [128k]                    │
│  ▶ Z.AI (1 model)                                                   │
│                                                                      │
│                                                                      │
│  Enter: select | ←→: collapse/expand | r: refresh | Tab: providers  │
│  [R] Reasoning | [V] Vision | 📁 Profile (local server)             │
└──────────────────────────────────────────────────────────────────────┘
```

**What changed:**
- Context windows now reflect actual per-model values from models.dev
- o3 shows `[200k]` instead of OpenAI's default `[128k]`
- Profile models can show per-model overrides (Qwen at `[32k]`)
- Compaction engine now uses these real values (not shown in UI but working)



## 3. AFTER MODEL-004 (custom models + facade override)

### 3a. Normal view — custom models visible with [C] badge

```
┌──────────────────────────────────────────────────────────────────────┐
│  Select Model                              (17 models)              │
│                                                                      │
│  ▼ Anthropic (3 models)                                              │
│      claude-sonnet-4              [R]      [200k]  (current)         │
│      claude-opus-4                [R]      [200k]                    │
│      claude-haiku-4                        [200k]                    │
│  ▶ Codex (ChatGPT) (4 models)                                       │
│  ▶ Google (3 models)                                                 │
│  ▼ 📁 openai: work-vllm (5 models)                                  │
│      Qwen/Qwen3-80B              [C]      [32k]                     │
│      meta-llama/Meta-Llama-3.1-405B       [128k]                    │
│    > my-fine-tuned-gpt            [C]      [8k]                      │
│      internal-codex-proxy         [C] [R]  [200k]                    │
│      test-model-v2                [C] [V]  [64k]                     │
│  ▼ 📁 openai: ollama-local (2 models)                               │
│      deepseek-coder-v2                     [128k]                    │
│      phi-4-reasoning              [C] [R]  [16k]                     │
│  ▶ Z.AI (1 model)                                                   │
│                                                                      │
│  Enter: select | ←→: collapse | a: add model | e: edit | d: delete  │
│  [R] Reasoning | [V] Vision | [C] Custom | 📁 Profile               │
└──────────────────────────────────────────────────────────────────────┘
```

**What's new:**
- `[C]` badge (yellow) marks custom/user-added models
- `Qwen/Qwen3-80B` has `[C]` because user added custom config overriding the auto-discovered entry (e.g. to set context to 32k)
- `my-fine-tuned-gpt`, `internal-codex-proxy`, `test-model-v2` are fully custom — not from `/v1/models`
- Footer shows `a: add model | e: edit | d: delete` keybinds
- Legend now includes `[C] Custom`


### 3b. Unreachable server — custom models still visible

```
┌──────────────────────────────────────────────────────────────────────┐
│  Select Model                              (3 models)               │
│                                                                      │
│  ▶ Anthropic (3 models)                                              │
│  ▼ 📁 openai: work-vllm (3 models)                                  │
│    > my-fine-tuned-gpt            [C]      [8k]                      │
│      internal-codex-proxy         [C] [R]  [200k]                    │
│      test-model-v2                [C] [V]  [64k]                     │
│                                                                      │
│                                                                      │
│                                                                      │
│                                                                      │
│  Enter: select | ←→: collapse | a: add model | e: edit | d: delete  │
│  [R] Reasoning | [V] Vision | [C] Custom | 📁 Profile               │
└──────────────────────────────────────────────────────────────────────┘
```

**Key:** Server is down but profile still shows 3 custom models. No `(unreachable)` shown because the section is functional.


### 3c. Add Custom Model form (pressing `a` on a profile section)

```
┌──────────────────────────────────────────────────────────────────────┐
│  Add Custom Model — openai: work-vllm                                │
│                                                                      │
│  ┌────────────────────────────────────────────────────────────────┐  │
│  │                                                                │  │
│  │  Model ID *        [my-custom-model                        ]  │  │
│  │                                                                │  │
│  │  Display Name      [My Custom Model                        ]  │  │
│  │                                                                │  │
│  │  Facade            < openai (default) >                       │  │
│  │                      openai │ codex │ claude │ gemini │ zai   │  │
│  │                                                                │  │
│  │  Context Window    [131072                                 ]  │  │
│  │                                                                │  │
│  │  Max Output        [16384                                  ]  │  │
│  │                                                                │  │
│  │  Reasoning         [ ] No                                     │  │
│  │                                                                │  │
│  │  Vision            [ ] No                                     │  │
│  │                                                                │  │
│  └────────────────────────────────────────────────────────────────┘  │
│                                                                      │
│  ↑↓: navigate fields | ←→: cycle options | Enter: save | Esc: cancel│
└──────────────────────────────────────────────────────────────────────┘
```


### 3d. Edit Custom Model form (pressing `e` on a custom model)

```
┌──────────────────────────────────────────────────────────────────────┐
│  Edit Custom Model — openai: work-vllm                               │
│                                                                      │
│  ┌────────────────────────────────────────────────────────────────┐  │
│  │                                                                │  │
│  │  Model ID *        [internal-codex-proxy                   ]  │  │
│  │                                                                │  │
│  │  Display Name      [Internal Codex Proxy                   ]  │  │
│  │                                                                │  │
│  │  Facade          ▸ < codex >                                  │  │
│  │                      openai │ codex │ claude │ gemini │ zai   │  │
│  │                                                                │  │
│  │  Context Window    [200000                                 ]  │  │
│  │                                                                │  │
│  │  Max Output        [4096                                   ]  │  │
│  │                                                                │  │
│  │  Reasoning         [✓] Yes                                    │  │
│  │                                                                │  │
│  │  Vision            [ ] No                                     │  │
│  │                                                                │  │
│  └────────────────────────────────────────────────────────────────┘  │
│                                                                      │
│  ↑↓: navigate fields | ←→: cycle options | Enter: save | Esc: cancel│
└──────────────────────────────────────────────────────────────────────┘
```

**Key:** The `Facade` field shows `codex` selected — this means when this model is active, the agent gets Codex tool schemas (`exec_command`, `shell`, `read_file`, etc.) instead of standard OpenAI function calling. The `▸` indicator shows the currently focused field.


### 3e. Delete confirmation (pressing `d` on a custom model)

```
┌──────────────────────────────────────────────────────────────────────┐
│  Select Model                              (17 models)              │
│                                                                      │
│  ▼ 📁 openai: work-vllm (5 models)                                  │
│      Qwen/Qwen3-80B              [C]      [32k]                     │
│      meta-llama/Meta-Llama-3.1-405B       [128k]                    │
│    > my-fine-tuned-gpt            [C]      [8k]                      │
│      ┌──────────────────────────────────────────────────┐            │
│      │  Delete custom model "my-fine-tuned-gpt"?        │            │
│      │                                                  │            │
│      │  This will remove the model from your config.    │            │
│      │  If it exists on the server, it will revert      │            │
│      │  to auto-discovered defaults.                    │            │
│      │                                                  │            │
│      │          [Yes, delete]     [Cancel]               │            │
│      └──────────────────────────────────────────────────┘            │
│      test-model-v2                [C] [V]  [64k]                     │
│                                                                      │
│                                                                      │
│  Enter: confirm | Esc: cancel                                        │
│  [R] Reasoning | [V] Vision | [C] Custom | 📁 Profile               │
└──────────────────────────────────────────────────────────────────────┘
```


## 4. WHAT THE CONFIG LOOKS LIKE

```json
// ~/.fspec/fspec-config.json
{
  "providers": {
    "openai": {
      "profiles": {
        "work-vllm": {
          "baseUrl": "http://10.0.0.5:8888",
          "apiKey": "sk-xxx",
          "contextWindow": 128000,
          "maxOutputTokens": 16384,
          "customModels": [
            {
              "id": "my-fine-tuned-gpt",
              "displayName": "My Fine-Tuned GPT",
              "contextWindow": 8192,
              "maxOutputTokens": 4096
            },
            {
              "id": "internal-codex-proxy",
              "displayName": "Internal Codex Proxy",
              "facade": "codex",
              "contextWindow": 200000,
              "maxOutputTokens": 4096,
              "reasoning": true
            },
            {
              "id": "test-model-v2",
              "displayName": "Test Model v2",
              "contextWindow": 65536,
              "hasVision": true
            },
            {
              "id": "Qwen/Qwen3-80B",
              "contextWindow": 32768
            }
          ]
        },
        "ollama-local": {
          "baseUrl": "http://localhost:11434/v1",
          "apiKey": "ollama",
          "customModels": [
            {
              "id": "phi-4-reasoning",
              "displayName": "Phi-4 Reasoning",
              "contextWindow": 16384,
              "reasoning": true
            }
          ]
        }
      }
    }
  }
}
```

**Notes on config:**
- `customModels` is optional — existing configs work without it
- `Qwen/Qwen3-80B` overrides the auto-discovered entry (only `contextWindow` changed)
- `facade` is only set on `internal-codex-proxy` — others use default OpenAI facade
- `id` is the only required field — everything else is optional


## 5. FACADE EFFECT (what the model sees)

### Default (OpenAI) facade — standard function calling:
```
Tools: Read, Write, Edit, Bash, Grep, Glob, Ls, Fspec, WebSearch, ...
Schema: OpenAI function_call format
```

### Codex facade — Codex CLI-compatible:
```
Tools: shell_command, read_file, write_file, apply_patch, grep_files,
       list_dir, exec_command, shell, write_stdin, request_user_input
Schema: Codex-native format with include/limit/indentation params
```

### Claude facade — Claude-native:
```
Tools: Read, Write, Edit, Bash, Grep, Glob, Ls (PascalCase)
Schema: Claude tool_use format
```

### Gemini facade — Gemini-native:
```
Tools: read_file, write_file, replace, run_shell_command,
       search_file_content, list_directory, google_web_search
Schema: Gemini snake_case with function_declarations
```

