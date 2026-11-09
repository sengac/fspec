# REFAC-034 structure research

`spec/features/generate-compaction-tool.feature` (300 lines) layout, from
`grep -n '^  Scenario\|^  @\|^  #' spec/features/generate-compaction-tool.feature`:

- Lines 1-8: feature tags (`@CMPCT-046 @CMPCT-047 @CMPCT-048 @done @agent-core @tools
  @context-management @CMPCT-045`); line 9 title; 10-12 doc string.
- Lines 14-50: generated example-mapping comment (22 rules, 5 examples). Rule 13 (line 31) is the
  provider tool-chain and handler-registration rule; rule 22 (line 40) is the CMPCT-046c
  provider-arm parity rule.
- Lines 229-242: `Compactor sub-agent provider arms match the DeepSearch clone`. Its tag block
  repeats `@context-management @session @regression` twice (lines 229-234).
- Lines 244-254: `GenerateCompaction tool wired into provider agent builders`. A misplaced section
  banner, `# Tool surface and registration lifecycle` (lines 248-250), sits between the title and
  the first step.
- Lines 256-264: `Handler registration lifecycle mirrors the DeepSearch pattern`.

Tests for the three scenarios, all in the `cmpct045_generate_compaction_behavioral` target:

| Scenario | Test |
|---|---|
| provider arms | rust/agent-loop/tests/cmpct045/part_07.rs:193-235 |
| provider agent builders | rust/agent-loop/tests/cmpct045/part_02.rs:102-144 |
| handler registration lifecycle | rust/agent-loop/tests/cmpct045/part_06.rs:156-222 |

Implementation the tests read:

- provider arms: rust/agent-loop/src/compaction_builder.rs:229-239 (the `match provider_name`).
- provider agent builders: the `.tool(DeepSearchTool::new(session_id))` /
  `.tool(GenerateCompactionTool::new(session_id))` pairs, e.g. rust/providers/src/claude.rs:560-562
  (the mapping recorded 548-560).
- handler registration: rust/agent-loop/src/agent_loop/services.rs:8-31 (registration) and
  :49-50 (`set_deep_search_handler(session.id, None)`), and
  rust/agent-loop/src/terminal_overflow_recovery.rs:41-48 (`impl Drop for CompactorRegistration`).
