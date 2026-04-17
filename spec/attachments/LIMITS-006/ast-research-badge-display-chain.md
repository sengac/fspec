# AST Research: Badge Display Chain (LIMITS-006)

## SessionHeader.tsx badge logic
```
const badgeValue = compactionThreshold ?? contextWindow;
```
Located at line 165. Badge uses `compactionThreshold` when available, falls back to `contextWindow`.

## AgentView.tsx rustModelInfo useMemo
Lines 1164-1237: Reads `rustModel.contextWindow` and `rustModel.compactionThreshold` from Rust snapshot. These are the clamped values from Rust ProviderManager.

## formatContextWindow (sessionHeaderUtils.ts)
Formats token count: >=1M → "1M", else → Math.round(n/1000) + "k"
- 191808 → "192k"
- 800000 → "800k"
- 102400 → "102k"

## Existing test files
- rust-authoritative-context-window.test.ts: Tests Rust state → TUI, contextWindow values
- per-model-compaction-threshold.test.ts: Tests compactionThreshold from Rust state
- sessionheader-badge-threshold.test.tsx: Tests SessionHeader renders badge with threshold

All existing tests already use correct clamped values (200k for Claude, not 1M).
