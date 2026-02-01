# Compaction Performance Analysis

## Current Issues Identified

### 1. Duplicate `/compact` Command Handlers
- **Location**: Two identical handlers in AgentView.tsx
  - Line ~2485 in `handleSubmit` 
  - Line ~3607 in `handleSubmitWithCommand`
- **Problem**: Execution flow confusion and redundant code
- **Impact**: Command doesn't execute until after first LLM message due to execution order

### 2. Execution Flow Issue
- **Problem**: `/compact` doesn't work until after sending a message to LLM
- **Root Cause**: 
  - `handleSubmit` delegates `/` commands to `executeSlashCommandRef.current`
  - Guard condition `if (!currentProvider || !inputValue.trim() || displayIsLoading)` blocks execution
  - `currentSessionId` only created after first real LLM message
  - Line 2487: "Compaction requires an active session. Send a message first."

### 3. Extremely Slow Performance (5+ Minutes)
- **Root Cause**: Multiple sequential LLM API calls during compaction
- **Details**:
  - **Anchor Detection Phase** (lines 112-116 in compactor.rs): Makes **one LLM call per turn**
  - For 50 conversation turns = 50 separate LLM API calls!
  - Each call takes 2-10 seconds through provider chain
  - **Summary Generation Phase**: Additional LLM call(s)

### 4. Missing Loading State Management
- **Problem**: No UI feedback during long compaction process
- **Issues**:
  - No loading state set during compaction
  - `displayIsLoading` managed by Rust layer (`rustSnapshot.isLoading`)
  - No completion callback to clear loading state
  - Input remains enabled during compaction
  - Next message can't be sent after compaction

## Technical Architecture

### Compaction Flow (Current)
```
User types /compact
  ↓
handleSubmit → handleSubmitWithCommand 
  ↓
sessionCompact(currentSessionId)
  ↓
execute_compaction(&mut inner) [Rust]
  ↓
For each turn: detector.detect(turn, idx, &llm_prompt) [50+ LLM calls]
  ↓
generate_weighted_summary() [Additional LLM calls]
  ↓
Returns CompactionResult
  ↓
UI updates token display but no loading state management
```

### LLM Provider Chain (Per Call)
```
llm_prompt closure
  ↓
ProviderManager::with_provider(provider_name)
  ↓
prompt_provider(&manager, &prompt)
  ↓
match provider: claude/openai/codex/gemini
  ↓
create_rig_agent(None, None)
  ↓
agent.prompt(prompt)
  ↓
Network API call (2-10 seconds each)
```

## Proposed Solutions

### 1. Remove Duplicate Handler ✅
- Keep only one `/compact` handler in `handleSubmitWithCommand`
- Remove redundant handler from `handleSubmit`
- Ensure proper delegation flow

### 2. Add Loading State Management ✅
- Set loading state at start of compaction
- Show progress indicator during anchor detection
- Clear loading state on completion/error
- Disable input during compaction

### 3. Batch Anchor Detection ✅
- **Current**: 1 LLM call per turn (50+ calls)
- **Proposed**: 1 LLM call for all turns (batch analysis)
- **Expected Impact**: 5+ minutes → 10-30 seconds
- **Implementation**: Modify `AnchorDetector::detect` to accept batch of turns

### 4. Add Compacting State Indicator ✅
- Progress bar or percentage during anchor analysis
- Turn count progress (e.g., "Analyzing anchors... 23/47 turns")
- Clear completion message
- Error state handling

### 5. Session Initialization Fix ✅
- Allow `/compact` to work even before first LLM message
- Check for existing session data to compact
- Better error message if truly nothing to compact

## Expected Improvements

### Performance
- **Before**: 50+ LLM API calls (5+ minutes)
- **After**: 1-2 LLM API calls (10-30 seconds)
- **Improvement**: ~90% reduction in compaction time

### User Experience
- Clear loading feedback during compaction
- Progress indication for long operations
- Input disabled to prevent confusion
- Proper error messages and state management
- Works immediately without requiring prior LLM message

### Code Quality
- Remove duplicate handlers
- Single responsibility for compaction logic
- Proper state management
- Better error handling and user feedback