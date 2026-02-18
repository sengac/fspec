# BLOCK-007: Blocklist Prompt Integration with Tool Pause System

## Problem Statement

The blocklist system has three action types: `block`, `allow`, and `prompt`. Currently:
- `block` actions work correctly - they block the operation
- `allow` actions work correctly - they explicitly permit the operation  
- `prompt` actions are **broken** - they currently behave the same as `block`

The issue is in `codelet/tools/src/blocklist/middleware.rs` lines 124 and 150:
```rust
// Block if either blocked OR prompt (since we can't prompt in tool context)
if !result.allowed {
    return Err(BlockedError { ... });
}
```

The comment explicitly says "since we can't prompt in tool context" - but we CAN, using the existing tool pause mechanism!

## Existing Infrastructure

### Tool Pause System (PAUSE-001)

The tool pause system already exists and works. It supports:
- `PauseKind::Continue` - Press Enter to continue
- `PauseKind::Confirm` - Press Y/N to approve/deny

Flow:
1. Tool calls `pause_for_user(PauseRequest { kind, tool_name, message, details })`
2. Global handler sets session status to Paused
3. Handler blocks on `session.wait_for_pause_response()` (mpsc channel)
4. TUI shows appropriate UI based on `pauseInfo.kind`
5. User presses key → TUI calls NAPI function
6. NAPI sends response → unblocks tool

### Session Allowances (BLOCK-005)

Session allowance infrastructure already exists:
- `SESSION_ALLOWANCES: LazyLock<RwLock<HashSet<String>>>` in middleware.rs
- `allow_for_session(pattern)` - adds pattern to session allowances
- `is_session_allowed(pattern)` - checks if pattern is allowed
- `clear_session_allowances()` - clears on TUI restart
- NAPI bindings: `blocklist_allow_session()`, `blocklist_is_session_allowed()`, `blocklist_clear_session_allowances()`

### Inline Pause UI in InputTransition

The TUI already handles pause states inline in `InputTransition.tsx` (lines 286-320):
- **Continue pause**: Shows `⏸ {toolName}: {message} (Press Enter to continue)`
- **Confirm pause**: Shows `⏸ {toolName}: {message}` with `[Y] Approve [N] Deny (Esc to cancel)`

We will add a third case for `triple` pause in the same inline style.

## Implementation Plan

### 1. Rust: Add Triple Pause Kind

**File:** `codelet/tools/src/tool_pause.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseKind {
    Continue,
    Confirm,
    Triple,  // NEW: For blocklist prompts
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseResponse {
    Resumed,
    Approved,
    Denied,
    Interrupted,
    AllowOnce,    // NEW: For triple mode
    AllowSession, // NEW: For triple mode
}
```

### 2. Rust: Modify Blocklist Middleware

**File:** `codelet/tools/src/blocklist/middleware.rs`

Replace the current check functions with prompt-aware versions:

```rust
pub fn check_file_path(file_path: &str) -> Result<(), BlockedError> {
    let config = load_blocklist_config(None);
    if config.rules.is_empty() {
        return Ok(());
    }
    
    let matcher = BlocklistMatcher::new(config);
    let result = matcher.check_command(file_path);
    
    // Hard block
    if result.blocked {
        return Err(BlockedError { ... });
    }
    
    // Prompt (not blocked, not allowed)
    if !result.allowed {
        let pattern = result.matched_rule_id.clone().unwrap_or_default();
        
        // Check session allowances first
        if is_session_allowed(&pattern) {
            return Ok(());
        }
        
        // Pause for user decision
        use crate::tool_pause::{pause_for_user, PauseKind, PauseRequest, PauseResponse};
        let response = pause_for_user(PauseRequest {
            kind: PauseKind::Triple,
            tool_name: "Read".to_string(),
            message: result.reason.unwrap_or_else(|| "Sensitive file access".to_string()),
            details: Some(file_path.to_string()),
        });
        
        match response {
            PauseResponse::AllowOnce => Ok(()),
            PauseResponse::AllowSession => {
                allow_for_session(&pattern);
                Ok(())
            }
            PauseResponse::Denied | PauseResponse::Interrupted => {
                Err(BlockedError {
                    reason: "User denied access".to_string(),
                    guidance: result.guidance,
                    rule_id: pattern,
                })
            }
            _ => Ok(()), // Default to allow for unexpected responses
        }
    } else {
        Ok(())
    }
}
```

### 3. NAPI: Add Triple Pause Response Handler

**File:** `codelet/napi/src/session_manager.rs`

```rust
/// Handle triple pause choice (BLOCK-007)
/// Called when user selects Allow Once / Allow Session / Deny
#[napi]
pub fn session_pause_triple(session_id: String, choice: String) -> Result<()> {
    let session = SessionManager::instance().get_session(&session_id)?;
    
    let response = match choice.as_str() {
        "allowOnce" => PauseResponse::AllowOnce,
        "allowSession" => PauseResponse::AllowSession,
        "deny" => PauseResponse::Denied,
        _ => PauseResponse::Denied,
    };
    
    session.send_pause_response(response);
    Ok(())
}
```

### 4. TypeScript: Add Triple to PauseKind

**File:** `src/tui/types/pause.ts`

```typescript
export type PauseKind = 'continue' | 'confirm' | 'triple';

export function isValidPauseKind(kind: string): kind is PauseKind {
  return kind === 'continue' || kind === 'confirm' || kind === 'triple';
}
```

### 5. TypeScript: Modify InputTransition for Triple Mode

**File:** `src/tui/components/InputTransition.tsx`

Add a third case for `pauseInfo.kind === 'triple'` that shows inline UI (NOT a dialog):
- Tool name and message (like confirm mode)
- Three options: [Allow Once] [Allow Session] [Deny]
- Visual highlighting for selected option
- ←/→ navigation instructions

```tsx
// Show pause indicator when paused
if (isPaused && pauseInfo) {
  if (pauseInfo.kind === 'triple') {
    // Triple pause: show inline with three options
    return (
      <Text>
        <Text color="yellow">⏸ {pauseInfo.toolName}</Text>
        <Text>: </Text>
        <Text color="yellow">{pauseInfo.message}</Text>
        {pauseInfo.details && (
          <Text>
            {'\n'}
            <Text dimColor>  {pauseInfo.details}</Text>
          </Text>
        )}
        <Text>
          {'\n'}
          {/* Selection state managed by AgentView via prop */}
          <Text backgroundColor={tripleSelection === 'allowOnce' ? 'green' : undefined} 
                color={tripleSelection === 'allowOnce' ? 'white' : 'gray'}>
            [Allow Once]
          </Text>
          <Text> </Text>
          <Text backgroundColor={tripleSelection === 'allowSession' ? 'blue' : undefined}
                color={tripleSelection === 'allowSession' ? 'white' : 'gray'}>
            [Allow Session]
          </Text>
          <Text> </Text>
          <Text backgroundColor={tripleSelection === 'deny' ? 'red' : undefined}
                color={tripleSelection === 'deny' ? 'white' : 'gray'}>
            [Deny]
          </Text>
          <Text dimColor> (←/→ Navigate | Enter Select | Esc Deny)</Text>
        </Text>
      </Text>
    );
  } else if (pauseInfo.kind === 'confirm') {
    // ... existing confirm UI
  } else {
    // ... existing continue UI
  }
}
```

### 6. TypeScript: Add Triple Handler to AgentView

**File:** `src/tui/components/AgentView.tsx`

Add state for triple selection and extend the pause keyboard handler:

```typescript
// State for triple pause selection
const [tripleSelection, setTripleSelection] = useState<'allowOnce' | 'allowSession' | 'deny'>('allowOnce');

// Handle Triple pause (←/→ to navigate, Enter to select)
if (displayPauseInfo.kind === 'triple') {
  const options = ['allowOnce', 'allowSession', 'deny'] as const;
  const currentIndex = options.indexOf(tripleSelection);
  
  if (key.leftArrow) {
    const newIndex = currentIndex <= 0 ? options.length - 1 : currentIndex - 1;
    setTripleSelection(options[newIndex]);
    return true;
  } else if (key.rightArrow) {
    const newIndex = currentIndex >= options.length - 1 ? 0 : currentIndex + 1;
    setTripleSelection(options[newIndex]);
    return true;
  } else if (key.return) {
    try {
      sessionPauseTriple(currentSessionId, tripleSelection);
    } catch (e) {
      logger.error('[BLOCK-007] Error sending triple pause response:', e);
    }
    setTripleSelection('allowOnce'); // Reset for next time
    return true;
  } else if (key.escape) {
    try {
      sessionPauseTriple(currentSessionId, 'deny');
    } catch (e) {
      logger.error('[BLOCK-007] Error sending triple pause deny:', e);
    }
    setTripleSelection('allowOnce'); // Reset for next time
    return true;
  }
}
```

## File Changes Summary

| File | Action | Description |
|------|--------|-------------|
| `codelet/tools/src/tool_pause.rs` | MODIFY | Add `PauseKind::Triple`, `PauseResponse::AllowOnce`, `AllowSession` |
| `codelet/tools/src/blocklist/middleware.rs` | MODIFY | Implement prompt-aware check functions |
| `codelet/napi/src/session_manager.rs` | MODIFY | Add `session_pause_triple()` NAPI binding |
| `codelet/napi/index.d.ts` | AUTO | Generated TypeScript types |
| `src/tui/types/pause.ts` | MODIFY | Add 'triple' to PauseKind |
| `src/tui/components/InputTransition.tsx` | MODIFY | Handle triple pause UI inline |
| `src/tui/components/AgentView.tsx` | MODIFY | Handle triple pause keyboard + state |

## Testing Strategy

1. **Unit Tests:**
   - Test `PauseKind::Triple` and new responses in `tool_pause.rs`
   - Test session allowance check before pausing
   - Test each response path in middleware

2. **Integration Tests:**
   - Test end-to-end flow: Read tool → prompt → user choice → result
   - Test session allowance memory across operations
   - Test TUI restart clears allowances

3. **Manual Testing:**
   - Read `.env` file with prompt rule
   - Verify inline triple UI appears (not a dialog)
   - Test ←/→ navigation between options
   - Test each selection: Allow Once, Allow Session, Deny
   - Verify session allowances work on subsequent access
