# GIT-034: AI System Reminder Isolation Gap Analysis

## Date: 2026-02-19

## Problem Discovery

When investigating the git worktrees architecture (GIT-018), we discovered that the AI agent has no explicit awareness when running in an isolated session with a git worktree.

### Current State

| Layer | Isolation Aware? | Details |
|-------|-----------------|---------|
| **UI** | ✅ Yes | Shows `[ISOLATED]` badge in green on SessionHeader |
| **Tools** | ✅ Yes | Paths resolve via `effective_cwd()` callback |
| **AI Context** | ❌ No | No system reminder mentions isolation |

### What the AI Sees vs. Reality

```
AI's View:                          Reality:
─────────────────────────────────   ─────────────────────────────────
Working directory: /project/        Working directory: /project/.fspec/worktrees/abc123/
                                    
Read("src/auth.ts")                 Read("/project/.fspec/worktrees/abc123/src/auth.ts")
  → sees file contents                → resolved silently by effective_cwd
                                    
Write("src/auth.ts", content)       Write("/project/.fspec/worktrees/abc123/src/auth.ts", content)
  → thinks it modified project        → actually modified worktree only
```

### Impact

| Scenario | Problem |
|----------|---------|
| User asks "where are my changes?" | AI can't explain they're in a worktree |
| AI completes task | AI can't advise user to merge/discard |
| User asks about file location | AI gives wrong absolute path |
| Debugging file not found | AI can't suggest checking worktree vs main |

## Current Environment System Reminder

From `src/utils/system-reminder.ts`:

```typescript
export function buildEnvironmentReminder(): string {
  const platform = os.platform();
  const arch = os.arch();
  const shell = process.env.SHELL || 'unknown';
  const user = os.userInfo().username;
  const cwd = process.cwd();
  const date = new Date().toISOString().split('T')[0];
  
  return `<system-reminder>
<!-- type:environment -->
Platform: ${platform}
Architecture: ${arch}
Shell: ${shell}
User: ${user}
Working directory: ${cwd}
Date: ${date}
</system-reminder>`;
}
```

**Problem:** No session context passed, so isolation state unknown.

## Solution Architecture

### Proposed Environment Reminder Format

**Non-isolated session:**
```xml
<system-reminder>
<!-- type:environment -->
Platform: macos
Architecture: aarch64
Shell: /bin/zsh
User: rquast
Working directory: /Users/rquast/projects/fspec
Date: 2026-02-19
</system-reminder>
```

**Isolated session:**
```xml
<system-reminder>
<!-- type:environment -->
Platform: macos
Architecture: aarch64
Shell: /bin/zsh
User: rquast
Working directory: /Users/rquast/projects/fspec
Isolation: ACTIVE
Worktree: .fspec/worktrees/abc-123-def/
Base commit: 7a8b9c0d
Date: 2026-02-19
</system-reminder>
```

### Required Changes

1. **Extend `buildEnvironmentReminder` signature**

```typescript
// src/utils/system-reminder.ts

export interface IsolationContext {
  isIsolated: boolean;
  worktreePath?: string;   // Relative path like ".fspec/worktrees/abc123/"
  baseCommit?: string;     // Short SHA like "7a8b9c0d"
}

export function buildEnvironmentReminder(
  isolation?: IsolationContext
): string {
  // ... existing code ...
  
  let reminder = `<system-reminder>
<!-- type:environment -->
Platform: ${platform}
Architecture: ${arch}
Shell: ${shell}
User: ${user}
Working directory: ${cwd}`;

  if (isolation?.isIsolated) {
    reminder += `
Isolation: ACTIVE
Worktree: ${isolation.worktreePath}
Base commit: ${isolation.baseCommit}`;
  }

  reminder += `
Date: ${date}
</system-reminder>`;

  return reminder;
}
```

2. **Get isolation state from session**

The `IsolationStateChange` chunk already carries this data:

```rust
// codelet/napi/src/types.rs
IsolationStateChange {
    is_isolated: bool,
    worktree_path: Option<String>,
}
```

Need to also include `base_commit` in this chunk, or fetch it separately.

3. **Pass isolation context when building system reminder**

The system reminder is built/injected at various points. Need to trace where and pass isolation context:

```typescript
// Likely in session initialization or message sending
const isolationContext: IsolationContext = {
  isIsolated: sessionState.isIsolated,
  worktreePath: sessionState.worktreePath,
  baseCommit: sessionState.baseCommit,
};

const envReminder = buildEnvironmentReminder(isolationContext);
```

## Data Flow Investigation

Where does environment reminder get injected?

```
Session Created
       │
       ▼
IsolationStateChange chunk emitted (has isIsolated, worktreePath)
       │
       ▼
AgentView receives chunk, updates state
       │
       ▼
??? Where is buildEnvironmentReminder called?
       │
       ▼
System prompt assembled with environment context
```

### Files to Investigate

| File | Purpose |
|------|---------|
| `src/utils/system-reminder.ts` | Builds environment reminder |
| `src/utils/activationMessage.ts` | May inject environment context |
| `codelet/napi/src/session_manager.rs` | Emits IsolationStateChange |
| `src/tui/components/AgentView.tsx` | Receives chunks, manages state |

## Additional Considerations

### Base Commit Display

Should show short SHA (7-8 chars) for readability:

```typescript
const shortSha = baseCommit?.substring(0, 8);
```

### Worktree Path Display

Show relative path from project root:

```typescript
// Full: /Users/rquast/projects/fspec/.fspec/worktrees/abc123/
// Display: .fspec/worktrees/abc123/
const relativePath = worktreePath.replace(projectRoot, '').replace(/^\//, '');
```

### AI Behavioral Guidance

Consider adding guidance to fspec workflow system-reminder:

```markdown
## Isolated Session Behavior

When `Isolation: ACTIVE` appears in environment:
- All file changes are made in the worktree, not the main project
- Changes require explicit merge to apply to main project
- User can discard changes without affecting main project
- Use relative paths in explanations (e.g., `src/auth.ts` not full worktree path)
```

## Testing Strategy

1. **Unit test**: `buildEnvironmentReminder` with isolation context
2. **Integration test**: Create isolated session, verify environment reminder includes isolation fields
3. **Snapshot test**: Verify exact format of isolated vs non-isolated reminders

## Estimate

**3 points** - Clear scope, need to trace reminder injection point, straightforward string building.

## Dependencies

- GIT-018 (done): Worktree infrastructure
- May need to extend `IsolationStateChange` chunk to include `baseCommit`
