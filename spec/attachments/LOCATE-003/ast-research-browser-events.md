# AST Research: browser-events.ts Integration Points

## Purpose
Identify where ref-state.ts needs to integrate with browser-events.ts for automatic invalidation.

## Findings

### Exported Interfaces in browser-events.ts
- `ChromeTabsEvents` — DI interface for chrome.tabs events
- `BrowserEventsToolRegistry` — tool cleanup on tab close
- `NotificationEnvelope` — JSON-RPC 2.0 notification wrapper
- `BrowserEventListenerOptions` — config for createBrowserEventListeners

### Integration Points
1. **tabs.onUpdated handler (line 88-116)**: When `changeInfo.url` is a string (line 95), a navigation notification is sent. This is where `clearTabScanState(tabId)` must be added.
2. **tabs.onRemoved handler (line 130-143)**: When a tab is closed, tool cleanup happens. This is where `clearTabScanState(tabId)` must be added (before the notification).

### Pattern: Dependency Injection
All modules in extension/src/background/ use DI for Chrome APIs. browser-events.ts receives `tabs: ChromeTabsEvents` as a parameter. The ref-state module is a pure in-memory store — no Chrome API dependencies — so it can be imported directly (no DI needed).

### Existing Test Pattern
Tests in browser-events.test.ts use mock chrome.tabs with `_fireUpdated`, `_fireCreated`, `_fireRemoved` helpers. Integration tests for ref-state invalidation should follow the same pattern.

### All Exported Interfaces in background/
- native-connection.ts: ChromeRuntimeLike, PortLike, NativeConnectionOptions, NativeConnectionAPI
- message-router.ts: ChromeTabsLike, ChromeRuntimeForRouter, MessageRouterOptions, MessageRouterAPI
- tool-registry.ts: ToolRegistryAPI
- browser-tools.ts: ChromeTabsForTools, ChromeScriptingForTools, ChromeWindowsForTools, ChromeUserScriptsForTools, BrowserToolsDeps, BrowserToolsAPI
- webmcp-injector.ts: ChromeScriptingForInjector, ChromeTabsForInjector, WebMCPInjectorOptions, WebMCPInjectorAPI
- browser-events.ts: ChromeTabsEvents, BrowserEventsToolRegistry, NotificationEnvelope, BrowserEventListenerOptions
