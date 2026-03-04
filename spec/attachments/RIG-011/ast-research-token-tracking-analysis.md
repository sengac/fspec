# AST Research: Token Tracking and Debug Metadata Analysis

## AstGrep Queries Performed

### 1. Usage struct location
**Pattern:** `pub struct Usage { $$$FIELDS }`
**Path:** `codelet/patches/rig-core/src/completion/`
**Result:** `request.rs:295:1` — Missing `reasoning_tokens` field.

### 2. ApiTokenUsage struct location
**Pattern:** `pub struct ApiTokenUsage { $$$FIELDS }`
**Path:** `codelet/core/src/token_usage.rs`
**Result:** `token_usage.rs:20:1` — Missing `reasoning_tokens` field.

### 3. OutputTokensDetails struct (has reasoning_tokens at provider level)
**Pattern:** `pub struct OutputTokensDetails { $$$FIELDS }`
**Path:** `codelet/patches/rig-core/src/providers/openai/`
**Result:** `responses_api/mod.rs:537:1` — Contains `pub reasoning_tokens: u64` ✅

### 4. ResponsesUsage struct (has output_tokens_details)
**Pattern:** `pub struct ResponsesUsage { $$$FIELDS }`
**Path:** `codelet/patches/rig-core/src/providers/openai/responses_api/mod.rs`
**Result:** `mod.rs:461:1` — Contains `output_tokens_details: OutputTokensDetails` ✅

### 5. Streaming token_usage() — drops reasoning_tokens
**Pattern:** `fn token_usage(&self) -> Option<crate::completion::Usage> { $$$BODY }`
**Path:** `codelet/patches/rig-core/src/providers/openai/responses_api/`
**Result:** `streaming.rs:43:5` — Sets input/output/total but NOT reasoning_tokens.

### 6. update_from_usage — drops reasoning_tokens
**Pattern:** `pub fn update_from_usage(&mut self, $$$ARGS) { $$$BODY }`
**Path:** `codelet/core/src/token_usage.rs`
**Result:** `token_usage.rs:64:5` — Extracts input/output/cache but NOT reasoning_tokens.

### 7. total_context() — doesn't include reasoning_tokens
**Pattern:** `pub fn total_context(&self) -> u64 { $$$BODY }`
**Path:** `codelet/core/src/token_usage.rs`
**Result:** `token_usage.rs:59:5` — Returns `total_input() + output_tokens`, no reasoning.

## Summary of Findings

The `reasoning_tokens` data EXISTS at the provider level (`OutputTokensDetails.reasoning_tokens`)
but is DROPPED during conversion to `completion::Usage` (which lacks the field), and therefore
never reaches `ApiTokenUsage` or debug capture events.

Debug metadata model field uses `current_provider_name()` instead of `current_model_id()` in
repl_loop.rs:46, stream_loop.rs:382, and napi/session_manager.rs:7162,7200 (confirmed via Grep).
