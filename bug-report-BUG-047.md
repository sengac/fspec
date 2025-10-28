# Bug Report: BUG-047

## Type Mismatches in record-metric and record-tokens Commands

**Discovered during**: LOCK-002 file locking refactoring
**Date**: 2025-10-28
**Reporter**: Claude (AI Assistant)

---

## Summary

Two command files have type mismatches between their function signatures and how they're called in command registration:
- `src/commands/record-metric.ts`
- `src/commands/record-tokens.ts`

These bugs existed **before** the LOCK-002 refactoring work began.

---

## Detailed Findings

### File 1: `src/commands/record-metric.ts`

**Function Signature** (lines 19-23):
```typescript
export async function recordMetric(options: {
  workUnitId: string;
  tokens: number;
  cwd?: string;
}): Promise<{ success: boolean; totalTokens?: number }>
```

**Command Registration Call** (lines 66-70):
```typescript
await recordMetric({
  metric,                    // ❌ Not in function signature
  value: parseFloat(value),  // ❌ Not in function signature
  unit: options.unit,        // ❌ Not in function signature
});
```

**Command Arguments** (lines 60-62):
```typescript
.argument('<metric>', 'Metric name')
.argument('<value>', 'Metric value')
.option('--unit <unit>', 'Unit of measurement')
```

**Problem**: The function expects `{ workUnitId, tokens, cwd }` but is called with `{ metric, value, unit }`. These are completely different parameter sets.

---

### File 2: `src/commands/record-tokens.ts`

**Function Signature** (lines 19-23):
```typescript
export async function recordTokens(options: {
  workUnitId: string;
  tokens: number;
  cwd?: string;
}): Promise<{ success: boolean; totalTokens?: number }>
```

**Command Registration Call** (lines 76-79):
```typescript
await recordTokens({
  workUnitId,
  tokens: parseInt(tokens, 10),
  operation: options.operation,  // ❌ Not in function signature
});
```

**Command Option** (lines 65-68):
```typescript
.option(
  '--operation <operation>',
  'Operation type (e.g., specification, implementation)'
)
```

**Problem**: The function doesn't accept an `operation` parameter but the command tries to pass it anyway.

---

## Impact Analysis

### Current State
- TypeScript compilation may be passing due to the `[key: string]: unknown` index signature in the options interface
- Commands may be silently failing or ignoring parameters
- Tests may not be covering these code paths

### Expected Behavior
The function signatures should match the command registration calls exactly.

### Potential Fixes

#### Option A: Update Function Signatures (Recommended)
Match the functions to what the commands actually need:

**record-metric.ts:**
```typescript
export async function recordMetric(options: {
  metric: string;
  value: number;
  unit?: string;
  cwd?: string;
}): Promise<{ success: boolean }>
```

**record-tokens.ts:**
```typescript
export async function recordTokens(options: {
  workUnitId: string;
  tokens: number;
  operation?: string;
  cwd?: string;
}): Promise<{ success: boolean; totalTokens?: number }>
```

#### Option B: Update Command Registration
Change how commands call the functions to match existing signatures (may require rethinking command purpose).

---

## Discovery Context

These bugs were discovered during LOCK-002 refactoring when:
1. Systematically refactoring command files to use `fileManager.transaction()`
2. Applied standard pattern to these files
3. Ran full test suite: **103 tests failed**
4. Investigated failures and found pre-existing type mismatches
5. Reverted changes to these files to isolate the issue

**Git References:**
- Last good commit before discovery: `efee27c`
- Files were at HEAD~1 when bug discovered

---

## Reproduction Steps

1. Review `src/commands/record-metric.ts` lines 19-23 and 66-70
2. Review `src/commands/record-tokens.ts` lines 19-23 and 76-79
3. Note the parameter mismatch between function signatures and calls
4. Run: `npm run build` - TypeScript may pass due to index signature
5. Run: `npm test` - Tests may fail or pass if not covering these paths

---

## Recommended Actions

1. **Investigate Intent**: Determine what these commands are supposed to do
   - Are they for recording metrics OR token usage?
   - Should they operate on work units?

2. **Fix Type Signatures**: Align function signatures with command usage

3. **Add Tests**: Ensure test coverage for these commands

4. **Update Documentation**: Clarify command purpose and usage

5. **Continue LOCK-002**: These files can be refactored after types are fixed

---

## Notes

- These bugs existed **before LOCK-002 refactoring**
- Type system may not catch due to index signature: `[key: string]: unknown`
- May need to check if similar patterns exist in other command files
- Should verify if commands are actually used in production

---

## Related Work Units

- **LOCK-002**: File locking refactoring (blocked by this bug for these 2 files)
