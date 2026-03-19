# fspec Browser Agent — Extension Skill

Connect this AI session to the fspec Browser Agent Chrome Extension, giving you direct control over the user's Chrome browser: navigate pages, scan and interact with DOM elements (including inside iframes), execute JavaScript, capture screenshots, and use any WebMCP tools registered by websites.

## Action Required

When this skill is referenced, immediately connect to the WebMCP extension:

```
ConnectMCP(transport: "http", url: "http://localhost:19876/mcp")
```

If a custom URL is provided as a parameter, use that instead of the default.

## How to Use This Skill

Reference this file with `@` in your prompt to load the skill:

```
@extension/extension-skill.md                                         # Connect to http://localhost:19876/mcp
@extension/extension-skill.md please connect to http://localhost:8080/mcp  # Connect to custom URL
```

---

## Prerequisites

- **fspec Browser Agent Chrome Extension** installed and active
- **Native messaging host** registered (`node extension/host/native-host.mjs --register --extension-id <id>`)
- **Chrome 133+** (for improved iframe scanning with `match_origin_as_fallback`)
- **Chrome 146+** with "WebMCP for testing" flag if using WebMCP tools (`chrome://flags`)
- **"Allow User Scripts"** enabled for `browser_execute_script` (`chrome://extensions` → fspec Browser Agent → Details)

---

## What You Get After Connecting

Once connected, you have **14 native browser control tools** plus any **WebMCP tools** that websites have registered via `navigator.modelContext.registerTool()`.

You also receive **real-time browser event notifications** via SSE — tab navigation, page loads, tab creation, tab closure, and tool list changes.

---

## Native Browser Control Tools

### Navigation & Tab Management

#### `browser_navigate`
Navigate a tab to a URL. Waits for page load to complete before returning.

```json
{ "url": "https://example.com" }
{ "url": "https://example.com", "tabId": 123 }
```

- `url` (string, **required**): The URL to navigate to
- `tabId` (number, optional): Target tab ID. Defaults to the active tab.

Returns: `{ "url": "...", "title": "..." }`

#### `browser_create_tab`
Create a new browser tab, optionally navigating to a URL. Waits for page load when a URL is provided.

```json
{}
{ "url": "https://example.com" }
{ "url": "https://example.com", "active": false }
{ "url": "https://example.com", "pinned": true }
```

- `url` (string, optional): URL to open. Defaults to New Tab page.
- `active` (boolean, optional): Whether to make it the active tab. Defaults to true.
- `windowId` (number, optional): Window to create the tab in.
- `pinned` (boolean, optional): Whether to pin the tab.

Returns: `{ "tabId": 42, "url": "...", "title": "...", "active": true, "windowId": 1 }`

**Use this instead of `browser_execute_script` with `window.open()`** — Chrome's popup blocker blocks `window.open()` without a user gesture, but `browser_create_tab` uses `chrome.tabs.create()` which always works.

#### `browser_list_tabs`
List all open browser tabs with their IDs, URLs, titles, and active state.

```json
{}
```

Returns: Array of `{ "id": number, "url": string, "title": string, "active": boolean }`

**Use this first** to discover tab IDs for targeting other tools at specific tabs.

#### `browser_switch_tab`
Activate a tab and focus its window.

```json
{ "tabId": 123 }
```

- `tabId` (number, **required**): The tab ID to switch to.

#### `browser_close_tab`
Close a browser tab.

```json
{ "tabId": 123 }
```

- `tabId` (number, **required**): The tab ID to close.

#### `browser_go_back` / `browser_go_forward`
Navigate back or forward in browser history.

```json
{}
{ "tabId": 123 }
```

- `tabId` (number, optional): Target tab. Defaults to active tab.

---

### Page Content & Screenshots

#### `browser_get_page_content`
Get the text content or full HTML of a page.

```json
{}
{ "format": "html" }
{ "format": "text", "tabId": 123 }
```

- `tabId` (number, optional): Target tab. Defaults to active tab.
- `format` (string, optional): `"text"` (default) for `innerText`, `"html"` for full `outerHTML`.

**Use `"text"` format** for reading page content (much smaller payload). Only use `"html"` when you need DOM structure.

#### `browser_screenshot`
Capture a screenshot of a tab's visible viewport, or crop to a specific element by CSS selector or `@ref` from `browser_scan_page`.

```json
{}
{ "tabId": 123 }
{ "selector": "@e5" }
{ "selector": "#hero-image" }
{ "selector": "@f2e1" }
```

- `tabId` (number, optional): Tab to capture. Defaults to active tab.
- `selector` (string, optional): CSS selector or `@ref` (e.g., `@e5`, `@f2e1`) to screenshot a specific element instead of the full viewport. The element is scrolled into view before capture. Supports iframe refs.

**Without `selector`:** Captures the full visible viewport (backward compatible).

**With `selector`:** Scrolls the element into view, captures the viewport, then crops to the element's bounding rect. Handles DPR scaling automatically (Retina displays produce correctly cropped images).

Returns: One or more JPEG image content blocks (resized to ≤1568px long edge, 80% quality, tiled if >800KB). You will see the screenshot directly.

**Errors:**
- Ref not found in scan state → `"Ref @e3 not found. Run browser_scan_page first to scan the page."`
- Ref resolved but element gone from DOM → `"Element for @e5 (resolved to \"#hero\") not found in DOM. The page may have changed since the last scan."`
- Element has zero dimensions (e.g., `display:none`) → `"Element has no visible dimensions"`
- CSS selector matches nothing → `"Element not found: {selector}"`

#### `browser_execute_script`
Execute arbitrary JavaScript in a tab's page context using the USER_SCRIPT world.

```json
{ "code": "document.title" }
{ "code": "document.querySelectorAll('a').length", "tabId": 123 }
```

- `code` (string, **required**): JavaScript code to execute. The return value is sent back.
- `tabId` (number, optional): Target tab. Defaults to active tab.

Returns: The evaluated result as text.

**Requires "Allow User Scripts"**: The user must enable this in the extension settings (`chrome://extensions` → fspec Browser Agent → Details → Allow User Scripts).

**Powerful but use responsibly** — this runs arbitrary code in the page. Useful for extracting data, checking state, or manipulating the DOM when the specialized tools aren't sufficient.

---

### DOM Scanning & Interaction (Iframe-Aware)

These tools form the core **scan → interact → verify** workflow. They are fully iframe-aware — elements inside same-origin and cross-origin iframes are discovered and interactable.

#### `browser_scan_page`
Scan the page's DOM and build an accessibility-tree-like representation with interactive element refs. Automatically discovers and scans elements inside iframes.

```json
{}
{ "interactive": true }
{ "interactive": false, "tabId": 123 }
{ "selector": "#main-content" }
{ "maxFrames": 5 }
```

- `tabId` (number, optional): Tab to scan. Defaults to active tab.
- `interactive` (boolean, optional): Only include interactive elements like buttons, links, inputs. Defaults to `true`.
- `selector` (string, optional): CSS selector to scope the scan to a subtree of the page.
- `maxFrames` (number, optional): Maximum number of iframes to scan. Defaults to 10. Prevents timeout on ad-heavy pages.

Returns: A text representation of the page's accessibility tree with ref labels:
```
- document "Page Title"
  - navigation
    - link "Home" [ref=e1]
    - link "About" [ref=e2]
  - main
    - heading "Welcome"
    - textbox "Search..." [ref=e3]
    - button "Submit" [ref=e4]
    - iframe [src=https://payments.stripe.com/card]
      - textbox "Card Number" [ref=f5e1]
      - textbox "Expiry" [ref=f5e2]
      - textbox "CVC" [ref=f5e3]
      - button "Pay" [ref=f5e4]
```

**Ref format:**
- **Main frame**: `e1`, `e2`, `e3` — simple sequential refs
- **Iframe elements**: `f{frameId}e{N}` — e.g., `f5e1` means frame 5, element 1
- The `@` prefix works for both: `@e1` and `@f5e3`

**Iframe scanning details:**
- Uses `chrome.webNavigation.getAllFrames()` to discover all frames (including nested iframes)
- Each frame is scanned via `chrome.scripting.executeScript()` with `frameIds` targeting
- Works for both same-origin AND cross-origin iframes (extension has `<all_urls>` host permission)
- Sandboxed iframes without `allow-scripts` CAN still be scanned (extension runs in ISOLATED world which bypasses sandbox restrictions)
- Non-scannable frames (`chrome://`, `chrome-extension://`) are skipped gracefully
- `about:blank` and `about:srcdoc` iframes are scanned when they have content
- Same-origin frames are prioritized over cross-origin when `maxFrames` limit applies
- Frames exceeding `maxFrames` appear as `[skipped]` in the tree

#### `browser_click_element`
Click an element on the page by CSS selector or ref. Automatically targets the correct frame for iframe refs.

```json
{ "selector": "#submit-button" }
{ "selector": "@e3" }
{ "selector": "@f5e4" }
{ "selector": ".nav-link:first-child", "tabId": 123 }
```

- `selector` (string, **required**): CSS selector OR ref (e.g., `@e3`, `@f5e1`) of the element to click.
- `tabId` (number, optional): Target tab. Defaults to active tab.

When you pass an iframe ref like `@f5e4`, the extension automatically resolves it to frameId=5 and executes the click within that iframe's context via `chrome.scripting.executeScript({ target: { tabId, frameIds: [5] } })`.

#### `browser_fill_form`
Fill a form input field. Dispatches `input` + `change` events to trigger React/Vue/Angular change handlers. Works across frame boundaries.

```json
{ "selector": "#email", "value": "user@example.com" }
{ "selector": "@e1", "value": "user@example.com" }
{ "selector": "@f5e1", "value": "4242424242424242" }
```

- `selector` (string, **required**): CSS selector OR ref of the input element.
- `value` (string, **required**): Value to set.
- `tabId` (number, optional): Target tab. Defaults to active tab.

#### `browser_diff_page`
Show what changed on the page since the last `browser_scan_page` call. Returns a unified diff of the merged multi-frame accessibility tree with change statistics.

```json
{}
{ "tabId": 123 }
```

- `tabId` (number, optional): Tab to diff. Defaults to active tab.

Returns: A unified diff showing additions (`+`) and removals (`-`) since the last scan:
```
@@ changes @@
- [button] @e4 "Submit"
+ [button] @e4 "Loading..."
+ [alert] "Form submitted successfully"

Stats: 1 added, 1 removed, 0 unchanged
```

Frame-level changes (iframes dynamically created or destroyed) appear as tree-level additions/removals.

**Use this after actions** (clicking, filling forms, navigating) to verify what changed without re-scanning the entire page.

---

## Ref Lifecycle

- Refs (like `@e1`, `@e2`, `@f5e3`) are assigned when you call `browser_scan_page`
- Refs are **ephemeral** — invalidated when the page navigates or undergoes significant DOM changes
- Always re-scan with `browser_scan_page` after navigation or major page changes
- If a ref is not found, you'll get an error — re-scan to get fresh refs
- Each call to `browser_scan_page` replaces all previous refs with new ones
- Iframe refs encode the frameId, so click/fill targets the correct frame automatically

---

## Browser Event Notifications

While connected, you receive real-time notifications via SSE:

| Event | Description |
|-------|-------------|
| `notifications/browser/navigation` | A tab navigated to a new URL. Params: `tabId`, `url`, `title` |
| `notifications/browser/load_complete` | A page finished loading. Params: `tabId`, `url`, `title` |
| `notifications/browser/tab_created` | A new tab was opened. Params: `tabId`, `url` |
| `notifications/browser/tab_closed` | A tab was closed. Params: `tabId` |
| `notifications/tools/list_changed` | WebMCP tool list changed (tool registered or unregistered from a page) |

Use these notifications to stay aware of what the user is doing in the browser and react accordingly.

---

## Common Workflows

### Read a web page
```
1. browser_navigate → URL
2. browser_get_page_content → format: "text"
```

### Interact with page elements using refs
```
1. browser_navigate → URL
2. browser_scan_page → get tree with @e1, @e2, @e3 refs
3. browser_fill_form → { selector: "@e1", value: "user@test.com" }
4. browser_fill_form → { selector: "@e2", value: "password123" }
5. browser_click_element → { selector: "@e3" }
6. browser_diff_page → verify what changed
7. browser_scan_page → re-scan after navigation
```

### Fill a form inside an iframe (e.g., Stripe payment)
```
1. browser_navigate → URL of checkout page
2. browser_scan_page → tree shows iframe content with f{N}e{M} refs
3. browser_fill_form → { selector: "@f5e1", value: "4242424242424242" }
4. browser_fill_form → { selector: "@f5e2", value: "12/28" }
5. browser_fill_form → { selector: "@f5e3", value: "123" }
6. browser_click_element → { selector: "@f5e4" }
7. browser_diff_page → verify payment processing state
```

### Extract structured data from a page
```
1. browser_navigate → URL
2. browser_execute_script → JavaScript that queries DOM and returns JSON
```

### Screenshot a specific element
```
1. browser_navigate → URL
2. browser_scan_page → get tree with @e1, @e2, @e3 refs
3. browser_screenshot → { selector: "@e3" }   # Crops to just that element
```

Or with a CSS selector (no scan needed):
```
1. browser_screenshot → { selector: "#hero-image" }
```

### Work with a specific tab
```
1. browser_list_tabs → find the tab ID you need
2. browser_switch_tab → activate it
3. Use any tool with tabId parameter
```

---

## Injecting JavaScript

`browser_execute_script` runs code in Chrome's **USER_SCRIPT world** — an isolated JavaScript context that shares the DOM but has its own JavaScript globals.

### Basic Usage

```
browser_execute_script({ code: "document.title" })
browser_execute_script({ code: "document.querySelectorAll('a').length" })
browser_execute_script({ code: "JSON.stringify(Array.from(document.querySelectorAll('h2')).map(h => h.textContent))" })
```

### MAIN World vs USER_SCRIPT World

There are two JavaScript execution worlds:

| World | Access | How to reach |
|-------|--------|--------------|
| **USER_SCRIPT** | DOM access, isolated globals | `browser_execute_script` directly |
| **MAIN** | Page's own JavaScript globals, `navigator.modelContext` | Inject a `<script>` tag from USER_SCRIPT |

**Why it matters:** `navigator.modelContext.registerTool()` only works in MAIN world. Code run via `browser_execute_script` runs in USER_SCRIPT world and cannot see MAIN world globals like `window.fetch` interceptors, React state, or page-defined functions.

### Reaching the MAIN World

To execute code in the page's MAIN world, inject a `<script>` tag:

```javascript
// This runs in USER_SCRIPT world (via browser_execute_script)
// but creates a <script> tag that executes in MAIN world
(() => {
  const script = document.createElement('script');
  script.textContent = `
    // This code runs in MAIN world — has access to page globals
    console.log(window.myAppState);
  `;
  document.head.appendChild(script);
  script.remove(); // Clean up — code already executed
  return 'Injected into MAIN world';
})();
```

### Cross-World Data Sharing

Since MAIN world and USER_SCRIPT world are isolated, use the **DOM as a bridge**:

**MAIN → USER_SCRIPT (reading page globals):**

```javascript
// In browser_execute_script (USER_SCRIPT world):
(() => {
  const s = document.createElement('script');
  s.textContent = `
    document.documentElement.setAttribute(
      'data-my-result',
      JSON.stringify(window.__myData || {})
    );
  `;
  document.head.appendChild(s);
  s.remove();

  // Read it back in USER_SCRIPT world
  const data = document.documentElement.getAttribute('data-my-result');
  document.documentElement.removeAttribute('data-my-result');
  return data;
})();
```

**USER_SCRIPT → MAIN (triggering page functions):**

```javascript
// In browser_execute_script (USER_SCRIPT world):
(() => {
  const s = document.createElement('script');
  s.textContent = `
    // Has access to MAIN world globals like fetch interceptors
    window.fetch('/api/data').then(r => r.json()).then(data => {
      document.documentElement.setAttribute('data-fetch-result', JSON.stringify(data));
    });
  `;
  document.head.appendChild(s);
  s.remove();
  return 'Fetch triggered in MAIN world';
})();
```

---

## WebMCP Tools — Website-Registered Tools

Websites using Chrome's WebMCP API (`navigator.modelContext.registerTool()`) expose additional tools dynamically. These appear with the naming pattern:

```
mcp__webmcp__<sanitized-hostname>__<toolName>
```

For example, a travel site at `travel-demo.bandarra.me` registering a `searchFlights` tool would appear as:

```
mcp__webmcp__travel-demo-bandarra-me__searchFlights
```

- WebMCP tools appear and disappear as you navigate between pages
- You receive a `notifications/tools/list_changed` event when tools change
- After receiving that notification, re-list tools to see what's available
- Requires Chrome 146+ with the WebMCP flag enabled

### Interact with WebMCP tools on a website
```
1. browser_navigate → URL of site with WebMCP tools
2. Wait for notifications/tools/list_changed notification
3. Call the discovered tool (e.g., mcp__webmcp__example-com__searchFlights)
```

---

## Injecting Custom WebMCP Tools

You can inject your own MCP-callable tools into any web page at runtime. Tools are registered via `navigator.modelContext.registerTool()` and automatically discovered by the extension.

### Tool Registration API

The `navigator.modelContext.registerTool()` method accepts:

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `name` | `string` | ✅ | Tool name (becomes `mcp__webmcp__<sanitized-hostname>__<name>`) |
| `description` | `string` | ✅ | Human-readable description for the LLM |
| `inputSchema` | `object` | ✅ | JSON Schema describing the tool's parameters |
| `execute` | `function` | ✅ | Callback that receives params and returns a result |

**⚠️ The property is `execute`, NOT `handler`.** Using `handler` will fail.

The `execute` function:
- Receives a single `params` object matching the `inputSchema`
- Must return a **string** (or a value that will be stringified)
- Can be synchronous or return a Promise
- Errors thrown inside `execute` are caught and returned as tool errors

### Critical: Must Inject into MAIN World

`browser_execute_script` runs in USER_SCRIPT world, which has a separate `navigator.modelContext` instance. **Tools registered in USER_SCRIPT world are NOT detected by the extension.**

You must inject via `<script>` tag to reach MAIN world:

```javascript
// In browser_execute_script:
(() => {
  const script = document.createElement('script');
  script.textContent = `
    (() => {
      // Guard against double-injection
      if (window.__myToolInstalled) return;
      window.__myToolInstalled = true;

      navigator.modelContext.registerTool({
        name: 'myTool',
        description: 'A custom tool that does something useful on this page',
        inputSchema: {
          type: 'object',
          properties: {
            action: {
              type: 'string',
              description: 'The action to perform'
            }
          }
        },
        execute: (params) => {
          return JSON.stringify({
            action: params.action || 'default',
            pageTitle: document.title,
            pageUrl: location.href
          });
        }
      });
    })();
  `;
  document.head.appendChild(script);
  script.remove();
  return 'Tool injected into MAIN world';
})();
```

### How Extension Discovery Works

The extension uses a layered discovery system to detect tool registrations:

1. **Extension injects discovery script** into each page's MAIN world via `chrome.scripting.executeScript`
2. The discovery script **intercepts `navigator.modelContext.registerTool()`** calls
3. When a tool is registered, it sends a `FSPEC_WEBMCP_TOOL_REGISTERED` message via `window.postMessage`
4. The **content script relay** forwards this to the service worker via `chrome.runtime.sendMessage`
5. The service worker **registers the tool** in its registry and notifies the native host
6. The native host **broadcasts `notifications/tools/list_changed`** to connected MCP clients via SSE

After injection, you'll see:
```
[MCP:webmcp] Server tools list changed — refreshed N tools
```

### Complete Example: API Request Interceptor

This wraps `fetch()` and `XMLHttpRequest` to log all API requests, then exposes a `getApiRequests` tool to retrieve the log.

```
browser_execute_script(tabId: <tabId>, code: `
(() => {
  const script = document.createElement('script');
  script.textContent = \\\`
  (() => {
    if (window.__apiInterceptorInstalled) return;
    window.__apiInterceptorInstalled = true;
    window.__requestLog = [];

    const requestLog = window.__requestLog;

    function record(method, url, responseSize, status, durationMs) {
      requestLog.push({
        method, url, responseSize, status,
        durationMs: Math.round(durationMs),
        timestamp: new Date().toISOString()
      });
    }

    // Wrap fetch()
    if (!window.__fetchWrappedMain) {
      const originalFetch = window.fetch;
      window.fetch = async function (...args) {
        const request = new Request(...args);
        const method = request.method;
        const url = request.url;
        const t0 = performance.now();
        try {
          const response = await originalFetch.apply(this, args);
          const durationMs = performance.now() - t0;
          const clone = response.clone();
          clone.arrayBuffer()
            .then(buf => record(method, url, buf.byteLength, response.status, durationMs))
            .catch(() => record(method, url, null, response.status, durationMs));
          return response;
        } catch (err) {
          record(method, url, null, 0, performance.now() - t0);
          throw err;
        }
      };
      window.__fetchWrappedMain = true;
    }

    // Wrap XMLHttpRequest
    if (!window.__xhrWrappedMain) {
      const XHROpen = XMLHttpRequest.prototype.open;
      const XHRSend = XMLHttpRequest.prototype.send;
      XMLHttpRequest.prototype.open = function (method, url, ...rest) {
        this.__interceptMeta = { method, url: new URL(url, location.href).href };
        return XHROpen.call(this, method, url, ...rest);
      };
      XMLHttpRequest.prototype.send = function (...args) {
        const meta = this.__interceptMeta || { method: '?', url: '?' };
        const t0 = performance.now();
        this.addEventListener('loadend', function () {
          const durationMs = performance.now() - t0;
          let size = null;
          try {
            const cl = this.getResponseHeader('content-length');
            if (cl) { size = parseInt(cl, 10); }
            else if (this.response) {
              if (typeof this.response === 'string') size = new Blob([this.response]).size;
              else if (this.response instanceof ArrayBuffer) size = this.response.byteLength;
              else if (this.response instanceof Blob) size = this.response.size;
            }
          } catch (e) {}
          record(meta.method, meta.url, size, this.status, durationMs);
        });
        return XHRSend.apply(this, args);
      };
      window.__xhrWrappedMain = true;
    }

    // Register the WebMCP tool
    navigator.modelContext.registerTool({
      name: 'getApiRequests',
      description: 'Returns all intercepted API requests (fetch and XHR) with URL, response size, HTTP status, and timing. Pass clear=true to reset the log.',
      inputSchema: {
        type: 'object',
        properties: {
          clear: {
            type: 'boolean',
            description: 'If true, clears the request log after returning results'
          }
        }
      },
      execute: (params) => {
        const log = window.__requestLog || [];
        const summary = {
          totalRequests: log.length,
          requests: log.map(r => ({
            method: r.method,
            url: r.url,
            responseSize: r.responseSize !== null
              ? r.responseSize >= 1024
                ? (r.responseSize / 1024).toFixed(1) + ' KB'
                : r.responseSize + ' B'
              : 'unknown',
            status: r.status,
            durationMs: r.durationMs,
            timestamp: r.timestamp
          }))
        };
        if (params && params.clear) { window.__requestLog.length = 0; }
        return JSON.stringify(summary, null, 2);
      }
    });
  })();
  \\\`;
  document.head.appendChild(script);
  script.remove();
  return 'API interceptor + WebMCP tool injected into MAIN world';
})();
`)
```

After injection and tool discovery:
```
mcp__webmcp__app-example-com__getApiRequests()           # View all captured requests
mcp__webmcp__app-example-com__getApiRequests(clear: true) # View and clear the log
```

### Cleanup / Unregistering Tools

```
browser_execute_script(tabId: <tabId>, code: `
(() => {
  const s = document.createElement('script');
  s.textContent = \`
    if (navigator.modelContext) {
      navigator.modelContext.unregisterTool('myTool');
    }
    window.__myToolInstalled = false;
  \`;
  document.head.appendChild(s);
  s.remove();
  return 'Tool unregistered and state cleaned up';
})();
`)
```

### Tool Ideas

| Tool Name | Description |
|-----------|-------------|
| `getApiRequests` | Log and retrieve all fetch/XHR requests with timing |
| `getConsoleOutput` | Capture and retrieve console.log/error/warn output |
| `getPerformanceMetrics` | Collect Web Vitals (LCP, FID, CLS) |
| `getLocalStorage` | Read/write localStorage entries |
| `getPageStructure` | Return a simplified DOM tree for analysis |
| `getNetworkErrors` | Track and report failed network requests |
| `getFormData` | Extract all form field values on the page |
| `getReactState` | Extract React component state via `__REACT_DEVTOOLS_GLOBAL_HOOK__` |
| `getCookies` | Read document.cookie in structured format |

---

## Troubleshooting

### Connection Issues

- **Connection refused**: The native messaging host isn't running. The user needs the Chrome extension installed and the native host registered (`node extension/host/native-host.mjs --register --extension-id <id>`).
- **Tool call timeout (30s)**: The extension didn't respond. Chrome may be busy, the tab might be on a `chrome://` URL (restricted), or the extension service worker restarted.

### Element Interaction Issues

- **Element not found**: CSS selector didn't match any element. Try `browser_get_page_content` with `format: "html"` to inspect the DOM, or use `browser_execute_script` to query available elements.
- **Ref not found**: The ref (e.g., `@e3`) is stale — the page has navigated or changed since the last `browser_scan_page`. Re-scan to get fresh refs.

### Iframe Issues

- **Iframe content not appearing in scan**: The iframe may be non-scannable (`chrome://`, `chrome-extension://` URLs). The iframe element itself still appears in the tree.
- **Too many iframes**: Use `maxFrames` parameter to limit scanning. Same-origin and larger frames are prioritized.
- **Sandboxed iframe elements missing**: Sandboxed iframes WITHOUT `allow-scripts` CAN still be scanned — the extension's ISOLATED world bypasses sandbox restrictions. If content is truly empty, it's likely the iframe hasn't loaded yet.

### JavaScript Injection Issues

- **Cannot access page globals**: `browser_execute_script` runs in USER_SCRIPT world. Use a `<script>` tag to reach MAIN world (see "Reaching the MAIN World" section above).
- **Cannot access tab**: Some pages (`chrome://`, `chrome-extension://`, Chrome Web Store) block extension access. Navigate to a regular web page instead.
- **CSP blocking inline scripts**: Some pages block inline `<script>` tags. Try injecting via a blob URL:
  ```javascript
  const blob = new Blob([code], { type: 'text/javascript' });
  script.src = URL.createObjectURL(blob);
  ```

### WebMCP Tool Issues

- **Tool not appearing after injection**: Code ran in USER_SCRIPT world, not MAIN world. Wrap in a `<script>` tag.
- **`navigator.modelContext` not available**: The WebMCP flag is not enabled in Chrome, or the page is a `chrome://` URL.
- **Tool lost after page navigation**: Injected tools exist in the page's JavaScript context. A full navigation destroys them — re-inject after navigation.
- **Escaping template literals**: When nesting template literals across `browser_execute_script` → `<script>` tag → tool code, use regular string concatenation inside the `<script>` tag or test with a simple tool first.

---

## Execute Now

Connect to the fspec Browser Agent Chrome Extension at `http://localhost:19876/mcp`:
