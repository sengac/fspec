# TEST-036: Overhaul SessionHeader.test.tsx

**Addresses:** C7
**Priority:** 2 (high — 58% of tests are non-behavioral)

---

## Current State: 419 Lines, 58% Source-Reading Tests

### Test Breakdown

| Line Range | Section | Test Type | Count |
|------------|---------|-----------|-------|
| 58–92 | SessionHeader subscribes to sessionStore | **Source-code reading** | 3 |
| 94–139 | sessionStore provides currentWorkUnitId | **Source-code reading** | 5 |
| 141–157 | AgentView syncs Rust snapshot | **Source-code reading** | 1 |
| 165–216 | Singleton File Watcher | **Source-code reading** | 4 |
| 222–247 | Status change updates header | Behavioral render | 1 |
| 249–267 | Header without status | Behavioral render | 1 |
| 269–293 | Opening AgentView shows work unit info | **Source-code reading / Hybrid** | 1 |
| 300–313 | Legacy: work unit display | Behavioral render | 1 |
| 315–339 | Reasoning and vision badges | Behavioral render | 2 |
| 341–366 | Compaction percentage formatting | Behavioral render | 2 |
| 372–418 | ISOLATED badge | Behavioral render | 3 |
| **Total** | | **14 source / 10 behavioral** | **24** |

### `fs.readFileSync` Occurrences (18 total)

Lines: 62, 71, 80, 98, 107, 116, 125, 134, 145, 154, 169, 181, 193, 202, 213, 273, 281, 290

**Pattern:** Read source file → `expect(source).toContain('functionName')` — verifies a string exists in source code, not that the code actually works at runtime.

---

## Proposed Split

### 1. `SessionHeader.rendering.test.tsx` (~150 lines)
- Status change updates header (lines 222–247)
- Header without status (lines 249–267)
- Compaction percentage formatting (lines 341–366)
- ISOLATED badge tests (lines 372–418)

### 2. `SessionHeader.badges.test.tsx` (~80 lines)
- Reasoning and vision badge tests (lines 315–339)
- Any new badge behavioral tests

### 3. `SessionHeader.integration.test.tsx` (~120 lines)
**Replace ALL 14 source-reading tests** with proper integration tests that:
- Actually create a Zustand store with test state
- Render `<SessionHeader>` with the real store
- Verify rendered output contains expected content
- Test the actual wiring (not string presence in source)

Example replacement:
```typescript
// ❌ BEFORE: Source-code reading (doesn't test runtime)
it('should import useCurrentWorkUnitId', () => {
  const source = fs.readFileSync('src/tui/components/SessionHeader.tsx', 'utf-8');
  expect(source).toContain('useCurrentWorkUnitId');
});

// ✅ AFTER: Behavioral integration test
it('should display work unit ID from store', () => {
  useSessionStore.setState({ currentWorkUnitId: 'AUTH-001' });
  const { lastFrame } = render(<SessionHeader {...defaultProps} />);
  expect(lastFrame()).toContain('AUTH-001');
});
```

### 4. Remove legacy prop-based tests (lines 300–313)
These contradict business rule #2 from `session-header-realtime-status.feature`: "SessionHeader MUST use Zustand store directly - no props for dynamic state."

---

## Verification

1. All new test files are under 300 lines
2. Zero `fs.readFileSync` calls in any SessionHeader test
3. All scenarios from `session-header-realtime-status.feature` have behavioral tests
4. Each `@step` comment maps to an actual assertion
5. All tests pass with `npm test`
