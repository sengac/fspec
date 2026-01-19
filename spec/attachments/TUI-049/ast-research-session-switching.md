# AST Research: Session Switching Implementation

## 1. History Navigation Pattern (to follow)

### AgentView.tsx - History Handlers
```
fspec research --tool=ast --pattern="const handleHistoryPrev" --lang=tsx --path=src/tui/components/
```
Result: `src/tui/components/AgentView.tsx:2986:3`

```
fspec research --tool=ast --pattern="const handleHistoryNext" --lang=tsx --path=src/tui/components/
```
Result: `src/tui/components/AgentView.tsx:3002:3`

### MultiLineInput.tsx - Key Detection
Location: `src/tui/components/MultiLineInput.tsx:122-142`
- Escape sequences: `[1;2A` (Shift+Up), `[1;2B` (Shift+Down)
- Also checks `key.shift && key.upArrow` / `key.downArrow`

## 2. Session Management

### Session Manager List
```
fspec research --tool=ast --pattern="sessionManagerList" --lang=tsx --path=src/tui/components/
```
Used in AgentView.tsx for getting background sessions.

### Session Attach/Detach
```
fspec research --tool=ast --pattern="sessionAttach" --lang=tsx --path=src/tui/components/
```
Used for attaching callback to session.

```
fspec research --tool=ast --pattern="sessionDetach" --lang=tsx --path=src/tui/components/
```
Used for detaching from session.

## 3. Rust BackgroundSession Structure

Location: `codelet/napi/src/session_manager.rs`

Key fields to add:
- `pending_input: RwLock<Option<String>>` - stores input text when detaching

Key functions to modify:
- `session_detach` - save input text before detaching
- `session_attach` - return saved input text on attach

## 4. NAPI Bindings

Location: `codelet/napi/index.d.ts`

Existing bindings:
- `sessionManagerList(): Array<SessionInfo>`
- `sessionAttach(sessionId: string, callback: (err, chunk) => any): void`
- `sessionDetach(sessionId: string): void`

New bindings needed:
- `sessionSetPendingInput(sessionId: string, input: string): void`
- `sessionGetPendingInput(sessionId: string): string | null`

## 5. Implementation Plan

1. **Rust changes** (`codelet/napi/src/session_manager.rs`):
   - Add `pending_input: RwLock<Option<String>>` to BackgroundSession
   - Add `session_set_pending_input` and `session_get_pending_input` NAPI functions

2. **TypeScript changes** (`src/tui/components/MultiLineInput.tsx`):
   - Add Shift+Left/Right detection (escape sequences `[1;2C` and `[1;2D`)
   - Call `onSessionNext`/`onSessionPrev` props

3. **TypeScript changes** (`src/tui/components/AgentView.tsx`):
   - Add `handleSessionNext`/`handleSessionPrev` callbacks
   - Compute current session index from `sessionManagerList()`
   - On switch: save input, detach old, attach new, restore input, load conversation
