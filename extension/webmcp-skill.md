# Browser Agent — Browser Control Skill

Connect this AI session to the fspec Browser Agent Chrome Extension, giving you direct control over the user's Chrome browser and access to any WebMCP tools registered by websites.

## Action Required

When this skill is referenced, immediately connect to the WebMCP extension:

```
ConnectMCP(transport: "http", url: "http://localhost:19876/mcp")
```

If a custom URL is provided as a parameter, use that instead of the default.

## How to Use This Skill

Reference this file with `@` in your prompt to load the skill:

```
@extension/webmcp-skill.md                                         # Connect to http://localhost:19876/mcp
@extension/webmcp-skill.md please connect to http://localhost:8080/mcp  # Connect to custom URL
```

---

## What You Get After Connecting

Once connected, you have **14 native browser control tools** plus any **WebMCP tools** that websites have registered via `navigator.modelContext.registerTool()`.

You will also receive **real-time browser event notifications** via SSE — tab navigation, page loads, tab creation, and tab closure.

---

## Native Browser Control Tools

These tools are always available after connection:

### `browser_navigate`
Navigate a tab to a URL. Waits for page load to complete before returning.

```json
{ "url": "https://example.com" }
{ "url": "https://example.com", "tabId": 123 }
```

- `url` (string, **required**): The URL to navigate to
- `tabId` (number, optional): Target tab ID. Defaults to the active tab.

Returns: `{ "url": "...", "title": "..." }`

### `browser_screenshot`
Capture a PNG screenshot of a tab.

```json
{}
{ "tabId": 123 }
{ "tabId": 123, "fullPage": true }
```

- `tabId` (number, optional): Tab to capture. Defaults to active tab.
- `fullPage` (boolean, optional): If true, captures the full scrollable page instead of just the visible viewport.

Returns: An image content block (base64 PNG). You will see the screenshot directly.

### `browser_list_tabs`
List all open browser tabs with their IDs, URLs, titles, and active state.

```json
{}
```

No parameters required.

Returns: Array of `{ "id": number, "url": string, "title": string, "active": boolean }`

**Use this first** to discover tab IDs for targeting other tools at specific tabs.

### `browser_execute_script`
Execute arbitrary JavaScript in a tab's page context using the USER_SCRIPT world.

```json
{ "code": "document.title" }
{ "code": "document.querySelectorAll('a').length", "tabId": 123 }
```

- `code` (string, **required**): JavaScript code to execute. The return value is sent back.
- `tabId` (number, optional): Target tab. Defaults to active tab.

Returns: The evaluated result as text.

**Requires "Allow User Scripts"**: This tool uses `chrome.userScripts.execute()` to bypass Content Security Policy restrictions. The user must enable "Allow User Scripts" in the extension settings (`chrome://extensions` → fspec Browser Agent → Details → Allow User Scripts). If the toggle is not enabled, the tool returns an error with instructions.

**Powerful but use responsibly** — this runs arbitrary code in the page. Useful for extracting data, checking state, or manipulating the DOM when the specialized tools aren't sufficient.

### `browser_switch_tab`
Activate a tab and focus its window.

```json
{ "tabId": 123 }
```

- `tabId` (number, **required**): The tab ID to switch to.

Returns: `{ "switched": true, "tabId": 123, "url": "...", "title": "..." }`

### `browser_close_tab`
Close a browser tab.

```json
{ "tabId": 123 }
```

- `tabId` (number, **required**): The tab ID to close.

Returns: `{ "closed": true, "tabId": 123, "url": "...", "title": "..." }`

### `browser_get_page_content`
Get the text content or full HTML of a page.

```json
{}
{ "format": "html" }
{ "format": "text", "tabId": 123 }
```

- `tabId` (number, optional): Target tab. Defaults to active tab.
- `format` (string, optional): `"text"` (default) for `innerText`, `"html"` for full `outerHTML`.

Returns: `{ "title": "...", "url": "...", "content": "..." }`

**Use `"text"` format** for reading page content (much smaller payload). Only use `"html"` when you need DOM structure.

### `browser_click_element`
Click an element on the page by CSS selector or ref.

```json
{ "selector": "#submit-button" }
{ "selector": "@e3" }
{ "selector": ".nav-link:first-child", "tabId": 123 }
```

- `selector` (string, **required**): CSS selector OR ref (e.g., `@e3` from `browser_scan_page`) of the element to click.
- `tabId` (number, optional): Target tab. Defaults to active tab.

Returns: `{ "clicked": true, "selector": "..." }` or an error if the element is not found.

**Tip:** Use `browser_scan_page` to get refs like `@e1`, `@e2`, `@e3`, then pass them as the `selector` — no need to figure out CSS selectors manually.

### `browser_fill_form`
Fill a form input field and dispatch `input` + `change` events (triggering React/Vue/Angular change handlers).

```json
{ "selector": "#email", "value": "user@example.com" }
{ "selector": "@e1", "value": "user@example.com" }
{ "selector": "input[name='search']", "value": "query", "tabId": 123 }
```

- `selector` (string, **required**): CSS selector OR ref (e.g., `@e1` from `browser_scan_page`) of the input element.
- `value` (string, **required**): Value to set.
- `tabId` (number, optional): Target tab. Defaults to active tab.

Returns: `{ "filled": true, "selector": "...", "value": "..." }` or an error if not found.

**Tip:** Use `browser_scan_page` to get refs like `@e1`, `@e2`, then pass them as the `selector` instead of writing CSS selectors.

### `browser_go_back`
Navigate the tab back in browser history.

```json
{}
{ "tabId": 123 }
```

- `tabId` (number, optional): Target tab. Defaults to active tab.

### `browser_go_forward`
Navigate the tab forward in browser history.

```json
{}
{ "tabId": 123 }
```

- `tabId` (number, optional): Target tab. Defaults to active tab.

### `browser_create_tab`
Create a new browser tab, optionally navigating to a URL. Waits for page load when a URL is provided.

```json
{}
{ "url": "https://example.com" }
{ "url": "https://example.com", "active": false }
{ "url": "https://example.com", "pinned": true }
{ "url": "https://example.com", "windowId": 1 }
```

- `url` (string, optional): URL to open. Defaults to New Tab page.
- `active` (boolean, optional): Whether to make it the active tab. Defaults to true.
- `windowId` (number, optional): Window to create the tab in. Defaults to current window.
- `pinned` (boolean, optional): Whether to pin the tab. Defaults to false.

Returns: `{ "tabId": 42, "url": "...", "title": "...", "active": true, "windowId": 1 }`

**Use this instead of `browser_execute_script` with `window.open()`** — Chrome's popup blocker blocks `window.open()` without a user gesture, but `browser_create_tab` uses `chrome.tabs.create()` which always works.

### `browser_scan_page`
Scan the active tab's DOM and build an accessibility-tree-like representation with interactive element refs. Use refs (e.g., `@e1`, `@e2`) with `browser_click_element` and `browser_fill_form` instead of writing CSS selectors.

```json
{}
{ "interactive": true }
{ "interactive": false, "tabId": 123 }
{ "selector": "#main-content" }
```

- `tabId` (number, optional): Tab to scan. Defaults to active tab.
- `interactive` (boolean, optional): Only include interactive elements like buttons, links, inputs. Defaults to `true`.
- `selector` (string, optional): CSS selector to scope the scan to a subtree of the page.

Returns: A text representation of the page's accessibility tree with ref labels:
```
[document] Page Title
  [navigation]
    [link] @e1 "Home"
    [link] @e2 "About"
  [main]
    [heading] "Welcome"
    [textbox] @e3 "Search..."
    [button] @e4 "Submit"
```

Use the `@e1`, `@e2`, etc. refs in subsequent `browser_click_element` and `browser_fill_form` calls:
- `browser_click_element({ selector: "@e2" })` — clicks the "About" link
- `browser_fill_form({ selector: "@e3", value: "query" })` — fills the search box

### `browser_diff_page`
Show what changed on the page since the last `browser_scan_page` call. Returns a unified diff of the accessibility tree with change statistics.

```json
{}
{ "tabId": 123 }
```

- `tabId` (number, optional): Tab to diff. Defaults to active tab.

Returns: A unified diff showing additions (`+`) and removals (`-`) since the last scan, plus summary stats:
```
@@ changes @@
- [button] @e4 "Submit"
+ [button] @e4 "Loading..."
+ [alert] "Form submitted successfully"

Stats: 1 added, 1 removed, 0 unchanged
```

**Use this after actions** (clicking, filling forms, navigating) to verify what changed without re-scanning the entire page.

---

## WebMCP Website Tools

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
- WebMCP tools require Chrome 146+ with the WebMCP flag enabled (`chrome://flags` → "WebMCP for testing" → Enabled)

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

### Fill and submit a form
```
1. browser_navigate → URL
2. browser_fill_form → selector, value (repeat for each field)
3. browser_click_element → submit button selector
4. browser_screenshot → verify result
```

### Extract structured data from a page
```
1. browser_navigate → URL
2. browser_execute_script → JavaScript that queries DOM and returns JSON
```

### Work with a specific tab
```
1. browser_list_tabs → find the tab ID you need
2. browser_switch_tab → activate it
3. Use any tool with tabId parameter
```

### Interact with WebMCP tools on a website
```
1. browser_navigate → URL of site with WebMCP tools
2. Wait for notifications/tools/list_changed notification
3. Call the discovered tool (e.g., mcp__webmcp__example-com__searchFlights)
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

---

## Ref Lifecycle

- Refs (like `@e1`, `@e2`, `@e3`) are assigned when you call `browser_scan_page`
- Refs are **ephemeral** — they're invalidated when the page navigates or undergoes significant DOM changes
- Always re-scan with `browser_scan_page` after navigation or major page changes
- If a ref is not found, you'll get an error — re-scan to get fresh refs
- Each call to `browser_scan_page` replaces all previous refs with new ones

---

## Troubleshooting

- **Connection refused**: The native messaging host isn't running. The user needs to have the Chrome extension installed and the native host registered (`node extension/host/native-host.mjs --register --extension-id <id>`).
- **Tool call timeout (30s)**: The extension didn't respond. Chrome may be busy, the tab might be on a `chrome://` URL (restricted), or the extension service worker restarted.
- **Element not found**: CSS selector didn't match any element. Try `browser_get_page_content` with `format: "html"` to inspect the DOM, or use `browser_execute_script` to query available elements.
- **Ref not found**: The ref (e.g., `@e3`) is stale — the page has navigated or changed since the last `browser_scan_page`. Re-scan with `browser_scan_page` to get fresh refs.
- **Cannot access tab**: Some pages (`chrome://`, `chrome-extension://`, Chrome Web Store) block extension access. Navigate to a regular web page instead.

---

## Execute Now

Connect to the fspec Browser Agent Chrome Extension at `http://localhost:19876/mcp`:
