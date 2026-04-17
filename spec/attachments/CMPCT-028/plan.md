# CMPCT-028 — Correct Post-Compaction Retry Prompt Semantics

**Parent:** CMPCT-022
**Bug:** BUG 7

## The Problem

`codelet/cli/src/interactive/compaction_retry.rs:128-133`:

```rust
let mut retry_stream = agent
    .prompt_streaming_with_history_and_hook(
        "Continue",                   // ← hardcoded
        &mut session.messages,
        retry_hook,
    )
    .await;
```

### When does `"Continue"` make sense?

When compaction fires BEFORE the API call (`on_completion_call` hook site 412 in rig), the user's prompt was never sent to the LLM. `execute_compaction(session, flag, Some(prompt))` embeds the original prompt into the compaction instruction that the LLM reads:

```rust
// From interactive_helpers.rs:311-317
let instruction = match last_user_message {
    Some(prompt) => format!(
        "{base_instruction}\n\nAfter building the DAG and calling inject_summary, resume working on:\n{prompt}"
    ),
    None => base_instruction,
};
```

So "Continue" works because the LLM is expected to read the embedded prompt out of the instruction and resume the task. This relies on:
1. The LLM actually parsing and honoring the embedded prompt.
2. The compaction flow producing exactly one User message (the instruction) followed by any system reminders.
3. No loss of fidelity when the prompt is re-embedded as a quoted string in a larger instruction block.

### What if compaction fires AFTER tokens were emitted?

When the hook cancels at rig sites 460, 486, 509, 542, or 586 (mid-turn), the user's prompt HAS already been sent and partially processed. "Continue" now has TWO plausible meanings:
- "Continue the task from where you left off" — the LLM's preferred interpretation if partial text was preserved.
- "Resume the compaction instruction" — if the DAG embedding is the primary signal.

This ambiguity is silent. No test proves which interpretation the LLM picks.

### Pre-prompt path acknowledges this

`stream_loop.rs:449-455`:
```rust
// After compaction, the original prompt is embedded in the compaction instruction.
// Use a synthetic prompt so rig doesn't duplicate it.
let effective_prompt = if compaction_just_ran {
    "Continue"
} else {
    prompt
};
```

The post-cancel path (`compaction_retry.rs:128-133`) has no such comment and unconditionally uses "Continue". There is no `compaction_just_ran` tracking across the post-loop boundary.

## The Fix

Make the retry prompt policy explicit, tied to whether the user prompt was delivered to the LLM or not:

```rust
enum CompactionRecoveryPolicy {
    /// User prompt was embedded in compaction instruction; LLM reads it from there.
    /// Sends `"Continue"` as the rig prompt so rig doesn't double-count.
    EmbedInInstruction { resume_text: &'static str },
    
    /// User prompt was partially processed; partial Assistant message is already saved.
    /// Sends a specific resume prompt that references the preserved partial work.
    ResumeWithPreservedPartial,
    
    /// User prompt was never sent; re-send it explicitly after compaction.
    /// (Alternative if we don't trust embedding.)
    Resend(String),
}
```

Then `handle_compaction_retry` selects a policy:

```rust
let policy = if partial_assistant_text_saved {
    CompactionRecoveryPolicy::ResumeWithPreservedPartial
} else {
    CompactionRecoveryPolicy::EmbedInInstruction { resume_text: "Continue" }
};

let retry_prompt = match &policy {
    CompactionRecoveryPolicy::EmbedInInstruction { resume_text } => *resume_text,
    CompactionRecoveryPolicy::ResumeWithPreservedPartial => "Please continue from where you left off.",
    CompactionRecoveryPolicy::Resend(p) => p.as_str(),
};
```

## Why This Matters for the User

Today, a user could send a complex prompt, hit compaction mid-generation, and get a response that IGNORES their original prompt because:
- The partial Assistant text was lost (BUG 2).
- The LLM sees only "Continue" — no referent to continue FROM.
- The embedded prompt in the compaction instruction is buried under DAG-building boilerplate.

## Acceptance Criteria

1. When compaction fires BEFORE any API call → retry sends "Continue" (current behavior preserved).
2. When compaction fires AFTER partial text was emitted (BUG 2 fixed by CMPCT-024) → retry uses a resume prompt that references the preserved work ("Please continue from where you left off.").
3. When compaction fires AFTER a tool result was consumed → retry resumes from the tool result context.
4. The choice of retry prompt is explicit, documented, and tested — no ambiguity.

## Files to Modify

- `codelet/cli/src/interactive/compaction_retry.rs` (lines 128-133)
- Plumbing in `stream_loop.rs` to pass partial-state info into the retry handler

## Testing

- Integration test: mock stream yielding partial text + PromptCancelled → after compaction, verify the retry prompt is the resume prompt, not "Continue".
- Integration test: mock stream yielding PromptCancelled at site 412 (before any tokens) → verify the retry prompt is still "Continue".
- Observability: add a debug log or capture event that records which policy was selected.

## Relationship to CMPCT-023

This card depends on CMPCT-023's unified entry-point helper because the partial-state info needs to flow from the primary loop's error handler into the retry handler. If CMPCT-023 introduces a `CompactionRecoveryContext` struct, this card populates its `partial_assistant_text: Option<String>` field.
