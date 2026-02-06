# TUI-059: Work Unit Context in Environment Information

## Overview

This document captures the technical details for implementing work unit context tracking in the fspec TUI, enabling:

1. **Environment Information**: When selecting a work unit from the kanban board, add the work unit ID to the environment information (system prompt area)
2. **Status Change Notifications**: When `update-work-unit-status` is executed, detect if the work unit differs from the currently assigned one and notify the LLM
3. **Rust-Zustand State Synchronization**: Ensure the currently assigned work unit is tracked in both Rust and Zustand state

---

## Current Architecture

### State Management Locations

#### 1. Zustand Stores (TypeScript)

**`src/tui/store/fspecStore.ts`**
```typescript
interface FspecState {
  // ... other fields
  currentWorkUnitId: string | null;  // Currently tracked work unit
  sessionAttachments: Map<string, string>;  // workUnitId → sessionId mapping
}
```

**`src/tui/store/sessionStore.ts`**
```typescript
interface SessionStoreState {
  currentSessionId: string | null;
  isReadyForNewSession: boolean;
  shouldAutoCreateSession: boolean;
  navigationTargetSessionId: string | null;
  showCreateSessionDialog: boolean;
}
```

#### 2. Rust State (via NAPI)

**SessionManager** (in codelet-napi):
- Tracks sessions in insertion order (IndexMap)
- Tracks the currently active (attached) session
- Does NOT currently track work unit context

#### 3. React Local State (BoardView)

```typescript
const [selectedWorkUnit, setSelectedWorkUnit] = useState<any>(null);
```

### Existing Patterns (MUST FOLLOW)

The codebase follows clear separation of concerns:

| Layer | Purpose | Examples |
|-------|---------|----------|
| **Services** | Pure async functions for operations | `sessionService.ts` |
| **Hooks** | React integration with external state | `useRustSessionState.ts`, `useSessionNavigation.ts` |
| **Helpers** | Small utility functions | `sessionAttachment.ts` |
| **State Sources** | NAPI abstraction layer | `rustStateSource.ts` |
| **Stores** | Zustand state management | `fspecStore.ts`, `sessionStore.ts` |

---

## SOLID/DRY/COMPOSABLE Design

### New Files (Separation of Concerns)

```
src/tui/
├── services/
│   ├── sessionService.ts           # Existing
│   └── workUnitContextService.ts   # NEW: Work unit context operations
├── hooks/
│   ├── useRustSessionState.ts      # Existing
│   └── useWorkUnitContext.ts       # NEW: React hook for work unit context
├── types/
│   └── workUnitContext.ts          # NEW: Type definitions
└── store/
    └── fspecStore.ts               # Existing (minor additions)
```

### Layer 1: Types (`src/tui/types/workUnitContext.ts`)

```typescript
/**
 * Work Unit Context Types
 * 
 * SOLID: Interface Segregation - only the data needed for context
 */

export interface WorkUnitContext {
  id: string;
  title: string;
  status: string;
  type?: 'story' | 'bug' | 'task';
}

export interface WorkUnitContextChange {
  previous: WorkUnitContext | null;
  current: WorkUnitContext;
  sessionId: string;
}
```

### Layer 2: Service (`src/tui/services/workUnitContextService.ts`)

```typescript
/**
 * Work Unit Context Service
 * 
 * SOLID: Single Responsibility - Only handles work unit context operations
 * DRY: All work unit context logic in one place
 * COMPOSABLE: Pure functions, no React dependencies
 */

import {
  sessionSetWorkUnitContext,
  sessionGetWorkUnitContext,
  sessionGetActive,
} from '@sengac/codelet-napi';
import type { WorkUnitContext, WorkUnitContextChange } from '../types/workUnitContext';
import { logger } from '../../utils/logger';

/**
 * Set work unit context for a session
 */
export function setWorkUnitContext(
  sessionId: string,
  context: WorkUnitContext | null
): void {
  logger.debug(`[WorkUnitContext] Setting context for session ${sessionId}:`, context);
  
  if (context) {
    sessionSetWorkUnitContext(sessionId, context.id, context.title, context.status);
  } else {
    sessionSetWorkUnitContext(sessionId, null, null, null);
  }
}

/**
 * Get work unit context for a session
 */
export function getWorkUnitContext(sessionId: string): WorkUnitContext | null {
  const rustContext = sessionGetWorkUnitContext(sessionId);
  
  if (!rustContext) {
    return null;
  }
  
  return {
    id: rustContext.id,
    title: rustContext.title,
    status: rustContext.status,
  };
}

/**
 * Get the currently active session's work unit context
 */
export function getActiveWorkUnitContext(): WorkUnitContext | null {
  const activeSessionId = sessionGetActive();
  
  if (!activeSessionId) {
    return null;
  }
  
  return getWorkUnitContext(activeSessionId);
}

/**
 * Detect if work unit context has changed
 * Returns change details if different, null if same
 */
export function detectWorkUnitChange(
  sessionId: string,
  newWorkUnitId: string,
  newWorkUnit: { title: string; status: string }
): WorkUnitContextChange | null {
  const currentContext = getWorkUnitContext(sessionId);
  
  // No change if same work unit
  if (currentContext?.id === newWorkUnitId) {
    return null;
  }
  
  return {
    previous: currentContext,
    current: {
      id: newWorkUnitId,
      title: newWorkUnit.title,
      status: newWorkUnit.status,
    },
    sessionId,
  };
}

/**
 * Format work unit change as system reminder
 */
export function formatWorkUnitChangeReminder(change: WorkUnitContextChange): string {
  if (change.previous) {
    return (
      `Work unit context changed:\n` +
      `  Previous: ${change.previous.id} (${change.previous.title})\n` +
      `  Current: ${change.current.id} (${change.current.title})\n\n` +
      `You are now working on ${change.current.id}.`
    );
  }
  
  return (
    `Work unit context set:\n` +
    `  Current: ${change.current.id} (${change.current.title})\n\n` +
    `You are now working on ${change.current.id}.`
  );
}
```

### Layer 3: React Hook (`src/tui/hooks/useWorkUnitContext.ts`)

```typescript
/**
 * useWorkUnitContext - React hook for work unit context
 * 
 * SOLID: Single Responsibility - Only React integration for work unit context
 * DRY: Delegates to service layer, no duplicated logic
 * COMPOSABLE: Can be composed with other hooks
 */

import { useCallback, useEffect } from 'react';
import { useFspecStore } from '../store/fspecStore';
import {
  setWorkUnitContext,
  getWorkUnitContext,
  detectWorkUnitChange,
} from '../services/workUnitContextService';
import type { WorkUnitContext } from '../types/workUnitContext';

interface UseWorkUnitContextOptions {
  sessionId: string | null;
  workUnitId?: string;
}

interface UseWorkUnitContextResult {
  currentContext: WorkUnitContext | null;
  setContext: (context: WorkUnitContext | null) => void;
  syncWithSession: () => void;
}

export function useWorkUnitContext(
  options: UseWorkUnitContextOptions
): UseWorkUnitContextResult {
  const { sessionId, workUnitId } = options;
  
  // Zustand state
  const workUnits = useFspecStore(state => state.workUnits);
  const setCurrentWorkUnitId = useFspecStore(state => state.setCurrentWorkUnitId);
  
  // Get current context from Rust
  const currentContext = sessionId ? getWorkUnitContext(sessionId) : null;
  
  // Set context (updates both Rust and Zustand)
  const setContext = useCallback(
    (context: WorkUnitContext | null) => {
      if (!sessionId) {
        return;
      }
      
      // Update Rust state
      setWorkUnitContext(sessionId, context);
      
      // Update Zustand state
      setCurrentWorkUnitId(context?.id ?? null);
    },
    [sessionId, setCurrentWorkUnitId]
  );
  
  // Sync work unit ID prop with session context
  const syncWithSession = useCallback(() => {
    if (!sessionId || !workUnitId) {
      return;
    }
    
    const workUnit = workUnits.find(wu => wu.id === workUnitId);
    if (!workUnit) {
      return;
    }
    
    setContext({
      id: workUnit.id,
      title: workUnit.title,
      status: workUnit.status,
    });
  }, [sessionId, workUnitId, workUnits, setContext]);
  
  // Auto-sync on mount when workUnitId is provided
  useEffect(() => {
    if (workUnitId && sessionId) {
      syncWithSession();
    }
  }, [workUnitId, sessionId, syncWithSession]);
  
  return {
    currentContext,
    setContext,
    syncWithSession,
  };
}
```

### Layer 4: Rust State Source Extension (`src/tui/hooks/rustStateSource.ts`)

Add work unit context to the existing abstraction:

```typescript
// Add to existing RustStateSource interface
export interface RustStateSource {
  // ... existing methods
  
  // NEW: Work unit context
  getWorkUnitContext: (sessionId: string) => WorkUnitContext | null;
  setWorkUnitContext: (
    sessionId: string,
    id: string | null,
    title: string | null,
    status: string | null
  ) => void;
}
```

### Layer 5: Integration Points (Minimal Changes)

#### AgentView.tsx - Use Hook (NOT direct NAPI calls)

```typescript
// Instead of adding logic directly, just use the hook:
const { syncWithSession } = useWorkUnitContext({
  sessionId: currentSessionId,
  workUnitId,
});

// Call syncWithSession when session is created/resumed
// The hook handles all the details
```

#### update-work-unit-status.ts - Use Service (NOT direct NAPI calls)

Create a separate integration file to avoid modifying the command directly:

**NEW: `src/commands/hooks/workUnitStatusHook.ts`**
```typescript
/**
 * Work Unit Status Hook
 * 
 * SOLID: Open/Closed - Extends status update without modifying original
 * DRY: Reuses workUnitContextService
 */

import {
  getActiveWorkUnitContext,
  detectWorkUnitChange,
  formatWorkUnitChangeReminder,
  setWorkUnitContext,
} from '../../tui/services/workUnitContextService';
import { sessionGetActive } from '@sengac/codelet-napi';
import { wrapInSystemReminder } from '../../utils/system-reminder';

export interface WorkUnitStatusHookResult {
  systemReminder: string | null;
}

/**
 * Called after status update to handle work unit context changes
 */
export async function onWorkUnitStatusUpdated(
  workUnitId: string,
  newStatus: string,
  workUnitTitle: string
): Promise<WorkUnitStatusHookResult> {
  const activeSessionId = sessionGetActive();
  
  if (!activeSessionId) {
    // No active session, nothing to do
    return { systemReminder: null };
  }
  
  const change = detectWorkUnitChange(activeSessionId, workUnitId, {
    title: workUnitTitle,
    status: newStatus,
  });
  
  if (!change) {
    // Same work unit, just update status in context
    setWorkUnitContext(activeSessionId, {
      id: workUnitId,
      title: workUnitTitle,
      status: newStatus,
    });
    return { systemReminder: null };
  }
  
  // Work unit changed - update context and generate reminder
  setWorkUnitContext(activeSessionId, change.current);
  
  const reminderText = formatWorkUnitChangeReminder(change);
  return {
    systemReminder: wrapInSystemReminder(reminderText),
  };
}
```

---

## Rust Changes (codelet-napi)

### New Module: `src/work_unit_context.rs`

```rust
/// Work Unit Context Module
/// 
/// SOLID: Single Responsibility - Only work unit context for sessions
/// Separated from session.rs to maintain SRP

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkUnitContext {
    pub id: Option<String>,
    pub title: Option<String>,
    pub status: Option<String>,
}

impl WorkUnitContext {
    pub fn new(id: String, title: String, status: String) -> Self {
        Self {
            id: Some(id),
            title: Some(title),
            status: Some(status),
        }
    }
    
    pub fn clear(&mut self) {
        self.id = None;
        self.title = None;
        self.status = None;
    }
    
    pub fn is_set(&self) -> bool {
        self.id.is_some()
    }
    
    /// Format for environment information (ID only)
    pub fn format_for_environment(&self) -> Option<String> {
        self.id.as_ref().map(|id| format!("Current work unit: {}", id))
    }
}
```

### Session.rs - Composition (NOT Modification)

```rust
// Add work unit context as a composed field
use crate::work_unit_context::WorkUnitContext;

pub struct Session {
    // ... existing fields
    work_unit_context: WorkUnitContext,  // Composed, not mixed in
}

impl Session {
    // Delegate to composed module
    pub fn set_work_unit_context(&mut self, id: String, title: String, status: String) {
        self.work_unit_context = WorkUnitContext::new(id, title, status);
    }
    
    pub fn get_work_unit_context(&self) -> &WorkUnitContext {
        &self.work_unit_context
    }
    
    pub fn clear_work_unit_context(&mut self) {
        self.work_unit_context.clear();
    }
}
```

### Environment Generation - Composition

```rust
// In environment info generation
fn generate_environment_info(&self, session: &Session) -> String {
    let mut lines = vec![
        format!("Platform: {}", self.platform),
        format!("Architecture: {}", self.arch),
        format!("Shell: {}", self.shell),
        format!("User: {}", self.user),
        format!("Working directory: {}", self.cwd),
    ];
    
    // Compose work unit context if set
    if let Some(wu_info) = session.get_work_unit_context().format_for_environment() {
        lines.push(wu_info);
    }
    
    lines.join("\n")
}
```

---

## File Structure Summary

### New Files (TypeScript)
```
src/tui/types/workUnitContext.ts          # Type definitions
src/tui/services/workUnitContextService.ts # Service layer (pure functions)
src/tui/hooks/useWorkUnitContext.ts        # React hook
src/commands/hooks/workUnitStatusHook.ts   # Status update integration
```

### New Files (Rust)
```
src/work_unit_context.rs                   # Work unit context module
```

### Modified Files (Minimal)
```
src/tui/hooks/rustStateSource.ts           # Add work unit context to interface
src/tui/components/AgentView.tsx           # Use useWorkUnitContext hook (1 line)
src/commands/update-work-unit-status.ts    # Call onWorkUnitStatusUpdated (1 line)
codelet-napi/src/session.rs                # Add composed WorkUnitContext field
codelet-napi/src/lib.rs                    # Register NAPI functions
```

---

## Comparison: Original vs SOLID Design

| Aspect | Original Design | SOLID Design |
|--------|-----------------|--------------|
| **AgentView changes** | Add logic directly (bloats 60k file) | Use hook (1 line) |
| **Command changes** | Mix session awareness into command | Call hook function (1 line) |
| **NAPI calls** | Scattered across files | Centralized in service |
| **Testability** | Hard (coupled to React/commands) | Easy (pure functions) |
| **Reusability** | Low (embedded in specific files) | High (composable modules) |
| **Rust changes** | Add fields to Session directly | Compose WorkUnitContext module |

---

## Testing Strategy

### Unit Tests (Each Layer)

```
src/tui/services/__tests__/workUnitContextService.test.ts
src/tui/hooks/__tests__/useWorkUnitContext.test.ts
src/commands/hooks/__tests__/workUnitStatusHook.test.ts
```

### Integration Tests

```
src/tui/__tests__/work-unit-context-integration.test.tsx
```

### Rust Tests

```
codelet-napi/src/work_unit_context.rs  # #[cfg(test)] module
```

---

## Open Questions

1. Should the work unit context persist across session compaction?
2. Should we add a `/workunit` slash command to display/change the current work unit?
3. How should we handle watchers - do they inherit parent's work unit context?
4. Should work unit context be stored in session persistence (manifest)?
