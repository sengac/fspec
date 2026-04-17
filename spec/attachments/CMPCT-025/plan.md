# CMPCT-025 — Replace Stringly-Typed `is_compaction_cancelled` with Structural Downcast

**Parent:** CMPCT-022
**Bug:** BUG 3
**Related:** PROV-045 (broader scope; this is a focused subset)

## The Problem

`codelet/cli/src/interactive/error_classifiers.rs:115-117`:

```rust
pub(super) fn is_compaction_cancelled(error: &anyhow::Error) -> bool {
    error.to_string().contains("PromptCancelled")
}
```

### Why this is fragile

1. **`to_string()` only renders the top-level error via `Display`.** `anyhow::Error::to_string()` on a wrapped error shows the outermost context, not the root cause. If rig or any middleware layer ever adds `.context("Streaming error")`, the substring match silently fails.

2. **No `.source()` chain traversal.** Compare with `stream_loop.rs:1422-1427` which DOES walk the chain for logging — classifier helpers should have the same rigor.

3. **String matching on a typed enum variant.** `PromptError::PromptCancelled` is a typed enum variant from `codelet/patches/rig-core/src/completion/request.rs:147-148`:
   ```rust
   #[error("PromptCancelled")]
   PromptCancelled { chat_history: Box<Vec<Message>> },
   ```
   Today it renders as the literal `"PromptCancelled"`, but the error derive could change, the variant could be renamed, or the error could be wrapped — all of which break the substring match.

4. **`StreamingError::Prompt(Box<PromptError>)` wrapping.** Rig yields:
   ```rust
   yield Err(StreamingError::Prompt(PromptError::prompt_cancelled(...).into()));
   ```
   which is then converted via `anyhow::Error::from`. The `Display` of `StreamingError::Prompt` may or may not delegate to the inner `PromptError`.

## The Fix

Replace the substring match with a structural downcast that traverses the error chain:

```rust
use rig::completion::PromptError;

/// CMPCT-025: Check if an error is (or wraps) a PromptError::PromptCancelled.
/// 
/// Uses structural downcasting with source() chain traversal, robust against:
/// - `.context(...)` wrapping
/// - `StreamingError::Prompt` wrapping
/// - Multiple layers of error composition
/// 
/// Optionally returns the chat_history payload for callers that want to recover it.
pub(super) fn is_compaction_cancelled(error: &anyhow::Error) -> bool {
    extract_prompt_cancelled(error).is_some()
}

/// Extract the chat_history from a PromptCancelled error if one exists in the chain.
pub(super) fn extract_prompt_cancelled(
    error: &anyhow::Error,
) -> Option<&Vec<rig::message::Message>> {
    // Walk the anyhow chain
    for err in error.chain() {
        if let Some(PromptError::PromptCancelled { chat_history }) = 
            err.downcast_ref::<PromptError>() 
        {
            return Some(chat_history);
        }
    }
    None
}
```

This gives us TWO wins:
1. Robust detection (BUG 3 fixed).
2. Access to the `chat_history` payload (which today is dropped unread — related to CMPCT-029).

## Acceptance Criteria

1. `is_compaction_cancelled` correctly returns `true` for:
   - Direct `anyhow::Error::new(PromptError::PromptCancelled { ... })`
   - `anyhow::Error::new(PromptError::PromptCancelled).context("upstream")`
   - `anyhow::Error::new(StreamingError::Prompt(Box::new(PromptError::PromptCancelled)))` (current rig yield path)
   - Nested `.context()` wrapping two layers deep

2. `is_compaction_cancelled` returns `false` for:
   - `anyhow::Error::msg("PromptCancelled")` (bare string, not a typed error — we only want REAL cancellations)
   - `anyhow::Error::msg("Some other error")`
   - Real `PromptError` variants that are NOT `PromptCancelled`

3. New helper `extract_prompt_cancelled` returns the `chat_history` for later use.

4. No change to public API — `is_compaction_cancelled` signature is preserved.

## Relationship to PROV-045

PROV-045 proposes a full `StreamErrorKind` enum with classifier returning the kind:

```rust
pub enum StreamErrorKind {
    PromptTooLong,
    TruncatedToolCall,
    ImageContent,
    CompactionCancelled { chat_history: Vec<Message> },
    StallTimeout,
    TransientNetwork,
    Terminal(anyhow::Error),
}

pub fn classify_stream_error(e: &anyhow::Error) -> StreamErrorKind { ... }
```

CMPCT-025 is the **first step** — land the structural downcast for the compaction case, prove it works, then PROV-045 can generalize. Do NOT try to do both in this card.

## Files to Modify

- `codelet/cli/src/interactive/error_classifiers.rs` (lines 113-117)
- Possibly add `rig` crate re-export in `codelet_cli::interactive` if not already present

## Testing

New tests in the `#[cfg(test)]` module of `error_classifiers.rs`:

```rust
#[test]
fn structurally_detects_prompt_cancelled() {
    let err: anyhow::Error = PromptError::PromptCancelled {
        chat_history: Box::new(vec![]),
    }.into();
    assert!(is_compaction_cancelled(&err));
}

#[test]
fn detects_wrapped_prompt_cancelled() {
    let inner: anyhow::Error = PromptError::PromptCancelled {
        chat_history: Box::new(vec![]),
    }.into();
    let wrapped = inner.context("upstream streaming error");
    assert!(is_compaction_cancelled(&wrapped));
}

#[test]
fn detects_streaming_error_wrapped_prompt_cancelled() {
    // Mirror the exact type rig yields
    let err: anyhow::Error = StreamingError::Prompt(
        Box::new(PromptError::PromptCancelled {
            chat_history: Box::new(vec![]),
        })
    ).into();
    assert!(is_compaction_cancelled(&err));
}

#[test]
fn does_not_match_bare_string() {
    let err = anyhow::Error::msg("PromptCancelled");
    // Bare string errors are NOT typed PromptCancelled — reject
    assert!(!is_compaction_cancelled(&err));
}

#[test]
fn does_not_match_other_prompt_errors() {
    let err: anyhow::Error = PromptError::MaxDepthError { /* ... */ }.into();
    assert!(!is_compaction_cancelled(&err));
}
```

## Cross-Compatibility Check

Grep for callers of `is_compaction_cancelled`:
```
codelet/cli/src/interactive/stream_loop.rs:1141
codelet/cli/src/interactive/gemini_continuation.rs:331
```

Both call it with `&anyhow::Error` — signature is preserved, no caller changes needed.
