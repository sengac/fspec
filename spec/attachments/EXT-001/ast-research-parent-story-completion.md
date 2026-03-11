# AST Research: EXT-001 Parent Story Completion

> **Work Unit:** EXT-001 — fspec WebMCP Chrome Extension (Parent Story)
> **Date:** 2026-03-11
> **Status:** All child work units completed

## Summary

EXT-001 is the parent story for the WebMCP Chrome Extension epic. All implementation was performed through child work units:

- **EXT-002** (done): Extension Scaffolding & Build System
- **EXT-003** (done): Native Messaging Host & MCP Streamable HTTP Server
- **EXT-004** (done): Service Worker & Content Script Message Routing
- **EXT-005** (done): Native Browser Control Tools
- **EXT-006** (done): WebMCP Tool Discovery & Invocation
- **EXT-007** (done): Bidirectional Browser Event Notifications
- **EXT-008** (done): Extension Popup UI
- **EXT-011** (done): Add browser_create_tab tool

## Extension Architecture (Final)

```
Main-World Injected Script (WebMCP discovery/invocation per tab)
  → Content Script (isolated world relay, postMessage ↔ chrome.runtime bridge)
    → Service Worker (tool registry, browser control APIs, event aggregation, native messaging client)
      → Native Messaging Host (Node.js, Streamable HTTP MCP server with GET-based SSE)
        → ConnectMCP (fspec agent client)
```

## Key Files

- `extension/manifest.json` - MV3 manifest
- `extension/src/background/` - Service worker, browser tools, message router, WebMCP injector, browser events
- `extension/src/content/` - Content script relay
- `extension/src/popup/` - Popup UI
- `extension/host/` - Native messaging host with MCP server
- `extension/src/types/` - Shared TypeScript types

All AST research for individual components was performed in respective child work units (EXT-002 through EXT-011).
