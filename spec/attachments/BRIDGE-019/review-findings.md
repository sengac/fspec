# Epic Review: BRIDGE-019 — Relay Server

**Date:** 2026-03-24
**Reviewer:** Claude Code (fspec review skill)
**Work Units Reviewed:** 1

## Summary
- 🔴 Critical: 2 issues across 1 work unit
- 🟡 Warnings: 3 issues across 1 work unit
- 🟢 Observations: 1

## Work Unit Results

### BRIDGE-019: Relay Server — FAIL

## 🔴 Critical Issues (Must Fix)

1. **Implementation file exceeds 300 lines**: `bridge/relay-server.ts` is 357 lines. Standard requires files under 300 lines. Must refactor — extract the CLI entry point and WebSocket wrapper into a separate file.

2. **Missing JSDoc on internal functions**: `addToChannel`, `removeFromChannel`, `broadcastToChannel`, `trackConnectedSession`, `handleAuth` all lack JSDoc comments. Project standard: "All functions must have JSDoc comments."

## 🟡 Warnings (Should Fix)

1. **Type assertions on parsed JSON**: Lines 129, 152-154, 216, 222 use `as` casts on parsed JSON data. Should use type guards or validation functions instead. Example: `msg.session_id as string | undefined` at line 129 should use proper narrowing.

2. **Missing `sessionControl` test**: Rule [5] explicitly lists all 5 session-scoped message types (input, sessionControl, command, commandResponse, chunk). The `sessionControl` type has no dedicated scenario or test. While the implementation handles it via the catch-all broadcast, there's no explicit verification.

3. **Test file exceeds 300 lines**: `bridge/__tests__/relay-server.test.ts` is 555 lines. Should be refactored — extract test helpers and auth setup utility into a shared module.

## 🟢 Observations (Nice to Have)

1. **Test auth setup repetition**: ~8 tests repeat the same auth handshake pattern (create client, send auth JSON, clear sentMessages). A shared `authenticateClient(server, channelId)` helper would reduce boilerplate by ~100 lines.

## Coverage Verification
- Feature file: `spec/features/relay-server.feature` — OK
- Test file(s): `bridge/__tests__/relay-server.test.ts` — OK (all @step comments match exactly)
- Impl file(s): `bridge/relay-server.ts` — OK (line ranges verified)
- Scenario coverage: 13/13 scenarios covered

## Files Reviewed
- spec/features/relay-server.feature
- bridge/relay-server.ts
- bridge/__tests__/relay-server.test.ts
- spec/attachments/BRIDGE-019/ast-research-bridge-patterns.md
