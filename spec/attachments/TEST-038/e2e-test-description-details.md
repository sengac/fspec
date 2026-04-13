# TEST-038: Fix E2E Test "NO MOCKS" Claim

**Addresses:** C10
**Priority:** 6 (documentation accuracy)

---

## Issue

**File:** `src/tui/__tests__/isolated-session-header-e2e.test.ts` (364 lines)

The test file header claims:
```
NO MOCKS - uses real NAPI bindings
```

But 4 test scenarios use `manager.simulateChunk()`:
- Line 170: `manager.simulateChunk(testSessionId, isolationChunk);`
- Line 216: `manager.simulateChunk(testSessionId, isolationChunk);`
- Line 295: `manager.simulateChunk(testSessionId, isolationChunk);`
- Line 345: `manager.simulateChunk(testSessionId, isolationChunk);`

### Is `simulateChunk` a Mock?

**Nuance:** `simulateChunk` is a **real method** on `GlobalSessionStreamManager` (line 270 of the class):
```typescript
/**
 * Simulate a chunk being received from a session (for testing).
 */
public simulateChunk(sessionId: string, chunk: StreamChunk): void {
  this.handleChunk(sessionId, chunk);
}
```

It feeds chunks through the **real handler pipeline** — not a vi.mock or stub. However, it bypasses the actual NAPI chunk emission path (Rust → Node → JS), meaning the test doesn't verify the full Rust-to-TypeScript data flow.

### Verdict: PARTIALLY CONFIRMED

The claim "NO MOCKS" is technically defensible (no `vi.mock`, no `vi.fn`, no stubs), but **misleading** because `simulateChunk` bypasses the actual NAPI emission pathway. A reader would expect "NO MOCKS" to mean chunks arrive through the real Rust channel.

---

## Fix

### 1. Update File Header
```typescript
/**
 * Feature: spec/features/session-header-isolation-badge.feature
 *
 * Integration tests for isolated session header behavior.
 * Uses real NAPI bindings for session management.
 * Chunk delivery is simulated via GlobalSessionStreamManager.simulateChunk()
 * to test the JS-side handler pipeline without requiring a running agent loop.
 */
```

### 2. Update Scenario Descriptions
Where relevant, clarify the simulation:
```typescript
it('should update store when isolation chunk is received via handler pipeline', ...);
```
Instead of implying full end-to-end NAPI emission.

### 3. Consider File Size
At 364 lines, the file is slightly over the 300-line limit (21%). If practical, split by scenario type during this fix.

---

## Verification

1. File header accurately describes the testing approach
2. No misleading "NO MOCKS" claims remain
3. Test behavior is unchanged (no functional modifications)
4. File is ideally under 300 lines (or close, with justification)
