# HITL TUI Integration — Remaining Work

## Overview

BUG-117 implemented the Rust plumbing for the HITL (Human-in-the-Loop) pause pattern, including the handler cleanup, headless mode error, and NAPI response/cancellation bindings. **12 of 16 scenarios remain uncovered** and need tests + implementation. This card covers all the remaining TypeScript UI integration and the outstanding Rust/NAPI work.

The feature file is `spec/features/hitl-handler-wiring.feature` — it should be **shared** between BUG-117 (completed scenarios) and this card (remaining scenarios).

---

## Architecture Reference

The HITL tool follows the **PAUSE pattern** (not the StreamChunk pattern):

```
LLM calls request_user_input
  → Rust HITL handler closure fires
    → Stores questions in hitl_request: RwLock<Option<HitlRequestState>>
    → Sets session status to Paused
    → Blocks on mpsc channel (wait_for_hitl_response)
  → TypeScript polls via NAPI getter (session_get_hitl_request)
    → useRustSessionState includes hitlRequest in snapshot
    → InputTransition renders inline question UI
    → AgentView keyboard handler captures ↑/↓/Enter/Esc
    → On submit: sessionSendHitlResponse → mpsc channel unblocks
  → Rust handler resumes
    → Clears hitl_request state
    → Sets status back to Running
    → Returns response to caller
```

---

## Remaining Scenarios (12)

### Category 1: Rust Handler State (2 scenarios)

#### 1.1 "HITL handler stores questions in session state and pauses" (line 44)
```gherkin
Given a BackgroundSession with hitl_request state and hitl_response channel pair
When the HITL handler closure is invoked with a request containing 2 questions
Then the handler should store the questions in hitl_request state
And the handler should set session status to Paused
And the handler should block on wait_for_hitl_response
When a response is sent via send_hitl_response
Then the handler should clear the hitl_request state
And the handler should set session status back to Running
And the handler should return the response to the caller
```

**What to implement:**
- The full HITL handler closure in `codelet/napi/src/session_manager.rs` that follows the pause pattern
- `set_hitl_request()` and `clear_hitl_request()` methods on `BackgroundSession`
- `wait_for_hitl_response()` blocking method using `hitl_response_rx.recv()`
- Integration with `set_status(Paused)` and `set_status(Running)`

**What to test:**
- Verify the handler stores questions, pauses, blocks, and unblocks correctly
- Verify state transitions: Running → Paused → Running
- Test the full lifecycle in a TypeScript unit test (mock NAPI)

---

#### 1.2 "BackgroundSession has HITL request state and response channel pair" (line 69)
```gherkin
Given a new BackgroundSession is created
Then it should have a hitl_request field of type RwLock Option HitlRequestState
And it should have a hitl_response_tx sender and hitl_response_rx receiver
And set_hitl_request should store questions for TypeScript to poll
And get_hitl_request should return the stored questions
And clear_hitl_request should remove the stored questions
```

**What to implement:**
- `hitl_request: RwLock<Option<HitlRequestState>>` field on `BackgroundSession`
- `hitl_response_tx` / `hitl_response_rx` channel pair (std::sync::mpsc) on `BackgroundSession`
- `set_hitl_request()`, `get_hitl_request()`, `clear_hitl_request()` methods

**What to test:**
- Unit test for BackgroundSession construction — verify fields exist
- Test set/get/clear cycle for hitl_request state

---

### Category 2: NAPI Getter (1 scenario)

#### 2.1 "NAPI getter returns HITL request when session is paused" (line 79)
```gherkin
Given a session is paused with hitl_request state containing questions
When TypeScript calls session_get_hitl_request with the session ID
Then it should return the questions array with id, header, question, and options
And when the session is not paused or has no hitl_request it should return null
```

**What to implement:**
- `session_get_hitl_request(session_id: String)` NAPI function in `codelet/napi/src/session_manager.rs`
- Returns `Option<NapiHitlRequestState>` with questions array
- Follow same pattern as `session_get_pause_state`

**What to test:**
- TypeScript test calling the NAPI getter when paused with questions → returns data
- TypeScript test calling when not paused → returns null
- TypeScript test calling when paused but no hitl_request → returns null

---

### Category 3: TypeScript State Polling (2 scenarios)

#### 3.1 "useRustSessionState includes hitlRequest in snapshot when paused" (line 99)
```gherkin
Given a session is paused and has HITL request state
When useRustSessionState fetches the snapshot
Then snapshot.hitlRequest should contain the questions array
And snapshot.isPaused should be true
```

**What to implement:**
- Add `hitlRequest` field to `RustSessionSnapshot` interface
- In `useRustSessionState` hook, when `isPaused` is true, call `getHitlRequest(sessionId)` from `rustStateSource`
- Add `getHitlRequest(sessionId: string)` method to `RustStateSource` interface in `rustStateSource.ts`

**Files to modify:**
- `src/tui/hooks/rustStateSource.ts` — add `getHitlRequest` to interface + default implementation
- `src/tui/hooks/useRustSessionState.ts` — add `hitlRequest` to snapshot, fetch when paused

**What to test:**
- Mock rustStateSource to return hitl request data when paused
- Verify snapshot includes hitlRequest

---

#### 3.2 "useRustSessionState returns null hitlRequest when not paused" (line 105)
```gherkin
Given a session is running with no HITL request
When useRustSessionState fetches the snapshot
Then snapshot.hitlRequest should be null
```

**What to test:**
- Mock rustStateSource with no pause state
- Verify snapshot.hitlRequest is null

---

### Category 4: TypeScript Inline Rendering (3 scenarios, all @integration)

#### 4.1 "InputTransition renders HITL question with options inline" (line 113)
```gherkin
Given isPaused is true and hitlRequest contains a question with options
When InputTransition renders
Then it should show the question header and question text
And it should show selectable options with selected and unselected indicators
And it should show navigation hints for up down Enter and Esc
```

**What to implement:**
- New branch in `InputTransition.tsx` that checks for `hitlRequest` when `isPaused`
- Render: ⏸ icon, question header `[1/N]`, question text, option list with ● selected / ○ unselected
- Navigation hints: ↑/↓ navigate, Enter select, Esc cancel
- Same rendering location as existing pause UI

**What to test:**
- Render test with ink-testing-library
- Verify question text, options, indicators render correctly

---

#### 4.2 "InputTransition renders freeform-only HITL question" (line 121)
```gherkin
Given isPaused is true and hitlRequest contains a question without options
When InputTransition renders
Then it should show the question text
And it should show a text input area for freeform response
```

**What to implement:**
- Handle case where question has no `options` array
- Render text input area instead of option selector

**What to test:**
- Render test with question without options
- Verify text input renders

---

#### 4.3 "Multi-step HITL advances through questions" (line 128)
```gherkin
Given isPaused is true and hitlRequest contains 2 questions
And the user is on question 1 of 2
When the user selects an option and presses Enter
Then InputTransition should advance to question 2 of 2
And the first question answer should be stored
```

**What to implement:**
- Local state tracking current question index and collected answers
- On Enter for non-last question: store answer, advance index
- On Enter for last question: submit all collected answers

**What to test:**
- Multi-step progression test
- Verify answers are accumulated

---

### Category 5: TypeScript Keyboard Handling (3 scenarios, all @integration)

#### 5.1 "AgentView HITL keyboard handler navigates options" (line 138)
```gherkin
Given a session is paused with HITL questions containing options
When the user presses up arrow
Then the selected option should move up
When the user presses down arrow
Then the selected option should move down
```

**What to implement:**
- `useInputCompat` handler in `AgentView.tsx` for HITL mode
- Track `selectedOptionIndex` state
- ↑ decrements (with wrap), ↓ increments (with wrap)

**What to test:**
- Keyboard event simulation
- Verify option index changes correctly

---

#### 5.2 "AgentView HITL keyboard handler submits all answers" (line 146)
```gherkin
Given a session is paused with HITL questions and all questions answered
When the user presses Enter on the last question
Then sessionSendHitlResponse should be called with all collected answers
And cancelled should be false
```

**What to implement:**
- On Enter during last question: call `sessionSendHitlResponse({ answers: [...], cancelled: false })`
- Wire up NAPI call from keyboard handler

**What to test:**
- Verify sessionSendHitlResponse called with correct payload
- Verify cancelled is false

---

#### 5.3 "User cancels HITL with Escape" (line 153)
```gherkin
Given a session is paused with HITL questions
When the user presses Escape
Then sessionSendHitlResponse should be called with cancelled true
And the handler should unblock and return Cancelled
```

**What to implement:**
- Esc key handler: call `sessionSendHitlResponse({ answers: [], cancelled: true })`

**What to test:**
- Verify Esc triggers cancellation
- Verify cancelled is true

---

### Category 6: Cleanup (1 scenario)

#### 6.1 "HitlRequest StreamChunk variant removed" (line 161)
```gherkin
Given the codebase previously had a HitlRequest StreamChunk variant
Then the HitlRequest variant should not exist in StreamChunk
And GlobalSessionStreamManager should not have setHitlHandler method
And GlobalSessionStreamManager should not have clearHitlHandler method
And GlobalSessionStreamManager should not have handleHitlRequest method
```

**What to implement:**
- Remove `HitlRequest` variant from `StreamChunk` enum (Rust `codelet/napi/src/types.rs`)
- Remove `setHitlHandler`, `clearHitlHandler`, `handleHitlRequest` from `GlobalSessionStreamManager` (TypeScript)
- Remove any HITL-related intercept code from `globalSessionStreamManager.ts`

**What to test:**
- Negative assertion: grep codebase for removed patterns
- Verify no compilation errors after removal

---

## Files Requiring Changes

### Rust (codelet/)
| File | Changes |
|------|---------|
| `codelet/napi/src/session_manager.rs` | Add `hitl_request` field, set/get/clear methods, NAPI getter, handler closure |
| `codelet/napi/src/types.rs` | Remove `HitlRequest` StreamChunk variant |

### TypeScript (src/tui/)
| File | Changes |
|------|---------|
| `src/tui/hooks/rustStateSource.ts` | Add `getHitlRequest(sessionId)` to interface + default impl |
| `src/tui/hooks/useRustSessionState.ts` | Add `hitlRequest` to `RustSessionSnapshot`, fetch when paused |
| `src/tui/components/InputTransition.tsx` | New HITL question rendering branch (options, freeform, multi-step) |
| `src/tui/components/AgentView.tsx` | New `useInputCompat` handler for HITL keyboard (↑/↓/Enter/Esc) |
| `src/tui/services/globalSessionStreamManager.ts` | Remove `setHitlHandler`, `clearHitlHandler`, `handleHitlRequest` |

### Tests
| File | Changes |
|------|---------|
| `src/tui/services/__tests__/hitl-handler-wiring.test.ts` | Add tests for remaining 12 scenarios |
| New: `src/tui/components/__tests__/hitl-input-transition.test.tsx` | React render tests for InputTransition HITL branch |
| New: `src/tui/components/__tests__/hitl-keyboard-handler.test.tsx` | Keyboard handler tests |

---

## Estimation

This work involves:
- 2 Rust state management scenarios
- 1 NAPI binding scenario
- 2 TypeScript state polling scenarios
- 6 React component rendering + keyboard handling scenarios (all @integration)
- 1 cleanup scenario

**Suggested estimate: 8 story points** (multiple components, React rendering, keyboard handling, cross-layer integration)

---

## Dependencies

- **Depends on BUG-117** (must be done first — provides the Rust plumbing this builds on)
- **Related to TOOL-017** (Request User Input HITL Tool — done)
- **Related to BUG-116** (Codex facade maps request_user_input — done)
