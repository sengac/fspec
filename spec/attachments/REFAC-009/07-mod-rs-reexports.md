# Updated `mod.rs` — Re-export Map

**Path**: `codelet/cli/src/interactive/mod.rs`  
**Responsibility**: After decomposition, all public items must remain accessible at the same import path.

---

## Current Re-exports (from `stream_loop`)

```rust
pub use stream_loop::{
    build_user_content_with_images, is_image_content_error, is_prompt_too_long_error,
    is_truncated_tool_call_error, build_truncation_recovery_message,
    build_truncation_budget_exhausted_message, MAX_TRUNCATION_RETRIES,
    is_thinking_exhaustion, build_thinking_exhaustion_recovery_message,
    build_thinking_budget_exhausted_message, downgrade_thinking_level,
    MAX_THINKING_EXHAUSTION_RETRIES, THINKING_EXHAUSTION_OUTPUT_THRESHOLD,
    THINKING_EXHAUSTION_CROSS_TURN_THRESHOLD,
    run_agent_stream, run_agent_stream_with_images, sanitize_image_content, BridgeImage,
};
```

---

## After Decomposition

```rust
// New submodules
mod error_classifiers;
mod recovery_truncation;
mod recovery_thinking;
mod recovery_image;
mod multimodal;
// Existing (unchanged)
mod agent_runner;
mod message_helpers;
pub mod output;
mod repl_loop;
mod stream_handlers;
pub mod stream_loop;  // Slimmed — orchestration only

// Re-exports: error classifiers
pub use error_classifiers::{
    is_prompt_too_long_error,
    is_image_content_error,
    is_truncated_tool_call_error,
};

// Re-exports: truncation recovery (PROV-040)
pub use recovery_truncation::{
    MAX_TRUNCATION_RETRIES,
    build_truncation_recovery_message,
    build_truncation_budget_exhausted_message,
};

// Re-exports: thinking recovery (PROV-041)
pub use recovery_thinking::{
    MAX_THINKING_EXHAUSTION_RETRIES,
    THINKING_EXHAUSTION_OUTPUT_THRESHOLD,
    THINKING_EXHAUSTION_CROSS_TURN_THRESHOLD,
    is_thinking_exhaustion,
    build_thinking_exhaustion_recovery_message,
    build_thinking_budget_exhausted_message,
    downgrade_thinking_level,
};

// Re-exports: image recovery (EXT-016)
pub use recovery_image::sanitize_image_content;

// Re-exports: multimodal content (BRIDGE-007)
pub use multimodal::{BridgeImage, build_user_content_with_images};

// Re-exports: stream loop entry points
pub use stream_loop::{run_agent_stream, run_agent_stream_with_images};

// Re-exports: existing (unchanged)
pub use output::{
    CliOutput, ContextFillInfo, StreamEvent, StreamOutput, TokenInfo, ToolCallEvent,
    ToolResultEvent,
};
```

---

## Verification

All 5 test files import via `codelet_cli::interactive::*` — as long as `mod.rs` re-exports the same set of symbols, zero test changes are needed.

Run after migration:
```bash
cargo test --package codelet-cli
```
