# TUI-087: Remove supervisorInfo Dead Code

**Addresses:** C3
**Priority:** 4 (straightforward cleanup)

---

## Dead Code Inventory

### 1. `SessionHeader.tsx` — Prop and Rendering Logic

| Location | What to Remove |
|----------|---------------|
| Lines 47–52 | `SupervisorHeaderInfo` interface (2 fields: `slug`, `instanceNumber`) |
| Line 84 | `supervisorInfo?: SupervisorHeaderInfo` prop |
| Line 125 | `supervisorInfo` destructuring |
| Lines 162–164 | `if (supervisorInfo) { ... }` rendering block |

### 2. `useSupervisorHeaderInfo.ts` — Entire Unused Hook

**File:** `src/tui/hooks/useSupervisorHeaderInfo.ts`
- Fully implemented (lines 44–81) but **never imported** by any component
- Contains a DIFFERENT `SupervisorHeaderInfo` interface (lines 18–27, 4 fields: `slug`, `instanceNumber`, `roleName`, `subordinateId`)
- Two different interfaces with the same name across files — clear sign of incomplete refactoring

**Evidence:** Grep for `import.*useSupervisorHeaderInfo` returns **zero matches** across entire codebase.

### 3. Test References

Check and remove any tests that reference `supervisorInfo`:
- `SessionHeader.test.tsx` — may have test cases for the prop
- If tests exist, remove them (they test dead code)

---

## Verification Steps

1. Grep for `supervisorInfo` — should return zero results after cleanup
2. Grep for `SupervisorHeaderInfo` — should return zero results
3. Grep for `useSupervisorHeaderInfo` — should return zero results
4. All existing tests pass
5. Build compiles successfully

---

## Note on Future Work

If supervisor-subordinate header display is needed in the future, it should:
1. Be re-implemented from scratch with a single aligned interface
2. Use the Zustand store pattern (not props) per business rule from `session-header-realtime-status.feature`
3. Be wired through AgentView at the point of implementation
