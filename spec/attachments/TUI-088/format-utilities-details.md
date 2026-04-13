# TUI-088: Align formatContextWindow & Hoist formatPercentage

**Addresses:** C5, C9
**Priority:** 5 (cosmetic consistency)

---

## C5: formatContextWindow Inconsistency

### Current Implementations

**`src/tui/utils/sessionHeaderUtils.ts` (lines 15–20):**
```typescript
export const formatContextWindow = (contextWindow: number): string => {
  if (contextWindow >= 1000000) {
    return `${(contextWindow / 1000000).toFixed(0)}M`;  // No decimal
  }
  return `${Math.round(contextWindow / 1000)}k`;
};
```

**`src/tui/components/ModelSelectorView.tsx` (line ~54):**
```typescript
// Uses .toFixed(1) for M values — with decimal
`${(contextWindow / 1000000).toFixed(1)}M`
```

### Output Comparison

| Token Count | SessionHeader | ModelSelector | Mismatch? |
|-------------|:---:|:---:|:---:|
| 200,000 | `200k` | `200k` | No |
| 1,000,000 | `1M` | `1.0M` | **Yes** |
| 1,500,000 | `2M` (rounded!) | `1.5M` | **Yes** |
| 2,000,000 | `2M` | `2.0M` | **Yes** |

### Fix

Import and use `formatContextWindow` from `sessionHeaderUtils.ts` in `ModelSelectorView.tsx`. If the display needs differ between contexts (header wants compact, selector wants precise), add an optional `precision` parameter:

```typescript
export const formatContextWindow = (
  contextWindow: number,
  precision: 'compact' | 'precise' = 'compact'
): string => {
  if (contextWindow >= 1000000) {
    const decimals = precision === 'precise' ? 1 : 0;
    return `${(contextWindow / 1000000).toFixed(decimals)}M`;
  }
  return `${Math.round(contextWindow / 1000)}k`;
};
```

---

## C9: formatPercentage Inside Render

### Current Code (`SessionHeader.tsx` lines 136–138)
```typescript
// Inside component body — new function created every render
const formatPercentage = (num: number): string => {
  return num.toFixed(2);
};
```

### Fix

Hoist to module level — function is pure, captures no closure variables:

```typescript
// Module level — allocated once
const formatPercentage = (num: number): string => {
  return num.toFixed(2);
};

export const SessionHeader: React.FC<SessionHeaderProps> = ({ ... }) => {
  // ... uses formatPercentage without re-creating it
};
```

**Impact:** Negligible performance gain, but follows project coding standards and eliminates a code review nit.

---

## Verification

1. All contexts show consistent formatting for the same token count
2. `formatContextWindow` is imported (not duplicated) in ModelSelectorView
3. `formatPercentage` is at module level in SessionHeader.tsx
4. All tests pass
