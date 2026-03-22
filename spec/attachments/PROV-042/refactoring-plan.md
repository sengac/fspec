# PROV-042: Grand Refactoring Plan — Stream Loop Decomposition

## The Problem

`stream_loop.rs` is a **2,331-line file** with a **1,721-line god function** (`run_agent_stream_internal`). It has:

- **17 levels of nesting** at worst case
- **3 copy-pasted stream-processing loops** (main loop, Gemini continuation, post-compaction retry)
- **12 distinct responsibilities** crammed into one function
- **11 instances** of identical debug-capture boilerplate (7 lines each)
- **10 parameters** on the god function signature (4 `#[allow(clippy::too_many_arguments)]` suppressions)
- **15+ mutable local variables** tracking state that should be in a struct
- **80+ ticket-ID comments** acting as a geological record of accretion

The `Session` struct is equally problematic: **6 of 7 fields are `pub`**, with 72+ external field accesses to `session.messages` and 45+ to `session.token_tracker`. It's a naked data bag, not an encapsulated domain object.

## Refactoring Principles

Every PROV-043 through PROV-050 card is simultaneously a **feature card** and a **refactoring vehicle**. No card adds code INTO `stream_loop.rs` — every card **extracts code OUT** while adding its feature in the new, clean module.

### The Rule

> If you're touching `stream_loop.rs`, you must leave it **shorter** than you found it.

## Target Architecture

### Before (current)

```
codelet/cli/src/interactive/
├── mod.rs                     (76 lines)
├── stream_loop.rs             (2,331 lines) ← EVERYTHING lives here
├── stream_handlers.rs         (437 lines)
├── output.rs                  (509 lines)
├── repl_loop.rs               (201 lines)
├── agent_runner.rs            (68 lines)
└── message_helpers.rs         (31 lines)
```

### After (target)

```
codelet/cli/src/interactive/
├── mod.rs                     (~80 lines)  — entry points, re-exports
├── stream_loop.rs             (~400 lines) — orchestration ONLY
├── stream_processor.rs        (~250 lines) — THE reusable stream-processing loop
├── stream_context.rs          (~80 lines)  — StreamContext struct (replaces 15 local vars)
├── stream_errors.rs           (~150 lines) — StreamErrorKind enum + classify + recovery messages
├── stream_retry.rs            (~120 lines) — RetryOrchestrator + backoff policy
├── circuit_breaker.rs         (~80 lines)  — ApiHealthTracker (circuit breaker + failure tracking)
├── loop_detection.rs          (~50 lines)  — response repetition detector
├── image_handling.rs          (~130 lines) — BridgeImage, sanitize, build_user_content
├── debug_capture.rs           (~30 lines)  — capture_event() helper (eliminates 11x boilerplate)
├── history_persistence.rs     (~80 lines)  — pre-compaction history saver
├── stream_handlers.rs         (~400 lines) — existing (slightly reduced)
├── output.rs                  (~509 lines) — existing (unchanged)
├── repl_loop.rs               (~201 lines) — existing (unchanged)
├── agent_runner.rs            (~68 lines)  — existing (unchanged)
└── message_helpers.rs         (~31 lines)  — existing (unchanged)

codelet/cli/src/session/
├── mod.rs                     (~180 lines) — Session with encapsulated sub-components
├── thinking_state.rs          (~60 lines)  — ThinkingState (extracted data clump)
├── conversation_state.rs      (~100 lines) — encapsulated message/turn management
└── ...existing files...

codelet/core/src/
├── retry_policy.rs            (~60 lines)  — StreamRetryPolicy (shared with providers)
├── compaction/
│   ├── split_safety.rs        (~100 lines) — find_safe_split_point, validate_tool_pairing
│   └── ...existing...
└── ...existing...
```

**Total: ~2,331 lines → distributed across 12 focused modules, largest ~400 lines**

## The StreamProcessor: Heart of the Refactoring

The single biggest win is extracting a **reusable `StreamProcessor`** that unifies the 3 copy-pasted stream-processing loops.

### Current: 3 Duplicated Loops

| Loop | Lines | Location | Purpose |
|------|-------|----------|---------|
| Main | 919–2007 (1,088 lines) | Primary streaming | Normal turn processing |
| Gemini continuation | 1302–1562 (260 lines) | Nested inside main | Empty-response retry |
| Post-compaction retry | 2132–2275 (143 lines) | After main loop | Retry after context compaction |

All three share the same structure:
1. Check `is_interrupted` → emit interrupted → break
2. Match `stream.next()` on 7+ variants
3. `handle_text_chunk()` + display update + token emission
4. `handle_tool_call()` with same arguments
5. `handle_tool_result()` with same arguments
6. `Usage` event processing
7. `FinalResponse` handling
8. Error classification and recovery
9. `output.flush()`

### After: One StreamProcessor

```rust
/// Owns the mutable state for processing a single stream.
/// Replaces 15+ local `mut` variables with a cohesive struct.
pub(crate) struct StreamContext {
    pub assistant_text: String,
    pub accumulated_reasoning: String,
    pub tool_calls_buffer: Vec<AssistantContent>,
    pub last_tool_name: Option<String>,
    pub final_stop_reason: Option<String>,
    pub turn_tool_infos: Vec<ToolInfo>,
    pub streaming_display: StreamingTokenDisplay,
}

/// Processes stream items from any source (main, retry, continuation).
/// The three copy-pasted loops become three calls to this.
pub(crate) async fn process_stream<M, O>(
    ctx: &mut StreamContext,
    stream: &mut impl StreamExt<Item = Result<MultiTurnStreamItem<M::StreamingResponse>>>,
    session: &mut Session,
    output: &O,
    is_interrupted: &AtomicBool,
    input_queue: Option<&mut InputQueue>,
) -> StreamOutcome
where
    M: CompletionModel,
    M::StreamingResponse: WasmCompatSend + GetTokenUsage,
    O: StreamOutput,
{
    // ONE implementation of the stream processing loop
}

/// What happened when the stream finished.
pub(crate) enum StreamOutcome {
    Completed { stop_reason: Option<String> },
    Interrupted,
    CompactionNeeded,
    Error(StreamErrorKind),
    ThinkingExhausted { usage: TokenUsageSnapshot },
    GeminiContinuationNeeded { strategy: ContinuationStrategy },
}
```

The god function becomes orchestration:

```rust
// ~300 lines of orchestration instead of 1,721
async fn run_agent_stream_internal(...) -> Result<()> {
    let mut ctx = StreamContext::new(session);

    // 1. Pre-prompt compaction check
    if should_compact_before_prompt(session, prompt) {
        execute_compaction(session, ...);
    }

    // 2. Prepare and start stream
    prepare_provider_specific(session); // Gemini thought signatures
    let mut stream = start_stream(&agent, prompt, session, threshold).await;

    // 3. Process stream
    loop {
        match process_stream(&mut ctx, &mut stream, session, output, ...).await {
            StreamOutcome::Completed { .. } => break,
            StreamOutcome::Interrupted => return Ok(()),
            StreamOutcome::CompactionNeeded => {
                execute_compaction(session, ...);
                stream = start_retry_stream(&agent, session, threshold).await;
                ctx.reset();
                continue;
            }
            StreamOutcome::Error(kind) => {
                return handle_stream_error(kind, session, &agent, &mut stream, &mut ctx, output).await;
            }
            StreamOutcome::ThinkingExhausted { usage } => {
                if retry_orchestrator.should_retry_thinking(session) {
                    stream = start_thinking_retry(&agent, session, &ctx, threshold).await;
                    ctx.reset();
                    continue;
                }
                break; // budget exhausted, accept what we got
            }
            StreamOutcome::GeminiContinuationNeeded { strategy } => {
                stream = start_continuation(&agent, session, &strategy, threshold).await;
                ctx.reset_for_continuation();
                continue;
            }
        }
    }

    // 4. Finalize
    ctx.finalize_turn(session);
    Ok(())
}
```

## Card-by-Card Extraction Map

Each card has a **primary extraction** (what it pulls OUT of `stream_loop.rs`) and a **feature addition** (what it adds in the new module).

| Card | Primary Extraction | Feature Addition | Net Lines Removed |
|------|-------------------|-----------------|-------------------|
| **PROV-045** | 5 `is_*()` functions + 5 `build_*()` functions + constants → `stream_errors.rs` | `StreamErrorKind` enum, `classify_stream_error()` | ~250 |
| **PROV-043** | Retry creation boilerplate (TokenState/Hook/Display) × 3 → `stream_retry.rs` | `RetryOrchestrator`, `StreamRetryPolicy`, backoff delay | ~150 |
| **PROV-044** | (nothing to extract — new feature) → `circuit_breaker.rs` | `ApiHealthTracker`, pre-stream check, failure recording | ~0 (but adds outside) |
| **PROV-046** | (nothing to extract — new feature) → `history_persistence.rs` | `persist_before_compaction()`, JSONL writer | ~0 (but adds outside) |
| **PROV-047** | Post-FinalResponse duplicate check → `loop_detection.rs` | `detect_response_loop()` | ~20 |
| **PROV-048** | Thinking state from Session → `session/streaming_health.rs` | `StreamingHealth` struct (merges thinking + failure tracking) | ~30 |
| **PROV-049** | (extends PROV-045) → `stream_errors.rs` | `parse_retry_after()`, `RateLimit` variant handler | ~0 |
| **PROV-050** | (nothing from stream_loop) → `compaction/split_safety.rs` | `find_safe_split_point()`, `validate_tool_pairing()` | ~0 |

**Cross-cutting extractions** (done opportunistically across cards):
- Debug capture boilerplate → `debug_capture.rs` helper (~70 lines saved)
- `BridgeImage` + image handling → `image_handling.rs` (~130 lines saved)
- 15 local `mut` variables → `StreamContext` struct (~50 lines saved)
- 3 duplicated loops → `StreamProcessor` (~400 lines saved)

**Total: ~1,100 lines removed from `stream_loop.rs`**, leaving it at ~1,200 lines. With the StreamProcessor extraction (which should accompany PROV-043), it drops to ~400 lines.

## Session Struct Refactoring

Aligned with PROV-044 and PROV-048, the Session struct should be decomposed:

### Current (7 fields, 6 public)

```rust
pub struct Session {
    provider_manager: ProviderManager,
    pub messages: Vec<Message>,
    pub turns: Vec<ConversationTurn>,
    pub token_tracker: TokenTracker,
    pub annotations: HashMap<usize, Vec<StructuralAnnotation>>,
    pub thinking_exhaustion_cross_turn_count: u32,
    pub session_thinking_level: ThinkingLevel,
}
```

### Target (encapsulated sub-components)

```rust
pub struct Session {
    provider_manager: ProviderManager,
    conversation: ConversationState,     // messages + turns + annotations
    pub token_tracker: TokenTracker,     // keep pub temporarily (72+ call sites)
    thinking: ThinkingState,             // exhaustion_count + level
    api_health: ApiHealthTracker,        // PROV-044: circuit breaker
    streaming_health: StreamingHealth,   // PROV-048: failure tracking
}
```

The `ThinkingState` extraction (data clump) is the first step:

```rust
pub struct ThinkingState {
    exhaustion_count: u32,
    level: ThinkingLevel,
}

impl ThinkingState {
    pub fn record_exhaustion(&mut self) { self.exhaustion_count += 1; }
    pub fn should_downgrade(&self) -> bool { self.exhaustion_count >= 3 }
    pub fn downgrade(&mut self) {
        self.level = downgrade_thinking_level(self.level);
        self.exhaustion_count = 0;
    }
    pub fn level(&self) -> ThinkingLevel { self.level }
    pub fn reset(&mut self) { self.exhaustion_count = 0; }
}
```

## Execution Order

The cards should be implemented in this order to minimize conflicts:

```
1. PROV-045 (Error Classification Enum)     — FOUNDATION: extracts error functions
     ↓
2. PROV-043 (Retry + StreamProcessor)       — BIG WIN: extracts retry + loop dedup
     ↓
3. PROV-044 (Circuit Breaker)               — Depends on PROV-043 for delay logic
     ↓
4. PROV-049 (Retry-After Parsing)           — Extends PROV-045 enum + PROV-043 backoff
     
5. PROV-047 (Loop Detection)               — Independent, small
6. PROV-048 (Streaming Failure Tracking)    — Independent, Session refactoring
7. PROV-046 (History Persistence)           — Independent, compaction path
8. PROV-050 (Split-Safe Compaction)         — Independent, core compaction
```

Cards 5–8 can be done in any order or in parallel.

## Definition of Done (Refactoring)

For each card, the refactoring is complete when:

1. ✅ `stream_loop.rs` is **shorter** than before the card started
2. ✅ No new `#[allow(clippy::too_many_arguments)]` suppressions added
3. ✅ No new `pub` fields added to `Session` (use methods)
4. ✅ All extracted modules are ≤300 lines
5. ✅ The 3 copy-pasted loops are reduced to 1 (by PROV-043 completion)
6. ✅ All 19 existing tests pass + new tests for the feature
7. ✅ `cargo clippy` clean (no suppressed warnings)
