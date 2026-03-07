# AST Research: browser-tools.ts patterns for LOCATE-004

## Handler Registration Pattern

All tool handlers follow the same pattern using `handlers.set(name, async (args) => {...})`:

```
extension/src/background/browser-tools.ts:234:3:handlers.set('browser_navigate', async args => {
extension/src/background/browser-tools.ts:249:3:handlers.set('browser_screenshot', async args => {
extension/src/background/browser-tools.ts:264:3:handlers.set('browser_list_tabs', async () => {
extension/src/background/browser-tools.ts:288:3:handlers.set('browser_execute_script', async args => {
extension/src/background/browser-tools.ts:332:3:handlers.set('browser_switch_tab', async args => {
extension/src/background/browser-tools.ts:351:3:handlers.set('browser_close_tab', async args => {
extension/src/background/browser-tools.ts:367:3:handlers.set('browser_get_page_content', async args => {
extension/src/background/browser-tools.ts:389:3:handlers.set('browser_click_element', async args => {
extension/src/background/browser-tools.ts:415:3:handlers.set('browser_fill_form', async args => {
extension/src/background/browser-tools.ts:447:3:handlers.set('browser_go_back', async args => {
extension/src/background/browser-tools.ts:454:3:handlers.set('browser_go_forward', async args => {
extension/src/background/browser-tools.ts:461:3:handlers.set('browser_create_tab', async args => {
```

## Interfaces (Dependency Injection)

```
extension/src/background/browser-tools.ts:17:8:interface ChromeTabsForTools { ... }
extension/src/background/browser-tools.ts:57:8:interface ChromeScriptingForTools { ... }
extension/src/background/browser-tools.ts:64:8:interface ChromeWindowsForTools { ... }
extension/src/background/browser-tools.ts:80:8:interface ChromeUserScriptsForTools { ... }
extension/src/background/browser-tools.ts:89:8:interface BrowserToolsDeps { ... }
extension/src/background/browser-tools.ts:119:8:interface BrowserToolsAPI { ... }
```

## Ref State Functions (LOCATE-003)

```
extension/src/background/ref-state.ts:42:8:function setTabScanState(
extension/src/background/ref-state.ts:53:8:function getTabScanState(
extension/src/background/ref-state.ts:64:8:function clearTabScanState(tabId: number): void {
extension/src/background/ref-state.ts:72:8:function _resetForTesting(): void {
extension/src/background/ref-state.ts:84:8:function resolveRef(
```

## Key Patterns

- All handlers use `resolveTabId()` for optional tabId
- `scripting.executeScript()` is used with `target: { tabId }` for page injection
- Handlers return via `textResult()` or `errorResult()`
- The `func` parameter to `executeScript` must be a serializable function (no closures)
- Test mocks: `createMockTabs()`, `createMockScripting()`, `createMockWindows()`, `createMockUserScripts()`

## Implementation Plan

browser_scan_page will:
1. Use `resolveTabId()` pattern for optional tabId
2. Use `scripting.executeScript()` to inject the DOM scanner in ISOLATED world
3. Process raw results in the service worker (ref assignment, tree formatting)
4. Store state via `setTabScanState()` from ref-state.ts
5. Return tree text + metadata via `textResult()`

Given browser-tools.ts is already 508 lines, the scanning logic should be extracted to a separate module (e.g., `dom-scanner.ts`) with only the handler registration remaining in browser-tools.ts.
