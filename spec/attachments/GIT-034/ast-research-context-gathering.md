# AST Research: Context Gathering for GIT-034

## Date: 2026-02-20

## Research Objective

Understand the data flow for environment system reminders and identify where isolation context needs to be injected.

## Key Files Analyzed

### 1. `codelet/cli/src/session/context_gathering.rs`

**EnvironmentInfo struct (lines 19-33):**
```rust
pub struct EnvironmentInfo {
    pub platform: String,
    pub arch: String,
    pub shell: Option<String>,
    pub user: Option<String>,
    pub cwd: Option<String>,
    pub date: String,
}
```

**MISSING:** No fields for isolation state, worktree path, or base commit.

**`to_reminder_content()` method (lines 36-60):**
- Formats info as multi-line string
- Currently outputs: Platform, Architecture, Shell, User, Working directory, Date
- **NEEDS:** Conditional isolation fields

**`gather_environment_info()` function (lines 127-158):**
- Returns `EnvironmentInfo` with OS/env-level data
- **PROBLEM:** No isolation context available here

### 2. `codelet/cli/src/session/mod.rs`

**`inject_context_reminders()` method (lines 248-262):**
```rust
pub fn inject_context_reminders(&mut self) {
    use context_gathering::{discover_claude_md, gather_environment_info};

    // Inject CLAUDE.md/AGENTS.md content if found
    if let Some(content) = discover_claude_md(None) {
        self.add_system_reminder(SystemReminderType::ClaudeMd, &content);
    }

    // Inject environment information
    let env_info = gather_environment_info();
    self.add_system_reminder(
        SystemReminderType::Environment,
        &env_info.to_reminder_content(),
    );
}
```

**PROBLEM:** `gather_environment_info()` has no isolation context

### 3. `codelet/napi/src/session_manager.rs`

**BackgroundSession has isolation info:**
- `worktree_path: Option<PathBuf>` (field)
- `is_isolated() -> bool` (computed from worktree_path.is_some())
- `effective_cwd() -> PathBuf` (returns worktree or project root)

**`clear_history()` calls `inject_context_reminders()` (line 1552):**
```rust
inner.inject_context_reminders();
```

**`get_info()` returns isolation state (lines 1583-1584):**
```rust
is_isolated: self.worktree_path.is_some(),
worktree_path: self.worktree_path.as_ref().map(|p| p.to_string_lossy().to_string()),
```

### 4. `codelet/git/src/isolated_session.rs`

**SessionIsolationInfo struct (lines 75-90):**
```rust
pub struct SessionIsolationInfo {
    pub project_root: PathBuf,
    pub worktree_path: Option<PathBuf>,
}

impl SessionIsolationInfo {
    pub fn effective_cwd(&self) -> PathBuf { ... }
    pub fn is_isolated(&self) -> bool { self.worktree_path.is_some() }
}
```

## Data Flow Diagram

```
BackgroundSession (NAPI)
   │
   ├── worktree_path: Option<PathBuf>  ← HAS isolation info
   ├── base_commit: Option<String>     ← May need to add
   │
   └── inner: Session (CLI)
          │
          └── inject_context_reminders()
                   │
                   └── gather_environment_info()  ← NO isolation info passed!
                              │
                              └── EnvironmentInfo  ← Missing isolation fields
                                       │
                                       └── to_reminder_content()  ← No isolation output
```

## Solution Architecture

### Option A: Pass Isolation Context Through

1. Add optional isolation fields to `EnvironmentInfo`
2. Add `IsolationContext` parameter to `inject_context_reminders()`
3. BackgroundSession passes its isolation state when calling

### Option B: Override at NAPI Level

1. Add `inject_context_reminders_with_isolation()` method
2. NAPI layer builds custom environment reminder with isolation
3. CLI layer unchanged (for backward compatibility)

### Recommended: Option A

More maintainable - single source of truth for environment reminder format.

## Required Changes

### 1. context_gathering.rs

```rust
// Add optional isolation fields
pub struct EnvironmentInfo {
    pub platform: String,
    pub arch: String,
    pub shell: Option<String>,
    pub user: Option<String>,
    pub cwd: Option<String>,
    pub date: String,
    // GIT-034: Isolation context
    pub is_isolated: bool,
    pub worktree_path: Option<String>,
    pub base_commit: Option<String>,
}

impl EnvironmentInfo {
    pub fn to_reminder_content(&self) -> String {
        // ... existing fields ...
        
        // GIT-034: Add isolation fields when isolated
        if self.is_isolated {
            if let Some(ref path) = self.worktree_path {
                lines.push(format!("Isolation: ACTIVE"));
                lines.push(format!("Worktree: {path}"));
            }
            if let Some(ref commit) = self.base_commit {
                lines.push(format!("Base commit: {commit}"));
            }
        }
        
        // Date always last
        lines.push(format!("Date: {}", self.date));
        
        lines.join("\n")
    }
}

// Update gather function to accept optional isolation context
pub fn gather_environment_info_with_isolation(
    isolation: Option<&IsolationContext>
) -> EnvironmentInfo {
    let mut info = gather_environment_info();
    
    if let Some(ctx) = isolation {
        info.is_isolated = ctx.is_isolated;
        info.worktree_path = ctx.worktree_path.clone();
        info.base_commit = ctx.base_commit.clone();
    }
    
    info
}
```

### 2. session/mod.rs

```rust
pub fn inject_context_reminders(&mut self) {
    self.inject_context_reminders_with_isolation(None);
}

pub fn inject_context_reminders_with_isolation(
    &mut self,
    isolation: Option<&IsolationContext>
) {
    // ... discover CLAUDE.md ...
    
    let env_info = gather_environment_info_with_isolation(isolation);
    self.add_system_reminder(
        SystemReminderType::Environment,
        &env_info.to_reminder_content(),
    );
}
```

### 3. session_manager.rs (BackgroundSession)

```rust
pub fn clear_history(&self) {
    // ... clear state ...
    
    // Build isolation context from self
    let isolation = IsolationContext {
        is_isolated: self.worktree_path.is_some(),
        worktree_path: self.worktree_path.as_ref()
            .map(|p| p.strip_prefix(&self.project).ok()
                .map(|rel| rel.to_string_lossy().to_string()))
            .flatten(),
        base_commit: self.get_base_commit_short(),
    };
    
    inner.inject_context_reminders_with_isolation(Some(&isolation));
}
```

## Base Commit Consideration

The `IsolationStateChange` chunk currently includes:
- `is_isolated: bool`
- `worktree_path: Option<String>`

**Missing:** `base_commit`

Need to either:
1. Add `base_commit` to BackgroundSession state
2. Look it up from git when building isolation context
3. Store it when worktree is created

## Testing Strategy

1. Unit test: `EnvironmentInfo::to_reminder_content()` with isolation fields
2. Unit test: `gather_environment_info_with_isolation()` with Some/None
3. Integration test: BackgroundSession isolation reminder injection
4. Snapshot test: Exact format of isolated vs non-isolated reminders
