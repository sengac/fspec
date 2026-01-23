# Watcher Templates and Improved Creation UX - Design Document

## Problem Statement

The current watcher creation flow (WATCH-009) has several UX pain points:

1. **Model selection is painful** - Scrolling through 50+ models with arrow keys takes too long
2. **No way to reuse briefs** - Every watcher creation requires typing the brief from scratch
3. **Watchers don't persist** - No way to save configurations as templates for reuse
4. **One-off creation model** - Current design treats each watcher as a fresh form fill

## Design Goals

1. **Templates as first-class citizens** - Save watcher configurations, spawn instances from them
2. **Fast model selection** - Type-to-filter, only show configured models, sensible defaults
3. **TUI-native navigation** - Arrow keys for field navigation, collapse/expand for hierarchy
4. **Clear authority explanations** - Users understand Peer vs Supervisor when they see it

## Core Concepts

### Template vs Instance

```
Template = Saved configuration (name, model, authority, brief, auto-inject)
Instance = Running watcher spawned from a template
```

A template can have 0 or more active instances. Users work primarily with templates and spawn instances from them.

### Data Model

```typescript
interface WatcherTemplate {
  id: string;                         // UUID
  name: string;                       // "Security Reviewer"
  modelId: string;                    // "anthropic/claude-sonnet-4-20250514"
  authority: 'peer' | 'supervisor';
  brief: string;                      // Watching instructions
  autoInject: boolean;
  createdAt: string;
  updatedAt: string;
}

interface WatcherInstance {
  sessionId: string;                  // UUID of the watcher session
  templateId: string;                 // Which template it was spawned from
  status: 'running' | 'idle';
}
```

### Storage Location

Templates stored at user level: `~/.fspec/watcher-templates.json`

Uses the existing `getFspecUserDir()` utility from `src/utils/config.ts` for consistency with other user-level storage (config, credentials, logs).

Rationale: Watcher roles (Security Reviewer, Test Enforcer) are typically consistent across projects.

## UI Design

### Main Watcher Overlay (`/watcher`)

Template-centric view with collapse/expand for instances:

```
┌─────────────────────────────────────────────────────────────┐
│  Watchers                                                   │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ▼ Security Reviewer (Supervisor)         [2 active]        │
│      claude-sonnet-4                                        │
│      ├─ #1 running                                          │
│      └─ #2 idle                                             │
│                                                             │
│  ▶ Test Enforcer (Supervisor)             [1 active]        │
│      claude-sonnet-4                                        │
│                                                             │
│  ▶ Architecture Advisor (Peer)                              │
│      gemini-2.0-flash                                       │
│                                                             │
│    + Create new template...                                 │
│                                                             │
├─────────────────────────────────────────────────────────────┤
│  ←→: Collapse/Expand | Enter: Spawn/Open | E: Edit | D: Del │
└─────────────────────────────────────────────────────────────┘
```

**Visual indicators:**
- `▼` = expanded (showing instances)
- `▶` = collapsed (has instances but not showing)
- No arrow = no active instances

**Navigation:**
- ↑/↓ moves through flat list (templates and visible instances)
- ← collapses current template
- → expands current template (if has instances)
- Tab/Shift+Tab also works for field navigation in forms

**Actions:**
- Enter on template → Spawn new instance
- Enter on instance → Open that watcher session
- E on template → Edit template
- D on template → Delete template (with confirmation)
- D on instance → Kill instance
- N or Enter on "+ Create new template..." → Open template creation form

### Template Creation/Edit Form

```
┌─────────────────────────────────────────────────────────────┐
│  Create Watcher Template                                    │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Name:                                                      │
│  > Security Reviewer█                                       │
│                                                             │
│  Model:                                                     │
│    claude-sonnet-4                                          │
│                                                             │
│  Authority:                                                 │
│    [●] Peer  [ ] Supervisor                                 │
│                                                             │
│  Auto-inject:                                               │
│    [●] Enabled  [ ] Disabled                                │
│                                                             │
│  Brief:                                                     │
│  ┌─────────────────────────────────────────────────────────┐│
│  │ Watch for security vulnerabilities including SQL        ││
│  │ injection, XSS, exposed credentials...█                 ││
│  └─────────────────────────────────────────────────────────┘│
│                                                             │
├─────────────────────────────────────────────────────────────┤
│  ↑↓: Navigate | Enter: Save | Esc: Cancel                   │
└─────────────────────────────────────────────────────────────┘
```

**Field Navigation:**
- ↑/↓ moves between fields (TUI-native)
- Tab/Shift+Tab also works
- ←/→ toggles Authority and Auto-inject when focused
- Enter saves the template
- Esc cancels

### Model Selection (Type-to-Filter)

When Model field is focused:

```
│  Model:                                                     │
│  > son█                                                     │
│    ┌───────────────────────────────────────────────────────┐│
│    │ anthropic/claude-sonnet-4-20250514                    ││
│    │ anthropic/claude-3-5-sonnet-20241022                  ││
│    └───────────────────────────────────────────────────────┘│
```

**Behavior:**
- Only shows models that are actually configured (have API keys)
- Defaults to parent session's current model
- Start typing to filter
- ↑/↓ navigates filtered list (when filter dropdown is visible)
- Enter selects highlighted model
- Backspace removes filter characters

### Authority Explanation

When Authority field is focused, show inline help:

```
│  Authority:                                                 │
│    [●] Peer  [ ] Supervisor                                 │
│                                                             │
│    Peer: Suggestions the AI can consider or ignore          │
│    Supervisor: Directives the AI should follow              │
```

Authority is also visible in:
- Template list: "Security Reviewer (Supervisor)"
- Instance list (inherited from template)

### Empty State

When no templates exist:

```
┌─────────────────────────────────────────────────────────────┐
│  Watchers                                                   │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  No watcher templates yet.                                  │
│                                                             │
│  Watchers are AI agents that observe your session and       │
│  can interject with feedback (security issues, test         │
│  coverage, architecture suggestions, etc.)                  │
│                                                             │
│  Press N to create your first template.                     │
│                                                             │
├─────────────────────────────────────────────────────────────┤
│  N: Create Template | Esc: Close                            │
└─────────────────────────────────────────────────────────────┘
```

## Navigation Pattern (Copied from Model Selector)

Uses the same collapse/expand pattern as the existing model selector in AgentView.tsx:

```typescript
// Track which templates are expanded
const [expandedTemplates, setExpandedTemplates] = useState<Set<string>>(new Set());

// Build flat list for navigation
type WatcherListItem = 
  | { type: 'template'; template: WatcherTemplate; isExpanded: boolean }
  | { type: 'instance'; template: WatcherTemplate; instance: WatcherInstance }
  | { type: 'create-new' };

const buildFlatWatcherList = (
  templates: WatcherTemplate[],
  instances: WatcherInstance[],
  expandedTemplates: Set<string>
): WatcherListItem[] => {
  const items: WatcherListItem[] = [];
  templates.forEach(template => {
    const isExpanded = expandedTemplates.has(template.id);
    const templateInstances = instances.filter(i => i.templateId === template.id);
    items.push({ 
      type: 'template', 
      template, 
      isExpanded: isExpanded && templateInstances.length > 0 
    });
    if (isExpanded) {
      templateInstances.forEach(instance => {
        items.push({ type: 'instance', template, instance });
      });
    }
  });
  items.push({ type: 'create-new' });
  return items;
};
```

## Key Decisions

1. **Templates are primary** - The overlay shows templates, not instances
2. **Instances nest under templates** - Using collapse/expand pattern from model selector
3. **Enter spawns, not opens** - On a template, Enter creates new instance
4. **Model defaults to parent** - New templates default to parent session's model
5. **Only configured models** - Filter model list to those with API keys
6. **Type-to-filter for models** - No more arrow-key scrolling through 50+ models
7. **Authority shown everywhere** - In lists and forms, with explanation when focused
8. **User-level storage** - Templates in `~/.fspec/watcher-templates.json` (via `getFspecUserDir()`)
9. **Quick spawn command** - `/watcher spawn <slug>` for power users
10. **Auto-kill on template deletion** - Deleting template with active instances auto-kills them with warning
11. **Alphabetical ordering** - Templates listed alphabetically with type-to-filter search

## Related Stories

- WATCH-009: Original watcher creation dialog (superseded by this)
- WATCH-021: Auto-inject toggle (incorporated into this)
- WATCH-008: Watcher management overlay (replaced by this)

## Resolved Questions

1. **Quick spawn command?** → Yes, `/watcher spawn <slug>` where slug is kebab-case of name
2. **Template deletion with active instances?** → Auto-kill with warning in confirmation dialog
3. **Template ordering?** → Alphabetical with type-to-filter search
