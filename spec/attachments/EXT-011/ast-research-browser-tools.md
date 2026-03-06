# AST Research: browser-tools.ts structure for browser_create_tab

## Date: 2026-03-06

## Research: ChromeTabsForTools interface

The interface that needs `create` added is at line 17 of `browser-tools.ts`:

```
/Users/rquast/projects/fspec/extension/src/background/browser-tools.ts:17:1:export interface ChromeTabsForTools {
```

Current methods: query, update, remove, captureVisibleTab, goBack, goForward, get, onUpdated.
**Missing**: `create` — needs to be added for `browser_create_tab` handler.

## Research: Existing handler registrations

All 11 existing handlers are registered via `handlers.set()` pattern:

```
browser-tools.ts:226:3: handlers.set('browser_navigate', async args => { ... })
browser-tools.ts:241:3: handlers.set('browser_screenshot', async args => { ... })
browser-tools.ts:256:3: handlers.set('browser_list_tabs', async () => { ... })
browser-tools.ts:280:3: handlers.set('browser_execute_script', async args => { ... })
browser-tools.ts:324:3: handlers.set('browser_switch_tab', async args => { ... })
browser-tools.ts:343:3: handlers.set('browser_close_tab', async args => { ... })
browser-tools.ts:359:3: handlers.set('browser_get_page_content', async args => { ... })
browser-tools.ts:381:3: handlers.set('browser_click_element', async args => { ... })
browser-tools.ts:407:3: handlers.set('browser_fill_form', async args => { ... })
browser-tools.ts:439:3: handlers.set('browser_go_back', async args => { ... })
browser-tools.ts:446:3: handlers.set('browser_go_forward', async args => { ... })
```

New `browser_create_tab` handler should be added after `browser_go_forward` (line 446+) following the same pattern.

## Research: waitForTabLoad helper

The `waitForTabLoad` function at line 195 already exists and is used by `browser_navigate`.
The `browser_create_tab` handler should reuse this when a URL is provided.

## Research: MCP Server NATIVE_TOOLS

The `mcp-server.mjs` file at `extension/host/lib/mcp-server.mjs` has a `NATIVE_TOOLS` array (line 21-143) with 11 entries.
A 12th entry for `browser_create_tab` needs to be appended.
