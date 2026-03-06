# Research: browser_create_tab — Missing Tab Creation Tool

## Date: 2026-03-06

## Problem

The fspec WebMCP Chrome Extension exposes 11 native browser control tools but has no way to **create new tabs**. The AI agent can:

- ✅ `browser_close_tab` — close a tab
- ✅ `browser_switch_tab` — activate an existing tab
- ✅ `browser_list_tabs` — enumerate all open tabs
- ✅ `browser_navigate` — navigate an existing tab to a new URL (replaces current page)
- ❌ **No `browser_create_tab`** — cannot open a URL in a new tab

### Failed Workaround

Using `browser_execute_script` with `window.open('https://example.com', '_blank')` does NOT work. Chrome's popup blocker silently blocks it because:

1. The script runs in the `USER_SCRIPT` world via `chrome.userScripts.execute()`
2. There is no user gesture context — `window.open()` requires a trusted user activation event
3. The call returns `true` but no tab is created

---

## Solution: chrome.tabs.create()

### API Signature

```typescript
chrome.tabs.create(createProperties: {
  url?: string;           // URL to open (defaults to New Tab page)
  active?: boolean;       // Whether to make it the active tab (default: true)
  index?: number;         // Position in tab strip (clamped to valid range)
  windowId?: number;      // Which window (defaults to current)
  openerTabId?: number;   // Which tab "opened" it
  pinned?: boolean;       // Whether to pin it (default: false)
}): Promise<chrome.tabs.Tab>
```

### Return Value

Returns a `tabs.Tab` object containing:
- `id` — the new tab's unique ID
- `url` — the committed URL (may be empty initially)
- `title` — the page title
- `active` — whether the tab is active
- `windowId` — which window it's in
- `index` — its position in the tab strip

### Permissions

**No additional permissions required.** Per Chrome documentation:

> "Most features don't require any permissions to use. For example: creating a new tab, reloading a tab, navigating to another URL, etc."

The extension already declares `"tabs"` permission in `manifest.json` (line 8), which gives access to `url`, `title`, `pendingUrl`, and `favIconUrl` properties on the returned Tab object.

---

## Implementation Plan

### 1. Extension Service Worker (`extension/src/background/browser-tools.ts`)

**Add `create` to `ChromeTabsForTools` interface:**

```typescript
export interface ChromeTabsForTools {
  // ... existing methods ...
  create: (createProperties: {
    url?: string;
    active?: boolean;
    index?: number;
    windowId?: number;
    openerTabId?: number;
    pinned?: boolean;
  }) => Promise<chrome.tabs.Tab>;
}
```

**Add handler to `createBrowserTools()`:**

```typescript
handlers.set('browser_create_tab', async args => {
  const createProperties: Record<string, unknown> = {};
  if (args.url !== undefined) createProperties.url = args.url as string;
  if (args.active !== undefined) createProperties.active = args.active as boolean;
  if (args.windowId !== undefined) createProperties.windowId = args.windowId as number;
  if (args.pinned !== undefined) createProperties.pinned = args.pinned as boolean;

  const tab = await tabs.create(createProperties);

  // Optionally wait for load if a URL was provided
  if (args.url && tab.id !== undefined) {
    const loadedTab = await waitForTabLoad(tab.id);
    return textResult({
      tabId: loadedTab.id,
      url: loadedTab.url ?? args.url,
      title: loadedTab.title ?? '',
      active: loadedTab.active,
      windowId: loadedTab.windowId,
    });
  }

  return textResult({
    tabId: tab.id,
    url: tab.url ?? '',
    title: tab.title ?? '',
    active: tab.active,
    windowId: tab.windowId,
  });
});
```

### 2. MCP Server (`extension/host/lib/mcp-server.mjs`)

**Add to `NATIVE_TOOLS` array:**

```javascript
{
  name: 'browser_create_tab',
  description: 'Create a new browser tab, optionally navigating to a URL',
  inputSchema: {
    type: 'object',
    properties: {
      url: { type: 'string', description: 'URL to open (defaults to New Tab page)' },
      active: { type: 'boolean', description: 'Whether to make it the active tab (defaults to true)' },
      windowId: { type: 'number', description: 'Window to create the tab in (defaults to current window)' },
      pinned: { type: 'boolean', description: 'Whether to pin the tab (defaults to false)' },
    },
  },
},
```

### 3. Skill Documentation (`extension/webmcp-skill.md`)

Add documentation for the new tool alongside existing tool docs.

---

## Other Potentially Missing Tab Operations

While investigating, these additional `chrome.tabs` methods were identified as potentially useful but not currently exposed:

| Method | Use Case | Priority |
|--------|----------|----------|
| `chrome.tabs.reload(tabId)` | Force-refresh a page | Medium |
| `chrome.tabs.duplicate(tabId)` | Clone a tab | Low |
| `chrome.tabs.move(tabId, {index})` | Reorder tabs | Low |
| `chrome.tabs.group({tabIds})` | Group tabs together | Low |
| `chrome.tabs.discard(tabId)` | Free memory for background tabs | Low |

These could be added in future work units if needed.

---

## Reference Links

### Chrome Extension API Documentation

- **chrome.tabs API overview**: https://developer.chrome.com/docs/extensions/reference/api/tabs
- **chrome.tabs.create() method**: https://developer.chrome.com/docs/extensions/reference/api/tabs#method-create
- **tabs.create() (MDN/W3C docs)**: https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/API/tabs/create
- **Detailed parameter reference**: https://docs.w3cub.com/web_extensions/api/tabs/create.html

### Chromium Source Code (Verified 2026-03-06)

- **tabs.json API schema** — defines all chrome.tabs methods, parameters, and types (including `create`).
  The file's namespace description confirms: "Use the `chrome.tabs` API to interact with the browser's tab system. You can use this API to create, modify, and rearrange tabs in the browser."
  https://chromium.googlesource.com/chromium/src/+/main/chrome/common/extensions/api/tabs.json

- **tabs_api.h** — C++ header declaring `TabsCreateFunction` (and all other tabs API functions).
  The class hierarchy uses `DECLARE_EXTENSION_FUNCTION("tabs.create", TABS_CREATE)` to map the JS API name to the C++ implementation. Located at:
  https://chromium.googlesource.com/chromium/src/+/main/chrome/browser/extensions/api/tabs/tabs_api.h
  Also viewable via Chromium Code Search:
  https://source.chromium.org/chromium/chromium/src/+/main:chrome/browser/extensions/api/tabs/tabs_api.h

- **tabs_api.cc** — C++ implementation file containing `TabsCreateFunction::Run()`.
  This calls into `ExtensionTabUtil::OpenTab()` to actually create the tab in the browser.
  https://chromium.googlesource.com/chromium/src/+/main/chrome/browser/extensions/api/tabs/tabs_api.cc

- **extension_tab_util.h** — Helper utility class with `OpenTab()`, `CreateTabObject()`, and other tab manipulation helpers used by `TabsCreateFunction`.
  Contains error constants like `kTabNotFoundError`, `kLockedFullscreenModeNewTabError`, and `kTabStripNotEditableError` that define error conditions for tab creation.
  https://chromium.googlesource.com/chromium/src/+/main/chrome/browser/extensions/extension_tab_util.h

- **writing_a_new_api.md** — Chromium documentation that explicitly confirms the architecture:
  > "the chrome.tabs.create() API function is called by an extension to create a tab, and is implemented in C++ by the TabsCreateFunction, an instance of the ExtensionFunction class."
  https://chromium.googlesource.com/chromium/src/+/HEAD/extensions/docs/writing_a_new_api.md

- **api_functions.md** — Chromium documentation describing the extension function call flow:
  > "JavaScript in the renderer process calls an API (e.g., chrome.tabs.create()). Renderer bindings validate the call and forward the arguments to the browser if the call is valid."
  https://chromium.googlesource.com/chromium/src/+/HEAD/extensions/docs/api_functions.md

### Permissions Reference (Verified 2026-03-06)

- **Extension permissions guide**: https://developer.chrome.com/docs/extensions/reference/api/tabs#permissions
- **Verified quote from official Chrome docs** (still present on the page as of 2026-03-06):
  > "Most features don't require any permissions to use. For example: creating a new tab, reloading a tab, navigating to another URL, etc."
- **Verified**: The `"tabs"` permission description on the same page states:
  > "This permission does not give access to the chrome.tabs namespace. Instead, it grants an extension the ability to call tabs.query() against four sensitive properties on tabs.Tab instances: url, pendingUrl, title, and favIconUrl."
- This confirms: The `"tabs"` permission only controls access to sensitive Tab properties — it does NOT gate tab creation/manipulation methods. Our extension already has this permission for reading tab URLs/titles.

---

## Open Source Element Highlighting Tools (Context Research)

These projects were researched as part of the broader investigation into browser element highlighting for AI agent remote control. They are relevant to future WebMCP extension features:

| Project | Description | URL |
|---------|-------------|-----|
| **reworkd/tarsier** | Tags interactable elements with numbered brackets `[1]`, `[2]` for LLM-driven web agents | https://github.com/reworkd/tarsier |
| **microsoft/OmniParser** | Overlays numbered bounding boxes on UI elements for pure vision-based GUI agents | https://github.com/microsoft/OmniParser |
| **microsoft/SoM** | Set-of-Mark visual prompting — numbered spatial marks on screenshots for GPT-4V grounding | https://github.com/microsoft/SoM |
| **browser-use/browser-use** | Screenshot highlighting system with numbered DOM element overlays for AI browser agents | https://github.com/browser-use/browser-use |
| **The-Agentic-Intelligence-Co/dom-engine** | Turns website DOMs into actionable context for browser agents (extension + Puppeteer) | https://github.com/The-Agentic-Intelligence-Co/dom-engine |
| **matatk/element-highlighter** | Browser extension that highlights elements matching CSS selectors/XPath with overlay tinting | https://github.com/matatk/element-highlighter |
| **metaory/pickr** | Chrome extension for interactive element selection with overlay interface and legend hints | https://github.com/metaory/pickr |
| **izzywdev/fuzepicker** | AI-powered Chrome extension for DOM element picking with Playwright/React code generation | https://github.com/izzywdev/fuzepicker |
| **nanobrowser/nanobrowser** | Open source Chrome extension for AI web agent automation with multi-agent system | https://github.com/nanobrowser/nanobrowser |

### Key Approaches to Element Highlighting for AI Agents

1. **Text-based tagging** (Tarsier): Injects numbered bracket labels `[1]`, `[2]` next to interactable elements directly into the DOM. The LLM sees these tags in screenshots or page text and references them by number. Lightweight, no vision model needed.

2. **Vision-based bounding boxes** (OmniParser, SoM, browser-use): Captures screenshots and overlays numbered bounding boxes/marks on detected interactive elements. Requires a vision model but works on any UI without DOM access.

3. **DOM extraction** (dom-engine): Extracts the DOM tree into a structured text format with element indices, providing the LLM with a compact actionable representation of the page without screenshots.
