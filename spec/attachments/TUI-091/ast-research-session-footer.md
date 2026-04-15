# AST Research: SessionFooter Component Design

## Research Date: 2026-04-14
## Work Unit: TUI-091

## 1. Existing 1-Line Components (Pattern Reference)

### SessionHeader — `src/tui/components/SessionHeader.tsx:102`
```
export const SessionHeader: React.FC<SessionHeaderProps>
```
- Uses `Box height={1} backgroundColor="#333333"` for grey background
- Single-Text-element pattern with chalk for left side
- `flexGrow={1} flexShrink={1} minWidth={0}` for truncation

### RoleBanner — `src/tui/components/RoleBanner.tsx:29`
```
export const RoleBanner: React.FC<RoleBannerProps>
```
- Returns null when no role (zero height)
- `Box height={1} width="100%"` with `Text wrap="truncate-end"`

## 2. AgentView Input Area Border

`src/tui/components/AgentView.tsx:5446-5452`:
```tsx
<Box
  borderStyle="single"
  borderTop={true}
  borderBottom={false}
  borderLeft={false}
  borderRight={false}
  paddingX={1}
>
```
This border will be removed when SessionFooter is inserted.

## 3. NAPI Git Functions Available

From `codelet/napi/index.d.ts`:
- `getCurrentBranch(dir: string): string | null` (line 779)
- `getStagedFiles(dir: string): Array<string>` (line 821)
- `getUnstagedFiles(dir: string): Array<string>` (line 867)
- `getUntrackedFiles(dir: string): Array<string>` (line 875)
- `sessionGetEffectiveCwd(sessionId: string): string | null` (line 2070)

All are synchronous NAPI calls — safe for React render with periodic refresh.

## 4. Store State Available

From `src/tui/store/fspecStore.ts`:
- `cwd: string` — initialized to `process.cwd()`

From `src/tui/store/sessionStore.ts`:
- `isIsolated: boolean`
- `worktreePath: string | null`

## 5. Similar Component Tests Pattern

From `src/tui/components/__tests__/SessionHeader.rendering.test.tsx`:
- Uses `vi.mock('@sengac/codelet-napi', ...)` for NAPI mocks
- Uses `render()` from `ink-testing-library`
- Asserts on `lastFrame()` with `toContain()`
