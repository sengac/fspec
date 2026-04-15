# TUI-091 Research Findings: SessionFooter Component

## Research Summary

Three parallel research agents investigated the codebase to inform the design and implementation of the SessionFooter component.

---

## 1. SessionHeader Architecture (Reference Design)

**File:** `src/tui/components/SessionHeader.tsx` (200 lines)

### Key Design Patterns

- **Single Text Element Pattern**: The left side uses ONE `<Text>` element with `chalk`-styled content and `textWrap="truncate-end"`. This avoids Ink's flex layout issues with dynamic content and multiple Text children.
- **Dark Grey Background**: `backgroundColor="#333333"` on the inner `<Box>` element.
- **Height**: `height={1}` for a single line.
- **Flex Layout**: Left side has `flexGrow={1} flexShrink={1} minWidth={0}` (truncates), right side has `flexShrink={0}` (never shrinks).
- **Zustand Integration**: Reads work unit info from `sessionStore` via hooks (`useCurrentWorkUnitId`, `useCurrentWorkUnitStatus`).
- **Props-based**: Model/capability info comes from props, work unit info from store.

### JSX Structure (SessionFooter should mirror this)

```tsx
<Box flexDirection="column" width="100%">
  <Box height={1} width="100%" flexDirection="row" backgroundColor="#333333">
    {/* Left side: single Text with truncation */}
    <Box flexGrow={1} flexShrink={1} minWidth={0}>
      <Text wrap="truncate-end">{leftContent}</Text>
    </Box>
    <Text> </Text>
    {/* Right side: never shrink */}
    <Box flexShrink={0} flexDirection="row">
      <Text ...>{rightContent}</Text>
    </Box>
  </Box>
</Box>
```

---

## 2. AgentView Layout Structure

**File:** `src/tui/components/AgentView.tsx` (5652 lines)

### Current Layout Order (main return at line ~5239)

```
1. <SessionHeader ... />           — 1 line, grey bg
2. <RoleBanner ... />              — 1 line (or null when no role)
3. <Box flexGrow={1}>              — conversation VirtualList (fills remaining space)
     <VirtualList items={...} />
   </Box>
4. <Box borderStyle="single"       — input area with TOP border only
     borderTop={true}
     borderBottom={false}
     borderLeft={false}
     borderRight={false}
     paddingX={1}>
     <Text color="green">> </Text>
     <InputTransition ... />
   </Box>
5. Overlays (slash commands, file search, modals, dialogs)
```

### Bottom Border Analysis

The input area (item 4) currently has `borderStyle="single" borderTop={true}` which draws a single-line horizontal rule above the input prompt. This acts as the visual bottom boundary of the conversation area.

**Plan**: The SessionFooter should be inserted between the conversation area (3) and the input area (4). The input area's `borderTop` should be removed since the SessionFooter with its grey background will provide visual separation.

---

## 3. RoleBanner Component (Similar 1-line Component)

**File:** `src/tui/components/RoleBanner.tsx` (41 lines)

Simple, well-documented component that serves as a template for SessionFooter:

```tsx
export const RoleBanner: React.FC<RoleBannerProps> = ({ roleText }) => {
  if (!roleText) { return null; }
  return (
    <Box height={1} width="100%">
      <Text wrap="truncate-end">
        {chalk.cyan('Role:')} {chalk.dim(roleText)}
      </Text>
    </Box>
  );
};
```

---

## 4. Git Branch Detection — ALREADY AVAILABLE

### Rust Layer: `codelet/git/src/status.rs`

```rust
pub fn get_current_branch(dir: impl AsRef<Path>) -> Result<Option<String>>
```

Uses gitoxide (gix) — returns `Some("main")` for branches, `None` for detached HEAD, `Some(name)` for unborn branches.

### NAPI Binding: `codelet/napi/src/git.rs`

```rust
#[napi]
pub fn get_current_branch(dir: String) -> napi::Result<Option<String>>
```

### TypeScript Declaration: `codelet/napi/index.d.ts`

```typescript
export declare function getCurrentBranch(dir: string): string | null;
```

### TypeScript Wrapper: `src/git/status.ts`

```typescript
export async function getCurrentBranch(dir: string, options?: GitStatusOptions): Promise<string | undefined>
```

**FINDING**: Branch detection is fully implemented through the entire stack. No TUI component currently calls it.

---

## 5. Git Status (Dirty/Untracked) — ALREADY AVAILABLE

### NAPI Bindings Available:

| Function | Returns | TypeScript Declaration |
|----------|---------|----------------------|
| `getStagedFiles(dir)` | `Array<string>` | `getStagedFiles(dir: string): string[]` |
| `getUnstagedFiles(dir)` | `Array<string>` | `getUnstagedFiles(dir: string): string[]` |
| `getUntrackedFiles(dir)` | `Array<string>` | `getUntrackedFiles(dir: string): string[]` |

### Status Indicator Mapping:

- `*` (dirty) — `getUnstagedFiles(dir).length > 0` or `getStagedFiles(dir).length > 0`
- `%` (untracked) — `getUntrackedFiles(dir).length > 0`

**FINDING**: All git status functions already exist and are exposed via NAPI. We just need to call them from the footer component.

---

## 6. Current Working Directory — ALREADY AVAILABLE

### Primary Source: `fspecStore` (Zustand)

```typescript
// src/tui/store/fspecStore.ts
interface FspecState {
  cwd: string;  // Initialized to process.cwd()
  setCwd: (cwd: string) => void;
}
```

### Per-Session CWD (for isolated sessions):

```typescript
// codelet/napi/index.d.ts
export declare function sessionGetEffectiveCwd(sessionId: string): string | null;
```

### AgentView Current Usage:

```typescript
// AgentView.tsx line ~927
const currentProjectRef = useRef<string>(process.cwd());
```

### Store Selectors:

```typescript
// sessionStore.ts
export const useIsIsolated = () => useSessionStore(state => state.isIsolated);
export const useWorktreePath = () => useSessionStore(state => state.worktreePath);
```

**FINDING**: CWD available via `fspecStore.cwd` (project root) and `sessionGetEffectiveCwd(sessionId)` (per-session, worktree-aware). The `~/` shortening needs to be implemented (replace `$HOME` prefix with `~`).

---

## 7. Testing Patterns for TUI Components

### Test File Structure

Tests for SessionHeader follow this pattern:
- `SessionHeader.rendering.test.tsx` — behavioral render tests
- `SessionHeader.badges.test.tsx` — badge display tests
- `SessionHeader.store-integration.test.tsx` — Zustand store integration

### Standard Mocks Required

```tsx
vi.mock('@sengac/codelet-napi', () => ({
  sessionSetActive: vi.fn(),
  sessionClearActive: vi.fn(),
  JsThinkingLevel: { Off: 0, Low: 1, Medium: 2, High: 3 },
}));
```

### Test Pattern

```tsx
const defaultProps: SessionHeaderProps = { modelId: 'claude-sonnet-4', /* ... */ };

it('should display something', () => {
  const { lastFrame } = render(<SessionHeader {...defaultProps} />);
  const output = lastFrame();
  expect(output).toContain('expected text');
});
```

### Zustand Store Testing

```tsx
useSessionStore.setState({ currentWorkUnitId: 'TUI-060', currentWorkUnitStatus: 'specifying' });
const { lastFrame } = render(<Component {...props} />);
expect(lastFrame()).toContain('TUI-060');
```

---

## 8. Design Decisions

### SessionFooter Placement

```
SessionHeader       — dark grey bg, 1 line   (top)
RoleBanner          — optional, 1 line
VirtualList         — conversation (fills space)
SessionFooter       — dark grey bg, 1 line   (NEW — between conversation and input)
Input Area          — borderTop removed, just prompt + input
Overlays            — modals/popups
```

### Content Layout

```
Left side (truncated):  ~/projects/fspec [⎇ codelet-integration*%]
Right side (fixed):     (currently empty, reserved for future use)
```

### Data Sources

| Data | Source | Sync? |
|------|--------|-------|
| CWD | `fspecStore.cwd` or `sessionGetEffectiveCwd(sessionId)` | Sync (NAPI) |
| Branch | `getCurrentBranch(dir)` from `@sengac/codelet-napi` | Sync (NAPI) |
| Dirty `*` | `getUnstagedFiles(dir).length > 0 \|\| getStagedFiles(dir).length > 0` | Sync (NAPI) |
| Untracked `%` | `getUntrackedFiles(dir).length > 0` | Sync (NAPI) |

### Refresh Strategy

Git status can change frequently. Options:
1. **On render only** — cheapest, may be stale
2. **Interval polling** (every 5-10s) — balanced approach
3. **File watcher triggered** — already exists in fspecStore for file status

Recommendation: Compute on initial render + use a lightweight interval (e.g., 10s) since NAPI calls are synchronous and fast.

### `~` Path Shortening

```typescript
const shortenPath = (fullPath: string): string => {
  const home = process.env.HOME || process.env.USERPROFILE || '';
  if (home && fullPath.startsWith(home)) {
    return '~' + fullPath.slice(home.length);
  }
  return fullPath;
};
```

### Branch Display Format

```
~/projects/fspec [⎇ codelet-integration*%]
```

Where:
- `⎇` — branch symbol (U+238B)
- `*` — dirty working tree (unstaged or staged changes)
- `%` — untracked files present
- No suffix if clean

---

## 9. Files to Create/Modify

### New Files

1. `src/tui/components/SessionFooter.tsx` — The component (~80-120 lines)
2. `src/tui/components/__tests__/SessionFooter.test.tsx` — Tests

### Modified Files

1. `src/tui/components/AgentView.tsx`:
   - Import SessionFooter
   - Add SessionFooter between conversation Box and input Box
   - Remove `borderTop` from input area Box (SessionFooter provides visual separation)
   - Pass CWD/sessionId props to SessionFooter

### Utility Functions (optional)

Could add to `src/tui/utils/sessionFooterUtils.ts`:
- `shortenPath(fullPath: string): string`
- `getGitStatusIndicators(dir: string): { branch: string | null, dirty: boolean, untracked: boolean }`

---

## 10. Risk Assessment

| Risk | Mitigation |
|------|-----------|
| NAPI calls blocking UI | Calls are synchronous but fast (<5ms); interval polling prevents per-render overhead |
| Layout shift in AgentView | SessionFooter is fixed 1-line height, same as SessionHeader |
| Detached HEAD (no branch) | Show abbreviated commit hash or "(detached)" |
| Non-git directory | Hide branch info entirely, just show CWD |
| Path too long for terminal | Left side uses `textWrap="truncate-end"` pattern from SessionHeader |
