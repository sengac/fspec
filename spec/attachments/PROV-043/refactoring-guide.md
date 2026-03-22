# PROV-043: Refactoring Guide — Retry Orchestrator + StreamProcessor Extraction

## This Card Is the Big Win

PROV-043 delivers the **largest single refactoring**: extracting the 3 copy-pasted stream-processing loops into a reusable `StreamProcessor`, AND introducing structured retry with backoff. It depends on PROV-045 (error enum) being done first.

## What to Extract FROM `stream_loop.rs`

### The 3 Duplicated Loops

| Loop | Lines | Copy-Pasted Pattern |
|------|-------|-------------------|
| Main loop | 919–2007 | check interrupt → match stream.next() → 7 variants → flush |
| Gemini continuation | 1302–1562 | Same pattern, nested inside FinalResponse handling |
| Post-compaction retry | 2132–2275 | Same pattern, after execute_compaction() |

All three share identical:
- Interrupt check → `emit_interrupted` → `handle_final_response` → break
- `handle_text_chunk()` + `streaming_display.record_chunk()` + `emit_tokens()`
- `handle_tool_call()` with same 6 arguments
- `handle_tool_result()` with same 4 arguments
- `Usage` event → `start_new_segment` vs `update_from_usage`
- `FinalResponse` handling
- Error arm
- `output.flush()`

### Retry Boilerplate (×3 occurrences)

Each retry (thinking exhaustion ~line 1682, truncation ~line 1868, post-compaction ~line 2093) creates identical:
```rust
let retry_token_state = Arc::new(Mutex::new(TokenState {
    input_tokens: session.token_tracker.input_tokens,
    cache_read_input_tokens: 0,
    cache_creation_input_tokens: 0,
    output_tokens: 0,
    compaction_needed: false,
}));
let retry_hook = CompactionHook::new(Arc::clone(&retry_token_state), threshold);
```
Plus identical `StreamingTokenDisplay::new(...)` construction.

### 15+ Local `mut` Variables

```rust
let mut assistant_text = String::new();
let mut accumulated_reasoning = String::new();
let mut tool_calls_buffer: Vec<AssistantContent> = Vec::new();
let mut last_tool_name: Option<String> = None;
let mut final_stop_reason: Option<String> = None;
let mut turn_tool_infos: Vec<ToolInfo> = Vec::new();
let mut previous_turn_tool_infos: Vec<ToolInfo> = Vec::new();
let mut truncation_retry_count: u32 = 0;
let mut thinking_exhaustion_retry_count: u32 = 0;
let mut streaming_display = StreamingTokenDisplay::new(...);
```

## New Modules to Create

### 1. `stream_context.rs` (~80 lines)

```rust
/// Replaces the 15+ local `mut` variables in run_agent_stream_internal.
pub(crate) struct StreamContext {
    pub assistant_text: String,
    pub accumulated_reasoning: String,
    pub tool_calls_buffer: Vec<AssistantContent>,
    pub last_tool_name: Option<String>,
    pub final_stop_reason: Option<String>,
    pub turn_tool_infos: Vec<ToolInfo>,
    pub previous_turn_tool_infos: Vec<ToolInfo>,
    pub streaming_display: StreamingTokenDisplay,
}

impl StreamContext {
    pub fn new(session: &Session) -> Self { /* initialize from session state */ }

    /// Reset mutable tracking for a retry stream (same turn, fresh stream).
    pub fn reset_for_retry(&mut self, session: &Session) {
        self.assistant_text.clear();
        self.accumulated_reasoning.clear();
        self.tool_calls_buffer.clear();
        self.last_tool_name = None;
        self.final_stop_reason = None;
        self.turn_tool_infos.clear();
        self.streaming_display = StreamingTokenDisplay::new(
            session.token_tracker.input_tokens,
            session.token_tracker.output_tokens,
            session.token_tracker.cache_read_input_tokens.unwrap_or(0),
            session.token_tracker.cache_creation_input_tokens.unwrap_or(0),
        );
    }

    /// Finalize: process annotations, flush state to session.
    pub fn finalize_turn(&self, session: &mut Session) {
        process_turn_annotations(session, ...);
    }
}
```

### 2. `stream_processor.rs` (~250 lines)

```rust
/// What happened when the stream finished processing.
pub(crate) enum StreamOutcome {
    /// Normal completion — assistant finished responding.
    Completed { stop_reason: Option<String> },
    /// User interrupted (ESC / ctrl+C).
    Interrupted,
    /// CompactionHook triggered — need to compact and retry.
    CompactionNeeded,
    /// Classified error — caller decides recovery strategy.
    Error(StreamErrorKind),
    /// Thinking token exhaustion detected on FinalResponse.
    ThinkingExhausted { reasoning_tokens: u64, output_tokens: u64 },
    /// Gemini empty-response — needs continuation prompt.
    GeminiContinuationNeeded { strategy: ContinuationStrategy },
}

/// Processes stream items from ANY source (main, retry, continuation).
/// This is the ONE implementation replacing 3 copy-pasted loops.
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
    loop {
        // Interrupt check (unified)
        if is_interrupted.load(Acquire) {
            let queued = input_queue.as_mut().map_or(vec![], |iq| iq.dequeue_all());
            output.emit_interrupted(&queued);
            if !ctx.assistant_text.is_empty() {
                handle_final_response(&ctx.assistant_text, &mut session.messages)?;
            }
            output.emit_done_with_stop_reason(ctx.final_stop_reason.take());
            return StreamOutcome::Interrupted;
        }

        match stream.next().await {
            Some(Ok(MultiTurnStreamItem::StreamAssistantItem(item))) => {
                match item {
                    StreamedAssistantContent::Text(text) => { /* unified text handling */ }
                    StreamedAssistantContent::ToolCall(tc) => { /* unified tool call handling */ }
                    StreamedAssistantContent::ReasoningDelta { reasoning } => { /* unified */ }
                    _ => {}
                }
            }
            Some(Ok(MultiTurnStreamItem::StreamUserItem(item))) => { /* tool result */ }
            Some(Ok(MultiTurnStreamItem::Usage(usage))) => { /* token tracking */ }
            Some(Ok(MultiTurnStreamItem::FinalResponse(resp))) => {
                handle_final_response(&ctx.assistant_text, &mut session.messages)?;

                // Check Gemini continuation
                if let Some(strategy) = check_gemini_continuation(session, &ctx) {
                    return StreamOutcome::GeminiContinuationNeeded { strategy };
                }

                // Check thinking exhaustion (PROV-041)
                let usage = resp.usage();
                if is_thinking_exhaustion(...) {
                    return StreamOutcome::ThinkingExhausted { ... };
                }

                ctx.final_stop_reason = extract_stop_reason(&resp);
                return StreamOutcome::Completed { stop_reason: ctx.final_stop_reason.take() };
            }
            Some(Err(e)) => {
                let kind = classify_stream_error(&e);
                if matches!(&kind, StreamErrorKind::CompactionCancelled) {
                    return StreamOutcome::CompactionNeeded;
                }
                return StreamOutcome::Error(kind);
            }
            None => return StreamOutcome::Completed { stop_reason: None },
        }

        output.flush();
    }
}
```

### 3. `stream_retry.rs` (~120 lines)

```rust
/// Retry policy with exponential backoff.
pub struct StreamRetryPolicy {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub multiplier: f64,
}

impl StreamRetryPolicy {
    pub fn default_stream() -> Self {
        Self { max_attempts: 3, initial_delay: Duration::from_secs(2), max_delay: Duration::from_secs(30), multiplier: 2.0 }
    }

    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let delay_ms = self.initial_delay.as_millis() as f64 * self.multiplier.powi(attempt as i32 - 1);
        Duration::from_millis(delay_ms.min(self.max_delay.as_millis() as f64) as u64)
    }
}

/// Manages retry state across a single turn.
pub(crate) struct RetryOrchestrator {
    truncation_count: u32,
    thinking_exhaustion_count: u32,
    policy: StreamRetryPolicy,
}

impl RetryOrchestrator {
    pub fn new() -> Self { ... }

    /// Create fresh TokenState + CompactionHook for a retry.
    /// Replaces the 3× copy-pasted boilerplate.
    pub fn create_retry_hook(&self, session: &Session, threshold: u64) -> (Arc<Mutex<TokenState>>, CompactionHook) {
        let token_state = Arc::new(Mutex::new(TokenState {
            input_tokens: session.token_tracker.input_tokens,
            cache_read_input_tokens: 0,
            cache_creation_input_tokens: 0,
            output_tokens: 0,
            compaction_needed: false,
        }));
        let hook = CompactionHook::new(Arc::clone(&token_state), threshold);
        (token_state, hook)
    }

    /// Sleep for backoff delay, emitting status to user.
    pub async fn backoff_delay(&self, attempt: u32, output: &impl StreamOutput) {
        let delay = self.policy.delay_for_attempt(attempt);
        output.emit_status(&format!("Retrying in {:.1}s...", delay.as_secs_f64()));
        tokio::time::sleep(delay).await;
    }

    pub fn can_retry_truncation(&self) -> bool { ... }
    pub fn can_retry_thinking(&self) -> bool { ... }
    pub fn record_truncation_retry(&mut self) { ... }
    pub fn record_thinking_retry(&mut self) { ... }
}
```

## How `stream_loop.rs` Shrinks

### Before: 1,721-line god function
### After: ~350-line orchestration function

```rust
async fn run_agent_stream_internal<M, O, E>(...) -> Result<()> {
    let mut ctx = StreamContext::new(session);
    let mut retry = RetryOrchestrator::new();

    // 1. Pre-prompt compaction (~50 lines, same as before)
    if should_compact_before_prompt(session, prompt, threshold) {
        execute_compaction(session, compaction_in_progress.clone(), Some(prompt)).await?;
        ctx.reset_for_retry(session);
    }

    // 2. Provider-specific prep (~15 lines)
    prepare_gemini_thought_signatures(session);

    // 3. Start stream (~30 lines)
    let (token_state, hook) = retry.create_retry_hook(session, threshold);
    let mut stream = agent.prompt_streaming_with_history_and_hook(prompt, &mut session.messages, hook).await;

    // 4. Process loop (~100 lines of match on StreamOutcome)
    loop {
        match process_stream::<M, O>(&mut ctx, &mut stream, session, output, &is_interrupted, input_queue.as_deref_mut()).await {
            StreamOutcome::Completed { stop_reason } => {
                output.emit_done_with_stop_reason(stop_reason);
                break;
            }
            StreamOutcome::Interrupted => return Ok(()),
            StreamOutcome::CompactionNeeded => {
                execute_compaction(session, compaction_in_progress.clone(), Some(prompt)).await?;
                let (ts, hook) = retry.create_retry_hook(session, threshold);
                token_state = ts;
                stream = agent.prompt_streaming_with_history_and_hook("Continue", &mut session.messages, hook).await;
                ctx.reset_for_retry(session);
                continue;
            }
            StreamOutcome::Error(kind) => {
                match kind {
                    StreamErrorKind::PromptTooLong { .. } if has_compactable_turns(session) => {
                        pop_last_user_message(session);
                        signal_compaction_needed(&token_state);
                        // will be caught by CompactionNeeded on next iteration
                        break; // post-loop compaction handles it
                    }
                    StreamErrorKind::TruncatedToolCall { raw_message } if retry.can_retry_truncation() => {
                        retry.record_truncation_retry();
                        retry.backoff_delay(retry.truncation_count, output).await;
                        let recovery = build_truncation_recovery_message(&raw_message);
                        let (ts, hook) = retry.create_retry_hook(session, threshold);
                        stream = agent.prompt_streaming_with_history_and_hook(&recovery, &mut session.messages, hook).await;
                        ctx.reset_for_retry(session);
                        continue;
                    }
                    StreamErrorKind::ImageContent { raw_message } => {
                        pop_last_user_message(session);
                        if sanitize_image_content(&mut session.messages) {
                            output.emit_error(&format!("{raw_message}\n\n[Images removed]"));
                            break;
                        }
                        return Err(anyhow!("Agent error: {raw_message}"));
                    }
                    other => {
                        output.emit_error(other.raw_message());
                        return Err(anyhow!("Agent error: {}", other.raw_message()));
                    }
                }
            }
            StreamOutcome::ThinkingExhausted { reasoning_tokens, output_tokens } => {
                session.record_thinking_exhaustion(); // encapsulated!
                if retry.can_retry_thinking() {
                    retry.record_thinking_retry();
                    retry.backoff_delay(retry.thinking_exhaustion_count, output).await;
                    let recovery = build_thinking_exhaustion_recovery_message(reasoning_tokens, output_tokens, ctx.captured_reasoning());
                    let (ts, hook) = retry.create_retry_hook(session, threshold);
                    stream = agent.prompt_streaming_with_history_and_hook(&recovery, &mut session.messages, hook).await;
                    ctx.reset_for_retry(session);
                    continue;
                }
                output.emit_status(&build_thinking_budget_exhausted_message(MAX_THINKING_EXHAUSTION_RETRIES));
                break;
            }
            StreamOutcome::GeminiContinuationNeeded { strategy } => {
                let (ts, hook) = retry.create_retry_hook(session, threshold);
                stream = agent.prompt_streaming_with_history_and_hook(&strategy.prompt(), &mut session.messages, hook).await;
                ctx.reset_for_retry(session);
                continue;
            }
        }
    }

    // 5. Post-loop compaction check (~30 lines)
    // ... same as current but using retry.create_retry_hook()

    // 6. Finalize (~10 lines)
    ctx.finalize_turn(session);
    Ok(())
}
```

## Debug Capture Cleanup (Opportunistic)

While touching stream processing, extract the 11× repeated pattern:

```rust
// FROM (7 lines, repeated 11 times):
if let Ok(manager_arc) = get_debug_capture_manager() {
    if let Ok(mut manager) = manager_arc.lock() {
        if manager.is_enabled() {
            manager.capture("event", json!({...}), opts);
        }
    }
}

// TO (1 line):
capture_debug_event("event", json!({...}), Some(&request_id));
```

Create `debug_capture.rs` (~30 lines) with one helper function. Saves ~70 lines.

## Estimated Impact

- **Lines removed from `stream_loop.rs`**: ~1,400 (3 loops → 1, retry boilerplate → helper, local vars → struct)
- **Lines added across new modules**: ~450 (stream_processor + stream_context + stream_retry + debug_capture)
- **Net reduction**: ~950 lines eliminated
- **God function**: 1,721 → ~350 lines
- **Max nesting depth**: 17 → ~6
- **`too_many_arguments` suppressions**: 4 → 0 (StreamContext replaces params)
