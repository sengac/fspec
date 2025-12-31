# Prompt Caching Analysis for Multi-Turn Conversations

## Executive Summary

Analysis of debug session `session-2025-12-31T04-33-54.jsonl` reveals that **only the system prompt is being cached** in multi-turn conversations. Conversation history (user messages 2+, all assistant responses) is sent uncached with each API request, leading to unnecessary token costs.

## Current Implementation

### What IS Being Cached

1. **System prompt** (4849 tokens) - correctly has `cache_control: {type: "ephemeral"}` applied
2. **First user message** - has `cache_control` applied via `transform_user_message_cache_control()`

### What is NOT Being Cached

1. Subsequent user messages (2nd, 3rd, etc.)
2. All assistant responses
3. Tool use results

## Evidence from Debug Log

```
First API call:
  cacheCreationInputTokens: 4849  (system prompt cached)
  cacheReadInputTokens: 0         (no cache to read yet)
  inputTokens: 470                (user message + overhead)

Second API call:
  cacheCreationInputTokens: 0     (no new cache created)
  cacheReadInputTokens: 4849      (system prompt read from cache)
  inputTokens: 1201               (conversation history sent UNCACHED)
```

The **1201 input tokens** on the second call represents the conversation history being sent as uncached content, even though it was already sent in the first call.

## Root Cause Analysis

### Code Location

File: `codelet/providers/src/caching_client.rs:95-116`

```rust
pub fn transform_user_message_cache_control(body: &mut Value) {
    if let Some(messages) = body.get_mut("messages").and_then(|m| m.as_array_mut()) {
        for msg in messages.iter_mut() {
            // Only transform first user message
            if msg.get("role").and_then(|r| r.as_str()) == Some("user") {
                if let Some(content) = msg.get("content") {
                    if content.is_string() {
                        let text = content.as_str().unwrap_or_default();
                        msg["content"] = json!([
                            {
                                "type": "text",
                                "text": text,
                                "cache_control": { "type": "ephemeral" }
                            }
                        ]);
                    }
                }
                // Only transform the first user message
                break;  // <-- PROBLEM: stops after first user message
            }
        }
    }
}
```

### The Problem

The `break` statement on line 113 causes the function to stop after transforming only the **first** user message. As the conversation grows, all subsequent messages are sent without `cache_control`, meaning they cannot be cached by Anthropic's API.

## Anthropic Prompt Caching Behavior

According to [Anthropic's documentation](https://docs.anthropic.com/en/docs/build-with-claude/prompt-caching):

1. Content is cached as a **prefix** from the beginning of the request
2. You mark **breakpoints** with `cache_control: {type: "ephemeral"}`
3. Everything **before and including** the breakpoint is cached

### Optimal Strategy for Multi-Turn Conversations

For optimal caching, add `cache_control` to the **last message before the current turn**:

```
Turn 1:
  System (cache_control) → User1 (cache_control)
  └── 5319 tokens cached ──────────────────────┘

Turn 2:
  System (cached) → User1 (cached) → Assistant1 → User2 (cache_control)
  └────── from cache ─────────────┘              └── NEW breakpoint ──┘

Turn 3:
  System → User1 → Assistant1 → User2 (cache_control) → Assistant2 → User3
  └───────────────── all cached ─────────────────────┘
```

## Proposed Fix

Modify `transform_user_message_cache_control` to add `cache_control` to the **second-to-last user message** instead of the first. This ensures the entire conversation prefix (system + all previous turns) gets cached.

### Pseudocode

```rust
pub fn transform_user_message_cache_control(body: &mut Value) {
    if let Some(messages) = body.get_mut("messages").and_then(|m| m.as_array_mut()) {
        // Find all user message indices
        let user_indices: Vec<usize> = messages.iter()
            .enumerate()
            .filter(|(_, m)| m.get("role").and_then(|r| r.as_str()) == Some("user"))
            .map(|(i, _)| i)
            .collect();

        // Add cache_control to second-to-last user message (if exists)
        // This caches the entire prefix up to the previous turn
        if user_indices.len() >= 2 {
            let target_idx = user_indices[user_indices.len() - 2];
            // Apply cache_control to messages[target_idx]
        } else if user_indices.len() == 1 {
            // First turn: cache the first user message
            let target_idx = user_indices[0];
            // Apply cache_control to messages[target_idx]
        }
    }
}
```

## Expected Benefits

1. **Cost Reduction**: ~75% reduction in billed input tokens for multi-turn conversations
2. **Latency Improvement**: Cached content is processed faster by Anthropic
3. **Consistent Caching**: All previous conversation history is cached, not just system prompt

## Test Cases to Add

1. Single turn conversation - cache system + first user message
2. Two turn conversation - cache system + first user + first assistant
3. Multi-turn conversation - cache growing prefix each turn
4. Tool use conversation - ensure tool results are in cached prefix
5. Verify cache_control is on correct message (second-to-last user)

## Files to Modify

1. `codelet/providers/src/caching_client.rs` - Fix `transform_user_message_cache_control`
2. `codelet/providers/tests/caching_http_middleware_test.rs` - Add multi-turn test cases

## References

- Debug session: `~/.fspec/debug/session-2025-12-31T04-33-54.jsonl`
- Session data: `~/.fspec/sessions/67058a91-cbc7-4311-bbd2-bb0a6dd4f8fb.json`
- Anthropic caching docs: https://docs.anthropic.com/en/docs/build-with-claude/prompt-caching
