# AST Research: LOCATE-006 Integration Points

## browser-tools.ts — Handler Registration Pattern

13 existing handlers registered via `handlers.set(name, async args => { ... })`:
- browser_navigate (line 240)
- browser_screenshot (line 255)
- browser_list_tabs (line 270)
- browser_execute_script (line 294)
- browser_switch_tab (line 338)
- browser_close_tab (line 357)
- browser_get_page_content (line 373)
- browser_click_element (line 418)
- browser_fill_form (line 452)
- browser_go_back (line 492)
- browser_go_forward (line 499)
- browser_create_tab (line 506)
- browser_scan_page (line 545)

**New handler `browser_diff_page` will follow the same pattern.**

## ref-state.ts — State Management API

Exported interfaces:
- `RefEntry` (line 16): { selector, role, name }
- `TabScanState` (line 26): { refs: Map<string, RefEntry>, treeText: string, timestamp: number }

Exported functions:
- `setTabScanState(tabId, state)` (line 42)
- `getTabScanState(tabId)` → TabScanState | undefined (line 53)
- `clearTabScanState(tabId)` (line 64)
- `_resetForTesting()` (line 72)
- `resolveRef(tabId, ref)` → RefEntry | undefined (line 84)

**browser_diff_page uses `getTabScanState` to get previous tree, `setTabScanState` to update.**

## dom-scanner.ts — formatAccessibilityTree

`formatAccessibilityTree(elements: RawElement[]): string` (line 163)
- Takes raw elements, returns indented tree text
- Shared between browser_scan_page and browser_diff_page

## Key Integration Points for browser_diff_page

1. Add `getTabScanState` import to browser-tools.ts (already imports `setTabScanState` and `resolveRef`)
2. Create new `myers-diff.ts` module (pure function, no Chrome deps)
3. Register `browser_diff_page` handler after `browser_scan_page` handler
4. Reuse same scanning pipeline: `scripting.executeScript({ func: scanPageDOM }) → formatAccessibilityTree()`
