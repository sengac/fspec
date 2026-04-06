# Z.AI GLM Provider Performance Gap Analysis

**Date:** 2026-04-06
**Source:** Comparison of `zai-org/GLM-5` reference repository vs fspec's ZAI integration
**Story:** PROV-052

## Executive Summary

Analysis of the official GLM-5 reference repository against fspec's ZAI provider implementation reveals 8 concrete gaps that affect GLM tool-calling accuracy, reasoning quality, and deployment flexibility. The highest-impact issues are inconsistent tool naming (9 of 16 tools violate GLM's snake_case preference) and the completely unused thinking_config parameter.

---

## Gap #1: Tool Naming Inconsistency (HIGH IMPACT)

**Problem:** 9 of 16 tools registered with the ZAI agent use PascalCase or mixed naming, violating GLM's documented preference for snake_case.

fspec created ZAI-specific facades for 7 core tools with proper snake_case names. But the remaining 9 tools are registered with their generic names:

| Generic Tool | Registered Name | GLM-Preferred Name |
|---|---|---|
| AstGrep | `AstGrep` | `ast_grep` |
| AstGrepRefactor | `AstGrepRefactor` | `ast_grep_refactor` |
| WebSearch | `WebSearch` | `web_search` |
| ConnectMcp | `ConnectMcp` | `connect_mcp` |
| SessionSearch | `SessionSearch` | `session_search` |
| GraphSearch | `GraphSearch` | `graph_search` |
| DeepSearch | `DeepSearch` | `deep_search` |
| AgentManager | `AgentManager` | `agent_manager` |
| Schedule | `Schedule` | `schedule` |

Tools already correctly named: `inject_summary`, `request_user_input`, `run_fspec`, `manage_bridge`

**Why it matters:** GLM models were trained with snake_case tool names. When the model sees a mix of `read_file` and `AstGrep` and `WebSearch`, the inconsistent naming convention confuses tool selection heuristics. The non-facaded tools also likely have complex nested JSON schemas that GLM handles worse than flat ones.

**Evidence from codebase:**
- `codelet/tools/src/facade/zai.rs` lines 6-10: *"GLM models work best with: snake_case tool names, Flat JSON schemas with explicit required and default values, additionalProperties: false"*
- `codelet/providers/src/zai.rs` lines 183-184: *"Uses Z.AI/GLM-specific tool facades for optimal tool calling behavior. GLM models work best with snake_case tool names and flat JSON schemas."*

**Fix options:**
1. **Full facades** (best): Create ZAI facade structs for all 9 remaining tools with snake_case names and flat schemas
2. **Naming adapter** (lighter): Create a lightweight wrapper that renames tools to snake_case and simplifies their schemas without full facade logic

---

## Gap #2: thinking_config Completely Ignored (HIGH IMPACT)

**Problem:** The `create_rig_agent()` method accepts a `thinking_config` parameter but discards it entirely (prefixed with `_`).

```rust
// codelet/providers/src/zai.rs line 191
pub fn create_rig_agent(
    &self,
    session_id: uuid::Uuid,
    preamble: Option<&str>,
    _thinking_config: Option<serde_json::Value>,  // UNUSED
)
```

The comment says: *"GLM models handle reasoning internally and don't use the same thinking config format as Claude or Gemini."*

**Why it matters:** The Z.AI API actually supports `thinking: {type: "enabled"}` in request bodies. Self-hosted GLM uses `--reasoning-parser glm45` explicitly. By not forwarding any thinking config:
- Users cannot control whether reasoning is on/off
- Users cannot set a reasoning budget if the API supports it
- There's no way to disable reasoning for simple tasks where it adds latency

**Provider parity comparison:**

| Provider | thinking_config handling |
|----------|------------------------|
| Claude | Fully consumed — merged into `additional_params` as `thinking: {type, budget_tokens}` |
| Gemini | Fully consumed — injected as `generationConfig.thinkingConfig` with defaults for Gemini 3 |
| OpenAI | Ignored (but OpenAI o-series handles reasoning implicitly) |
| **ZAI** | **Ignored — no thinking config forwarded** |

**Streaming already works:** The patched rig-core correctly parses `reasoning_content` from SSE deltas and yields `ReasoningDelta` events. The gap is only on the **request** side — we never tell the API to enable thinking.

**Fix:** Forward `thinking_config` to `additional_params` for reasoning-capable models. Default to `thinking: {type: "enabled"}` when `supports_reasoning()` returns true and no explicit config is provided.

---

## Gap #3: supports_reasoning() Won't Recognize GLM-5 (MEDIUM IMPACT)

**Problem:** The reasoning detection method only checks for `glm-4.*` patterns:

```rust
// codelet/providers/src/zai.rs lines 170-179
pub fn supports_reasoning(&self) -> bool {
    let model = &self.model_name;
    if model.contains("glm-4.6v") || model.contains("glm-4.5v") {
        return false;
    }
    model.contains("glm-4-plus")
        || model.contains("glm-4.7")
        || model.contains("glm-4.6")
        || model.contains("glm-4.5")
}
```

If a user passes `glm-5` or `glm-5-fp8` as the model name, `supports_reasoning()` returns `false`. This silently disables any reasoning-related logic.

**Fix options:**
1. Add `model.contains("glm-5")` to the check
2. **Better:** Invert the logic — default to `true` and only blacklist known non-reasoning models (vision variants). Future models would then work correctly by default.

---

## Gap #4: Hardcoded max_output_tokens With No Override (MEDIUM IMPACT)

**Problem:** `MAX_OUTPUT_TOKENS` is a compile-time constant at 8,192 with no runtime override:

```rust
// codelet/providers/src/zai.rs line 33
pub const MAX_OUTPUT_TOKENS: usize = 8192;
```

Used in:
- `LlmProvider::max_output_tokens()` (line 329)
- `create_rig_agent()` via `.max_tokens()` (line 233)
- `complete_with_tools()` via `.max_tokens()` (line 360)
- DeepSearch sub-agent config (8192)

**OpenAI has this solved:** `OPENAI_MAX_OUTPUT_TOKENS` env var at runtime (manager.rs line 552). ZAI has no equivalent.

**Why it matters:** GLM-4.7 and GLM-5 may support higher output limits. Self-hosted GLM-5 via vLLM has no inherent output cap beyond `--max-model-len`. Users running self-hosted GLM through the ZAI provider (via custom base URL) are artificially capped at 8K tokens.

**Fix:** Add `ZAI_MAX_OUTPUT_TOKENS` env var support mirroring OpenAI's pattern.

---

## Gap #5: No tool_choice Configuration (MEDIUM IMPACT)

**Problem:** The ZAI provider never sends `tool_choice` in API requests. It relies entirely on the API's default behavior.

**Evidence from GLM-5 repo:** The vLLM deployment config explicitly requires `--enable-auto-tool-choice` as a flag. On the API side, GLM supports the standard OpenAI `tool_choice` parameter: `"auto"`, `"required"`, `"none"`, or `{type: "function", function: {name: "..."}}`.

**Why it matters:** Without explicit `tool_choice: "auto"`, behavior may vary between:
- Z.AI hosted API (likely defaults to auto)
- Self-hosted vLLM (requires the flag)
- Self-hosted SGLang (implicit when parser is set)

**Fix:** Explicitly send `tool_choice: "auto"` in the request when tools are provided, or make it configurable via env var.

---

## Gap #6: Hardcoded context_window With No Override (MEDIUM IMPACT)

**Problem:** `CONTEXT_WINDOW` is hardcoded at 128,000 for all models:

```rust
// codelet/providers/src/zai.rs line 30
pub const CONTEXT_WINDOW: usize = 128_000;
```

The comment says "GLM-4.7" but it's applied to every ZAI model. No env var override exists.

**From the GLM-5 reference repo:** Self-hosted GLM-5 deployment configs show varying `--max-model-len` values: 66600 (Ascend quantized), 8192 (multi-node BF16), up to 280000 (`--max-prefill-tokens` in SGLang).

**Fix:** Add `ZAI_CONTEXT_WINDOW` env var support.

---

## Gap #7: No ZAI Web Search Facade (LOW-MEDIUM IMPACT)

**Problem:** Claude has `ClaudeWebSearchFacade`, Gemini has `GeminiGoogleWebSearchFacade` + `GeminiWebFetchFacade` + `GeminiWebScreenshotFacade`. ZAI uses the raw generic `WebSearchTool` with PascalCase name and complex nested schema.

**Provider web search comparison:**

| Provider | Implementation | Facade | Tool Name |
|----------|---------------|--------|-----------|
| Claude | `ClaudeWebSearchFacade` | Provider-specific | `WebSearch` (complex oneOf schema) |
| Gemini | `GeminiGoogleWebSearchFacade` | Provider-specific | `google_web_search` (simple query) |
| ZAI | Generic `WebSearchTool` | **None** | `WebSearch` (generic schema) |
| OpenAI | Generic `WebSearchTool` | None | `WebSearch` (generic schema) |

Even though Z.AI's API doesn't have a native web search capability, the **tool schema presentation** to the model still matters. A `ZAIWebSearchFacade` that renames it to `web_search` with a flattened schema would align with the snake_case convention.

---

## Gap #8: Hardcoded Generation Config (LOW IMPACT)

**Problem:** Temperature and top_p are hardcoded:

```rust
// codelet/providers/src/zai.rs lines 273-276
let generation_config = serde_json::json!({
    "temperature": 1.0,
    "top_p": 0.95
});
```

These values were derived from "opencode research" for GLM-4.6/4.7. GLM-5, a 744B MoE model, may have different optimal parameters.

**Fix:** Make configurable via `ZAI_TEMPERATURE` and `ZAI_TOP_P` env vars, or at least document the current values.

---

## Priority Matrix

| # | Gap | Impact | Effort | Priority |
|---|-----|--------|--------|----------|
| 1 | Tool naming inconsistency (9 non-snake_case) | 🔴 High | Medium | **P1** |
| 2 | thinking_config ignored | 🔴 High | Low | **P1** |
| 3 | supports_reasoning() misses GLM-5 | 🟡 Medium | Trivial | **P2** |
| 4 | No ZAI_MAX_OUTPUT_TOKENS env var | 🟡 Medium | Low | **P2** |
| 5 | No tool_choice parameter | 🟡 Medium | Low | **P2** |
| 6 | No ZAI_CONTEXT_WINDOW env var | 🟡 Medium | Low | **P2** |
| 7 | No ZAI web search facade | 🟠 Low-Med | Medium | **P3** |
| 8 | Hardcoded generation config | 🟢 Low | Low | **P3** |

---

## Relevant Source Files

| File | Role |
|------|------|
| `codelet/providers/src/zai.rs` | Core ZAIProvider — LlmProvider trait impl (~411 lines) |
| `codelet/tools/src/facade/zai.rs` | 7 GLM-optimized tool facades (~772 lines) |
| `codelet/tools/src/facade/fspec_facade.rs` | ZAIFspecFacade for fspec commands (~80 lines) |
| `codelet/tools/src/facade/bridge_facade.rs` | ZAIBridgeFacade for WebSocket bridge (~30 lines) |
| `codelet/providers/src/manager.rs` | ProviderType::ZAI + get_zai() factory |
| `codelet/providers/src/credentials.rs` | has_zai() credential detection |
| `codelet/napi/src/deep_search_provider_config.rs` | DeepSearch ZAI config |
| `src/utils/provider-config.ts` | TypeScript provider registry entry |

## Reference Repository Files

| File | Content |
|------|---------|
| `/tmp/GLM-5/README.md` | English docs — model card, deployment, benchmarks |
| `/tmp/GLM-5/README_zh.md` | Chinese docs |
| `/tmp/GLM-5/example/ascend.md` | Ascend NPU deployment (~940 lines) |
| `/tmp/GLM-5/skills/glm-master-skill/SKILL.md` | Skills catalog |
| `/tmp/GLM-5/requirements.txt` | `transformers>=5.3.0`, `accelerate>=1.13.0` |

---

## Streaming & Reasoning Pipeline (Already Working)

For completeness, here's what IS already working correctly:

**Streaming path:** The patched rig-core at `patches/rig-core/src/providers/openai/completion/streaming.rs` correctly handles GLM's `reasoning_content` field in SSE deltas:

```rust
struct StreamingDelta {
    content: Option<String>,
    reasoning_content: Option<String>,  // Z.AI GLM reasoning
    tool_calls: Vec<StreamingToolCall>,
}
```

When `reasoning_content` is present, it's yielded as `ReasoningDelta` before text content.

**Token accounting:** `completion_tokens_details.reasoning_tokens` is tracked correctly.

**Non-streaming path:** `convert_assistant_content()` handles `AssistantContent::Reasoning` (converts to `ContentPart::Text`).

**Thinking exhaustion recovery:** If GLM produces reasoning tokens but empty output, fspec retries with accumulated reasoning context and automatically downgrades thinking level.

The gap is exclusively on the **request side** — we never configure thinking in outbound requests.

## Methodology

- Cloned `https://github.com/zai-org/GLM-5` (the official Zhipu AI reference repo for GLM-5)
- Analyzed all documentation: README.md, README_zh.md, example/ascend.md, skills/glm-master-skill/SKILL.md
- Compared deployment configurations (vLLM, SGLang, xLLM flags and parameters)
- Deep-searched fspec's ZAI provider code across `codelet/providers/src/zai.rs`, `codelet/tools/src/facade/zai.rs`, and all related files
- Cross-referenced with Claude, Gemini, and OpenAI provider implementations for parity analysis

---

## GLM-5 Reference Repository Overview

The `zai-org/GLM-5` repo is **documentation-only** (zero executable code). It is the official hub for:

| Content | Details |
|---------|---------|
| Model card | 744B total params, 40B active (sparse MoE), 28.5T tokens pre-training |
| Download links | HuggingFace, ModelScope (BF16, FP8 variants) |
| Deployment guides | vLLM, SGLang, xLLM/Ascend NPU |
| Benchmarks | CC-Bench-V2, Vending Bench 2 |
| Skills catalog | GLM-OCR, GLM-Image, GLM-V |
| API endpoints | `https://docs.z.ai/guides/llm/glm-5`, `https://chat.z.ai` |
| License | Apache 2.0 |

### Key Deployment Configuration (from README.md)

```shell
# vLLM deployment
vllm serve zai-org/GLM-5-FP8 \
     --tool-call-parser glm47 \
     --reasoning-parser glm45 \
     --enable-auto-tool-choice \
     --speculative-config.method mtp \
     --speculative-config.num_speculative_tokens 1

# SGLang deployment
sglang serve \
  --model-path zai-org/GLM-5-FP8 \
  --tool-call-parser glm47  \
  --reasoning-parser glm45 \
  --speculative-algorithm EAGLE
```

The two parser flags are critical signals:
- **`glm47`** = tool-call format introduced with GLM-4.7, reused by GLM-5
- **`glm45`** = reasoning/thinking format introduced with GLM-4.5, reused by GLM-5
- **`--enable-auto-tool-choice`** = required for automatic tool call detection in vLLM

---
