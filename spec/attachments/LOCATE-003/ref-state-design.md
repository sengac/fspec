# LOCATE-003: Ref State Management Module — Design Document

## Purpose

Centralized state management for scan results, enabling the scan→interact→verify workflow. All scan/ref/diff tools share this state.

## Module: `extension/src/background/ref-state.ts`

### Interfaces

```typescript
interface RefEntry {
  selector: string;   // CSS selector to find the element
  role: string;       // ARIA/semantic role (e.g., 'button', 'textbox', 'link')
  name: string;       // Accessible name (text content, aria-label, etc.)
}

interface TabScanState {
  refs: Map<string, RefEntry>;  // e.g. 'e1' → { selector: '#email', role: 'textbox', name: 'Email' }
  treeText: string;             // Full tree output text (for diff comparison)
  timestamp: number;            // Date.now() when scan was performed
}
```

### Exports

```typescript
// Store a new scan result for a tab
export function setTabScanState(tabId: number, state: TabScanState): void;

// Retrieve the current scan state for a tab (undefined if no scan or invalidated)
export function getTabScanState(tabId: number): TabScanState | undefined;

// Clear scan state for a tab (called on navigation, tab close)
export function clearTabScanState(tabId: number): void;

// Resolve a ref key to its entry (convenience wrapper)
export function resolveRef(tabId: number, ref: string): RefEntry | undefined;
```

### Internal State

```typescript
// Map<tabId, TabScanState>
const tabStates = new Map<number, TabScanState>();
```

## Invalidation Integration

### browser-events.ts Changes

Add ref map invalidation to existing event handlers (minimal ~10 line change):

```typescript
// In tabs.onUpdated listener, when changeInfo.url exists:
import { clearTabScanState } from './ref-state';

// After the navigation notification:
if (typeof changeInfo.url === 'string') {
  clearTabScanState(tabId);
  // ... existing notification code
}

// In tabs.onRemoved listener:
clearTabScanState(tabId);
// ... existing cleanup code
```

### service-worker.ts Changes

Import ref-state module so it's included in the build:

```typescript
import './ref-state';  // Ensure module is included
```

## Why In-Memory (Not chrome.storage)

1. **Performance**: Ref maps are read on every click/fill, needs sub-millisecond access
2. **Ephemeral by design**: Refs are invalidated on navigation — no persistence needed
3. **Size**: A scan of 50 elements is ~5KB — trivial for service worker memory
4. **Lifecycle**: Service worker restarts clear memory naturally, which is correct behavior (stale refs should require re-scan)

## Testing Strategy

Unit tests for ref-state.ts:
- `setTabScanState` / `getTabScanState` round-trip
- `clearTabScanState` removes state
- `resolveRef` returns correct entry
- `resolveRef` returns undefined for unknown ref
- `resolveRef` returns undefined for unknown tabId
- Multiple tabs maintain independent state
- `clearTabScanState` doesn't affect other tabs

Integration test pattern (for browser-events.ts):
- Navigation event triggers `clearTabScanState`
- Tab close triggers `clearTabScanState`
