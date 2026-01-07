# Gemini CLI vs fspec/codelet Agentic Loop Comparison

## Executive Summary

This document provides a comprehensive analysis of the architectural differences between the fspec/codelet agentic loop (Rust, Claude-optimized) and the Gemini CLI agentic loop (TypeScript, Gemini-optimized). It identifies key differences in how each handles tool execution, turn completion, context management, and model-specific behaviors—particularly for Gemini 3 Pro Preview.

---

## 1. Architectural Overview

### fspec/codelet (Rust) - Claude-Optimized

```
┌────────────────────────────────────────────────────────────┐
│                  run_agent_stream_internal()               │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  agent.prompt_streaming_with_history_and_hook()      │  │
│  │    └─ rig's multi_turn() handles internally:         │  │
│  │         • Tool execution                              │  │
│  │         • History management                          │  │
│  │         • Turn chaining                               │  │
│  └──────────────────────────────────────────────────────┘  │
│                          │                                  │
│                    Stream Items:                            │
│         StreamAssistantItem::Text ──────► emit_text()       │
│         StreamAssistantItem::ToolCall ──► handle_tool()     │
│         StreamUserItem::ToolResult ─────► handle_result()   │
│         Usage ──────────────────────────► track_tokens()    │
│         FinalResponse ──────────────────► done!             │
└────────────────────────────────────────────────────────────┘
```

**Key characteristics:**
- **Single event loop** - One `loop {}` processes all stream items
- **Rig library handles tool dispatch** - Tools execute *inside* the stream
- **Unlimited depth** (`DEFAULT_MAX_DEPTH = usize::MAX - 1`)
- **CompactionHook** - Pre-emptive context compaction before API calls

### Gemini CLI (TypeScript) - Gemini-Optimized

```
┌────────────────────────────────────────────────────────────┐
│                      GeminiClient                           │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  sendMessageStream()                                  │  │
│  │    ├─ fireBeforeAgentHook()                          │  │
│  │    ├─ processTurn() ─────────────────────────────────│──│─┐
│  │    ├─ checkNextSpeaker() ← GEMINI-SPECIFIC           │  │ │
│  │    └─ fireAfterAgentHook()                           │  │ │
│  └──────────────────────────────────────────────────────┘  │ │
└────────────────────────────────────────────────────────────┘ │
                                                               │
┌────────────────────────────────────────────────────────────┐ │
│                       Turn                                  │◄┘
│  ┌──────────────────────────────────────────────────────┐  │
│  │  run() → yields ServerGeminiStreamEvent              │  │
│  │    ├─ Content                                         │  │
│  │    ├─ Thought ← GEMINI 3 SPECIFIC                    │  │
│  │    ├─ ToolCallRequest → CoreToolScheduler            │  │
│  │    └─ Finished (with finishReason)                   │  │
│  └──────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌────────────────────────────────────────────────────────────┐
│                  CoreToolScheduler                          │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  schedule() → _processNextInQueue()                  │  │
│  │    ├─ validating → awaiting_approval → scheduled     │  │
│  │    ├─ Interactive confirmation UI                    │  │
│  │    └─ Sequential execution with state machine        │  │
│  └──────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────┘
```

---

## 2. Critical Differences

### 2.1 Tool Execution Model

**fspec/codelet (Claude):**
```rust
// Tools execute INSIDE rig's stream - transparent to outer loop
Some(Ok(MultiTurnStreamItem::StreamAssistantItem(
    StreamedAssistantContent::ToolCall(tool_call),
))) => {
    handle_tool_call(&tool_call, ...)?;  // Just tracking
}
Some(Ok(MultiTurnStreamItem::StreamUserItem(
    StreamedUserContent::ToolResult(tool_result),
))) => {
    handle_tool_result(&tool_result, ...)?;  // Just tracking
}
// Rig internally: calls tool → gets result → sends to API → continues
```

**Gemini CLI:**
```typescript
// Tools are EXTRACTED and SCHEDULED separately
for await (const streamEvent of responseStream) {
    const functionCalls = resp.functionCalls ?? [];
    for (const fnCall of functionCalls) {
        yield { type: GeminiEventType.ToolCallRequest, value: toolCallRequest };
    }
}
// External scheduler then:
// 1. Shows confirmation UI
// 2. Executes tools
// 3. Sends results back in next turn
```

**Why this matters:** Claude's tool use protocol returns tool results inline, while Gemini's returns `functionCall` objects that need explicit handling before sending `functionResponse` parts back.

### 2.2 Turn Completion Detection

**fspec/codelet:**
```rust
Some(Ok(MultiTurnStreamItem::FinalResponse(final_resp))) => {
    // Turn is definitively complete
    output.emit_done();
    break;
}
```

**Gemini CLI:**
```typescript
// After turn completes, CHECK if model should continue
if (!turn.pendingToolCalls.length && signal && !signal.aborted) {
    const nextSpeakerCheck = await checkNextSpeaker(
        this.getChat(),
        this.config.getBaseLlmClient(),
        signal,
        prompt_id,
    );
    if (nextSpeakerCheck?.next_speaker === 'model') {
        // Auto-continue the conversation
        turn = yield* this.sendMessageStream([{ text: 'Please continue.' }], ...);
    }
}
```

**Why this matters:** Gemini models may stop mid-explanation (especially Gemini 3 with thinking). The `checkNextSpeaker()` heuristic uses a separate LLM call to determine if the model should continue. Claude doesn't need this - it naturally completes its turns.

### 2.3 Thought/Reasoning Handling

**Gemini 3 Pro Preview - CRITICAL:**
```typescript
// turn.ts - Extract thoughts from response
const thoughtPart = resp.candidates?.[0]?.content?.parts?.[0];
if (thoughtPart?.thought) {
    const thought = parseThought(thoughtPart.text ?? '');
    yield { type: GeminiEventType.Thought, value: thought, traceId };
    continue;  // Don't treat as regular content
}

// geminiChat.ts - MUST add synthetic thought signatures
ensureActiveLoopHasThoughtSignatures(requestContents: Content[]): Content[] {
    // For Gemini 3 Preview models, the first function call in each model
    // turn must have a thoughtSignature or the API returns 400
    if (!part.thoughtSignature) {
        newParts[j] = { ...part, thoughtSignature: SYNTHETIC_THOUGHT_SIGNATURE };
    }
}
```

**Claude:**
```rust
// No special thought handling needed
// Extended thinking is via API header, not in response format
Some(Ok(MultiTurnStreamItem::StreamAssistantItem(
    StreamedAssistantContent::Text(text),
))) => {
    handle_text_chunk(&text.text, ...)?;  // All text is content
}
```

### 2.4 System Prompt Variations

**Gemini 3 vs Gemini 2 (prompts.ts):**
```typescript
const isGemini3 = isPreviewModel(desiredModel);

// Gemini 3 REQUIRES explanation before tools
const mandatesVariant = isGemini3
    ? `- **Do not call tools in silence:** You must provide to the user 
        very short and concise natural explanation (one sentence) before calling tools.`
    : ``;

// Gemini 3 is more conversational - don't suppress chitchat
if (isGemini3) {
    return '';  // No "No Chitchat" rule
} else {
    return `- **No Chitchat:** Avoid conversational filler, preambles...`;
}
```

**fspec/codelet:** Uses a static system prompt without model-specific variations.

---

## 3. Context Management Comparison

### fspec/codelet - Proactive Hook-Based Compaction

```rust
// CompactionHook checks BEFORE each API call
async fn on_completion_call(&self, _prompt: &Message, _history: &[Message], cancel_sig: CancelSignal) {
    let total = state.total();  // input + cache_read + cache_creation + output
    if total > self.threshold {
        state.compaction_needed = true;
        cancel_sig.cancel();  // Stop current operation, trigger compaction
    }
}

// stream_loop.rs - Pre-prompt compaction (CTX-005)
let estimated_total = current_tokens + prompt_tokens;
if estimated_total > threshold && !session.messages.is_empty() {
    execute_compaction(session).await?;  // Compact BEFORE starting
}
```

### Gemini CLI - Service-Based Compression

```typescript
// ChatCompressionService with XML state_snapshot
async compress(chat: GeminiChat, prompt_id: string, force: boolean, ...): Promise<...> {
    // Uses specialized compression prompt
    // Output: <state_snapshot> XML with structured summary
}

// processTurn checks before processing
const compressed = await this.tryCompressChat(prompt_id, false);
if (compressed.compressionStatus === CompressionStatus.COMPRESSED) {
    yield { type: GeminiEventType.ChatCompressed, value: compressed };
}
```

---

## 4. How Gemini 3 Pro Preview Differs from Claude

| Aspect | Gemini 3 Pro Preview | Claude |
|--------|---------------------|--------|
| **Thought Output** | Explicit `thought` parts with `thoughtSignature` | Extended thinking via API headers (not visible in stream) |
| **Tool Protocol** | Returns `functionCall`, expects `functionResponse` parts | Native tool_use blocks, tool_result inline |
| **Turn Completion** | May need "next speaker" check | Natural turn boundaries |
| **Prompt Requirements** | Must explain before tools | No special requirements |
| **API Quirks** | 400 errors without thoughtSignature on preview models | Consistent behavior |
| **Retry Behavior** | InvalidStreamError retry with backoff | Standard error handling |
| **Model Routing** | Dynamic routing per query | Single model per session |

---

## 5. Recommendations for fspec/codelet to Support Gemini

### 5.1 Provider Facade Pattern

Following the existing facade pattern used for tool calls and prompts, implement a `ProviderBehavior` facade that abstracts provider-specific behaviors:

```rust
/// Provider-specific behavior facade
/// Abstracts differences between Claude, Gemini, OpenAI, etc.
pub trait ProviderBehavior: Send + Sync {
    /// Extract thought/reasoning content from stream items (Gemini 3 specific)
    fn extract_thought(&self, item: &StreamItem) -> Option<ThoughtInfo>;
    
    /// Whether this provider needs "next speaker" checks after turns
    fn needs_speaker_check(&self) -> bool;
    
    /// Transform tool response to provider-specific format
    fn format_tool_response(&self, response: &ToolResponse) -> ProviderToolResponse;
    
    /// Get model-specific system prompt additions
    fn get_prompt_additions(&self, model: &str) -> Option<String>;
    
    /// Prepare message history before sending to API (e.g., add thought signatures)
    fn prepare_history(&self, history: &mut Vec<Message>, model: &str);
    
    /// Whether to strip thinking content from history
    fn strip_thinking_from_history(&self) -> bool;
}
```

### 5.2 Claude Implementation (Default)

```rust
pub struct ClaudeBehavior;

impl ProviderBehavior for ClaudeBehavior {
    fn extract_thought(&self, _item: &StreamItem) -> Option<ThoughtInfo> {
        None  // Claude's extended thinking is via API headers, not stream
    }
    
    fn needs_speaker_check(&self) -> bool {
        false  // Claude completes turns naturally
    }
    
    fn format_tool_response(&self, response: &ToolResponse) -> ProviderToolResponse {
        // Rig handles this internally for Claude
        ProviderToolResponse::Native(response.clone())
    }
    
    fn get_prompt_additions(&self, _model: &str) -> Option<String> {
        None  // No model-specific additions needed
    }
    
    fn prepare_history(&self, _history: &mut Vec<Message>, _model: &str) {
        // No preparation needed
    }
    
    fn strip_thinking_from_history(&self) -> bool {
        false
    }
}
```

### 5.3 Gemini Implementation

```rust
pub struct GeminiBehavior;

const SYNTHETIC_THOUGHT_SIGNATURE: &str = "skip_thought_signature_validator";

impl ProviderBehavior for GeminiBehavior {
    fn extract_thought(&self, item: &StreamItem) -> Option<ThoughtInfo> {
        // Extract from thought parts if present
        if let Some(thought_part) = item.get_thought_part() {
            Some(parse_thought(&thought_part.text))
        } else {
            None
        }
    }
    
    fn needs_speaker_check(&self) -> bool {
        true  // Gemini may stop mid-explanation
    }
    
    fn format_tool_response(&self, response: &ToolResponse) -> ProviderToolResponse {
        // Gemini expects functionResponse format
        ProviderToolResponse::Gemini {
            function_response: FunctionResponse {
                id: response.call_id.clone(),
                name: response.tool_name.clone(),
                response: response.result.clone(),
            }
        }
    }
    
    fn get_prompt_additions(&self, model: &str) -> Option<String> {
        if is_gemini3_preview(model) {
            Some(
                "- **Do not call tools in silence:** You must provide a very short \
                 and concise natural explanation (one sentence) before calling tools."
                    .to_string()
            )
        } else {
            None
        }
    }
    
    fn prepare_history(&self, history: &mut Vec<Message>, model: &str) {
        if is_gemini3_preview(model) {
            ensure_thought_signatures(history);
        }
    }
    
    fn strip_thinking_from_history(&self) -> bool {
        true  // Strip thoughtSignature before compression
    }
}

fn is_gemini3_preview(model: &str) -> bool {
    model.contains("gemini-2.5") || model.contains("preview")
}

fn ensure_thought_signatures(history: &mut Vec<Message>) {
    // Find start of active loop (last user text message)
    let active_loop_start = history.iter().rposition(|m| {
        matches!(m, Message::User { content } if content.has_text())
    });
    
    if let Some(start_idx) = active_loop_start {
        for msg in history[start_idx..].iter_mut() {
            if let Message::Assistant { content } = msg {
                // Add synthetic signature to first function call in each turn
                for part in content.parts_mut() {
                    if part.is_function_call() && part.thought_signature.is_none() {
                        part.thought_signature = Some(SYNTHETIC_THOUGHT_SIGNATURE.to_string());
                        break;
                    }
                }
            }
        }
    }
}
```

### 5.4 Integration with Stream Loop

```rust
// In stream_loop.rs
pub async fn run_agent_stream_internal<M, O, E>(
    agent: RigAgent<M>,
    prompt: &str,
    session: &mut Session,
    provider_behavior: &dyn ProviderBehavior,  // NEW: Provider facade
    // ... other params
) -> Result<()> {
    // Prepare history before sending
    provider_behavior.prepare_history(&mut session.messages, session.current_model());
    
    // Add provider-specific prompt additions
    let prompt_with_additions = if let Some(additions) = provider_behavior.get_prompt_additions(session.current_model()) {
        format!("{}\n\n{}", prompt, additions)
    } else {
        prompt.to_string()
    };
    
    // ... existing stream loop ...
    
    // Handle thought extraction
    if let Some(thought) = provider_behavior.extract_thought(&stream_item) {
        output.emit_thought(&thought);
    }
    
    // After turn completes, check if model should continue
    if provider_behavior.needs_speaker_check() && !has_pending_tools {
        if should_continue_turn(session).await? {
            // Send "Please continue" prompt
        }
    }
}
```

### 5.5 Next Speaker Check Implementation

```rust
/// Check if the model should continue (Gemini-specific)
async fn should_continue_turn(session: &Session) -> Result<bool> {
    // Use a lightweight LLM call to check if response is complete
    let check_prompt = "Based on the conversation, should the assistant continue? Answer 'model' or 'user'.";
    
    // This would use a separate, fast model call
    let response = session.quick_completion(check_prompt).await?;
    
    Ok(response.trim().to_lowercase() == "model")
}
```

---

## 6. Summary of Agentic Loop Philosophies

### fspec/codelet Philosophy (Claude-centric):
> "Let the rig library handle the complexity. Trust the model to complete turns. Focus on token tracking and context management."

- **Delegation-heavy**: Rig handles tool execution, history, multi-turn
- **Single responsibility**: Stream loop only tracks tokens and emits events  
- **Proactive**: Pre-emptive compaction before problems occur

### Gemini CLI Philosophy (Gemini-centric):
> "Be defensive about model behavior. Schedule and confirm tools. Check if model is done. Handle quirks explicitly."

- **Control-heavy**: External tool scheduler with state machine
- **Multi-layered**: Client → Turn → Chat → Scheduler each has responsibilities
- **Reactive**: Next speaker checks, retry logic, thought extraction

---

## 7. Key Takeaways for Implementation

1. **Claude is "fire and forget"** - Set up the agent, let rig handle multi-turn, process what comes back

2. **Gemini needs babysitting** - Check speaker, handle thoughts, add signatures, schedule tools

3. **Token tracking is universal** - Both codebases carefully track context to avoid overflow

4. **Tool confirmation UI is Gemini-specific** - Claude's tool use is more integrated; Gemini's requires explicit UI flow

5. **Gemini 3 Preview is transitional** - The thoughtSignature requirement and prompt variations suggest it's still being refined

---

## 8. Files to Reference

### Gemini CLI (cloned to /tmp/gemini-cli)
- `/tmp/gemini-cli/packages/core/src/agents/local-executor.ts` - Sub-agent execution loop
- `/tmp/gemini-cli/packages/core/src/core/client.ts` - Main GeminiClient
- `/tmp/gemini-cli/packages/core/src/core/geminiChat.ts` - Chat session with thought signatures
- `/tmp/gemini-cli/packages/core/src/core/turn.ts` - Turn management and event types
- `/tmp/gemini-cli/packages/core/src/core/coreToolScheduler.ts` - Tool scheduling state machine
- `/tmp/gemini-cli/packages/core/src/core/prompts.ts` - System prompt with Gemini 3 variations

### fspec/codelet
- `/Users/rquast/projects/fspec/codelet/cli/src/interactive/stream_loop.rs` - Main streaming loop
- `/Users/rquast/projects/fspec/codelet/core/src/rig_agent.rs` - Rig agent wrapper
- `/Users/rquast/projects/fspec/codelet/core/src/compaction_hook.rs` - Context compaction hook
