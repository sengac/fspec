# AST Research: Native Messaging Host & MCP Server Patterns

## Date: 2026-03-05
## Work Unit: EXT-003
## Tool: AstGrep

---

## 1. Existing Extension Type Definitions

**Pattern:** `export interface $NAME { $$$FIELDS }`
**Path:** `extension/src/`

```
extension/src/types/index.ts:12:1 - ExtensionMessage { type: string; payload?: unknown }
extension/src/types/index.ts:18:1 - WebMCPToolInfo { name, description, inputSchema?, origin, tabId }
extension/src/types/index.ts:27:1 - ConnectionStatus { connected, port, clientCount }
extension/src/types/index.ts:34:1 - ToolRegistryEntry { name, description, inputSchema?, source, origin?, tabId? }
```

**Analysis:** Types already defined for tool registry and connection status. The native host will need additional types for:
- MCP JSON-RPC request/response
- Session state
- Native messaging frame protocol
- SSE event format
- Correlation ID tracking for request-response mapping

---

## 2. Existing MCP Constants

**Pattern:** `export const $NAME = $VALUE`
**Path:** `extension/src/`

```
extension/src/server/mcp-constants.ts:15:1 - MCP_DEFAULT_PORT = 19876
extension/src/server/mcp-constants.ts:16:1 - MCP_ENDPOINT = '/mcp'
extension/src/server/mcp-constants.ts:17:1 - NATIVE_MESSAGING_HOST_NAME = 'com.fspec.webmcp'
```

**Analysis:** Key constants already defined. The native host can reference these or duplicate them (since it's a standalone Node.js script with zero dependencies).

---

## 3. Chrome API Usage Patterns

**Pattern:** `chrome.runtime.$METHOD($$$ARGS)`
**Path:** `extension/src/`

```
extension/src/content/content-script.ts:28:5 - chrome.runtime.sendMessage(data)
```

**Pattern:** `chrome.runtime.onMessage.addListener($$$ARGS)`

```
extension/src/background/service-worker.ts:24:1 - onMessage listener (stub)
extension/src/content/content-script.ts:33:1 - onMessage listener (stub)
```

**Pattern:** `chrome.runtime.onInstalled.addListener($$$ARGS)`

```
extension/src/background/service-worker.ts:19:1 - onInstalled listener
```

**Analysis:** The service worker currently has stub message listeners. EXT-003 needs to add:
- `chrome.runtime.connectNative('com.fspec.webmcp')` in service worker
- Native port message handling (`port.onMessage`, `port.onDisconnect`)
- Message routing from native port to HTTP response resolution

---

## 4. Message Relay Pattern

**Pattern:** `window.addEventListener($$$ARGS)`

```
extension/src/content/content-script.ts:19:1 - window.addEventListener('message', ...)
```

**Analysis:** Content script already has postMessage relay pattern. The native host doesn't interact with content scripts directly — it communicates only with the service worker via stdin/stdout native messaging.

---

## 5. HTTP Server Patterns (fspec codebase)

**Pattern:** `createServer($$$ARGS)` in `src/` — No matches
**Pattern:** `import { $$$IMPORTS } from 'http'` — No matches
**Pattern:** `import { $$$IMPORTS } from 'node:http'` — No matches

**Analysis:** No existing HTTP server patterns in fspec codebase. The native host will be the first component using Node.js `http.createServer()`. This is expected per architecture note [0]: "Native host is pure Node.js with zero npm dependencies."

---

## 6. stdin/stdout Patterns (fspec codebase)

**Pattern:** `process.stdin.$METHOD($$$ARGS)` — No matches
**Pattern:** `process.stdout.$METHOD($$$ARGS)` — No matches

**Analysis:** No existing stdin/stdout usage in fspec codebase. The native host will be the first component using raw stdin/stdout for Chrome native messaging protocol (4-byte length prefix + JSON).

---

## 7. Existing Test Patterns (EXT-002)

**File:** `src/commands/__tests__/extension-scaffolding.test.ts`

The EXT-002 test file uses:
- Direct filesystem assertions (`existsSync`, `readFileSync`)
- `resolve(import.meta.dirname, ...)` for path resolution
- JSON parsing for manifest/package.json validation
- Descriptive scenario-matching test names with `@step` comments

**Analysis:** EXT-003 tests should follow the same pattern but will need:
- Programmatic testing of the HTTP server (start server, make HTTP requests, verify responses)
- Mock stdin/stdout streams for native messaging protocol tests
- Session management assertions
- SSE stream verification

---

## 8. Key Implementation Decisions

Based on AST research:

1. **Standalone script:** The native host at `extension/host/native-host.js` will be plain JavaScript (not TypeScript) since it runs directly via `node` and has zero npm dependencies. Chrome's native messaging host manifest requires a direct script path.

2. **No import sharing:** The host cannot import from `extension/src/` since it's a standalone Node.js process. Constants like `MCP_DEFAULT_PORT` and `NATIVE_MESSAGING_HOST_NAME` must be duplicated.

3. **Test in fspec test suite:** Tests live in `src/commands/__tests__/` following the EXT-002 pattern, testing the host by spawning it as a child process or importing/mocking its modules.

4. **Architecture layers confirmed:**
   - Native Host (Node.js) → HTTP server + stdin/stdout native messaging
   - Service Worker → `chrome.runtime.connectNative()` + message routing
   - Content Script → postMessage relay (already stubbed)
