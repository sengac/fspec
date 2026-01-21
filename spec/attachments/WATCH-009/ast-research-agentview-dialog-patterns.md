# AST Research: WatcherCreateView Design for WATCH-009

## Research Goal
Design WatcherCreateView component based on the architecture document (spec/attachments/WATCH-001/watcher-sessions-architecture.md).

## Source: WATCH-001 Architecture Document (Lines 806-834)

The architecture document specifies a full-screen form view for watcher creation:

```
┌─────────────────────────────────────────────────────────────┐
│  Create New Watcher                                         │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Role Name: Security Reviewer                               │
│                                                             │
│  Authority: [Supervisor ▼]  (Peer | Supervisor)             │
│                                                             │
│  Model: [anthropic/claude-sonnet-4 ▼]                       │
│                                                             │
│  Brief (watching instructions):                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ Watch for security vulnerabilities including:        │   │
│  │ - Unsafe file operations                             │   │
│  │ - Exposed credentials or API keys                    │   │
│  │ - SQL injection, XSS, or other injection attacks     │   │
│  │ - Insecure cryptographic practices                   │   │
│  │                                                      │   │
│  │ Interrupt immediately for critical security issues.  │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
├─────────────────────────────────────────────────────────────┤
│  Enter: Create | Tab: Next Field | Esc: Cancel              │
└─────────────────────────────────────────────────────────────┘
```

## Key Design Decisions

### 1. Component Structure
- Separate component: `src/tui/components/WatcherCreateView.tsx`
- Full-screen overlay view (similar to `/resume` and `/search` modes)
- Self-contained with its own keyboard handling

### 2. Form Fields
| Field | Type | Required | Default | Notes |
|-------|------|----------|---------|-------|
| Role Name | Text input | Yes | "" | e.g., "Security Reviewer" |
| Authority | Dropdown | No | "Peer" | Peer or Supervisor |
| Model | Dropdown | No | Parent's model | From available provider models |
| Brief | Multiline textarea | No | "" | Watching instructions |

### 3. Keyboard Navigation
| Key | Action |
|-----|--------|
| Tab | Cycle focus: Role Name → Authority → Model → Brief → Create |
| Shift+Tab | Cycle focus backwards |
| ←/→ | Toggle Authority (when focused) |
| ↑/↓ | Navigate Model dropdown (when focused) |
| Enter | Create watcher (if valid) |
| Esc | Cancel and return to overlay |

### 4. NAPI Calls
Creation flow:
1. `sessionCreateWatcher(parentId, model, project, name)` → returns watcherId
2. `sessionSetRole(watcherId, name, brief, authority)` → sets role info

The "Brief" field becomes the `roleDescription` parameter in `sessionSetRole`.

### 5. Component Props
```typescript
interface WatcherCreateViewProps {
  parentSessionId: string;
  currentModel: string;
  availableModels: string[];
  onCreate: (name: string, authority: 'peer' | 'supervisor', model: string, brief: string) => void;
  onCancel: () => void;
}
```

### 6. Integration with AgentView.tsx
- Add `isWatcherCreateMode` state
- Replace TODO in N key handler with `setIsWatcherCreateMode(true)`
- Conditional render: `if (isWatcherCreateMode) return <WatcherCreateView ... />`

## Example Workflow (from Architecture Doc lines 1209-1220)

```
1. User types `/watcher` to open Watcher Management
2. User presses `N` to create new watcher
3. User fills:
   - Role: "Security Reviewer"
   - Authority: Supervisor
   - Model: anthropic/claude-sonnet-4
   - Brief: "Watch for security vulnerabilities, unsafe file operations,
            exposed credentials. Interrupt immediately for critical issues."
4. User presses Enter → watcher created
5. View closes, overlay shows new watcher in list
```
