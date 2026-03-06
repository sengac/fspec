# AST Research: Notification Pipeline for EXT-007

## Research Summary

Investigated the existing notification pipeline in the fspec WebMCP Chrome extension to understand how browser events should flow to the agent.

## Key Findings

### 1. forwardNotification in message-router.ts (lines 238-243)
```typescript
forwardNotification(event: Record<string, unknown>): void {
  sendToNativeHost({
    type: MESSAGE_TYPES.NOTIFICATION,
    ...event,
  });
}
```
This method is **defined but never called** — it's the intended hook point for EXT-007.

### 2. MCP Server handles NOTIFICATION type (mcp-server.mjs lines 180-189)
```javascript
if (message.type === 'NOTIFICATION' && message.notification) {
  for (const [, session] of sessions) {
    for (const res of session.sseResponses) {
      const sseData = `data: ${JSON.stringify(message.notification)}\n\n`;
      res.write(sseData);
    }
  }
}
```
**Already implemented** — broadcasts any notification to all SSE streams.

### 3. TOOLS_CHANGED is a separate pathway (mcp-server.mjs lines 193-206)
The tools/list_changed notification goes through a different code path using `TOOLS_CHANGED` message type.
This is **already working** via notifyToolsChanged() in message-router.ts.

### 4. Service Worker (service-worker.ts)
- Has comment `EXT-007: Browser event notification handlers (future)` on line 13
- Currently creates: native connection, message router, tool registry, browser tools, webmcp injector
- **Missing**: Chrome event listener setup (tabs.onUpdated, tabs.onCreated, tabs.onRemoved)

### 5. MESSAGE_TYPES (types/index.ts)
- NOTIFICATION: 'NOTIFICATION' — already defined for SW → Native Host notifications

## Architecture Decision

The `forwardNotification()` method expects a `notification` field containing the JSON-RPC 2.0 notification object. The envelope format must be:
```json
{
  "type": "NOTIFICATION",
  "notification": {
    "jsonrpc": "2.0",
    "method": "notifications/browser/navigation",
    "params": { "tabId": 123, "url": "...", "title": "..." }
  }
}
```

`forwardNotification()` spreads the event: `sendToNativeHost({ type: 'NOTIFICATION', ...event })`
The server checks: `if (message.type === 'NOTIFICATION' && message.notification)`

So calls should be:
```typescript
messageRouter.forwardNotification({
  notification: {
    jsonrpc: '2.0',
    method: 'notifications/browser/navigation',
    params: { tabId, url, title }
  }
});
```

## New Module: browser-events.ts

Create `extension/src/background/browser-events.ts`:
- Factory function `createBrowserEventListeners(options)` with DI for chrome.tabs
- Registers listeners for: onUpdated, onCreated, onRemoved
- Each listener formats a JSON-RPC 2.0 notification and calls `onNotify` callback
- Tab closed also cleans up WebMCP tools for that tab via toolRegistry

## Integration Point

In `service-worker.ts`, after creating the message router:
```typescript
import { createBrowserEventListeners } from './browser-events';

createBrowserEventListeners({
  tabs: chrome.tabs,
  toolRegistry,
  onNotify: (event) => messageRouter.forwardNotification(event),
});
```
