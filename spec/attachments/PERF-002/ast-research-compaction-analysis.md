# AST Research Summary for PERF-002: Compaction Performance Optimization

## Research Scope
Analyzed codebase to understand current compaction implementation and identify optimization opportunities.

## Key Findings

### 1. Duplicate /compact Handlers in AgentView.tsx
**Location:** `src/tui/components/AgentView.tsx`
```
Line 2485: userMessage === '/compact' (in handleSubmit)
Line 3608: userMessage === '/compact' (in handleSubmitWithCommand)
```

**Analysis:**
- Two identical handlers doing the same work
- Creates execution conflicts and code duplication
- Both call `sessionCompact(currentSessionId)` 
- Need to remove one handler (keep handleSubmitWithCommand)

### 2. Session State Management in Rust
**SessionCompact Function:** `codelet/napi/src/session_manager.rs:4789`
```rust
pub async fn session_compact(session_id: String) -> Result<CompactionResult>
```

**Current State Enum:** SessionStatus (Idle, Running, Interrupted, Paused)
- Missing Compacting state for progress tracking
- Need to add Compacting variant with progress fields

### 3. Core Compaction Engine
**ContextCompactor:** `codelet/core/src/compaction/compactor.rs:40`
```rust
pub struct ContextCompactor {
```

**Performance Issue:** Current implementation calls LLM once per turn for anchor detection
- Lines 112-116: `for (idx, turn) in turns.iter().enumerate()`
- Each iteration calls `detector.detect(turn, idx, &llm_prompt).await?`
- For 50 turns = 50 LLM API calls = 5+ minutes

### 4. UI State Management
**InputTransition Component:** `src/tui/components/InputTransition.tsx`
- Currently shows generic "Thinking..." during loading
- Connected to `useRustSessionState` hook 
- `isLoading = status === 'running'` 
- Need to add compacting state support for detailed progress

## Implementation Plan

### Phase 1: Remove Duplicate Handlers
- Remove `/compact` handler from `handleSubmit` (line 2485)
- Keep only `handleSubmitWithCommand` implementation

### Phase 2: Add Compacting State
- Extend Rust `SessionStatus` enum with `Compacting { phase: String, current: u32, total: u32 }`
- Update NAPI bindings to expose compacting state
- Modify `useRustSessionState` to handle compacting status

### Phase 3: Batch Anchor Detection  
- Modify `ContextCompactor::compact()` to process all turns in single LLM call
- Replace per-turn loop with batch analysis
- Expected: 50+ calls → 1 call = ~90% time reduction

### Phase 4: Progress UI
- Update `InputTransition` to show "Analyzing anchors... 23/47 turns"
- Add compacting phase detection in UI components
- Integrate with existing ThreeButtonDialog for retry logic

## Files Requiring Changes

### Rust Layer
- `codelet/napi/src/session_manager.rs` - SessionStatus enum, session_compact function
- `codelet/core/src/compaction/compactor.rs` - Batch optimization  
- `codelet/napi/src/types.rs` - CompactionProgress type

### TypeScript Layer  
- `src/tui/components/AgentView.tsx` - Remove duplicate handler
- `src/tui/components/InputTransition.tsx` - Compacting progress display
- `src/tui/hooks/useRustSessionState.ts` - Compacting state support

## Risk Assessment
- **Medium complexity** - Changes span multiple layers (Rust, NAPI, TypeScript)
- **Breaking change risk** - Modifying SessionStatus enum requires careful NAPI coordination
- **Performance gain** - High confidence in 90%+ time reduction from batching
- **UX improvement** - Significant improvement in feedback and loading states