# AST Research: Extension Background Architecture for EXT-005

## Research Summary

Analysis of the extension service worker architecture to plan native browser tool implementation.

## Exported Factory Functions (Extension Background)

```
extension/src/background/native-connection.ts:50  → createNativeConnection(options: NativeConnectionOptions): NativeConnectionAPI
extension/src/background/message-router.ts:54     → createMessageRouter(options: MessageRouterOptions): MessageRouterAPI
extension/src/background/tool-registry.ts:22      → createToolRegistry(): ToolRegistryAPI
```

**Pattern**: All background modules use factory functions returning typed API objects. Browser tools should follow this same pattern → `createBrowserTools(deps): BrowserToolsAPI`.

## Exported Interfaces (Dependency Injection Points)

### Chrome API Abstractions
```
native-connection.ts:14 → ChromeRuntimeLike { connectNative, lastError }
native-connection.ts:20 → PortLike { postMessage, onMessage, onDisconnect, disconnect }
message-router.ts:19    → ChromeTabsLike { sendMessage }
message-router.ts:28    → ChromeRuntimeForRouter { sendMessage, lastError }
```

### Module APIs
```
native-connection.ts:43 → NativeConnectionAPI { connect, getPort, isConnected, disconnect }
message-router.ts:40    → MessageRouterAPI { handleNativeMessage, handleContentScriptMessage, handlePopupMessage, forwardNotification }
tool-registry.ts:12     → ToolRegistryAPI { register, unregister, getAll, getByTab, getByName, clear, size }
```

### Shared Types
```
types/index.ts:34 → ToolRegistryEntry { name, description, inputSchema?, source: 'native'|'webmcp', origin?, tabId? }
types/index.ts:44 → StatusResponse { connected, nativeConnected, toolCount, port }
```

## Message Router: Native Tool Call Handling (Current)

The message router's `handleNativeMessage` currently has a placeholder for native browser tools:

```typescript
// Line 110-117 of message-router.ts:
} else {
  // Native browser tool — no handler registered yet.
  // EXT-005 will register tool handlers; until then respond with an error.
  sendToNativeHost({
    correlationId,
    error: { code: -32601, message: `No handler registered for tool: ${toolName}` },
  });
}
```

This is the exact integration point where `browser-tools.ts` handlers will be wired in.

## MCP Server: NATIVE_TOOLS Definition (Current - 4 tools)

```
host/lib/mcp-server.mjs:21-65 → NATIVE_TOOLS array
  - browser_navigate (url, tabId?)
  - browser_screenshot (tabId?, fullPage?)
  - browser_list_tabs ()
  - browser_execute_script (code, tabId?)
```

**Missing 7 tools**: browser_switch_tab, browser_close_tab, browser_get_page_content, browser_click_element, browser_fill_form, browser_go_back, browser_go_forward

## Chrome APIs Required

For the new `browser-tools.ts` module, these Chrome APIs need DI interfaces:

1. **chrome.tabs** - query, update, remove, captureVisibleTab, goBack, goForward
2. **chrome.scripting** - executeScript
3. **chrome.windows** - update (for focusing window on tab switch)

## Integration Plan

1. Create `extension/src/background/browser-tools.ts`:
   - Factory function `createBrowserTools(deps)` returning a handler map
   - DI interfaces for Chrome APIs (tabs, scripting, windows)
   - Each handler: `async (params) => MCP result/error`

2. Update `extension/src/background/message-router.ts`:
   - Accept browser tools handler in options
   - Route native tool calls to the handler map instead of returning error

3. Update `extension/host/lib/mcp-server.mjs`:
   - Add 7 missing tool definitions to NATIVE_TOOLS array

4. Update `extension/src/background/service-worker.ts`:
   - Create browser tools instance and pass to message router
