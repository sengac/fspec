# RIG-001: rig-core Dependency Audit & Migration Estimate

> **Date:** 2026-04-14
> **Method:** GraphSearch AST analysis + DeepSearch code exploration + manual line counting
> **Scope:** `codelet/patches/rig-core/` (patched fork) vs. `codelet/{core,providers,tools,napi,cli}/` (consumers)

---

## TL;DR

| Metric | Lines |
|--------|------:|
| **Total patched rig-core we carry** | **56,042** (45,545 source + 10,497 examples/tests) |
| **Lines we actually use** | **~18,400** |
| **Lines that are pure dead weight** | **~27,100** (59.5% of source) |
| **Net patch delta** | ~169 lines of actual changes spread across 22 files |

---

## 1. What the Patch Actually Changes

The patch modifies **22 files** from upstream rig-core v0.28.0, but most changes are trivial `..Default::default()` additions. The **substantive changes** are in only **7 files**:

| File | Change | Delta |
|------|--------|------:|
| `agent/prompt_request/streaming.rs` | `parse_tool_result_content()` for image/PDF tool results, reasoning block capture, `Usage` variant | +138/−5 |
| `completion/request.rs` | `cache_read_input_tokens`, `cache_creation_input_tokens` fields on `Usage` | +40 |
| `providers/anthropic/completion.rs` | Cache token mapping, `ToolResultContent::Image` serialization fix | +17/−11 |
| `providers/anthropic/streaming.rs` | Cache token fields in `PartialUsage` | +11 |
| `providers/anthropic/client.rs` | OAuth token (`sk-ant-oat`) detection | +7 |
| `providers/gemini/completion.rs` | `ThinkingConfig` optional budget + `thinking_level` field | +14/−1 |
| `streaming.rs` | `Usage` variant in `RawStreamingChoice` and `StreamedAssistantContent` | +11 |
| `Cargo.toml` | Workspace→standalone conversion | +32/−100 |
| 13 other provider files | `..Default::default()` (1 line each) | +13 |

### Patch Purpose Summary

1. **Standalone compilation** — no workspace dependency
2. **Image/PDF vision** — tool results containing base64 images are properly typed as `ToolResultContent::Image` rather than dumped as text
3. **Prompt caching observability** — Anthropic's cache hit/creation tokens tracked through the full stack
4. **OAuth auth** — supports Claude Code's OAuth token flow
5. **Extended thinking** — reasoning blocks correctly preserved in multi-turn chat history; Gemini 3's `thinking_level` supported
6. **Real-time usage streaming** — `Usage` events emitted mid-stream (not just at end)
7. **Testing support** — `CancelSignal` internals exposed for test harnesses

---

## 2. What We Import from rig-core (~30 leaf types across ~80 files)

### By Subsystem

| Category | Types Used | Consuming Crate(s) |
|----------|-----------|-------------------|
| **Messages** (11 types) | `Message`, `UserContent`, `AssistantContent`, `Text`, `ToolCall`, `ToolFunction`, `ToolResult`, `ToolResultContent`, `Image`, `ImageMediaType`, `DocumentSourceKind` | cli, core, napi, tools |
| **Completion** (6 types) | `CompletionModel`, `CompletionRequestBuilder`, `ToolDefinition`, `Usage`, `GetTokenUsage`, `Prompt` | core, providers, tools |
| **Agent** (4 types) | `Agent`, `CancelSignal`, `StreamingPromptHook`, `MultiTurnStreamItem` | core, cli, napi |
| **Streaming** (3 types) | `StreamedAssistantContent`, `StreamedUserContent`, `StreamingPrompt` | cli, core |
| **Tool** (2 types) | `Tool` (trait), `ToolServerHandle` | tools (34 files!), mcp.rs |
| **Client** (2 types) | `CompletionClient`, `Provider` | providers |
| **Utility** (2 types) | `OneOrMany<T>`, `WasmCompatSend` | cli, core, napi, tools |
| **Providers** (3 modules) | `anthropic::*`, `openai::*`, `gemini::*` | providers |
| **HTTP** (1 trait) | `HttpClientExt` | provider tests only |

### By Consuming Crate

| Crate | Primary rig modules consumed |
|---|---|
| **codelet/core** | `agent`, `completion`, `message`, `streaming`, `wasm_compat` |
| **codelet/providers** | `client`, `completion`, `http_client`, `providers` |
| **codelet/tools** | `tool` (dominant), `completion::ToolDefinition`, `message`, `one_or_many` |
| **codelet/napi** | `client`, `completion`, `message`, `OneOrMany`, `tool::Tool`, `agent::MultiTurnStreamItem` |
| **codelet/cli** | `agent`, `completion`, `message` (heavy), `OneOrMany` (heavy), `streaming`, `wasm_compat` |

---

## 3. Full rig-core LOC Breakdown (45,545 source lines)

### Top-Level

| Subsystem | LOC | % of Source |
|-----------|----:|----------:|
| **src/providers/** | 26,596 | 58.4% |
| src/root files | 3,315 | 7.3% |
| src/agent/ | 2,538 | 5.6% |
| src/completion/ | 2,089 | 4.6% |
| src/pipeline/ | 2,153 | 4.7% |
| src/client/ | 1,830 | 4.0% |
| src/loaders/ | 1,582 | 3.5% |
| src/vector_store/ | 1,472 | 3.2% |
| src/tool/ | 1,113 | 2.4% |
| src/http_client/ | 1,083 | 2.4% |
| src/embeddings/ | 1,079 | 2.4% |
| src/integrations/ | 486 | 1.1% |
| src/telemetry/ | 112 | 0.2% |
| src/tools/ | 97 | 0.2% |

### Provider Breakdown (26,596 lines — the elephant in the room)

| Provider | LOC | Used? |
|----------|----:|:-----:|
| **OpenAI** (completion + responses_api) | 5,186 | ✅ |
| **Anthropic** (+ decoders) | 3,684 | ✅ |
| **Gemini** | 3,342 | ✅ |
| Ollama | 1,391 | ❌ |
| Cohere | 1,672 | ❌ |
| HuggingFace | 1,668 | ❌ |
| OpenRouter | 1,340 | ❌ |
| Azure | 1,060 | ❌ |
| DeepSeek | 1,003 | ❌ |
| Mistral | 950 | ❌ |
| Mira | 790 | ❌ |
| Groq | 775 | ❌ |
| Hyperbolic | 695 | ❌ |
| Together | 680 | ❌ |
| Galadriel | 652 | ❌ |
| Perplexity | 530 | ❌ |
| xAI | 490 | ❌ |
| Moonshot | 366 | ❌ |
| VoyageAI | 256 | ❌ |

**3 of 19 providers used = 12,212 lines kept / 14,384 lines wasted**

---

## 4. Needed vs. Dead Weight

```
TOTAL rig-core source:                  45,545 lines (100%)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

DEAD WEIGHT (can delete immediately):
  16 unused providers                   14,318 lines (31.4%)
  7 unused subsystems                    6,981 lines (15.3%)
    (embeddings, vector_store, pipeline,
     loaders, integrations, telemetry, tools/)
  Unused root files                      1,483 lines  (3.3%)
    (evals, extractor, image_gen,
     audio_gen, transcription, prelude)
  Provider feature trim                    899 lines  (2.0%)
    (embedding/audio/transcription in
     Anthropic, OpenAI, Gemini)
  In-file test blocks                    1,069 lines  (2.3%)
  WASM compat                               76 lines  (0.2%)
  ToolServer (MCP runtime)                 430 lines  (0.9%)
  Client unused features                   453 lines  (1.0%)
                                        ──────
  TOTAL DEAD WEIGHT:                   ~25,709 lines (56.4%)

MUST KEEP / REWRITE:
  Core framework                        ~5,997 lines (13.2%)
    agent loop, message types, completion
    model trait, streaming, OneOrMany
  Client + HTTP infrastructure          ~2,007 lines  (4.4%)
    CompletionClient, builder, SSE
    parsing, retry logic
  Anthropic provider                    ~3,684 lines  (8.1%)
  OpenAI provider                       ~4,702 lines (10.3%)
  Gemini provider                       ~2,927 lines  (6.4%)
  lib.rs + json_utils                     ~354 lines  (0.8%)
                                        ──────
  TOTAL TO KEEP:                       ~18,400 lines (40.4%)
```

### Detailed "Must Keep" Files

#### Core Framework (~5,997 lines)

| File | Lines | What It Provides |
|------|------:|-----------------|
| `agent/prompt_request/streaming.rs` | 801 | Multi-turn streaming loop, `parse_tool_result_content()`, `MultiTurnStreamItem`, `StreamingPromptHook` |
| `agent/prompt_request/mod.rs` | 571 | `CancelSignal`, prompt request builder |
| `agent/completion.rs` | 281 | `Agent<M>` struct |
| `agent/builder.rs` | 531 | Agent builder (could simplify) |
| `agent/mod.rs` | 122 | Module re-exports |
| `agent/tool.rs` | 50 | Tool dispatch helpers |
| `completion/request.rs` | 764 | `CompletionModel` trait, `Usage`, `ToolDefinition`, `CompletionRequestBuilder` |
| `completion/message.rs` | 1,085 | All 11 message types |
| `completion/mod.rs` | 5 | Module re-exports |
| `tool/mod.rs` | 570 | `Tool` trait, `ToolDyn`, `ToolSet` |
| `streaming.rs` | 513 | `StreamedAssistantContent`, `StreamedUserContent`, `StreamingPrompt`, `StreamingCompletion` |
| `one_or_many.rs` | 463 | `OneOrMany<T>` with iterators and conversions |
| `json_utils.rs` | 205 | JSON serialization helpers |
| `lib.rs` | 149 | Crate root, re-exports |

#### Client + HTTP (~2,007 lines)

| File | Lines | What It Provides |
|------|------:|-----------------|
| `client/mod.rs` | 663 | `Provider` trait, client infrastructure |
| `client/builder.rs` | 572 | Client builder pattern |
| `client/completion.rs` | 142 | `CompletionClient` trait |
| `http_client/mod.rs` | 423 | HTTP client extension traits |
| `http_client/sse.rs` | 292 | SSE event stream parsing |
| `http_client/retry.rs` | 120 | Retry logic |
| `http_client/multipart.rs` | 213 | Multipart form support (may not be needed) |

#### Anthropic Provider (~3,684 lines)

| File | Lines | Notes |
|------|------:|-------|
| `providers/anthropic/completion.rs` | 1,509 | Request/response types, completion model |
| `providers/anthropic/streaming.rs` | 1,073 | SSE streaming, partial usage |
| `providers/anthropic/decoders/line.rs` | 385 | Line decoder |
| `providers/anthropic/decoders/sse.rs` | 218 | SSE decoder |
| `providers/anthropic/decoders/jsonl.rs` | 147 | JSONL decoder |
| `providers/anthropic/client.rs` | 200 | Client config, OAuth detection |
| `providers/anthropic/metadata.rs` | 124 | Model metadata |
| `providers/anthropic/mod.rs` | 18 | Module re-exports |
| `providers/anthropic/decoders/mod.rs` | 10 | Decoder re-exports |

#### OpenAI Provider (~4,702 lines)

| File | Lines | Notes |
|------|------:|-------|
| `providers/openai/responses_api/mod.rs` | 1,594 | Responses API types + completion model |
| `providers/openai/completion/mod.rs` | 1,258 | Chat Completions types + model |
| `providers/openai/completion/streaming.rs` | 675 | Completions streaming |
| `providers/openai/client.rs` | 522 | Client config |
| `providers/openai/responses_api/streaming.rs` | 405 | Responses API streaming |
| `providers/openai/mod.rs` | 248 | Module re-exports |

#### Gemini Provider (~2,927 lines)

| File | Lines | Notes |
|------|------:|-------|
| `providers/gemini/completion.rs` | 2,079 | Request/response types, `ThinkingConfig` |
| `providers/gemini/streaming.rs` | 658 | SSE streaming |
| `providers/gemini/client.rs` | 124 | Client config |
| `providers/gemini/mod.rs` | 66 | Module re-exports |

---

## 5. Existing Abstractions We Already Own

We already maintain **~4,300 lines** of provider wrappers that sit on top of rig:

| File | Lines | Purpose |
|------|------:|---------|
| `providers/src/claude.rs` | 855 | Claude provider wrapper |
| `providers/src/codex/mod.rs` | 754 | Codex/Responses API wrapper |
| `providers/src/openai.rs` | 585 | OpenAI Completions wrapper |
| `providers/src/zai.rs` | 416 | Z.AI (OpenAI-compatible) wrapper |
| `providers/src/gemini.rs` | 353 | Gemini wrapper |
| `providers/src/copilot/provider.rs` | 331 | GitHub Copilot wrapper |
| `providers/src/copilot/rig_agent.rs` | 120 | Copilot rig agent adapter |
| `providers/src/caching_client.rs` | 434 | HTTP client with cache token extraction |
| `providers/src/cache_token_extractor.rs` | 140 | Cache token parsing |
| `providers/src/codex/refreshing_client.rs` | 319 | OAuth token refresh |

These wrappers would **merge into** the owned provider code during migration, eliminating the double-abstraction layer.

---

## 6. Migration Impact Estimate

### Codebase Change

| What | Lines |
|------|------:|
| Code to own after migration | ~18,400 (from rig) + ~4,300 (existing wrappers) ≈ **~22,700 total** |
| Code deleted | ~56,042 (patches dir) + merge simplification ≈ **~56,000 gross deletion** |
| **Net codebase change** | **−33,300 lines** |

### Simplifications During Migration

| Simplification | Impact |
|---------------|--------|
| Remove WASM compat (`WasmCompatSend`, `WasmCompatSync`, `WasmBoxedFuture`) | ~153 scattered bounds → plain `Send + Sync` |
| Remove `ToolEmbedding` / RAG support from tool trait | ~29 lines + simpler trait |
| Remove `ToolServer` (we have our own MCP via `mcp.rs`) | −430 lines |
| Remove agent builder complexity (dynamic context, etc.) | Simpler `Agent` construction |
| Remove `CompletionRequestBuilder` builder pattern overhead | Direct struct construction |
| Merge provider wrappers into provider implementations | Eliminate double-abstraction layer |
| Remove unused provider features (embedding, audio, transcription, image gen) | −899 lines from kept providers |

### Expected Benefits

- **Build time**: Removing 16 unused providers + 7 subsystems + ~73 unused transitive deps should significantly reduce compile times
- **Binary size**: Smaller binary without dead provider code
- **Maintenance**: No more patch rebasing against upstream rig releases
- **Simplicity**: Single source of truth instead of "rig types + our wrapper types"
- **Flexibility**: Can evolve message types, streaming, and tool trait without upstream constraints

### Risk Assessment

| Risk | Mitigation |
|------|-----------|
| SSE parsing correctness | Carry rig's battle-tested `http_client/sse.rs` as-is |
| Provider-specific edge cases | Copy provider implementations verbatim, simplify incrementally |
| Tool trait migration (34 files) | If API-compatible, migration is mechanical import path changes |
| Test coverage | Existing tests use rig types — update imports but keep test logic |

---

## 7. Recommended Child Story Sizing

Based on this audit, the RIG-001 child stories (RIG-002 through RIG-010) are correctly scoped:

| Story | Scope | Estimated Effort |
|-------|-------|-----------------|
| **RIG-002** Tool trait | ~570 lines to own + 34 files to migrate imports | Medium |
| **RIG-003** Message types | ~1,085 lines to own + ~50 files to migrate imports | Medium |
| **RIG-004** Agent loop | ~2,538 lines to own (most complex component) | Large |
| **RIG-005** Anthropic provider | ~3,684 lines to own + merge 855-line wrapper | Large |
| **RIG-006** OpenAI provider | ~4,702 lines to own + merge 1,874 lines of wrappers | Large |
| **RIG-007** Gemini provider | ~2,927 lines to own + merge 353-line wrapper | Medium |
| **RIG-008** Tool migration | Import path changes across 34+ files | Medium (mechanical) |
| **RIG-009** codelet-core migration | Hook/agent integration updates | Medium |
| **RIG-010** Remove rig-core | Delete patches/ dir, clean Cargo.toml | Small (victory lap) |
