# Inject WebMCP Tools — Runtime Tool Registration Skill

Inject custom JavaScript tools into any web page so they become callable MCP tools in this AI session. Tools are registered via Chrome's `navigator.modelContext.registerTool()` API and automatically discovered by the fspec WebMCP extension.

## Prerequisites

- The fspec WebMCP Chrome Extension must be installed and active
- This session must already be connected to WebMCP (run the `webmcp` skill first, or call `ConnectMCP(transport: "http", url: "http://localhost:19876/mcp")`)
- Chrome 146+ with the "WebMCP for testing" flag enabled (`chrome://flags`)
- "Allow User Scripts" enabled for the extension (`chrome://extensions` → fspec WebMCP Bridge → Details)

## Example Invocations

```
/skill inject-webmcp-tools    # Show this guide and inject tools interactively
```

---

## How It Works

The fspec WebMCP extension has a **layered discovery system** that watches for tool registrations in every page:

1. **Extension injects discovery script** into each page's MAIN world via `chrome.scripting.executeScript`
2. The discovery script **intercepts `navigator.modelContext.registerTool()`** calls
3. When a tool is registered, the discovery script sends a `FSPEC_WEBMCP_TOOL_REGISTERED` message via `window.postMessage`
4. The **content script relay** forwards this to the service worker via `chrome.runtime.sendMessage`
5. The service worker **registers the tool** in its registry and notifies the native host
6. The native host **broadcasts `notifications/tools/list_changed`** to connected MCP clients via SSE
7. On the next `tools/list` call, the tool appears as `mcp__webmcp__<sanitized-hostname>__<toolName>` (dots in hostnames become hyphens)

When the agent calls the tool:
1. The MCP server routes the call to the extension via native messaging
2. The service worker sends `FSPEC_INVOKE_TOOL` to the content script on the correct tab
3. The content script forwards it to the MAIN world via `window.postMessage`
4. The discovery script's invocation listener calls the tool's `execute` function
5. The result flows back through the same chain

---

## Critical: MAIN World vs USER_SCRIPT World

`browser_execute_script` runs code in Chrome's **USER_SCRIPT world** — an isolated JavaScript context that shares the DOM but NOT the `navigator.modelContext` instance used by the extension's discovery script.

**Tools registered in the USER_SCRIPT world are NOT detected by the extension.**

To register tools that the extension can discover, you must inject code into the **MAIN world** using a `<script>` tag:

```javascript
// This runs in USER_SCRIPT world (via browser_execute_script)
// but creates a <script> tag that executes in MAIN world
(() => {
  const script = document.createElement('script');
  script.textContent = `
    // This code runs in MAIN world — navigator.modelContext is the real one
    navigator.modelContext.registerTool({
      name: 'myTool',
      description: 'Does something useful',
      inputSchema: { type: 'object', properties: {} },
      execute: (params) => {
        return JSON.stringify({ result: 'hello' });
      }
    });
  `;
  document.head.appendChild(script);
  script.remove(); // Clean up — code already executed
  return 'Tool injected into MAIN world';
})();
```

---

## Tool Registration API

The `navigator.modelContext.registerTool()` method accepts an object with these properties:

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `name` | `string` | ✅ | Tool name (becomes `mcp__webmcp__<sanitized-hostname>__<name>`, dots→hyphens) |
| `description` | `string` | ✅ | Human-readable description for the LLM |
| `inputSchema` | `object` | ✅ | JSON Schema describing the tool's parameters |
| `execute` | `function` | ✅ | Callback that receives params and returns a result |

**⚠️ The property is `execute`, NOT `handler`.** Using `handler` will fail with: `Failed to read the 'execute' property from 'ToolRegistrationParams': Required member is undefined.`

The `execute` function:
- Receives a single `params` object matching the `inputSchema`
- Must return a **string** (or a value that will be stringified)
- Can be synchronous or return a Promise
- Errors thrown inside `execute` are caught and returned as tool errors

To unregister: `navigator.modelContext.unregisterTool('myTool')`

---

## Cross-World Data Sharing

Since MAIN world and USER_SCRIPT world are isolated JavaScript contexts, they cannot share variables directly. Use the **DOM as a bridge**:

### MAIN world → USER_SCRIPT world (reading data)

```javascript
// In browser_execute_script (USER_SCRIPT world):
(() => {
  // Inject script that writes to a DOM attribute from MAIN world
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

### USER_SCRIPT world → MAIN world (triggering actions)

```javascript
// In browser_execute_script (USER_SCRIPT world):
(() => {
  // Inject a <script> tag — its code runs in MAIN world
  const s = document.createElement('script');
  s.textContent = `
    // This has access to MAIN world globals
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

## Step-by-Step: Injecting a Custom Tool

### Step 1: Identify the target tab

```
browser_list_tabs()
```

Note the `tabId` of the page where you want to inject the tool.

### Step 2: Inject the tool via `<script>` tag

Use `browser_execute_script` with the target `tabId`. The code must:
1. Create a `<script>` element (to get into MAIN world)
2. Guard against double-injection
3. Register the tool with `navigator.modelContext.registerTool()`
4. Store any shared state on `window` for the execute callback to access

```
browser_execute_script(tabId: <tabId>, code: `
(() => {
  const script = document.createElement('script');
  script.textContent = \`
    (() => {
      // Guard against double-injection
      if (window.__myToolInstalled) return;
      window.__myToolInstalled = true;

      // Shared state (accessible to execute callback)
      window.__myToolState = { count: 0 };

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
          window.__myToolState.count++;
          return JSON.stringify({
            action: params.action || 'default',
            callCount: window.__myToolState.count,
            pageTitle: document.title,
            pageUrl: location.href
          });
        }
      });
    })();
  \`;
  document.head.appendChild(script);
  script.remove();
  return 'Tool injected';
})();
`)
```

### Step 3: Wait for tool discovery

After injection, the extension detects the registration and sends a `notifications/tools/list_changed` SSE event. You will see a notification like:

```
[MCP:webmcp] Server tools list changed — refreshed 12 tools
```

### Step 4: Call the tool

The tool is now available as `mcp__webmcp__<sanitized-hostname>__myTool` (dots in hostname become hyphens). Call it like any other MCP tool:

```
mcp__webmcp__example-com__myTool(action: "greet")
```

---

## Complete Example: API Request Interceptor

This real-world example wraps `fetch()` and `XMLHttpRequest` to log all API requests made by the page, then exposes a `getApiRequests` tool to retrieve the log.

### Injection Script

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
            responseSizeBytes: r.responseSize,
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

### Usage

After injection and tool discovery:

```
mcp__webmcp__app-example-com__getApiRequests()           # View all captured requests
mcp__webmcp__app-example-com__getApiRequests(clear: true) # View and clear the log
```

---

## Gotchas and Troubleshooting

### Tool not appearing after injection

1. **Wrong world**: Code ran in USER_SCRIPT world, not MAIN world. Wrap in a `<script>` tag.
2. **CSP blocking inline scripts**: Some pages block inline `<script>` tags via Content Security Policy. Try injecting via `script.src` with a blob URL instead:
   ```javascript
   const blob = new Blob([code], { type: 'text/javascript' });
   script.src = URL.createObjectURL(blob);
   ```
3. **`navigator.modelContext` not available**: The WebMCP flag is not enabled in Chrome, or the page is a `chrome://` URL.
4. **Double injection guard**: If you see "Already installed", reset the guard first:
   ```javascript
   window.__myToolInstalled = false;
   ```

### "Tool names must be unique" error on reconnect

If `ConnectMCP` is called again after tools are already registered, the extension properly handles this. But if the error persists, unregister the tool first from the page:

```
browser_execute_script(tabId: <tabId>, code: `
(() => {
  const s = document.createElement('script');
  s.textContent = 'navigator.modelContext.unregisterTool("myTool");';
  document.head.appendChild(s);
  s.remove();
  return 'Tool unregistered';
})();
`)
```

### Tool lost after page navigation

Tools are registered in the page's JavaScript context. If the page navigates (full reload), all injected tools are lost. The extension detects this and removes them from the registry. You must re-inject after navigation.

### Reading MAIN world state from the agent

Use the DOM bridge pattern described above. You cannot directly access `window.__myData` from `browser_execute_script` — it runs in a different world.

### Escaping template literals

When nesting template literals across `browser_execute_script` → `<script>` tag → tool code, escaping gets complex. Strategy:
- Use regular string concatenation inside the `<script>` tag instead of template literals
- Or use `\\` to escape backticks at each nesting level
- Test with a simple tool first, then add complexity

---

## Tool Ideas

| Tool Name | Description |
|-----------|-------------|
| `getApiRequests` | Log and retrieve all fetch/XHR requests with timing |
| `getConsoleOutput` | Capture and retrieve console.log/error/warn output |
| `getPerformanceMetrics` | Collect Web Vitals (LCP, FID, CLS) |
| `getLocalStorage` | Read/write localStorage entries |
| `getPageStructure` | Return a simplified DOM tree for analysis |
| `getNetworkErrors` | Track and report failed network requests |
| `getFormData` | Extract all form field values on the page |
| `getAccessibilityTree` | Return ARIA roles and labels for a11y audit |
| `getReactState` | Extract React component state via `__REACT_DEVTOOLS_GLOBAL_HOOK__` |
| `getCookies` | Read document.cookie in structured format |

---

## Cleanup

To remove an injected tool:

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
