# AST Research: Popup UI Dependencies (EXT-008)

## Key Types (extension/src/types/index.ts)

### StatusResponse (line 44)
```typescript
interface StatusResponse {
  connected: boolean;
  nativeConnected: boolean;
  toolCount: number;
  port: number;
}
```

### ToolRegistryEntry (line 34)
```typescript
interface ToolRegistryEntry {
  name: string;
  description: string;
  inputSchema?: Record<string, unknown>;
  source: 'native' | 'webmcp';
  origin?: string;
  tabId?: number;
}
```

## Message Router - handlePopupMessage (extension/src/background/message-router.ts:220-237)
```typescript
handlePopupMessage(message, sendResponse): boolean {
  if (type === MESSAGE_TYPES.GET_STATUS) {
    sendResponse({
      connected: true,
      nativeConnected: connection.isConnected(),
      toolCount: toolRegistry.size(),
      port: MCP_DEFAULT_PORT,
    });
    return true;
  }
  return false;
}
```

## Popup Script (extension/src/popup/popup.ts)
Stub only - sets static text for status and port.

## Popup HTML (extension/popup.html)
Has 4 rows: Server Status, Port, Connected Clients, Tools Available (counts only).

## Analysis
- StatusResponse needs expansion: add `clientCount`, `tools` array with grouping data
- handlePopupMessage needs to return full tools list for popup grouping
- popup.ts needs to send chrome.runtime.sendMessage and render response
- popup.html needs tool group sections added dynamically
- MCP_DEFAULT_PORT = 19876 (extension/src/server/mcp-constants.ts)
- ToolRegistry API: getAll(), size(), getByTab(), getByName()
