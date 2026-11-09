# BUG-375: known stale mappings found while working other cards

- `spec/features/specifying-reminder-names-ast-search-tool-per-mode.feature.coverage` maps `reminders.rs` lines 842–869, but that file has 78 lines. Found during REFAC-049/BUG-376 (2026-11-03).
- Earlier: 18 dynamic-mcp-connect / mcp-same-turn mappings left at pre-MCP line numbers (COV-066), and most `dynamic-mcp-connect` scenarios point at production code or the wrong test ranges (COV-067).

The audit that BUG-375 adds must catch all of these: ranges past the end of the file, and ranges whose text no longer contains the test or step.
- Found during BUG-394 (2026-11-03): two pause scenarios have impl mappings pointing at the exec-stdin block instead of the pause code. The BUG-394 agent left them, so find them by checking pause-related .feature.coverage impl ranges against their text.
