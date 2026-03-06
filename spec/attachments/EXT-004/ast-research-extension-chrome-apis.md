# AST Research: Extension Chrome API Usage & Types

## Research Context
Work Unit: EXT-004 - Service Worker & Content Script Message Routing

## Current Chrome API Usage in Extension Source

### Service Worker (`extension/src/background/service-worker.ts`)
- `chrome.runtime.onInstalled.addListener` — Extension install handler (stub)
- `chrome.runtime.onMessage.addListener` — Message listener (stub, returns false)
- **Missing**: `chrome.runtime.connectNative` — needs to be added for native messaging
- **Missing**: `chrome.tabs.sendMessage` — needs to be added for routing to content scripts

### Content Script (`extension/src/content/content-script.ts`)
- `chrome.runtime.sendMessage(data)` — Forwards FSPEC_WEBMCP_ prefixed messages to SW
- `chrome.runtime.onMessage.addListener` — Listens for FSPEC_INVOKE_ prefixed messages from SW
- `window.addEventListener('message', ...)` — Listens for postMessage from main world
- `window.postMessage(message, '*')` — Forwards tool invocations to main world

### Shared Types (`extension/src/types/index.ts`)
- `ExtensionMessage` — Base message with type + payload
- `WebMCPToolInfo` — Tool metadata: name, description, inputSchema, origin, tabId
- `ConnectionStatus` — connected, port, clientCount
- `ToolRegistryEntry` — name, description, inputSchema, source (native|webmcp), origin, tabId

## Types Needed for EXT-004

### Service Worker Additions
1. Tool registry: `Map<string, ToolRegistryEntry>` keyed by tool name
2. Native messaging port: `chrome.runtime.Port` from `connectNative`
3. Pending call tracking: `Map<string, PendingCall>` for correlation IDs
4. Reconnection timer state

### Message Types (from architecture notes)
- `FSPEC_WEBMCP_TOOL_REGISTERED` — content→SW tool discovery
- `FSPEC_WEBMCP_TOOL_UNREGISTERED` — content→SW tool removal
- `FSPEC_INVOKE_TOOL` — SW→content tool invocation request
- `FSPEC_INVOKE_RESULT` — content→SW tool invocation result
- `FSPEC_GET_STATUS` — popup→SW status query
- `TOOL_CALL` — native host→SW incoming tool calls
- `TOOLS_CHANGED` — SW→native host tool list updates
- `NOTIFICATION` — SW→native host browser event forwarding

### Native Messaging Host (EXT-003 - already implemented)
- `extension/host/lib/mcp-server.mjs` — Full MCP server with session management
- `extension/host/lib/native-messaging.mjs` — Encode/decode native messaging frames
- `extension/host/lib/registration.mjs` — Chrome host registration
- Host expects correlation-based request/response for tool calls
- Host broadcasts NOTIFICATION messages to SSE streams

## Key Architecture Insight
- Chrome's `chrome.runtime.onMessage.addListener` automatically provides `sender.tab.id` for content scripts
- Chrome's `chrome.runtime.connectNative()` returns a Port with `.postMessage()` and `.onMessage`
- Chrome handles native messaging frame encoding/decoding — service worker sends/receives plain JSON
- Service worker does NOT need 4-byte length-prefix framing (that's the host's concern)
