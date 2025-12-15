# AST Research: Codebase Analysis for CLI-009

**Work Unit**: CLI-009
**Research Date**: 2025-12-03
**Research Type**: Multi-Codebase Analysis (codelet + rig + codelet)
**Purpose**: Understand context compaction architecture for implementation planning

---

## Research Methodology

Performed comprehensive analysis of three codebases:
1. **codelet** (TypeScript) - Reference implementation with working anchor-point compaction
2. **rig** (Rust) - Framework we're building on, analyzed for token tracking capabilities
3. **codelet** (Rust) - Our existing codebase, analyzed for integration points

---

## 1. Codelet Context Compaction Architecture

### Files Analyzed

- `/home/rquast/projects/codelet/src/agent/anchor-point-compaction.ts` (lines 1-580)
- `/home/rquast/projects/codelet/src/agent/runner.ts` (lines 829-949)
- `/home/rquast/projects/codelet/src/agent/token-tracker.ts` (complete file)
- `/home/rquast/projects/codelet/src/agent/llm-summary-provider.ts` (complete file)

### Key Findings

#### A. Conversation Turn Structure

```typescript
// anchor-point-compaction.ts:14-22
interface ConversationTurn {
  userMessage: string;
  toolCalls: ToolCall[];        // All tools invoked in turn
  toolResults: ToolResult[];    // All tool outputs
  assistantResponse: string;
  tokens: number;
  timestamp: Date;
  previousError?: boolean;      // CRITICAL for anchor detection
}
```

**Integration Point**: We need equivalent Rust struct that groups rig messages into turns.

#### B. Anchor Point Detection Algorithm

```typescript
// anchor-point-compaction.ts:116-166
class AnchorPointDetector {
  private readonly CONFIDENCE_THRESHOLD = 0.9;  // 90%+ precision

  detectAnchorPoint(turn: ConversationTurn): AnchorPoint | null {
    const patterns = this.analyzeCompletionPatterns(turn);

    // Priority 1: Error resolution (0.9 weight, 0.95 confidence)
    if (patterns.errorResolution?.confidence >= 0.9) {
      return {
        type: 'error-resolution',
        weight: 0.9,
        confidence: 0.95
      };
    }

    // Priority 2: Task completion (0.8 weight, 0.92 confidence)
    if (patterns.taskCompletion?.confidence >= 0.9) {
      return {
        type: 'task-completion',
        weight: 0.8,
        confidence: 0.92
      };
    }

    return null;  // No high-confidence anchor
  }
}
```

**Pattern Detection Logic**:

```typescript
// anchor-point-compaction.ts:171-221
private analyzeCompletionPatterns(turn: ConversationTurn) {
  const hasTestSuccess = turn.toolResults.some(
    result => result.success &&
    result.output.toLowerCase().includes('test') &&
    (result.output.includes('pass') || result.output.includes('success'))
  );

  const hasFileModification = turn.toolCalls.some(
    call => call.tool === 'Edit' || call.tool === 'Write'
  );

  // Error resolution: Previous error + Fix + Success
  if (turn.previousError && hasFileModification && hasTestSuccess) {
    return {
      errorResolution: {
        confidence: 0.95,
        description: `Build error fixed in ${files} and tests now pass`
      }
    };
  }

  // Task completion: Modify + Test + Success (no previous error)
  if (!turn.previousError && hasFileModification && hasTestSuccess) {
    return {
      taskCompletion: {
        confidence: 0.92,
        description: `File changes implemented in ${files} and tests pass`
      }
    };
  }

  return {};  // No pattern detected
}
```

**Rust Implementation Plan**: Direct translation to pattern matching on `ConversationTurn` struct.

#### C. Turn Selection Algorithm

```typescript
// anchor-point-compaction.ts:321-363
export function selectTurnsForCompaction(
  flow: ConversationFlow,
  targetTokens: number
): TurnSelectionResult {
  const totalTurns = flow.turns.length;

  // ALWAYS preserve last 2-3 complete conversation turns
  const turnsToAlwaysKeep = Math.min(3, totalTurns);
  const recentTurns = flow.turns.slice(-turnsToAlwaysKeep);
  const olderTurns = flow.turns.slice(0, -turnsToAlwaysKeep);

  // Find most recent anchor point in older turns
  const anchorPointInOlderTurns = flow.anchorPoints
    .filter(anchor => anchor.turnIndex < totalTurns - turnsToAlwaysKeep)
    .sort((a, b) => b.turnIndex - a.turnIndex)[0];

  if (anchorPointInOlderTurns) {
    // Keep: anchor + everything after it + recent turns
    const turnsToKeep = [
      ...olderTurns.slice(anchorPointInOlderTurns.turnIndex),
      ...recentTurns
    ];
    const turnsToSummarize = olderTurns.slice(0, anchorPointInOlderTurns.turnIndex);

    return {
      turnsToKeep,
      turnsToSummarize,
      preservedAnchors: [anchorPointInOlderTurns],
      compressionEstimate: turnsToSummarize.length / totalTurns
    };
  }

  // No anchor found - keep recent, summarize the rest
  return {
    turnsToKeep: recentTurns,
    turnsToSummarize: olderTurns,
    preservedAnchors: [],
    compressionEstimate: olderTurns.length / totalTurns
  };
}
```

**Key Insight**: Algorithm is simple (slice-based), no complex graph traversal. Easy to port to Rust.

#### D. Token Tracking with Cache Awareness

```typescript
// token-tracker.ts:13-28
export interface TokenUsage {
  inputTokens: number;                    // From API
  outputTokens: number;                   // From API
  cachedInputTokens: number;              // LEGACY (deprecated)
  reasoningTokens: number;                // OpenAI only
  totalTokens: number;                    // Calculated
  cacheReadInputTokens: number;           // ✅ FROM ANTHROPIC API
  cacheCreationInputTokens: number;       // ✅ FROM ANTHROPIC API
}
```

**Effective Token Calculation**:

```typescript
// runner.ts:124-129
export function calculateEffectiveTokens(tracker: TokenUsage): number {
  const cacheDiscount = tracker.cacheReadInputTokens * 0.9;
  return tracker.inputTokens - cacheDiscount;
}
```

**Compaction Trigger**:

```typescript
// runner.ts:100-107
const COMPACTION_THRESHOLD_PERCENT = 0.9;  // 90%

function shouldTriggerCompaction(tracker: TokenUsage, threshold: number): boolean {
  const effectiveTokens = calculateEffectiveTokens(tracker);
  return effectiveTokens > threshold;
}
```

**Integration Point**: This is WHY we need custom `TokenTracker` (see token-tracking-architecture.md).

#### E. LLM-Based Summarization

```typescript
// llm-summary-provider.ts:29-40
const SUMMARY_PROMPT = `You are helping continue a conversation that has grown too long.

Create a concise continuation summary that captures:
1. The main topic/goal of the conversation
2. Key decisions made
3. Files changed or modified
4. Important context needed to continue
5. Current state/progress
6. Any blockers or issues

Format as a natural continuation message that allows the conversation to resume smoothly.
Keep it under 400 words and ensure the summary is under 1024 tokens.`;
```

**Retry Logic**:

```typescript
// llm-summary-provider.ts:69-104
async generateSummary(messages: MessageForCompaction[]): Promise<string> {
  const MAX_RETRIES = 3;
  const RETRY_DELAYS_MS = [0, 1000, 2000];  // Exponential backoff

  let lastError: Error | undefined;

  for (let attempt = 0; attempt < MAX_RETRIES; attempt++) {
    try {
      if (RETRY_DELAYS_MS[attempt] > 0) {
        await sleep(RETRY_DELAYS_MS[attempt]);
      }

      const model = await this.providerManager.getModel();
      const result = await generateText({
        model,
        prompt: fullPrompt,
        maxOutputTokens: 1024,
      });

      return result.text;
    } catch (error) {
      lastError = error;
      // Continue to next retry
    }
  }

  throw lastError;  // All retries exhausted
}
```

**Integration Point**: Straightforward Rust port using `tokio::time::sleep`.

---

## 2. Rig Framework Analysis

### Files Analyzed

- `/home/rquast/projects/rig/rig-core/src/providers/anthropic/completion.rs` (lines 1-150)
- `/home/rquast/projects/rig/rig-core/src/providers/anthropic/streaming.rs` (lines 1-100)
- `/home/rquast/projects/rig/rig-core/src/completion/request.rs` (lines 1-100)
- `/home/rquast/projects/rig/rig-core/src/agent/prompt_request/streaming.rs` (lines 1-150)

### Key Findings

#### A. Provider-Level Usage (Has Cache Data)

```rust
// rig-core/src/providers/anthropic/completion.rs:90-95
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Usage {
    pub input_tokens: u64,
    pub cache_read_input_tokens: Option<u64>,     // ✅ FROM API
    pub cache_creation_input_tokens: Option<u64>, // ✅ FROM API
    pub output_tokens: u64,
}
```

**Critical Finding**: Rig RECEIVES cache tokens from Anthropic correctly.

#### B. Generic Usage (Loses Cache Data)

```rust
// rig-core/src/completion/request.rs
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Usage {
    pub input_tokens: u64,     // Cache tokens merged here
    pub output_tokens: u64,
    pub total_tokens: u64,
    // ❌ NO cache_read_input_tokens field
    // ❌ NO cache_creation_input_tokens field
}
```

**Critical Finding**: Generic `crate::completion::Usage` LOSES cache granularity.

#### C. Lossy Conversion via GetTokenUsage

```rust
// rig-core/src/providers/anthropic/completion.rs:126-137
impl GetTokenUsage for Usage {
    fn token_usage(&self) -> Option<crate::completion::Usage> {
        let mut usage = crate::completion::Usage::new();

        // ⚠️ PROBLEM: Cache tokens MERGED into input_tokens!
        usage.input_tokens = self.input_tokens
            + self.cache_creation_input_tokens.unwrap_or_default()
            + self.cache_read_input_tokens.unwrap_or_default();  // LOST!

        usage.output_tokens = self.output_tokens;
        usage.total_tokens = usage.input_tokens + usage.output_tokens;

        Some(usage)
    }
}
```

**Critical Finding**: This is the DATA LOSS point. We must bypass this conversion.

#### D. Streaming Event Processing

```rust
// rig-core/src/providers/anthropic/streaming.rs:241-247
StreamingEvent::MessageStart { message } => {
    input_tokens = message.usage.input_tokens;

    let span = tracing::Span::current();
    span.record("gen_ai.response.id", &message.id);
    span.record("gen_ai.response.model_name", &message.model);
},
```

**Critical Finding**: `message.usage` is `anthropic::completion::Usage` (has cache fields), but rig only captures `input_tokens`.

**Integration Point**: We need to intercept at this level to extract cache tokens BEFORE they're lost.

#### E. MultiTurnStreamItem Structure

```rust
// rig-core/src/agent/prompt_request/streaming.rs:35-42
pub enum MultiTurnStreamItem<R> {
    StreamAssistantItem(StreamedAssistantContent<R>),
    StreamUserItem(StreamedUserContent),
    FinalResponse(FinalResponse),
}

pub struct FinalResponse {
    response: String,
    aggregated_usage: crate::completion::Usage,  // ⚠️ Generic type (no cache fields)
}
```

**Critical Finding**: By the time we reach `FinalResponse`, cache data is ALREADY LOST.

**Solution**: Must track tokens during streaming, not from FinalResponse.

---

## 3. Codelet Integration Points

### Files Analyzed

- `/home/rquast/projects/codelet/src/session/mod.rs` (complete file)
- `/home/rquast/projects/codelet/src/agent/rig_agent.rs` (complete file)
- `/home/rquast/projects/codelet/src/cli/interactive.rs` (lines 180-450)

### Key Findings

#### A. Session Structure (Owns Messages)

```rust
// src/session/mod.rs:14-32
#[derive(Debug)]
pub struct Session {
    provider_manager: ProviderManager,

    /// Message history - single source of truth for conversation context
    /// Uses rig::message::Message directly for rig integration (CLI-008)
    pub messages: Vec<rig::message::Message>,

    messages_before_interruption: Option<Vec<rig::message::Message>>,
}
```

**Integration Point**: Add `turns: Vec<ConversationTurn>` and `token_tracker: TokenTracker` to Session.

#### B. RigAgent Streaming

```rust
// src/agent/rig_agent.rs:88-103
pub async fn prompt_streaming_with_history(
    &self,
    prompt: &str,
    history: &mut [rig::message::Message],
) -> impl futures::Stream<
    Item = Result<rig::agent::MultiTurnStreamItem<M::StreamingResponse>, anyhow::Error>,
> + '_ {
    // Clone history for rig (rig takes ownership and manages it internally)
    let history_for_rig = history.to_vec();

    self.agent
        .stream_prompt(prompt)
        .with_history(history_for_rig)
        .multi_turn(self.max_depth)
        .await
        .map(|result| result.map_err(|e| anyhow::anyhow!("Streaming error: {}", e)))
}
```

**Integration Point**: Add version that extracts provider-specific usage during streaming.

#### C. Message Synchronization in Interactive

```rust
// src/cli/interactive.rs:195
messages.push(Message::User {
    content: OneOrMany::one(UserContent::text(prompt)),
});

// src/cli/interactive.rs:237
assistant_text.push_str(&text.text);  // Accumulate text

// src/cli/interactive.rs:287-288
tool_calls_buffer.push(AssistantContent::ToolCall(tool_call.clone()));

// src/cli/interactive.rs:338-340
messages.push(Message::User {
    content: OneOrMany::one(UserContent::ToolResult(tool_result_clone)),
});
```

**Integration Point**: Same locations where we synchronize messages, we also:
1. Group into conversation turns
2. Extract token usage from stream
3. Detect anchor points
4. Trigger compaction when threshold exceeded

---

## 4. Implementation Architecture

### Module Structure

Based on analysis, we need these new modules:

```
src/
├── agent/
│   ├── token_tracker.rs          # Custom TokenTracker with cache fields
│   ├── conversation_turn.rs      # ConversationTurn struct + grouping logic
│   ├── anchor_detection.rs       # AnchorPointDetector + pattern matching
│   ├── turn_selection.rs         # selectTurnsForCompaction algorithm
│   ├── llm_summarizer.rs         # LLMSummaryProvider with retry logic
│   └── compaction.rs             # Main compaction coordinator
├── session/mod.rs                # MODIFIED: Add turns + token_tracker fields
└── cli/interactive.rs            # MODIFIED: Group turns, trigger compaction
```

### Data Flow

```
1. User sends prompt
   ↓
2. RigAgent streams response (MultiTurnStreamItem)
   ↓
3. TokenTracker extracts cache tokens from stream events
   ↓
4. Messages accumulated in session.messages (for rig)
5. ConversationTurn built from messages (for anchor detection)
   ↓
6. Check if compaction threshold exceeded (effective tokens > 90%)
   ↓
   YES → Trigger Compaction
   ↓
7. AnchorPointDetector scans all turns
8. selectTurnsForCompaction chooses what to keep/summarize
9. LLMSummaryProvider generates summary with retries
10. Messages reconstructed: system + kept turns + summary + continuation
11. Prompt cache cleared
12. Session continues with compacted context
```

---

## 5. Critical Implementation Details

### A. Token Extraction Strategy

**Problem**: Rig's `FinalResponse.usage()` doesn't have cache fields.

**Solution**: Intercept streaming BEFORE conversion to generic type.

**Approach 1** (Recommended): Modify `RigAgent::prompt_streaming_with_history` to return both stream items AND provider-specific usage:

```rust
pub async fn prompt_streaming_with_cache_tracking(
    &self,
    prompt: &str,
    history: &mut [rig::message::Message],
) -> (
    impl Stream<Item = Result<MultiTurnStreamItem<M::StreamingResponse>, anyhow::Error>>,
    Arc<Mutex<Option<anthropic::completion::Usage>>>,  // Provider-specific usage
) {
    // Implementation intercepts MessageStart event
}
```

**Approach 2** (Simpler): Access rig's internal streaming events directly:

```rust
// In interactive.rs, when processing StreamAssistantItem:
match chunk {
    Some(Ok(MultiTurnStreamItem::StreamAssistantItem(content))) => {
        // TODO: Find way to access underlying provider response
        // This may require unsafe or reflection
    }
}
```

**Decision**: Start with Approach 2 (simpler), fall back to Approach 1 if needed.

### B. Turn Grouping Logic

**When to complete a turn**:
- User message received
- Assistant text accumulated
- Tool calls executed
- Tool results returned
- Assistant final response received

**Implementation**:

```rust
struct TurnBuilder {
    user_message: Option<String>,
    tool_calls: Vec<ToolCall>,
    tool_results: Vec<ToolResult>,
    assistant_response: String,
}

impl TurnBuilder {
    fn is_complete(&self) -> bool {
        self.user_message.is_some() && !self.assistant_response.is_empty()
    }

    fn build(self) -> ConversationTurn {
        ConversationTurn {
            user_message: self.user_message.unwrap(),
            tool_calls: self.tool_calls,
            tool_results: self.tool_results,
            assistant_response: self.assistant_response,
            tokens: 0,  // Calculate from token tracker
            timestamp: Utc::now(),
            previous_error: false,  // Detected from previous turn
        }
    }
}
```

### C. Compaction Trigger Integration

**Where to check**: After each agent response completes (in `interactive.rs`).

```rust
// src/cli/interactive.rs (after FinalResponse)
Some(Ok(MultiTurnStreamItem::FinalResponse(response))) => {
    // Update token tracker
    session.update_tokens(&provider_specific_usage);

    // Check compaction threshold
    if session.should_compact() {
        println!("\r\n[Generating summary...]");

        // Execute compaction
        let result = compact_conversation(
            &session.turns,
            &session.messages,
            &session.provider_manager,
        ).await?;

        // Reconstruct messages
        session.messages = result.compacted_messages;
        session.turns = result.compacted_turns;

        // Display metrics
        println!("\r\n[Context compacted]");
        println!("  Original: {} tokens", result.original_tokens);
        println!("  Compacted: {} tokens ({:.0}% reduction)",
            result.compacted_tokens, result.compression_ratio * 100.0);
    }

    break;
}
```

---

## 6. Testing Strategy

### Unit Tests

Based on codelet's test coverage:

- `test_anchor_detection_error_resolution` - Pattern matching for error → fix → success
- `test_anchor_detection_task_completion` - Pattern matching for modify → test → success
- `test_turn_selection_with_anchor` - Slice algorithm keeps anchor-to-present
- `test_turn_selection_no_anchor` - Fallback keeps last 3 turns
- `test_effective_token_calculation` - Cache discount math
- `test_compaction_trigger_threshold` - 90% threshold with cache awareness
- `test_llm_summary_retry_logic` - Exponential backoff (0ms, 1000ms, 2000ms)
- `test_compression_ratio_validation` - Warn if < 60%

### Integration Tests

- Full compaction cycle with real Anthropic API
- Token tracking accuracy vs API response
- Message reconstruction correctness
- Prompt cache invalidation after compaction

---

## 7. Risk Assessment

### High Risk

1. **Token Extraction Complexity**: Accessing provider-specific usage from rig may require unsafe code or forking rig.
   - **Mitigation**: Start with estimation fallback if direct access impossible.

2. **Turn Grouping Accuracy**: Incorrect turn boundaries could break anchor detection.
   - **Mitigation**: Extensive unit tests with known turn patterns.

### Medium Risk

1. **LLM Summary Quality**: Generated summaries may lose critical context.
   - **Mitigation**: Follow codelet's prompt template exactly, test with real sessions.

2. **Compression Ratio Variability**: Actual compression may vary from 60-80% target.
   - **Mitigation**: Implement quality gate (warn if < 60%), suggest fresh conversation.

### Low Risk

1. **Prompt Cache Invalidation**: Clearing cache after compaction is straightforward.
2. **Message Reconstruction**: Direct port of codelet's logic (well-tested).

---

## 8. Open Questions

1. **Q**: Can we access rig's internal streaming events without forking?
   **A**: TBD - requires experimentation with rig's API surface.

2. **Q**: Should we support multi-provider compaction (OpenAI, Gemini)?
   **A**: Phase 1 = Anthropic only. Phase 2 = Add OpenAI if needed.

3. **Q**: What happens if summary generation fails after 3 retries?
   **A**: Follow codelet: Use generic session continuation message, continue without summary.

---

## 9. Implementation Checklist

From codelet's implementation:

**Phase 1: Core Data Structures** ✅
- [x] Define ConversationTurn struct
- [x] Define AnchorPoint struct
- [x] Define TokenTracker struct
- [x] Define CompactionConfig struct

**Phase 2: Anchor Detection** ⏳
- [ ] Implement AnchorPointDetector
- [ ] Add analyzeCompletionPatterns method
- [ ] Set CONFIDENCE_THRESHOLD = 0.9
- [ ] Implement error resolution pattern (0.95 confidence)
- [ ] Implement task completion pattern (0.92 confidence)

**Phase 3: Turn Selection** ⏳
- [ ] Implement selectTurnsForCompaction function
- [ ] Always preserve last 2-3 turns
- [ ] Find most recent anchor in older turns
- [ ] Calculate compression estimate

**Phase 4: LLM Summarization** ⏳
- [ ] Implement LLMSummaryProvider
- [ ] Define SUMMARY_PROMPT template
- [ ] Implement retry logic (3 attempts)
- [ ] Add exponential backoff (0ms, 1000ms, 2000ms)

**Phase 5: Integration** ⏳
- [ ] Add TokenTracker to Session
- [ ] Add turns: Vec<ConversationTurn> to Session
- [ ] Implement turn grouping in interactive.rs
- [ ] Add compaction trigger logic
- [ ] Reconstruct messages after compaction
- [ ] Clear prompt cache

**Phase 6: Quality & Testing** ⏳
- [ ] Add compression ratio validation (>= 60%)
- [ ] Emit warnings on failure
- [ ] Test with long conversations
- [ ] Verify anchor detection accuracy
- [ ] Validate summary quality

---

## 10. References

All file paths and line numbers documented above for traceability.

**Key Files**:
- Codelet: `anchor-point-compaction.ts`, `runner.ts`, `token-tracker.ts`, `llm-summary-provider.ts`
- Rig: `providers/anthropic/completion.rs`, `providers/anthropic/streaming.rs`, `completion/request.rs`
- Codelet: `session/mod.rs`, `agent/rig_agent.rs`, `cli/interactive.rs`

**Attachments**:
- token-tracking-architecture.md - Detailed analysis of why custom tracking is required
- codelet-context-compaction-anchoring.md - High-level overview from user's transcript
