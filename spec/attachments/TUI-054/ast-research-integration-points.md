# AST Research: TUI-054 Integration Points

## Research Date: 2026-01-27

## Summary
Identified key integration points for ThinkingLevelDialog implementation by analyzing the existing pause state pattern in Rust and TypeScript.

## Rust NAPI Integration Points

### 1. Existing Pause State NAPI Function (Pattern to Follow)
```
codelet/napi/src/session_manager.rs:3973:1
pub fn session_get_pause_state(session_id: String) -> Result<Option<NapiPauseState>>
```

### 2. BackgroundSession get_pause_state Method
```
codelet/napi/src/session_manager.rs:1018:5
pub fn get_pause_state(&self) -> Option<PauseState>
```

### Pattern Analysis
- BackgroundSession stores `pause_state: RwLock<Option<PauseState>>`
- For thinking level, use `base_thinking_level: AtomicU8` (simpler, no Option needed)
- NAPI function returns via `session.get_pause_state().map(|s| s.into())`
- For thinking level, return `u8` directly (0=Off, 1=Low, 2=Medium, 3=High)

## TypeScript Integration Points

### 3. TypeScript Interfaces in Hooks
```
src/tui/hooks/rustStateSource.ts:38:8 - interface RustStateSource
src/tui/hooks/useRustSessionState.ts:57:8 - interface RustSessionSnapshot
src/tui/hooks/sessionSubscription.ts:13:8 - interface SessionSubscription
```

### Pattern Analysis
- RustStateSource defines methods like `getPauseState(sessionId: string): PauseInfo | null`
- Need to add: `getBaseThinkingLevel(sessionId: string): number` (returns JsThinkingLevel enum value)
- RustSessionSnapshot has fields like `isPaused: boolean`, `pauseInfo: PauseInfo | null`
- Need to add: `baseThinkingLevel: number` (JsThinkingLevel.Off = 0 by default)

## Files to Modify

### Rust Files
1. `codelet/napi/src/session_manager.rs`:
   - Add `base_thinking_level: AtomicU8` to BackgroundSession struct
   - Add `get_base_thinking_level(&self) -> u8` method
   - Add `set_base_thinking_level(&self, level: u8)` method
   - Add `#[napi] pub fn session_get_base_thinking_level(session_id: String) -> Result<u8>`
   - Add `#[napi] pub fn session_set_base_thinking_level(session_id: String, level: u8) -> Result<()>`

### TypeScript Files
1. `src/tui/hooks/rustStateSource.ts`:
   - Add `getBaseThinkingLevel(sessionId: string): number` to RustStateSource interface
   - Add `setBaseThinkingLevel(sessionId: string, level: number): void` to RustStateSource interface
   - Implement in defaultRustStateSource

2. `src/tui/hooks/useRustSessionState.ts`:
   - Add `baseThinkingLevel: number` to RustSessionSnapshot interface
   - Add to EMPTY_SNAPSHOT: `baseThinkingLevel: 0`
   - Update fetchFreshSnapshot to call source.getBaseThinkingLevel
   - Update snapshotsAreEqual to compare baseThinkingLevel

3. `src/tui/components/slashCommands.ts`:
   - Add `/thinking` to SLASH_COMMANDS registry

4. `src/tui/components/AgentView.tsx`:
   - Add `showThinkingLevelDialog` state
   - Handle `/thinking` command in handleSubmitWithCommand
   - Render ThinkingLevelDialog when showThinkingLevelDialog is true
   - Pass baseThinkingLevel from snapshot to SessionHeader

5. `src/tui/components/SessionHeader.tsx`:
   - Add `baseThinkingLevel` prop
   - Show badge when baseThinkingLevel > 0 (not just during loading)
   - During loading, show effective level badge

### New Files to Create
1. `src/tui/components/ThinkingLevelDialog.tsx` - Dialog component
2. `src/tui/components/__tests__/ThinkingLevelDialog.test.tsx` - Tests

## Integration Flow
1. User types `/thinking` → AgentView detects slash command
2. AgentView sets `showThinkingLevelDialog: true`
3. ThinkingLevelDialog opens with current level from `snapshot.baseThinkingLevel`
4. User navigates with up/down, selects with Enter
5. ThinkingLevelDialog calls `sessionSetBaseThinkingLevel(sessionId, level)`
6. NAPI updates BackgroundSession.base_thinking_level
7. AgentView calls `refresh()` to update snapshot
8. SessionHeader displays new level badge
