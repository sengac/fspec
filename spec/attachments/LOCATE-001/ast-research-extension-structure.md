# AST Research: Extension Source Structure

## Research Summary

Parent story LOCATE-001 delegates AST research to child cards. Each child performed targeted AST analysis:

- **LOCATE-003**: browser-events.ts analysis for ref-state integration points
- **LOCATE-004**: browser-tools.ts analysis for scan handler insertion points  
- **LOCATE-005**: browser-tools.ts click/fill handler analysis for ref resolution
- **LOCATE-006**: browser-tools.ts and myers-diff integration point analysis
- **LOCATE-007**: dom-scanner-helpers.ts analysis for heuristic insertion points
- **LOCATE-008**: mcp-server.mjs NATIVE_TOOLS array analysis

## Extension Source Files (Post-Implementation)

```
extension/src/background/
├── browser-events.ts          - Tab event listeners + clearTabScanState
├── browser-tools.ts           - 14 native tool handlers (scan, diff, ref resolution)
├── browser-tools-types.ts     - Type definitions for tool handlers
├── dom-scanner.ts             - Tree formatting + re-exports from helpers
├── dom-scanner-helpers.ts     - Selectors, visibility, interactivity checks
├── dom-scanner-heuristics.ts  - Label wrappers, bounding box, search/icon detection
├── myers-diff.ts              - Line-level Myers diff algorithm
├── ref-state.ts               - Per-tab scan state storage (refs, tree text)
├── scan-page-dom.ts           - Injected scanning function (self-contained)
├── service-worker.ts          - Service worker entry point
├── message-router.ts          - Native messaging router
├── native-connection.ts       - Native host connection
├── tool-registry.ts           - WebMCP tool registry
├── webmcp-injector.ts         - WebMCP API injection
└── webmcp-naming.ts           - Tool name sanitization
```
