# AST Research: browser-tools.ts Handler Analysis for LOCATE-005

## Purpose
Identify the exact handlers that need ref resolution and their current structure.

## Handler Map (13 handlers total)

| Line | Handler | Accepts `selector`? | Needs Ref Resolution? |
|------|---------|---------------------|-----------------------|
| 240 | browser_navigate | No (uses `url`) | No |
| 255 | browser_screenshot | No | No |
| 270 | browser_list_tabs | No args | No |
| 294 | browser_execute_script | No (uses `code`) | No |
| 338 | browser_switch_tab | No (uses `tabId`) | No |
| 357 | browser_close_tab | No (uses `tabId`) | No |
| 373 | browser_get_page_content | No (uses `format`) | No |
| **395** | **browser_click_element** | **Yes (`selector`)** | **YES** |
| **421** | **browser_fill_form** | **Yes (`selector`)** | **YES** |
| 453 | browser_go_back | No | No |
| 460 | browser_go_forward | No | No |
| 467 | browser_create_tab | No (uses `url`) | No |
| 506 | browser_scan_page | No (uses `selector` for scope, not element targeting) | No |

## browser_click_element (line 395)

```
const selector = args.selector as string;  // ← needs `let` for mutation
```

- Uses `const selector` — must change to `let selector`
- Calls `resolveTabId()` before `scripting.executeScript()`
- Ref resolution goes between `resolveTabId` and `executeScript`

## browser_fill_form (line 421)

```
const selector = args.selector as string;  // ← needs `let` for mutation
```

- Uses `const selector` — must change to `let selector`
- Calls `resolveTabId()` before `scripting.executeScript()`
- Ref resolution goes between `resolveTabId` and `executeScript`

## Current imports from ref-state

```typescript
import { setTabScanState } from './ref-state';
import type { RefEntry } from './ref-state';
```

Need to add `resolveRef` to the import:
```typescript
import { setTabScanState, resolveRef } from './ref-state';
```

## resolveRef API (from ref-state.ts)

```typescript
export function resolveRef(tabId: number, ref: string): RefEntry | undefined
```

- Returns `RefEntry` with `selector`, `role`, `name` fields
- Returns `undefined` if no scan state exists OR ref key not found
