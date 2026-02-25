# BUG-095: System Reminder Work Unit Leak Analysis

## Problem Summary

The environment system reminder incorrectly includes a "Current work unit" field from a **different background session**, not the current session. This causes the AI assistant to assume it's working on a card that doesn't belong to its session context.

## Observed Behavior

**Session A (background):** Working on TUI-072
**Session B (foreground/new):** No attached work unit

The system reminder for Session B incorrectly showed:
```
<system-reminder>
<!-- type:environment -->
Platform: macos
Architecture: aarch64
Shell: /bin/zsh
User: rquast
Working directory: /Users/rquast/projects/fspec
Date: 2026-02-24
Current work unit: TUI-072        <-- WRONG! This belongs to Session A
This supersedes earlier environment reminder
</system-reminder>
```

## Expected Behavior

Session B's system reminder should either:
1. Not include "Current work unit" at all (if no work unit attached)
2. Show "Current work unit: (none)" explicitly
3. Only show a work unit if Session B has one attached

## Impact

- AI assistant incorrectly assumes context from another session
- Confusion about which card is being worked on
- Potential for AI to make changes to wrong work unit
- User has to correct the AI's assumptions

## Root Cause Investigation

### Where to Look

1. **System reminder generation code**
   - How is the environment reminder constructed?
   - Where does it get the "Current work unit" value?

2. **Session state management**
   - How are sessions tracked?
   - Is there shared state between sessions that shouldn't be shared?

3. **Work unit attachment logic**
   - Where is the session-to-work-unit mapping stored?
   - Is it per-session or global?

### Key Questions

1. Is there a global "current work unit" being set somewhere?
2. Is the system reminder generator not session-aware?
3. Is the work unit being cached/persisted incorrectly?

## Files to Investigate

Based on the codebase structure, likely areas:

```
src/
├── session/           # Session management
├── tui/              # TUI components that may set work unit
├── commands/         # Work unit attachment commands
└── utils/            # System reminder generation?
```

Also check the Rust side if system reminders are generated there:
```
codelet-napi/src/
├── session.rs
└── ...
```

## Reproduction Steps

1. Start the TUI with `fspec` or similar
2. Create/attach a work unit to a background session (e.g., TUI-072)
3. Start a new foreground session WITHOUT attaching a work unit
4. Observe that the new session's system reminder incorrectly shows the background session's work unit

## Fix Requirements

1. System reminder must be session-scoped
2. Each session should only see its own attached work unit
3. No cross-session leakage of work unit context
4. If no work unit attached to current session, don't show one (or show "none")
