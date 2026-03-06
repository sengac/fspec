# WebMCP Chrome Extension Research

> **Work Unit:** EXT-001 — fspec WebMCP Chrome Extension
> **Date:** 2026-03-05
> **Status:** Research complete, reviewed and corrected against online specs (2026-03-05)
> **Corrections:** Removed fabricated declarative API details (§2.5), fixed MCP spec version (§4.4), corrected SSE lifecycle description (§4.4), added content script world isolation architecture (§3.4, §8.2, §8.3)

---

## Table of Contents

1. [What is WebMCP](#1-what-is-webmcp)
2. [WebMCP API Reference](#2-webmcp-api-reference)
3. [Chrome Extension Architecture (Manifest V3)](#3-chrome-extension-architecture-manifest-v3)
4. [MCP Transport for Browser Extensions](#4-mcp-transport-for-browser-extensions)
5. [Bidirectional Communication Design](#5-bidirectional-communication-design)
6. [Existing Implementations (Prior Art)](#6-existing-implementations-prior-art)
7. [Extension Development & Testing Guide](#7-extension-development--testing-guide)
8. [Proposed Architecture for fspec Extension](#8-proposed-architecture-for-fspec-extension)
9. [Tool Catalog](#9-tool-catalog)
10. [ConnectMCP Integration](#10-connectmcp-integration)

---

## 1. What is WebMCP

WebMCP (Web Model Context Protocol) is a **W3C Community Group draft standard** jointly developed by Google and Microsoft. It enables web pages to expose structured "tools" to AI agents through a browser-native JavaScript API: `navigator.modelContext`.

### Key Concepts

- **Web pages become MCP servers** — instead of running MCP on a backend, the page registers tools in client-side JavaScript
- **Two registration modes:**
  - **Imperative API** — `navigator.modelContext.registerTool()` with JavaScript callbacks
  - **Declarative API** — HTML `<form>` elements with `toolname` and `tooldescription` attributes
- **Human-in-the-loop** — WebMCP is designed for collaborative workflows, NOT headless automation
- **Requires visible browsing context** — no headless mode support

### Current Status (March 2026)

- Available in **Chrome 146+** behind `chrome://flags/#web-mcp` ("WebMCP for testing" flag)
- Chrome Canary channel has it enabled by default
- W3C Web Machine Learning Community Group is iterating on the spec
- Spec URL: https://webmachinelearning.github.io/webmcp/

### WebMCP vs Server-Side MCP

| Aspect | Server-Side MCP | WebMCP |
|--------|----------------|--------|
| Where tools run | Backend server process | Browser tab (client-side JS) |
| Transport | stdio / Streamable HTTP | In-browser API (`navigator.modelContext`) |
| Auth context | API keys, tokens | User's browser session (cookies, auth) |
| Headless | Yes | No — requires visible tab |
| Discovery | Client connects to known server | Extension/agent discovers tools on visited pages |

---

## 2. WebMCP API Reference

### 2.1 Navigator Interface Extension

```typescript
// The browser exposes this on navigator
interface Navigator {
  readonly modelContext: ModelContext;
}
```

### 2.2 ModelContext Interface

```typescript
interface ModelContext {
  // Replace all registered tools with a new set
  provideContext(options?: ModelContextOptions): void;

  // Remove all registered tools
  clearContext(): void;

  // Register a single tool (throws if name already exists)
  registerTool(tool: ModelContextTool): void;

  // Remove a tool by name (throws if not found)
  unregisterTool(name: string): void;
}
```

### 2.3 ModelContextTool Dictionary

```typescript
interface ModelContextTool {
  name: string;           // Unique tool identifier
  description: string;    // Natural language description for AI
  inputSchema?: object;   // JSON Schema for input parameters
  execute: ToolExecuteCallback;  // Called when agent invokes tool
  annotations?: ToolAnnotations;
}

interface ToolAnnotations {
  readOnlyHint?: boolean; // true = tool doesn't modify state
}

type ToolExecuteCallback = (
  input: object,
  client: ModelContextClient
) => Promise<any>;
```

### 2.4 ModelContextClient Interface

```typescript
interface ModelContextClient {
  // Request user interaction during tool execution
  requestUserInteraction(
    callback: () => Promise<any>
  ): Promise<any>;
}
```

### 2.5 Declarative API (HTML Forms)

```html
<form toolname="searchFlights"
      tooldescription="Search for available flights">
  <input name="origin" placeholder="Origin airport">
  <input name="destination" placeholder="Destination">
  <button type="submit">Search</button>
</form>
```

The browser automatically:
1. Synthesizes a JSON Schema from form fields based on `<input>` names and types
2. Pre-fills form fields when an agent invokes the tool

> **Note:** The declarative API is still evolving. The verified attributes are
> `toolname` (required — marks the form as a WebMCP tool) and `tooldescription`
> (required — natural language description). Some sources also reference
> `toolparamdescription` for per-field descriptions. The exact submit/response
> mechanics (how the agent receives the form result) are not yet fully specified
> in the W3C draft as of March 2026 — the spec notes these steps are "internal"
> and "have not been defined yet."

### 2.6 Example: Registering an Imperative Tool

```javascript
navigator.modelContext.registerTool({
  name: 'searchFlights',
  description: 'Search for flights between airports',
  inputSchema: {
    type: 'object',
    properties: {
      origin: { type: 'string', description: 'Origin code' },
      destination: { type: 'string', description: 'Dest code' },
      date: { type: 'string', description: 'YYYY-MM-DD' }
    },
    required: ['origin', 'destination', 'date']
  },
  execute: async (input, client) => {
    const results = await fetch(
      `/api/flights?from=${input.origin}&to=${input.destination}&date=${input.date}`
    );
    return await results.json();
  }
});
```


---

## 3. Chrome Extension Architecture (Manifest V3)

### 3.1 MV3 Key Concepts

Manifest V3 is the current Chrome extension platform. Key differences from MV2:

| Component | MV2 | MV3 |
|-----------|-----|-----|
| Background | Persistent page | **Service Worker** (event-driven, no DOM) |
| Remote code | Allowed | **Blocked** (CSP enforced) |
| Permissions | `permissions` | `permissions` + `host_permissions` |
| Networking | `XMLHttpRequest` | **`fetch()` only** |
| Module system | Scripts | **ES modules supported** |

### 3.2 Extension Components

```
extension/
├── manifest.json          # Extension manifest (MV3)
├── src/
│   ├── background/
│   │   └── service-worker.ts   # Service worker (event hub)
│   ├── content/
│   │   └── content-script.ts   # Injected into web pages
│   ├── popup/
│   │   ├── popup.html          # Extension popup UI
│   │   └── popup.ts            # Popup logic
│   ├── server/
│   │   └── mcp-server.ts       # MCP Streamable HTTP server
│   └── types/
│       └── index.ts            # Shared type definitions
├── package.json
├── tsconfig.json
└── vite.config.ts              # Build config
```

### 3.3 Manifest V3 Structure

```json
{
  "manifest_version": 3,
  "name": "fspec WebMCP Bridge",
  "version": "0.1.0",
  "description": "Bridge WebMCP tools to MCP for AI agent interaction",
  "permissions": [
    "activeTab",
    "tabs",
    "scripting",
    "storage",
    "offscreen",
    "nativeMessaging"
  ],
  "host_permissions": ["<all_urls>"],
  "background": {
    "service_worker": "dist/service-worker.js",
    "type": "module"
  },
  "content_scripts": [
    {
      "matches": ["<all_urls>"],
      "js": ["dist/content-script.js"],
      "run_at": "document_idle"
    }
  ],
  "action": {
    "default_popup": "popup.html",
    "default_icon": {
      "16": "icons/icon16.png",
      "48": "icons/icon48.png",
      "128": "icons/icon128.png"
    }
  },
  "icons": {
    "16": "icons/icon16.png",
    "48": "icons/icon48.png",
    "128": "icons/icon128.png"
  }
}
```

### 3.4 Communication Patterns in MV3

> **Important: Content Script World Isolation**
>
> Content scripts run in an **isolated world** — they share the page's DOM but
> NOT the page's JavaScript context. `navigator.modelContext` is only accessible
> from the page's **main world**. To bridge this gap, use
> `chrome.scripting.executeScript()` with `world: 'MAIN'` to inject scripts
> that can access page-level APIs, then relay data back via `window.postMessage()`.

```
┌──────────────┐     chrome.runtime      ┌─────────────────┐
│Content Script │ ──────────────────────> │  Service Worker  │
│ (per tab)     │ <────────────────────── │  (background)    │
└──────────────┘    .sendMessage()        └─────────────────┘
                    .onMessage                    │
                                                  │ chrome.runtime
                                                  │ .connect()
                                          ┌───────▼─────────┐
                                          │  Native Messaging│
                                          │  Host (Node.js)  │
                                          └─────────────────┘
                                                  │
                                           HTTP server
                                           (port 19876)
                                                  │
                                          ┌───────▼─────────┐
                                          │   ConnectMCP     │
                                          │   (fspec agent)  │
                                          └─────────────────┘
```

---

## 4. MCP Transport for Browser Extensions

### 4.1 The Problem

Chrome MV3 service workers **cannot open TCP server sockets**. They have no access to `net.createServer()` or any raw socket API. This means the MCP Streamable HTTP server cannot run directly in the service worker.

### 4.2 Solutions (Ranked by Reliability)

#### Option A: Native Messaging Host (Recommended) ✅

A small Node.js process registered as a Chrome Native Messaging Host. This is the pattern used by `mcp-chrome` (the most popular Chrome MCP extension).

**How it works:**
1. Node.js script is registered as a native messaging host via a JSON manifest
2. Chrome launches it on demand when extension calls `chrome.runtime.connectNative()`
3. The Node.js process runs the HTTP server (Streamable HTTP on port 19876)
4. Extension ↔ Node.js communication via stdin/stdout (native messaging protocol)
5. Agent connects to `http://localhost:19876/mcp` via ConnectMCP

**Pros:** Full Node.js API, can open ports, reliable, battle-tested pattern
**Cons:** Requires separate installation step (`npm install -g`), extra process

**Native Messaging Host Manifest** (`com.fspec.webmcp.json`):
```json
{
  "name": "com.fspec.webmcp",
  "description": "fspec WebMCP MCP Bridge",
  "path": "/usr/local/bin/fspec-webmcp-host",
  "type": "stdio",
  "allowed_origins": [
    "chrome-extension://EXTENSION_ID_HERE/"
  ]
}
```

**Manifest location:**
- macOS: `~/Library/Application Support/Google/Chrome/NativeMessagingHosts/`
- Linux: `~/.config/google-chrome/NativeMessagingHosts/`
- Windows: Registry key `HKCU\Software\Google\Chrome\NativeMessagingHosts\`

#### Option B: Offscreen Document

Chrome MV3's `chrome.offscreen` API allows creating hidden DOM pages.

**How it works:**
1. Service worker creates offscreen document: `chrome.offscreen.createDocument()`
2. Offscreen document runs the HTTP server using a library like `@anthropic/mcp-sdk`
3. Communication via `chrome.runtime.sendMessage()`

**Pros:** No external process, all within the extension
**Cons:** Offscreen documents have limited lifetime, Chrome may close them. Cannot use Node.js APIs (still browser JS). HTTP server in browser JS is non-trivial.

#### Option C: WebSocket Reverse Bridge

The extension connects OUT to a known WebSocket endpoint rather than hosting a server.

**How it works:**
1. Extension connects to `ws://localhost:19876` as a WebSocket client
2. A small relay process translates WebSocket ↔ MCP Streamable HTTP
3. ConnectMCP connects to the relay's HTTP endpoint

**Pros:** Service worker can initiate WebSocket connections
**Cons:** Extra relay layer, more complexity, less direct

### 4.3 Recommended Approach: Option A (Native Messaging Host)

The native messaging host approach is recommended because:
- It's the proven pattern (mcp-chrome uses it successfully)
- Full Node.js API access for the HTTP server
- Clean separation of concerns
- The host can be distributed as an npm package
- Installation can be automated via a `register` command

### 4.4 Streamable HTTP MCP Protocol

The MCP Streamable HTTP transport (spec version 2025-03-26, with latest revision 2025-11-25) works as follows:

```
Agent (Client)                    Extension (Server)
     │                                   │
     │─── POST /mcp ──────────────────>  │  Initialize request
     │<── 200 JSON (+ Mcp-Session-Id) ─  │  Returns InitializeResult
     │                                   │
     │─── GET /mcp ───────────────────>  │  Open SSE stream for notifications
     │<── 200 text/event-stream ──────   │  Persistent SSE channel
     │                                   │
     │─── POST /mcp ──────────────────>  │  tools/list request
     │<── 200 JSON response ───────────  │  Returns tool definitions
     │                                   │
     │                                   │  (browser event happens)
     │<── SSE: notification ───────────  │  Server pushes via GET SSE stream
     │                                   │
     │─── POST /mcp ──────────────────>  │  tools/call request
     │<── 200 JSON response ───────────  │  Returns tool result
     │                                   │
     │─── DELETE /mcp ────────────────>  │  Session termination
     │<── 200 ─────────────────────────  │  Cleanup
```

**Key details for bidirectionality:**

1. **POST responses** can return either `application/json` (single response) or
   `text/event-stream` (SSE stream). However, POST-scoped SSE streams are tied
   to that request and SHOULD close after all responses are sent. They are NOT
   persistent.

2. **GET-based SSE stream** is the mechanism for persistent server→client
   notifications. The client issues `GET /mcp` with `Accept: text/event-stream`,
   and the server keeps this stream open to push JSON-RPC notifications and
   requests at any time. This is how browser events reach the agent.

3. **Session management** uses the `Mcp-Session-Id` header. The server assigns
   it in the initialize response, and the client includes it on all subsequent
   requests.

4. **Stream resumability** is optional — servers MAY attach SSE event IDs, and
   clients MAY reconnect with `Last-Event-ID`.

---

## 5. Bidirectional Communication Design

### 5.1 Agent → Browser (Tool Calls)

Standard MCP flow: agent calls tools, extension executes them.

```
Agent calls: mcp__ext__browser_navigate({url: "https://example.com"})
     │
     ├── ConnectMCP routes to cached HTTP connection
     ├── POST /mcp with JSON-RPC tools/call
     ├── Native messaging host receives request
     ├── Forwards to service worker via native messaging
     ├── Service worker calls chrome.tabs.update()
     ├── Result sent back through the chain
     └── Agent receives: {title: "Example", url: "https://example.com"}
```

### 5.2 Browser → Agent (Server-Initiated Notifications)

This is the critical bidirectional piece. The extension fires events back to the agent through the MCP SSE channel.

```
User navigates to new page
     │
     ├── chrome.tabs.onUpdated listener fires in service worker
     ├── Service worker sends to native messaging host
     ├── Host writes JSON-RPC notification to SSE stream:
     │   {
     │     "jsonrpc": "2.0",
     │     "method": "notifications/browser/navigation",
     │     "params": {
     │       "tabId": 123,
     │       "url": "https://new-page.com",
     │       "title": "New Page"
     │     }
     │   }
     ├── ConnectMCP receives via SSE
     ├── rmcp ClientHandler callback fires
     └── Notification injected into session via watcher_input_tx
```

### 5.3 Event Types to Support

| Event | MCP Method | Trigger |
|-------|-----------|---------|
| Page navigation | `notifications/browser/navigation` | `chrome.tabs.onUpdated` |
| Tab created | `notifications/browser/tab_created` | `chrome.tabs.onCreated` |
| Tab closed | `notifications/browser/tab_closed` | `chrome.tabs.onRemoved` |
| WebMCP tools changed | `notifications/tools/list_changed` | Main-world injected script detects tool changes (monkey-patched `registerTool`/`unregisterTool` or polling) |
| Console error | `notifications/browser/console_error` | Main-world injected script intercepts `console.error` |
| Network request | `notifications/browser/network` | `chrome.webRequest` listeners |
| Page load complete | `notifications/browser/load_complete` | `chrome.webNavigation.onCompleted` |

### 5.4 ConnectMCP Integration for Notifications

The existing ConnectMCP implementation (MCP-001) already supports server-initiated notifications:

1. **`notifications/tools/list_changed`** — rmcp's `ClientHandler::on_tool_list_changed` callback automatically re-fetches `tools/list` and updates cached tools
2. **Custom notifications** — rmcp's `ClientHandler::on_notification` receives any JSON-RPC notification and can inject it into the session via `watcher_input_tx`

The extension should use `notifications/tools/list_changed` for WebMCP tool registry changes, and custom `notifications/browser/*` methods for browser events.

---

## 6. Existing Implementations (Prior Art)

### 6.1 mcp-chrome (hangwin/mcp-chrome)

**Most mature Chrome MCP extension.** Architecture closely matches what we need.

- **Extension:** Chrome MV3, content scripts for page interaction
- **Bridge:** `mcp-chrome-bridge` npm package (native messaging host)
- **Transport:** Streamable HTTP on port 12306
- **Tools:** 20+ tools (navigate, screenshot, click, fill, network capture, history, bookmarks)
- **Install:** `npm install -g mcp-chrome-bridge` → registers native messaging host

**Key learnings:**
- Native messaging host pattern is production-proven
- The bridge registers itself during `npm postinstall`
- Extension communicates with bridge via `chrome.runtime.connectNative()`
- Bridge exposes both Streamable HTTP and stdio MCP transports

### 6.2 MCP-B / WebMCP-org (MiguelsPizza/WebMCP)

**Reference implementation of the WebMCP polyfill.**

- **Package:** `@mcp-b/global` polyfills `navigator.modelContext` for browsers without native support
- **Extension:** MCP-B Extension acts as client that discovers WebMCP tools
- **Transport:** Tab transports (postMessage), Extension transports (chrome.runtime)
- **Native server:** `@mcp-b/native-server` bridges to local MCP clients

**Key learnings:**
- Shows how to discover WebMCP tools from an extension
- Tab transport uses `postMessage` for in-tab communication
- The extension injects a client that discovers tools registered by pages
- Native server bridges the extension to external MCP clients

### 6.3 Chrome DevTools MCP Server

**Google's official MCP server for Chrome DevTools.**

- Exposes debugging capabilities (DOM inspection, network, console, performance)
- Connects via Chrome DevTools Protocol (CDP)
- More developer-focused than user-focused

### 6.4 Key Differences for fspec Extension

Our extension uniquely combines:
1. **WebMCP tool discovery** (like MCP-B) — discover tools registered by websites
2. **Browser control tools** (like mcp-chrome) — navigate, screenshot, click
3. **Bidirectional events** — push browser events to agent via SSE notifications
4. **ConnectMCP native integration** — agent connects dynamically, not via config file

---

## 7. Extension Development & Testing Guide

### 7.1 Development Setup

```bash
# Create extension directory
mkdir -p extension/src/{background,content,popup,server,types}
cd extension

# Initialize package.json
npm init -y

# Install dev dependencies
npm install -D typescript vite @anthropic-ai/sdk
npm install -D @anthropic-ai/mcp-sdk  # For MCP protocol types
npm install -D @anthropic-ai/mcp-server  # For server implementation
```

### 7.2 Building the Extension

Use Vite with multiple entry points:

```typescript
// vite.config.ts
import { defineConfig } from 'vite';
import { resolve } from 'path';

export default defineConfig({
  build: {
    rollupOptions: {
      input: {
        'service-worker': resolve(__dirname, 'src/background/service-worker.ts'),
        'content-script': resolve(__dirname, 'src/content/content-script.ts'),
        'popup': resolve(__dirname, 'src/popup/popup.ts'),
      },
      output: {
        dir: 'dist',
        entryFileNames: '[name].js',
        format: 'es',
      },
    },
    outDir: 'dist',
    emptyOutDir: true,
  },
});
```

### 7.3 Loading as Developer Extension

1. Open Chrome and go to `chrome://extensions/`
2. Enable **"Developer mode"** (toggle in top right)
3. Click **"Load unpacked"**
4. Select the `extension/` directory (must contain `manifest.json`)
5. Extension appears in toolbar with your icon

### 7.4 Debugging

**Service Worker:**
- Go to `chrome://extensions/`
- Find your extension → click "Service Worker" link
- Opens DevTools for the service worker (console, sources, network)

**Content Scripts:**
- Open DevTools on any web page (F12)
- Content script logs appear in the page's console
- Source files appear under "Content Scripts" in Sources panel

**Popup:**
- Right-click extension icon → "Inspect popup"
- Opens DevTools for the popup page

### 7.5 Hot Reload During Development

Install `chrome-extension-reloader` or use manual reload:
- Any file change → go to `chrome://extensions/` → click reload (↻) on your extension
- Or use the `Extensions Reloader` extension for auto-reload

For automated development:
```bash
# Watch mode — rebuild on file changes
npx vite build --watch

# Then manually reload extension in Chrome
```

### 7.6 Enabling WebMCP Flag

For WebMCP tool discovery to work:

1. Open `chrome://flags/#web-mcp`
2. Set "WebMCP for testing" to **Enabled**
3. Relaunch Chrome

**Note:** This flag is only available in Chrome 146+ (currently Canary/Dev channels). Browser control tools work without this flag.

### 7.7 Testing with the Travel Demo

Google provides a WebMCP demo site:
- **URL:** https://travel-demo.bandarra.me/
- Registers `searchFlights` tool via `navigator.modelContext.registerTool()`
- Can test tool discovery and invocation

### 7.8 Native Messaging Host Registration

The native messaging host must be registered with Chrome:

```bash
# macOS registration
MANIFEST_DIR="$HOME/Library/Application Support/Google/Chrome/NativeMessagingHosts"
mkdir -p "$MANIFEST_DIR"

cat > "$MANIFEST_DIR/com.fspec.webmcp.json" << EOF
{
  "name": "com.fspec.webmcp",
  "description": "fspec WebMCP MCP Bridge",
  "path": "$(which fspec-webmcp-host)",
  "type": "stdio",
  "allowed_origins": [
    "chrome-extension://YOUR_EXTENSION_ID/"
  ]
}
EOF
```

The host process receives JSON messages on stdin and sends responses on stdout, using Chrome's native messaging protocol (4-byte length prefix + JSON payload).

---

## 8. Proposed Architecture for fspec Extension

### 8.1 Component Overview

```
┌─────────────────────────────────────────────────────┐
│                    Chrome Browser                    │
│                                                      │
│  ┌──────────────────┐  ┌──────────────────────────┐ │
│  │  Web Page (Tab)   │  │   Service Worker          │ │
│  │                    │  │   (background)            │ │
│  │  navigator.       │  │                            │ │
│  │  modelContext     │  │  • Tool registry           │ │
│  │  .registerTool()  │  │  • Browser API calls       │ │
│  │                    │  │  • Event listeners         │ │
│  │  ┌──────────────┐ │  │  • Native messaging client │ │
│  │  │Main-World    │ │  │  • chrome.scripting        │ │
│  │  │Injected Scrpt│ │  │    .executeScript()        │ │
│  │  │(discovery +  │ │  │                            │ │
│  │  │ invocation)  │ │  │                            │ │
│  │  └──────┬───────┘ │  └───────────┬──────────────┘ │
│  │         │postMsg  │              │                 │
│  │  ┌──────▼───────┐ │              │                 │
│  │  │Content Script │──>            │                 │
│  │  │(isolated world│ │              │                 │
│  │  │ relay only)  │ │              │ Native Messaging │
│  │  └──────────────┘ │              │                 │
│  └──────────────────┘              │                 │
│                                      │                 │
│  ┌──────────────────┐              │                 │
│  │  Popup UI         │              │                 │
│  │  (status display) │              │                 │
│  └──────────────────┘              │                 │
└─────────────────────────────────────┼─────────────────┘
                                      │
                            ┌─────────▼───────────┐
                            │ Native Messaging Host│
                            │ (Node.js process)    │
                            │                      │
                            │ • HTTP server :19876 │
                            │ • MCP protocol       │
                            │ • GET SSE stream     │
                            │ • JSON-RPC routing   │
                            └─────────┬───────────┘
                                      │
                              HTTP (Streamable)
                                      │
                            ┌─────────▼───────────┐
                            │ ConnectMCP           │
                            │ (fspec agent)        │
                            │                      │
                            │ transport: 'http'    │
                            │ url: localhost:19876  │
                            └─────────────────────┘
```

### 8.2 Data Flow: Tool Discovery

> **Critical Architecture Note: Content Script World Isolation**
>
> Content scripts run in an **isolated world** — they share the page's DOM but
> have a separate JavaScript execution context. This means a content script
> **cannot** directly access `navigator.modelContext` or see tools registered
> by the page's main world JavaScript.
>
> **Solution:** Use `chrome.scripting.executeScript()` with `world: 'MAIN'` to
> inject a discovery script into the page's main world. This injected script
> can access `navigator.modelContext`, read the tool registry, and communicate
> back to the content script (isolated world) via `window.postMessage()`. The
> content script then relays to the service worker via `chrome.runtime.sendMessage()`.
>
> This is the same pattern used by MCP-B/WebMCP-org for their extension.

```
1. Page loads, calls navigator.modelContext.registerTool()
2. Service worker injects a main-world discovery script via:
   chrome.scripting.executeScript({
     target: { tabId },
     world: 'MAIN',
     func: discoverWebMCPTools
   })
   The injected script accesses navigator.modelContext and reads
   registered tools (name, description, inputSchema — NOT execute).
3. Injected main-world script sends tool metadata via:
   window.postMessage({
     type: 'FSPEC_WEBMCP_TOOLS_DISCOVERED',
     tools: [{name, description, inputSchema}, ...]
   }, '*')
4. Content script (isolated world) listens for postMessage,
   then relays to service worker:
   chrome.runtime.sendMessage({
     type: 'WEBMCP_TOOL_REGISTERED',
     tabId, origin, tool: {name, description, inputSchema}
   })
5. Service worker updates internal tool registry
6. Service worker notifies native host:
   port.postMessage({
     type: 'TOOLS_CHANGED',
     tools: [...allTools]
   })
7. Native host sends MCP notification on GET SSE stream:
   {"jsonrpc":"2.0","method":"notifications/tools/list_changed"}
8. ConnectMCP receives, calls tools/list to refresh
```

**Discovery polling:** Since `navigator.modelContext` does not fire events when
tools are registered/unregistered, the main-world script must either:
- **Poll** the tool map at intervals (e.g., every 2 seconds)
- **Monkey-patch** `navigator.modelContext.registerTool` and `unregisterTool`
  to intercept calls (fragile but responsive)
- **Use MutationObserver** for declarative `<form toolname>` elements only

### 8.3 Data Flow: WebMCP Tool Invocation

> **Critical Architecture Note: Main World Execution Required**
>
> The `execute()` callback lives in the page's main world JavaScript context.
> A content script (isolated world) cannot call it directly. Tool invocation
> must go through the same main-world bridge used for discovery.

```
1. Agent calls: webmcp__example.com__searchFlights({...})
2. ConnectMCP sends POST /mcp: tools/call
3. Native host receives, forwards to extension via native messaging
4. Service worker injects a main-world invocation script via:
   chrome.scripting.executeScript({
     target: { tabId },
     world: 'MAIN',
     func: invokeWebMCPTool,
     args: ['searchFlights', {...}]
   })
   The injected script calls the tool's execute() function in the
   page's main world context.
5. The execute() function returns its result (Promise resolves)
6. Main-world script sends result via window.postMessage()
7. Content script receives and relays to service worker
8. Result travels back through native messaging to HTTP server
9. Agent receives structured result
```

**Alternative approach:** Instead of per-invocation script injection, a
persistent main-world bridge script can be injected once during discovery.
This bridge listens for invocation commands via `window.postMessage()` and
calls the appropriate tool's `execute()` function, posting results back.

### 8.4 Data Flow: Browser Event Notification

```
1. User clicks link, chrome.tabs.onUpdated fires
2. Service worker captures event
3. Service worker sends to native host via native messaging:
   {type: 'BROWSER_EVENT', event: 'navigation', data: {...}}
4. Native host sends SSE notification:
   {"jsonrpc":"2.0",
    "method":"notifications/browser/navigation",
    "params":{"tabId":123,"url":"...","title":"..."}}
5. ConnectMCP receives via SSE stream
6. rmcp injects notification into session via watcher_input_tx
7. Agent sees: "[MCP:ext] Browser navigation: https://..."
```

---

## 9. Tool Catalog

### 9.1 Native Browser Control Tools

These tools are always available regardless of WebMCP support:

| Tool Name | Description | Input Schema |
|-----------|-------------|-------------|
| `browser_navigate` | Navigate tab to URL | `{url: string, tabId?: number, newTab?: boolean}` |
| `browser_screenshot` | Capture tab screenshot | `{tabId?: number, fullPage?: boolean, format?: 'png'|'jpeg'}` |
| `browser_get_page_content` | Get page text/HTML | `{tabId?: number, format?: 'text'|'html'|'markdown'}` |
| `browser_list_tabs` | List all open tabs | `{}` |
| `browser_switch_tab` | Activate a tab | `{tabId: number}` |
| `browser_close_tab` | Close a tab | `{tabId: number}` |
| `browser_click_element` | Click element by selector | `{tabId?: number, selector: string}` |
| `browser_fill_form` | Fill form fields | `{tabId?: number, fields: {selector: string, value: string}[]}` |
| `browser_execute_script` | Run JavaScript in tab | `{tabId?: number, code: string}` |
| `browser_go_back` | Navigate back | `{tabId?: number}` |
| `browser_go_forward` | Navigate forward | `{tabId?: number}` |
| `browser_get_cookies` | Get cookies for URL | `{url: string}` |
| `browser_get_interactive_elements` | Find clickable elements | `{tabId?: number}` |

### 9.2 WebMCP Discovery Tools

| Tool Name | Description | Input Schema |
|-----------|-------------|-------------|
| `webmcp_list_tools` | List all discovered WebMCP tools | `{tabId?: number}` |
| `webmcp_get_tool_schema` | Get input schema for a tool | `{origin: string, toolName: string}` |

### 9.3 Dynamically Discovered WebMCP Tools

These appear when websites register tools via `navigator.modelContext`:

- Namespaced as `webmcp__<origin>__<toolName>`
- Example: `webmcp__travel-demo.bandarra.me__searchFlights`
- Input schema comes from the tool's `inputSchema` property
- Automatically added/removed as pages register/unregister tools

---

## 10. ConnectMCP Integration

### 10.1 How the Agent Connects

```
ConnectMCP(
  name: 'ext',
  transport: 'http',
  url: 'http://localhost:19876/mcp'
)
```

This establishes:
1. HTTP connection to the native messaging host's MCP server
2. MCP initialize handshake (protocol version, capabilities)
3. `tools/list` call returns all available tools (native + WebMCP)
4. SSE stream opened for server→client notifications
5. Tools become available as `mcp__ext__browser_navigate`, `mcp__ext__webmcp__origin__toolName`, etc.

### 10.2 Handling Server Notifications in rmcp

The existing ConnectMCP implementation handles these notification types:

```rust
// In ClientHandler implementation
fn on_notification(&self, notification: JsonRpcNotification) {
    match notification.method.as_str() {
        "notifications/tools/list_changed" => {
            // Already handled by rmcp — re-fetches tools/list
        }
        method if method.starts_with("notifications/browser/") => {
            // Inject into session via watcher_input_tx
            let msg = format!("[MCP:{}] {}: {:?}",
                self.server_name, method, notification.params);
            self.watcher_input_tx.send(msg);
        }
        _ => {
            // Unknown notification — log and inject
        }
    }
}
```

### 10.3 Disconnection

```
ConnectMCP(action: 'disconnect', name: 'ext')
```

This:
1. Sends `DELETE /mcp` to close the MCP session
2. SSE stream closes
3. Connection removed from session cache
4. All `mcp__ext__*` tools removed from next LLM turn

### 10.4 Session Lifecycle

When the fspec session ends:
1. All MCP connections are dropped (existing behavior from MCP-001)
2. The native messaging host's SSE stream closes
3. The extension detects disconnection and updates popup status
4. The native messaging host process continues running (serves future connections)

---

## References

- W3C WebMCP Spec: https://webmachinelearning.github.io/webmcp/
- Chrome MV3 Docs: https://developer.chrome.com/docs/extensions/develop
- MCP Streamable HTTP: https://modelcontextprotocol.io/specification/2025-03-26/basic/transports
- mcp-chrome: https://github.com/hangwin/mcp-chrome
- MCP-B / WebMCP: https://github.com/MiguelsPizza/WebMCP
- WebMCP docs: https://docs.mcp-b.ai/
- Chrome DevTools MCP: https://developer.chrome.com/blog/chrome-devtools-mcp
- Chrome flags primer: https://developer.chrome.com/docs/web-platform/chrome-flags
- Travel demo: https://travel-demo.bandarra.me/
