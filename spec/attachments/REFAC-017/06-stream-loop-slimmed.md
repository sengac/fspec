# Module 6: `stream_loop.rs` (Slimmed Core)

**Path**: `codelet/cli/src/interactive/stream_loop.rs` (in-place refactor)  
**Target lines**: ≤300 for the file itself, with sub-loops extracted  
**Responsibility**: Orchestration only — wires entry points to the internal loop, delegates error handling to extracted modules.

---

## What Stays

### Private helpers (small, loop-coupled):
| Function | Lines | Reason to Keep |
|----------|-------|----------------|
| `process_turn_annotations` | 49–71 | Mutates `session.annotations` — tightly coupled to loop state |
| `signal_compaction_needed` | 421–430 | Mutates `token_state` Arc — tightly coupled to loop state |

### Entry points (thin wrappers):
| Function | Lines | Delegates to |
|----------|-------|-------------|
| `run_agent_stream_with_interruption` | 437–465 | `run_agent_stream_internal` |
| `run_agent_stream` | 475–502 | `run_agent_stream_internal` |
| `run_agent_stream_with_images` | 509–537 | `run_agent_stream_internal` |

### Core loop:
| Function | Current Lines | Notes |
|----------|--------------|-------|
| `run_agent_stream_internal` | 611–2331 | **Still too large — needs further decomposition** |

---

## Further Decomposition of `run_agent_stream_internal` (Phase 2)

The 1,720-line monolith contains 3 deeply nested sub-loops that can be extracted:

### Sub-loop 1: Gemini Continuation Loop (lines ~1202–1582)
**Extract to**: `gemini_continuation.rs` (~200 lines)  
**Trigger**: When `GeminiTurnCompletionFacade` returns `ContinuationStrategy::FullLoop`  
**Pattern**: Start new stream, process chunks, handle nested continuations  
**Signature sketch**:
```rust
pub(super) async fn handle_gemini_continuation<M, O>(
    agent: &RigAgent<M>,
    session: &mut Session,
    output: &O,
    is_interrupted: &Arc<AtomicBool>,
    input_queue: &mut Option<&mut InputQueue>,
    token_state: &Arc<Mutex<TokenState>>,
    threshold: u64,
    continuation_prompt: &str,
    current_display: StreamingTokenDisplayValues,
    final_stop_reason: &mut Option<String>,
) -> Result<GeminiContinuationResult>
```

### Sub-loop 2: Post-Loop Compaction Retry (lines ~2026–2304)
**Extract to**: `compaction_retry.rs` (~180 lines)  
**Trigger**: When `compaction_needed == true` after main loop exits  
**Pattern**: Execute compaction, start retry stream, process retry chunks  
**Signature sketch**:
```rust
pub(super) async fn handle_compaction_retry<M, O>(
    agent: &RigAgent<M>,
    session: &mut Session,
    output: &O,
    is_interrupted: &Arc<AtomicBool>,
    input_queue: &mut Option<&mut InputQueue>,
    token_state: &Arc<Mutex<TokenState>>,
    threshold: u64,
    context_window: u64,
    compaction_in_progress: Arc<AtomicBool>,
    prompt: &str,
) -> Result<()>
```

### Sub-loop 3: Error Recovery Branch (lines ~1755–1926)
**Part of main match arm** — harder to extract but the truncation retry (lines 1849–1926) shares exact same pattern as thinking exhaustion retry. Both could use a shared retry helper:
```rust
pub(super) async fn start_recovery_stream<M>(
    agent: &RigAgent<M>,
    session: &mut Session,
    recovery_prompt: &str,
    threshold: u64,
) -> (impl Stream, StreamingTokenDisplay)
```

---

## After Full Decomposition: Target File Sizes

| Module | Est. Lines | Status |
|--------|-----------|--------|
| `error_classifiers.rs` | ~80 | Phase 1 |
| `recovery_truncation.rs` | ~80 | Phase 1 |
| `recovery_thinking.rs` | ~120 | Phase 1 |
| `recovery_image.rs` | ~120 | Phase 1 |
| `multimodal.rs` | ~80 | Phase 1 |
| `gemini_continuation.rs` | ~200 | Phase 2 |
| `compaction_retry.rs` | ~180 | Phase 2 |
| `stream_loop.rs` (core loop only) | ~250–300 | Phase 2 |
| **Total** | ~1,110–1,160 | vs 2,331 original |

The ~50% reduction comes from eliminating inline duplication (retry patterns, display tracking, debug capture boilerplate are repeated 3+ times in the current file).

---

## Migration Order

1. **Phase 1** (safe, no logic changes): Extract pure functions → `error_classifiers`, `recovery_truncation`, `recovery_thinking`, `recovery_image`, `multimodal`
2. **Phase 2** (structural): Extract sub-loops → `gemini_continuation`, `compaction_retry`  
3. **Phase 3** (DRY): Deduplicate shared retry patterns across sub-loops

Phase 1 is zero-risk — functions are self-contained and tests already cover them independently.
