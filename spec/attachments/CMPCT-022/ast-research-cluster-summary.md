# CMPCT-022 — Cluster Summary AST Research

Parent tracking card for the eight-bug PromptCancelled compaction recovery cluster.
This file consolidates the AST research attached to each child work unit; all
individual AST analyses are retained under their respective child attachment
directories.

## Children and their AST research artefacts

| Child     | Attachment                                                            |
|-----------|-----------------------------------------------------------------------|
| CMPCT-023 | `spec/attachments/CMPCT-023/ast-research-recovery-entry-sites.md`     |
| CMPCT-024 | `spec/attachments/CMPCT-024/ast-research-flush-sites.md`              |
| CMPCT-025 | (consolidated into CMPCT-026 research)                                |
| CMPCT-026 | `spec/attachments/CMPCT-026/ast-research-cancel-classification.md`    |
| CMPCT-027 | `spec/attachments/CMPCT-027/ast-research-in-loop-restart-sites.md`    |
| CMPCT-028 | `spec/attachments/CMPCT-028/plan.md`                                  |
| CMPCT-029 | (analysis inline in CMPCT-029 plan + rig patch comments)              |
| CMPCT-030 | (test-only work; AST used for grep-based structural assertions)       |

## Core touch sites verified across the cluster

The following production files were confirmed via AST search to be the only
sites requiring changes:

- `codelet/cli/src/interactive/stream_loop.rs` — Path A/B/C/D entry
- `codelet/cli/src/interactive/recovery_compaction.rs` — unified helper (new)
- `codelet/cli/src/interactive/gemini_continuation.rs` — Path D
- `codelet/cli/src/interactive/error_classifiers.rs` — PromptCancelled detection
- `codelet/cli/src/interactive_helpers.rs` — tool-call reconciliation helpers
- `codelet/patches/rig-core/src/agent/prompt_request/streaming.rs` — rig patch
  at `on_tool_result` site (lines 508–571 only; sites 485 and 541 intentionally
  untouched per revised CMPCT-029 analysis)

No additional compaction entry paths were discovered during AST scans.
