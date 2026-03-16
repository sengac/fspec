# WATCH-024 Post-Review Fixes

## Review Date: 2026-03-15

---

## 🔴 CATEGORY 1: COMPILE ERRORS (6 issues)

### Fix 1: `index.d.ts` NOT REGENERATED from Rust build
- The `.d.ts` still exports old names: `sessionCreateWatcher`, `sessionGetParent`, `sessionGetWatchers`, `sessionClearRole`, `watcherInject`, `SessionRoleInfo`, `WatcherInputImage`, `WatcherPendingInjectionInfo`
- TS code imports new names: `sessionCreateSupervisor`, `sessionGetSubordinate`, `sessionGetSupervisors`, etc.
- **Fix**: This file is auto-generated. Do NOT manually edit. After Rust build it will regenerate. But the TypeScript import sites must match whatever the Rust #[napi] macros emit. Since we can't rebuild Rust NAPI in this context, manually update the .d.ts to match the Rust function names so TypeScript compiles.

### Fix 2: `supervisorTemplateStorage.ts` imports non-existent types
- Line 17-18: imports `WatcherInstance`, `WatcherListItem`
- But `supervisorTemplate.ts` exports `SupervisorInstance`, `SupervisorListItem`
- **Fix**: Update imports.

### Fix 3: `SupervisorTemplateList.tsx` imports non-existent function
- Line 21: imports `buildFlatSupervisorList`
- But `supervisorTemplateStorage.ts` exports `buildFlatWatcherList`
- **Fix**: Rename function to `buildFlatSupervisorList` in storage file.

### Fix 4: `AgentView.tsx` imports non-existent type `WatcherInstance`
- Line 139: `WatcherInstance` — renamed to `SupervisorInstance` in `supervisorTemplate.ts`
- **Fix**: Update import, but the type is also used as variable types throughout AgentView. Must rename all usage.

### Fix 5: `AgentView.tsx` imports `SessionRoleInfo` — Rust renamed to `SupervisorRoleInfo`
- Line 116: `type SessionRoleInfo`
- But index.d.ts will export `SupervisorRoleInfo` after rebuild
- **Fix**: Update import and all usage of `SessionRoleInfo` → `SupervisorRoleInfo`.

### Fix 6: `sessionSetRole` calling convention mismatch
- 4 call sites pass `authority` arg that Rust no longer accepts
- **Fix**: Remove authority parameter from all call sites. Update to 4-arg form: `(sessionId, roleName, roleBrief, autoInject)`

---

## 🔴 CATEGORY 2: RUNTIME/FUNCTIONAL BUGS (3 issues)

### Fix 7: chunkProcessor regex will NEVER match
- Line 83: `/^\[SUPERVISOR: ([^|]+) \| Authority: (Supervisor|Peer) \| Session: ([^\]]+)\]\n?/`
- Rust now emits: `[SUPERVISOR: role | Session: id] message` (NO Authority field)
- **Fix**: Update regex to `/^\[SUPERVISOR: ([^|]+) \| Session: ([^\]]+)\]\n?/`
- Also update `ParsedWatcherInfo` interface to remove `authority` field.

### Fix 8: `chunk.type === 'WatcherInput'` checks (7 places)
- Rust enum variant is now `SupervisorInput`, NAPI emits `type: 'SupervisorInput'`
- 5 in AgentView.tsx, 2 in chunkProcessor.ts
- **Fix**: Change all to `chunk.type === 'SupervisorInput'`

### Fix 9: `ParsedWatcherInfo` interface still has `authority` field
- **Fix**: Remove `authority` field, rename interface to `ParsedSupervisorInfo`

---

## 🔴 CATEGORY 3: FEATURE FILE VIOLATIONS — authority NOT removed (4 issues)

### Fix 10: `SupervisorCreateView.tsx` still has Authority selector
- **Fix**: Remove authority from FocusField, FOCUS_ORDER, props, state, handler, and UI. Remove from `onCreate` callback signature.

### Fix 11: `SupervisorTemplateForm.tsx` still has Authority selector
- **Fix**: Remove authority from FocusField, FOCUS_ORDER, props, state, handler, and UI. Remove from `onSave` callback signature.

### Fix 12: `SupervisorTemplate` interface still has `authority` field
- **Fix**: Remove `authority` from the interface.

### Fix 13: `createTemplate`/`updateTemplate`/`formatTemplateDisplay` reference authority
- **Fix**: Remove authority parameter and references from all three functions.

---

## 🟡 CATEGORY 4: CODE QUALITY ISSUES (4 issues)

### Fix 14: Dangling `};` at line 490 of session_manager.rs
- Leftover from authority match block removal.
- **Fix**: Remove the dangling `};` and extra blank line.

### Fix 15: `/parent` command replaced with magic string instead of being deleted
- `if (userMessage === '__REMOVED_PARENT_CMD_WATCH024__')` — dead code
- **Fix**: Delete both blocks entirely (2 instances, ~30 lines each).

### Fix 16: `role?.description` and `role?.authority` at AgentView.tsx line 5279-5280
- Old field names from SessionRoleInfo.
- **Fix**: Change to `role?.brief` and remove authority arg.

### Fix 17: Inconsistent state variable naming
- `isSupervisorSessionView` paired with `setIsWatcherSessionView`
- **Fix**: Rename setter to `setIsSupervisorSessionView`.

---

## 🟡 CATEGORY 5: TEST FILE DEFICIENCIES (3 issues)

### Fix 18: Test for "TUI template form removes Authority toggle" is false-pass
- Doesn't check that Authority is absent.
- **Fix**: Add `expect(fileContains(formFile, 'Authority')).toBe(false)` or similar.

### Fix 19: No test verifies chunk type string constants match
- **Fix**: Add test checking `'SupervisorInput'` in TypeScript chunk processing code.

### Fix 20: "All existing tests pass" scenario is placeholder
- **Fix**: Leave as documentation-only (actual verification is running the suite).
