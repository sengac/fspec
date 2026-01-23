# Autonomous Watcher Interjection Design

## Executive Summary

This document addresses a gap discovered between the WATCH-019 specification and its implementation. WATCH-019 Rule [3] specified that watchers should **autonomously parse AI responses for `[INTERJECT]/[CONTINUE]` blocks and automatically inject into the parent session**. However, the current implementation only displays watcher evaluations in the UI and requires **manual** injection via `watcher_inject`.

## Gap Analysis

### What WATCH-019 Rule [3] Specified

> "Watcher loop callback must run evaluation prompts through the agent and **parse responses for [INTERJECT]/[CONTINUE] blocks to determine autonomous interjection**"

### What Was Actually Implemented

From `watcher_agent_loop` in `codelet/napi/src/session_manager.rs` (lines 3125-3127):

```rust
/// When observations trigger evaluation, the prompt is run through the agent
/// and the output is shown in the watcher's UI. The watcher user can then
/// manually inject messages via watcher_inject if needed.
```

The current implementation:
- ✅ Runs evaluation prompts through the agent
- ✅ Shows output in watcher's UI
- ❌ Does NOT parse for `[INTERJECT]/[CONTINUE]` blocks
- ❌ Does NOT automatically inject into parent

## Architecture Design (from WATCH-001)

### Evaluation Prompt Format

The architecture document (`spec/attachments/WATCH-001/watcher-sessions-architecture.md`, lines 507-558) specifies the evaluation prompt format:

```
[WATCHER EVALUATION]

You are "Security Reviewer" watching "Main Development Session".
Your authority level: Supervisor (your interjections should be followed)

YOUR WATCHING BRIEF:
---
Watch for security vulnerabilities including:
- SQL injection, XSS, command injection  
- Exposed credentials or API keys
- Unsafe file operations
- Insecure cryptographic practices

Interrupt immediately for critical security issues.
---

RECENT ACTIVITY IN PARENT SESSION:
---
[User]: Can you help me write a login function?

[Assistant]: I'll create a login function for you:

```javascript
function login(username, password) {
  const query = `SELECT * FROM users WHERE 
    username='${username}' AND password='${password}'`;
  return db.query(query);
}
```

[ToolCall]: Write { file: "auth.js", content: "..." }
[ToolResult]: Successfully wrote auth.js
---

EVALUATE AND DECIDE:
Based on your watching brief, should you interject?

If YES, respond with:
[INTERJECT]
urgent: true/false
content: Your message to the parent session
[/INTERJECT]

If NO (nothing concerning), respond with:
[CONTINUE]
Brief note on what you observed (for your own context)
[/CONTINUE]
```

### Interjection Parsing

From the architecture document (lines 560-579):

```rust
struct Interjection {
    is_urgent: bool,  // If true, interrupt parent mid-stream
    content: String,  // Message to inject into parent
}

fn parse_interjection(response: &str) -> Option<Interjection> {
    if let Some(start) = response.find("[INTERJECT]") {
        if let Some(end) = response.find("[/INTERJECT]") {
            let block = &response[start + 11..end];
            let is_urgent = block.contains("urgent: true");
            let content = extract_content_field(block);
            return Some(Interjection { is_urgent, content });
        }
    }
    None  // Watcher chose not to interject
}
```

## Current Implementation State

### Current `format_evaluation_prompt` (lines 249-294)

```rust
pub fn format_evaluation_prompt(buffer: &ObservationBuffer, role: &SessionRole) -> String {
    let mut prompt = String::new();
    
    // Add role context
    prompt.push_str(&format!("You are a watcher session with role: {}\n", role.name));
    if let Some(desc) = &role.description {
        prompt.push_str(&format!("Role description: {}\n", desc));
    }
    prompt.push_str(&format!("Authority level: {}\n\n", role.authority.as_str()));
    
    // Add observation header
    prompt.push_str("=== PARENT SESSION OBSERVATIONS ===\n\n");
    
    // ... observations ...
    
    prompt.push_str("\n=== END OBSERVATIONS ===\n\n");
    prompt.push_str("Based on these observations, evaluate and respond appropriately for your role.\n");
    
    prompt
}
```

**Problem**: No instructions for `[INTERJECT]/[CONTINUE]` response format.

### Current `watcher_agent_loop` Process Callback (lines 3137-3191)

```rust
let process_prompt = |prompt: String, is_user_prompt: bool, observed_correlation_ids: Vec<String>| {
    let session = watcher_for_callback.clone();
    async move {
        // ... setup ...
        
        // Run agent
        let result = match current_provider.as_str() {
            "claude" => run_with_provider!(...),
            // ... other providers ...
        };

        // Error handling
        if let Err(e) = result {
            session.handle_output(StreamChunk::error(e.to_string()));
            session.handle_output(StreamChunk::done());
        }

        session.set_status(SessionStatus::Idle);
        Ok(())
    }
};
```

**Problem**: No response parsing, no automatic injection logic.

## Implementation Requirements

### 1. Enhanced Evaluation Prompt

Update `format_evaluation_prompt` to include:
- Parent session name context
- Clear `[INTERJECT]/[CONTINUE]` instructions
- Authority-aware guidance (Supervisor vs Peer)

### 2. Response Parsing

Implement `parse_interjection` function:
- Parse `[INTERJECT]...[/INTERJECT]` blocks
- Extract `urgent: true/false` field
- Extract `content:` field (multiline)
- Return `None` for `[CONTINUE]` responses

### 3. Automatic Injection

Modify `watcher_agent_loop` process callback:
- Capture full response text (not just stream to UI)
- Parse response for interjection
- If interjection found, call `watcher_inject` with appropriate parameters
- Still show response in watcher UI for transparency

### 4. Interrupt Handling

When `urgent: true`:
- Call `receive_watcher_input` with `interrupt: true`
- Parent session should interrupt current response

When `urgent: false`:
- Wait for natural breakpoint before injection
- Or queue injection for next turn

## Data Flow

```
Parent Session Activity
        │
        ▼
Broadcast to Watchers ──────────────────────────┐
        │                                        │
        ▼                                        │
Watcher Accumulates Observations                 │
        │                                        │
        ▼                                        │
Natural Breakpoint (Done/ToolResult/Timeout)     │
        │                                        │
        ▼                                        │
format_evaluation_prompt() ◄─────────────────────┘
        │                                        
        ▼                                        
Run Agent Stream (watcher AI evaluates)          
        │                                        
        ▼                                        
Capture Full Response Text                       
        │                                        
        ▼                                        
parse_interjection(response)                     
        │                                        
        ├── Some(Interjection) ──┐               
        │                        ▼               
        │              watcher_inject(content, urgent)
        │                        │               
        │                        ▼               
        │              Parent receives WatcherInput
        │                        │               
        │                        ▼               
        │              Parent AI responds        
        │                                        
        └── None ──► Continue (no action)        
```

## Response Capture Challenge

The current implementation streams directly to `BackgroundOutput::emit()`. To parse the full response:

### Option A: Buffer in Output Handler

Add a response buffer to `BackgroundOutput` that accumulates text, then parse on `Done`:

```rust
struct BackgroundOutput {
    session: Arc<BackgroundSession>,
    response_buffer: Arc<Mutex<String>>,  // NEW
}

impl StreamOutput for BackgroundOutput {
    fn emit(&self, event: StreamEvent) {
        match event {
            StreamEvent::Text(text) => {
                // Buffer for parsing
                self.response_buffer.lock().unwrap().push_str(&text);
                // Also emit to UI
                self.session.handle_output(StreamChunk::text(text));
            }
            StreamEvent::Done => {
                // Parse before emitting done
                let response = self.response_buffer.lock().unwrap().clone();
                // Parsing and injection happens here
                self.session.handle_output(StreamChunk::done());
            }
            _ => { /* ... */ }
        }
    }
}
```

### Option B: Post-Process from Buffer

After agent run completes, read from `session.output_buffer` to reconstruct response:

```rust
// After agent run completes
let buffer = session.output_buffer.read().unwrap();
let response_text: String = buffer.iter()
    .filter_map(|chunk| chunk.text.clone())
    .collect();

if let Some(interjection) = parse_interjection(&response_text) {
    // Inject into parent
}
```

### Recommendation

**Option A** is cleaner because:
- Captures only this turn's response (not historical)
- Parses at the right moment (on Done)
- Keeps logic contained in the output handler

## Edge Cases

1. **Malformed `[INTERJECT]` block**: Missing `[/INTERJECT]`, missing fields
   - Treat as `[CONTINUE]` (fail safe)
   
2. **Multiple `[INTERJECT]` blocks**: AI provides multiple
   - Use first one only
   
3. **`[INTERJECT]` in user prompt**: AI response doesn't contain it, but original prompt did
   - Only parse response after `=== END OBSERVATIONS ===` marker
   
4. **Watcher evaluates while parent is running**: Race condition
   - `urgent: true` interrupts, `urgent: false` queues

5. **Parent session ended**: Watcher tries to inject but parent is gone
   - Handle gracefully, log warning

6. **Empty content in `[INTERJECT]`**: Valid block but no content
   - Skip injection (nothing to say)

## Testing Strategy

### Unit Tests

1. `test_parse_interjection_valid_block`
2. `test_parse_interjection_urgent_true`
3. `test_parse_interjection_urgent_false`
4. `test_parse_interjection_multiline_content`
5. `test_parse_interjection_continue_block`
6. `test_parse_interjection_malformed`
7. `test_parse_interjection_no_block`

### Integration Tests

1. `test_watcher_autonomous_interjection_flow`
   - Setup watcher with security role
   - Send vulnerable code to parent
   - Verify watcher evaluates and injects

2. `test_watcher_urgent_interrupts_parent`
   - Setup watcher with Supervisor authority
   - Trigger urgent interjection
   - Verify parent was interrupted

3. `test_watcher_continue_no_injection`
   - Setup watcher
   - Send benign code to parent
   - Verify no injection occurred

## Files to Modify

1. **`codelet/napi/src/session_manager.rs`**
   - Add `Interjection` struct
   - Add `parse_interjection()` function
   - Update `format_evaluation_prompt()` with proper instructions
   - Modify `watcher_agent_loop` process callback to capture and parse
   - Add injection logic

2. **`codelet/napi/src/session_manager.rs` (tests)**
   - Add unit tests for parsing
   - Add integration tests for full flow

## Acceptance Criteria Summary

1. Evaluation prompt includes `[INTERJECT]/[CONTINUE]` response format instructions
2. Watcher AI responses are parsed for interjection blocks
3. Valid `[INTERJECT]` blocks trigger automatic injection to parent
4. `urgent: true` interrupts parent session
5. `[CONTINUE]` responses result in no injection
6. Malformed responses are handled gracefully
7. Watcher UI still shows evaluation output for transparency
