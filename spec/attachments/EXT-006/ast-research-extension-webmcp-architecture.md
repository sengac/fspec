# AST Research: Extension WebMCP Architecture (EXT-006)

## Factory Functions (Extension Components)

| Function | File | Description |
|----------|------|-------------|
| `createNativeConnection` | `extension/src/background/native-connection.ts` | Native messaging port management |
| `createMessageRouter` | `extension/src/background/message-router.ts` | Message routing between SW/content/native |
| `createToolRegistry` | `extension/src/background/tool-registry.ts` | Tool registry (Map-based) |
| `createBrowserTools` | `extension/src/background/browser-tools.ts` | Native browser control handlers |
| `createContentRelay` | `extension/src/content/relay.ts` | Content script ↔ main world bridge |

## Interfaces (Extension APIs)

| Interface | File | Purpose |
|-----------|------|---------|
| `ChromeRuntimeLike` | native-connection.ts | DI for chrome.runtime |
| `PortLike` | native-connection.ts | DI for native messaging port |
| `NativeConnectionAPI` | native-connection.ts | Native host connection |
| `ChromeTabsLike` | message-router.ts | DI for chrome.tabs |
| `MessageRouterAPI` | message-router.ts | Route messages between components |
| `ToolRegistryAPI` | tool-registry.ts | Register/unregister/query tools |
| `BrowserToolsAPI` | browser-tools.ts | Browser tool handlers |
| `WebMCPToolInfo` | types/index.ts | WebMCP tool metadata |
| `ToolRegistryEntry` | types/index.ts | Registry entry (native/webmcp) |
| `WindowLike` | relay.ts | DI for window |
| `ContentRuntimeLike` | relay.ts | DI for chrome.runtime in content |
| `ContentRelayAPI` | relay.ts | Content relay methods |

## Message Types (inter-component)

| Constant | Value | Direction |
|----------|-------|-----------|
| TOOL_REGISTERED | FSPEC_WEBMCP_TOOL_REGISTERED | Content→SW |
| TOOL_UNREGISTERED | FSPEC_WEBMCP_TOOL_UNREGISTERED | Content→SW |
| INVOKE_RESULT | FSPEC_INVOKE_RESULT | Content→SW |
| INVOKE_TOOL | FSPEC_INVOKE_TOOL | SW→Content→Main |
| GET_STATUS | FSPEC_GET_STATUS | Popup→SW |
| TOOL_CALL | TOOL_CALL | NativeHost→SW |
| TOOLS_CHANGED | TOOLS_CHANGED | SW→NativeHost |
| NOTIFICATION | NOTIFICATION | SW→NativeHost |

## Key Findings for EXT-006

### What exists (EXT-004 scope):
1. **Content relay** already bridges main world ↔ SW via postMessage/chrome.runtime
2. **Message router** handles TOOL_REGISTERED/UNREGISTERED/INVOKE messages
3. **Tool registry** stores both native and webmcp tools

### What's MISSING (EXT-006 scope):
1. **Main-world discovery script** — No file exists yet that runs in the page's JS context to intercept `navigator.modelContext.registerTool()/unregisterTool()` calls
2. **Script injection** — No service worker code to inject discovery script via `chrome.scripting.executeScript({world: 'MAIN'})`
3. **Tool name format** — Currently `webmcp__${toolMeta.name}` needs to be `webmcp__${origin}__${toolName}`

### Message router tool name bug:
- Line 155 in message-router.ts: `name: \`webmcp__${toolMeta.name}\``
- Line 173: `toolRegistry.unregister(\`webmcp__${toolName}\`)`
- These need the origin prefix: `webmcp__${origin}__${name}`

### Invocation routing (already works):
- Line 92-111: SW detects `source === 'webmcp'` and routes to correct tab
- Line 96-98: Strips `webmcp__` prefix to get original tool name
- Needs update for the `webmcp__<origin>__<name>` format
