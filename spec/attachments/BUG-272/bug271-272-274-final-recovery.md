# Recovered final verification: BUG-271, BUG-272 and BUG-274

All three cards are **done**, confirmed by the running daemon over MCP after supported final transitions on 2026-10-28:

- BUG-271: done at 02:42:00.754 UTC; 13/13 acceptance scenarios.
- BUG-272: done at 02:42:11.943 UTC; 6/6 acceptance scenarios; causal RED 1 pass/5 fail preserved.
- BUG-274: done at 02:42:21.996 UTC; 2/2 acceptance scenarios; corrected existing-facade scope preserved.

Fspec PID 82606 has cwd `/Users/rquast/projects/fspec` and address `ws://127.0.0.1:56382`. PID 371 belongs to BPD and was excluded. The connection uses supported `fspec client --connect ws://127.0.0.1:56382 --mcp` syntax. Existing session `ba15d5a4-4ccf-4d70-9ec9-78594797286a` is Idle, has no pending interaction, and reports effective `codex/gpt-6.1-sol`. No session was created, resumed or messaged.

## Verified evidence

The existing validation JSON exactly reconstructs from original logs: **120 passing tests across 35 scoped targets**, including all **21 acceptance scenarios**. Its default and fixture builds, check, and four `-D warnings` Clippy groups have successful receipts. Passed build/test groups were not rerun. Relevant sources predate completed validation; fresh hashes are recorded without claiming retroactive original hashes.

Fresh read-only syntax, tag and coverage gates pass for all five owned features. All 21 scenarios retain one test mapping and bounded current implementation ranges. All five features already had `@done` and no `@wip`. `git diff --check` passes. Existing attachments, rules, examples, notes and state history remain intact. Unrelated cards and source files are unchanged.

Canonical fresh audit and transition receipts: `spec/attachments/BUG-271/bug271-272-274-final-recovery.json`. Original complete command receipts: `spec/attachments/BUG-272/bug271-272-274-validation-results.json`. Fresh gates and raw live MCP/transition receipts remain in `/tmp/fspec-bug271-272-274-recovery/`; the canonical JSON records gate and live-session hashes.

## Compaction settlement and limits

The existing session's final compaction **failed**: `compaction candidate rejected: Codex checkpoint lineage mismatch: generation must advance with a selected model`. Its durable transcript terminal event at **02:29:30.432108 UTC** records `compaction_failed`. No compaction was promoted: the manifest has 372 message references, null `compaction` and `compaction_commit`, a valid committed provider-state content digest and checkpoint prefix hash, and no pending persistence-recovery file. On card completion its manifest advanced from revision 936 to 937 and work-unit focus now has null context with source `watcher`; whole-manifest byte equality is not claimed. The provider-state digest, checkpoint prefix, message count and null compaction remain verified. This proves durable failed settlement, not successful compaction or a tested cold resume.

No compaction retry, fallback, ancestry reset, cleanup or live-history repair was attempted. The unresolved live compaction failure is outside these three cards' accepted scope and remains reported. The same 522 unrelated pre-existing global tag failures and the documented intermittent OAuth timing failure remain limitations; its original unchanged quiescent rerun passed. No workspace-wide clean claim or Windows run is made.

No installation, restart, push, deployment or additional disk cleanup occurred. Branch HEAD and staged entries are unchanged. Standard Fspec finalization creates temporary internal checkpoint objects and removes each completed card's auto-checkpoint refs; no source commit was made and checkpoint behavior was not suppressed. The real user-config SHA-256 remains `8eed6007c58b8f902efcf640fb8625a2662367cf4551f532ab354773650fce4c`.
