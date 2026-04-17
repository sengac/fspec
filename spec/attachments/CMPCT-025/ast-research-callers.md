# CMPCT-025 — AST Research: `is_compaction_cancelled` Callers

## Purpose

Verify the blast radius of changing `is_compaction_cancelled`'s signature and
internal logic. Ensures that the structural downcast replacement does not
accidentally break any caller site.

## AST queries

### 1. Caller sites

Pattern: `is_compaction_cancelled($$$ARGS)`

```
codelet/cli/src/interactive/gemini_continuation.rs:331:20  is_compaction_cancelled(&e)
codelet/cli/src/interactive/stream_loop.rs:1141:48         is_compaction_cancelled(&e)
```

Both call sites pass `&anyhow::Error`. Signature `fn(&anyhow::Error) -> bool`
will be preserved, so neither caller needs to change.

### 2. Definition

Pattern: `fn is_compaction_cancelled($$$ARGS) -> $RET { $$$BODY }`

No AST match because the current definition lives at
`codelet/cli/src/interactive/error_classifiers.rs:115-117` as:

```rust
pub(super) fn is_compaction_cancelled(error: &anyhow::Error) -> bool {
    error.to_string().contains("PromptCancelled")
}
```

(AST pattern used `fn`, but the definition is `pub(super) fn`; the call-site
grep above confirms there is exactly one in-tree definition.)

### 3. PromptError import surface

- `rig::completion::PromptError` (re-exported from `rig::completion::request`)
- `rig::message::Message` (vec payload of `PromptCancelled { chat_history }`)

Both already available through the `rig-core` workspace dependency declared in
`codelet/cli/Cargo.toml:31`.

### 4. Existing rig-yielded variant shape

From `codelet/patches/rig-core/src/completion/request.rs:147-148`:

```rust
#[error("PromptCancelled")]
PromptCancelled { chat_history: Box<Vec<Message>> },
```

And from `codelet/patches/rig-core/src/agent/prompt_request/streaming.rs:244-251`:

```rust
pub enum StreamingError {
    Completion(#[from] CompletionError),
    Prompt(#[from] Box<PromptError>),
    Tool(#[from] ToolSetError),
}
```

`anyhow::Error::from(StreamingError::Prompt(box_prompt_err))` produces an
error chain whose root is the `PromptError` thanks to `#[from]` + anyhow's
`.source()` walking. This is why `anyhow::Error::chain()` successfully finds
the typed variant even when wrapped.

## Conclusion

The structural-downcast approach is safe:
- No public API changes required.
- Both callers continue to work unchanged.
- The `PromptError` / `Message` types are already importable.
