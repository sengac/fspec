# LOCATE-005: Ref Resolution in Click and Fill — Implementation Guide

## Overview

This is a surgical change to two existing handlers in `browser-tools.ts`. When the `selector` argument starts with `@`, resolve it from the ref map before executing.

## Implementation

### browser_click_element handler (line ~389 in browser-tools.ts)

Add ref resolution at the top, before the existing `scripting.executeScript`:

```typescript
handlers.set('browser_click_element', async args => {
  let selector = args.selector as string;
  if (!selector) {
    return errorResult('Missing required parameter: selector');
  }
  
  const tabId = await resolveTabId(args.tabId as number | undefined);
  
  // --- NEW: Ref resolution ---
  if (selector.startsWith('@')) {
    const refKey = selector.slice(1);  // '@e3' → 'e3'
    const entry = resolveRef(tabId, refKey);
    if (!entry) {
      return errorResult(
        `Ref ${selector} not found. Run browser_scan_page first to scan the page.`
      );
    }
    selector = entry.selector;
  }
  // --- END NEW ---
  
  // ... existing executeScript code unchanged
});
```

### browser_fill_form handler (line ~415 in browser-tools.ts)

Identical pattern:

```typescript
handlers.set('browser_fill_form', async args => {
  let selector = args.selector as string;
  const value = args.value as string;
  if (!selector) {
    return errorResult('Missing required parameter: selector');
  }
  if (value === undefined) {
    return errorResult('Missing required parameter: value');
  }
  
  const tabId = await resolveTabId(args.tabId as number | undefined);
  
  // --- NEW: Ref resolution ---
  if (selector.startsWith('@')) {
    const refKey = selector.slice(1);
    const entry = resolveRef(tabId, refKey);
    if (!entry) {
      return errorResult(
        `Ref ${selector} not found. Run browser_scan_page first to scan the page.`
      );
    }
    selector = entry.selector;
  }
  // --- END NEW ---
  
  // ... existing executeScript code unchanged
});
```

### Import Required

At the top of `browser-tools.ts`:

```typescript
import { resolveRef } from './ref-state';
```

## Dependency Injection Consideration

The current `createBrowserTools(deps)` pattern uses dependency injection. `resolveRef` is a module-level function — if we want DI consistency, we could add it to `BrowserToolsDeps`. However, since ref-state is an internal extension module (not browser API), direct import is appropriate.

## Backward Compatibility

- Selectors NOT starting with `@` pass through unchanged — zero behavioral change
- `@` is not a valid start character for CSS selectors, so there's no ambiguity
- All 12 existing tools continue to work identically
- Only `browser_click_element` and `browser_fill_form` gain ref resolution (they're the only tools that take selectors)

## Error Messages

The error message explicitly suggests running `browser_scan_page`:
- This guides the AI through the correct workflow
- If refs are stale (page navigated), the scan state was cleared automatically
- The AI will see this error, scan, and retry with fresh refs

## Testing Strategy

1. **Ref resolves correctly**: Set up ref map, call click with `@e1`, verify CSS selector used
2. **Ref not found**: Call click with `@e99`, verify error message
3. **No scan state**: Call click with `@e1` on tab with no scan, verify error
4. **Raw CSS still works**: Call click with `#submit`, verify unchanged behavior
5. **@ in the middle is NOT treated as ref**: `div@e1` is a CSS selector, not a ref
