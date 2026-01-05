# TUI-040: Delete Session from Resume View - Research Findings

## Overview

This document contains research findings for implementing a "Delete Session" feature in the `/resume` view with a confirmation dialog that allows:
1. Delete this session (single)
2. Delete ALL sessions (bulk)
3. Cancel

## Existing Confirmation Dialog Components

### 1. Base Dialog Component (`src/components/Dialog.tsx`)

A base modal overlay component providing infrastructure only:

```typescript
export interface DialogProps {
  children: ReactNode;
  onClose: () => void;
  borderColor?: string;
  isActive?: boolean;
}
```

**Responsibilities:**
- Centered modal overlay rendering
- Border styling with optional color
- ESC key handling to call `onClose`
- Input capture control via `isActive` prop

**Does NOT handle:**
- Business logic (confirmation, forms, etc.)
- Content-specific keyboard interactions
- Callbacks other than `onClose`

### 2. ConfirmationDialog Component (`src/components/ConfirmationDialog.tsx`)

A confirmation-specific component wrapping `Dialog`:

```typescript
type ConfirmMode = 'yesno' | 'typed' | 'keypress';
type RiskLevel = 'low' | 'medium' | 'high';

export interface ConfirmationDialogProps {
  message: string;
  onConfirm: () => void;
  onCancel: () => void;
  confirmMode?: ConfirmMode;
  typedPhrase?: string;
  riskLevel?: RiskLevel;
  description?: string;
}
```

**Risk level mapping:**
- `low` → green border
- `medium` → yellow border  
- `high` → red border

**Confirmation modes:**
- `yesno` - Y to confirm, N to cancel
- `typed` - Requires typing exact phrase (e.g., "DELETE ALL")
- `keypress` - Any key to confirm

**Current limitation:** Only supports Y/N binary confirmation, not 3-button dialogs.

### 3. StatusDialog Component (`src/components/StatusDialog.tsx`)

Used for progress/status display (not applicable for our use case).

## Existing Delete Patterns

### Checkpoint Deletion in CheckpointViewer (`src/tui/components/CheckpointViewer.tsx`)

The CheckpointViewer implements a similar deletion pattern:

**Key bindings:**
- `D` - Delete single checkpoint (confirmMode='yesno', riskLevel='medium')
- `A` - Delete ALL checkpoints (confirmMode='typed', typedPhrase='DELETE ALL', riskLevel='high')

**Implementation pattern:**
```typescript
// State
const [showDeleteDialog, setShowDeleteDialog] = useState(false);
const [deleteMode, setDeleteMode] = useState<'single' | 'all'>('single');

// Key handler
if (input === 'd' || input === 'D') {
  if (sortedCheckpoints.length > 0) {
    setDeleteMode('single');
    setShowDeleteDialog(true);
  }
  return;
}

if (input === 'a' || input === 'A') {
  if (sortedCheckpoints.length > 0) {
    setDeleteMode('all');
    setShowDeleteDialog(true);
  }
  return;
}

// Dialog rendering
{showDeleteDialog && sortedCheckpoints.length > 0 && (
  <ConfirmationDialog
    message={deleteMode === 'single' ? `Delete checkpoint '${name}'?` : `Delete ALL ${count} checkpoints?`}
    description={deleteMode === 'all' ? 'This cannot be undone.' : undefined}
    confirmMode={deleteMode === 'single' ? 'yesno' : 'typed'}
    typedPhrase={deleteMode === 'all' ? 'DELETE ALL' : undefined}
    riskLevel={deleteMode === 'single' ? 'medium' : 'high'}
    onConfirm={handleDeleteConfirm}
    onCancel={handleDeleteCancel}
    isActive={true}
  />
)}
```

## Resume Mode Implementation in AgentView

### Current State Variables
```typescript
const [isResumeMode, setIsResumeMode] = useState(false);
const [availableSessions, setAvailableSessions] = useState<SessionManifest[]>([]);
const [resumeSessionIndex, setResumeSessionIndex] = useState(0);
const [resumeScrollOffset, setResumeScrollOffset] = useState(0);
```

### Current Keyboard Handling (lines 3325-3346)
```typescript
// NAPI-003: Resume mode keyboard handling
if (isResumeMode) {
  if (key.escape) {
    handleResumeCancel();
    return;
  }
  if (key.return) {
    void handleResumeSelect();
    return;
  }
  if (key.upArrow) {
    setResumeSessionIndex(prev => Math.max(0, prev - 1));
    return;
  }
  if (key.downArrow) {
    setResumeSessionIndex(prev => Math.min(availableSessions.length - 1, prev + 1));
    return;
  }
  // No text input in resume mode - just navigation
  return;
}
```

### Current Footer (line 4440)
```typescript
<Text dimColor>Enter Select | ↑↓ Navigate | Esc Cancel</Text>
```

## Session Persistence Backend (codelet-napi)

### Available NAPI Functions

**From `codelet/napi/src/persistence/napi_bindings.rs`:**

```rust
// Delete a single session
#[napi]
pub fn persistence_delete_session(id: String) -> Result<()> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|e| Error::from_reason(e.to_string()))?;
    delete_session(uuid).map_err(Error::from_reason)
}

// List all sessions for a project
#[napi]
pub fn persistence_list_sessions(project: String) -> Result<Vec<NapiSessionManifest>> {
    list_sessions(&PathBuf::from(project))
        .map(|sessions| sessions.into_iter().map(|s| s.into()).collect())
        .map_err(Error::from_reason)
}
```

**From `codelet/napi/src/persistence/mod.rs`:**

```rust
/// Delete a session
pub fn delete_session(id: Uuid) -> Result<(), String> {
    init_stores()?;
    let mut store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    store
        .as_mut()
        .ok_or("Session store not initialized")?
        .delete(id)
}
```

**From `codelet/napi/src/persistence/storage.rs` - SessionStore::delete():**

```rust
/// Delete a session
pub fn delete(&mut self, id: Uuid) -> Result<(), String> {
    let session = self.cache.remove(&id);
    if let Some(session) = session {
        // Remove from last_session if it was the last
        if self.last_session.get(&session.project) == Some(&id) {
            self.last_session.remove(&session.project);
        }

        // Delete the file
        let filename = format!("{}.json", id);
        let path = self.sessions_dir.join(&filename);
        if path.exists() {
            fs::remove_file(&path)
                .map_err(|e| format!("Failed to delete session file: {}", e))?;
        }
    }
    Ok(())
}
```

### TypeScript Import
```typescript
import { persistenceDeleteSession, persistenceListSessions } from '@sengac/codelet-napi';
```

## Design Decision: Three-Button Dialog

The user's requirement is for a **three-button confirmation dialog** with:
1. Delete This Session
2. Delete ALL Sessions  
3. Cancel

### Option A: Extend ConfirmationDialog
Add support for a `threeButton` confirmation mode with custom button labels.

**Pros:**
- Reuses existing infrastructure
- Consistent component pattern

**Cons:**
- Adds complexity to ConfirmationDialog
- Keyboard binding for middle button unclear (what key?)

### Option B: Create New ThreeButtonDialog Component
A specialized component for this use case.

**Pros:**
- Clean separation of concerns
- Clear keyboard bindings (1/2/3 or D/A/Esc)

**Cons:**
- Additional component to maintain

### Option C: Two-Step Confirmation (Like CheckpointViewer)
- D key → Single delete dialog (Y/N)
- Shift+D or A key → Delete ALL dialog (typed confirmation)

**Pros:**
- Follows established pattern in codebase
- Simpler UX (one decision at a time)
- Consistent with CheckpointViewer behavior

**Cons:**
- Requires two key bindings instead of one dialog

### Recommendation: Option C (Two-Step Pattern)

Based on existing patterns in the codebase (CheckpointViewer), the recommended approach is:

1. **D key** → Shows confirmation dialog for single session deletion
   - `confirmMode='yesno'`
   - `riskLevel='medium'` (yellow border)
   - Message: "Delete session '{name}'?"

2. **Shift+D or A key** → Shows confirmation dialog for ALL sessions deletion
   - `confirmMode='typed'`
   - `typedPhrase='DELETE ALL'`
   - `riskLevel='high'` (red border)
   - Message: "Delete ALL {count} sessions?"

This approach:
- Maintains UX consistency with existing checkpoint deletion
- Leverages existing ConfirmationDialog component
- Follows ACDD patterns established in the codebase

## Implementation Notes

### New State Variables Needed
```typescript
const [showSessionDeleteDialog, setShowSessionDeleteDialog] = useState(false);
const [sessionDeleteMode, setSessionDeleteMode] = useState<'single' | 'all'>('single');
```

### Keyboard Handler Addition
```typescript
// D key for single session deletion
if (input === 'd' || input === 'D') {
  if (availableSessions.length > 0) {
    setSessionDeleteMode('single');
    setShowSessionDeleteDialog(true);
  }
  return;
}

// Shift+D (or A) for delete ALL sessions
if ((input === 'D' && key.shift) || input === 'a' || input === 'A') {
  if (availableSessions.length > 0) {
    setSessionDeleteMode('all');
    setShowSessionDeleteDialog(true);
  }
  return;
}
```

### Delete Handler Functions
```typescript
const handleSessionDeleteConfirm = useCallback(async () => {
  try {
    const { persistenceDeleteSession } = await import('@sengac/codelet-napi');
    
    if (sessionDeleteMode === 'single') {
      const selectedSession = availableSessions[resumeSessionIndex];
      if (selectedSession) {
        await persistenceDeleteSession(selectedSession.id);
        // Refresh session list
        const sessions = persistenceListSessions(currentProjectRef.current);
        setAvailableSessions(sessions);
        // Adjust index if needed
        setResumeSessionIndex(prev => Math.min(prev, sessions.length - 1));
        // Exit resume mode if no sessions left
        if (sessions.length === 0) {
          setIsResumeMode(false);
        }
      }
    } else {
      // Delete ALL sessions
      for (const session of availableSessions) {
        await persistenceDeleteSession(session.id);
      }
      setAvailableSessions([]);
      setIsResumeMode(false);
    }
  } catch (err) {
    // Handle error
  } finally {
    setShowSessionDeleteDialog(false);
  }
}, [availableSessions, resumeSessionIndex, sessionDeleteMode]);
```

### Footer Update
```typescript
<Text dimColor>Enter Select | ↑↓ Navigate | D Delete | Shift+D Delete All | Esc Cancel</Text>
```

## Feature File Reference

See `spec/features/delete-checkpoint-or-all-checkpoints-from-checkpoint-viewer-with-confirmation.feature` for the pattern to follow when writing the feature file for this story.

## Test File Reference

See `src/tui/components/__tests__/CheckpointViewer-delete.test.tsx` for the testing pattern to follow.

## Summary

The implementation should:
1. Follow the established two-step deletion pattern from CheckpointViewer
2. Use existing ConfirmationDialog component
3. Add D and Shift+D (or A) key bindings in resume mode
4. Call `persistenceDeleteSession` from codelet-napi
5. Update the footer to show new keybindings
6. Refresh session list and handle edge cases (last session deleted)
