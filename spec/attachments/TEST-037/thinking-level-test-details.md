# TEST-037: Split TUI-058 Thinking Level Test & Fix Inline Mocks

**Addresses:** C8
**Priority:** 3 (medium-high — 89% over limit, inline mocks bypass real code)

---

## Current State: 567 Lines (89% Over Limit)

### Test Block Inventory

| Line Range | Block | Type | Lines |
|------------|-------|------|-------|
| 97–150 | Set default thinking level via D key | Behavioral render | 53 |
| 152–179 | Dialog footer shows D key option | Behavioral render | 27 |
| 186–234 | Dialog shows default indicator | Behavioral render | 48 |
| 236–265 | Dialog shows no indicator when no default | Behavioral render | 29 |
| 267–333 | Default indicator moves when D key pressed | Behavioral render | 66 |
| 339–358 | Restore default on new session | **Inline mock** | 19 |
| 360–380 | Use Off when no default is set | **Inline mock** | 20 |
| 386–436 | Current selection independent of default | Behavioral render | 50 |
| 442–467 | Handle corrupt config gracefully | **Inline mock** | 25 |
| 481–515 | loadDefaultThinkingLevel (3 tests) | **Inline mock** | 34 |
| 517–566 | saveDefaultThinkingLevel (2 tests) | **Inline mock** | 49 |

**Behavioral tests:** ~273 lines across 6 scenarios
**Inline mock tests:** ~147 lines across 5 blocks + config helpers

---

## Problem: Inline Mock Tests

Lines 481–567 contain config helper tests that **recreate logic inline** instead of importing the actual functions:

```typescript
// ❌ CURRENT: Manually reimplements the logic
it('should load default thinking level', () => {
  mockConfig.loadConfig.mockReturnValue({
    tui: { defaultThinkingLevel: 'medium' }
  });
  const config = mockConfig.loadConfig();
  const level = config?.tui?.defaultThinkingLevel ?? null;  // Reimplemented inline!
  expect(level).toBe('medium');
});
```

This tests the mock, not the real `loadDefaultThinkingLevel` function. The real function could be completely broken and this test would still pass.

---

## Proposed Split

### 1. `thinking-level-dialog-behavior.test.tsx` (~200 lines)
- Set default via D key (97–150)
- Dialog footer shows D key option (152–179)
- Dialog shows default indicator (186–234)
- Dialog shows no indicator (236–265)
- Default indicator moves (267–333)
- Current selection independent of default (386–436)

### 2. `thinking-level-config-persistence.test.tsx` (~120 lines)
**Replace inline mock tests** with tests that import actual config helper functions:

```typescript
// ✅ CORRECT: Test the real function
import { loadDefaultThinkingLevel, saveDefaultThinkingLevel } from '../helpers/thinkingLevelConfig';

it('should load default thinking level from config', () => {
  // Use a temp dir with a real config file
  const configDir = setupTempConfig({ tui: { defaultThinkingLevel: 'medium' } });
  const level = loadDefaultThinkingLevel(configDir);
  expect(level).toBe('medium');
});
```

Include:
- Restore default on new session (339–358) — rewrite with real functions
- Use Off when no default (360–380) — rewrite with real functions
- Handle corrupt config (442–467) — rewrite with real functions
- loadDefaultThinkingLevel tests — rewrite importing real function
- saveDefaultThinkingLevel tests — rewrite importing real function

---

## Verification

1. All new test files are under 300 lines
2. Config helper tests import and test actual functions (no inline reimplementation)
3. Zero `mockConfig.loadConfig()` calls that test reimplemented logic
4. All tests pass with `npm test`
5. All `@step` comments map to actual assertions
