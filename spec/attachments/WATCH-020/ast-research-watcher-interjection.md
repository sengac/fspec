# AST Research: Watcher Interjection Implementation

## Research Date: $(date)
## Story: WATCH-020 - Autonomous Watcher Interjection with Response Parsing

---

## Key Structures Found

### SessionRole (line 83)
```
codelet/napi/src/session_manager.rs:83:struct SessionRole
```
This struct defines the watcher's role configuration. Will need to add `auto_inject: bool` field here.

### BackgroundOutput (line 3206)
```
codelet/napi/src/session_manager.rs:3206:struct BackgroundOutput
```
This is the output handler used during watcher operations. We need to create WatcherOutput that wraps this to capture turn text.

### WatchGraph (line 909)
```
codelet/napi/src/session_manager.rs:909:struct WatchGraph
```
Manages parent-watcher relationships.

---

## Key Functions Found

### format_evaluation_prompt (line 250)
```
codelet/napi/src/session_manager.rs:250:fn format_evaluation_prompt
```
Formats the prompt sent to watcher AI for evaluation. Must be modified to include [INTERJECT]/[CONTINUE] format instructions.

### watcher_loop_tick (line 334)
```
codelet/napi/src/session_manager.rs:334:fn watcher_loop_tick
```
Handles each tick of the watcher loop. 

### watcher_agent_loop (line 3128)
```
codelet/napi/src/session_manager.rs:3128:fn watcher_agent_loop
```
Main watcher agent loop. This is where we need to add:
1. Wrap output with WatcherOutput
2. After turn completion, get turn text
3. If is_user_prompt=false, parse for [INTERJECT]/[CONTINUE]
4. If interjection found and auto_inject=true, call watcher_inject

### watcher_inject (line 3602)
```
codelet/napi/src/session_manager.rs:3602:fn watcher_inject
```
Existing function to inject messages into parent session. Will be called automatically when [INTERJECT] is parsed.

---

## Implementation Plan

### New Files Required
1. `codelet/napi/src/interjection.rs` - Interjection struct and parse_interjection() function
2. `codelet/napi/src/watcher_output.rs` - WatcherOutput wrapper struct

### Modifications Required
1. `codelet/napi/src/session_manager.rs`:
   - Add `auto_inject: bool` to SessionRole struct (line 83)
   - Modify format_evaluation_prompt (line 250) to add response format instructions
   - Modify watcher_agent_loop (line 3128) to:
     - Use WatcherOutput wrapper
     - Parse response after turn completion
     - Call watcher_inject if interjection found

2. `codelet/napi/src/lib.rs`:
   - Add mod declarations for new files

---

## AST Pattern Matches

### All Rust structs in session_manager.rs:
codelet/napi/src/session_manager.rs:27:12:struct
codelet/napi/src/session_manager.rs:83:5:struct
codelet/napi/src/session_manager.rs:104:5:struct
codelet/napi/src/session_manager.rs:162:5:struct
codelet/napi/src/session_manager.rs:514:5:struct
codelet/napi/src/session_manager.rs:529:5:struct
codelet/napi/src/session_manager.rs:539:5:struct
codelet/napi/src/session_manager.rs:551:5:struct
codelet/napi/src/session_manager.rs:909:5:struct
codelet/napi/src/session_manager.rs:2722:5:struct
codelet/napi/src/session_manager.rs:3206:1:struct
codelet/napi/src/session_manager.rs:3279:1:struct
codelet/napi/src/session_manager.rs:3461:5:struct

### All functions starting with 'watcher' or 'format_eval':
250:pub fn format_evaluation_prompt(buffer: &ObservationBuffer, role: &SessionRole) -> String {
334:pub(crate) async fn watcher_loop_tick(
3128:async fn watcher_agent_loop(
3602:pub fn watcher_inject(watcher_id: String, message: String) -> Result<()> {
