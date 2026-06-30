# Review: RPC-388 — Tool-call argument header parity

**Date:** 2026-06-29
**Reviewer:** Claude Code review-skill (subordinate reviewer a13ea3e0)
**Status:** PASS (with fixes to apply)

## 🔴 Critical Issues
None.

## 🟡 Warnings (Should Fix)
1. **Coverage `testMappings` line ranges are stale/incorrect.** The
   `.feature.coverage` maps each scenario's TEST to ranges (69-78, 80-87, 89-96,
   98-106, 108-115, 117-127, 129-136) that point at IMPLEMENTATION/helper lines,
   not the actual `#[test]` functions, which live at: edit_family 122-129,
   write_family 134-138, command_tool 143-147, action_type 152-157, no_command
   162-166, long_value 171-178, invalid_json 183-187. Re-link with correct test
   line ranges.

## 🟢 Observations
1. `value_to_plain` for non-string command values uses serde `.to_string()` vs TS
   `String(value)`; unreachable for in-scope tools — acceptable.
2. Parity edge cases confirmed (null bare, compact JSON for numbers/objects,
   insertion order preserved).
3. End-to-end wiring verified (`handle_tool_call` → header render).
4. **Feature file name is verbose** (the bug title). Project convention wants a
   SHORT capability name like `tool-call-argument-header.feature`. → Rename.

## Coverage
- Feature file: OK (G/W/T order, no placeholders, arch doc string, @RPC-388) — naming to fix.
- Tests: OK (@step char-for-char, exact-string assertions) — coverage ranges wrong (Warning 1).
- Impl: OK (three-branch port matches TS incl. punctuation; 100-char cap).
- Scenario coverage: 7/7.

## Fix Results
- 🟡 Warning 1 (coverage ranges) → ✅ Fixed: re-linked all 7 scenarios to correct test ranges.
- 🟢 Obs 4 (naming) → ✅ Fixed: renamed feature + .coverage to `tool-call-argument-header.feature`.
