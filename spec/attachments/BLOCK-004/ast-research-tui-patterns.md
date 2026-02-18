# AST Research: TUI Patterns for BLOCK-004

## 1. useInputCompat Usage in Overlay Components

Components using `useInputCompat` with `InputPriority.CRITICAL` for overlay handling:

| File | Line |
|------|------|
| WatcherTemplateList.tsx | 117, 119 |
| WatcherCreateView.tsx | 88, 90 |
| AnchorView.tsx | 74 |
| AttachmentDialog.tsx | 43 |
| ThinkingLevelDialog.tsx | 78 |
| WatcherTemplateForm.tsx | 108 |

## 2. Slash Command Registration

Current slash commands in `slashCommands.ts`:

| Name | Description |
|------|-------------|
| debug | Toggle debug capture mode |
| clear | Clear conversation history |
| compact | Compact context window |
| mode | Cycle through Edit/Plan/Agent modes |
| thinking | Set base thinking level |
| anchors | View conversation anchor points |
| resume | Resume a previous session |
| detach | Detach session from work unit |
| history | Show command history |
| search | Search command history |
| watcher | Manage watcher sessions |
| parent | Switch to parent session |
| mcp | Manage MCP providers |

**Need to add:** `blocklist` - Manage blocklist rules

## 3. NAPI Blocklist Functions Available

From `codelet/napi/src/blocklist.rs`:

| Function | Description |
|----------|-------------|
| `blocklist_init(project_root)` | Initialize blocklist system |
| `blocklist_load(project_root)` | Load merged config (system + project) |
| `blocklist_save(project_root, config)` | Save project config |
| `blocklist_check(command)` | Check command against blocklist |
| `blocklist_system_path()` | Get system config path |
| `blocklist_project_path(project_root)` | Get project config path |

## 4. Key Pattern: Mode State in AgentView

AgentView uses `useState<boolean>` for mode tracking:
- `isWatcherMode` → triggers WatcherTemplateList overlay
- `isWatcherCreateMode` → triggers WatcherCreateView overlay
- `isWatcherEditMode` → triggers edit mode
- `isTemplateFormMode` → triggers WatcherTemplateForm

**Need to add:** `isBlocklistMode` for BlocklistListView overlay

## 5. Implementation Pattern Summary

1. **Register slash command** in `slashCommands.ts`:
   ```typescript
   { name: 'blocklist', description: 'Manage blocklist rules' }
   ```

2. **Add mode state** in AgentView:
   ```typescript
   const [isBlocklistMode, setIsBlocklistMode] = useState(false);
   ```

3. **Create BlocklistListView** following WatcherTemplateList:
   - `position="absolute"` full-screen overlay
   - Black background
   - Header with border
   - Scrollable rule list
   - Footer with keyboard hints
   - `useInputCompat` with `InputPriority.CRITICAL`

4. **Session toggles** stored in React state:
   - `Map<string, boolean>` or `Set<string>` for disabled rule IDs
   - Resets on TUI restart (not persisted)
