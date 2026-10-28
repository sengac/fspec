# BUG-281 — authoritative source/ACDD closure

Date: 2026-10-28. Closure session: 38c9951d-1ec2-4d84-bd7b-ee6e066c78f0.
All applicable source-level requirements pass; normal validating → done follows
registration of this report and its raw/hash attachments. Canonical Fspec status
and the separately attached final native status receipt record the transition.
This is source/ACDD completion, not deployment or repair of the installed daemon.

## Supersession and preservation

This report supersedes the current-result and remaining-blocker claims in
`BUG-280-281-validation-evidence.md`, including its obsolete 160-pass/1-fail
result and the assertion that a standalone executable build is required.
That report, its archive, their historical copies, original causal RED and the
original `session-work-unit-research.md` remain preserved, not rewritten.
The research SHA256 remains
`5e43e550f4e63cbb577f595a3693cfdeea81a491d4aba4623a8d59cefed652ce`.
Earlier manifests are historical snapshots, not the current Rust manifest.

## Acceptance and implementation

New Agent and right-edge Shift Right capture absent story intent; normal and
isolated new agents start unassigned despite stale global selection or a current
agent's different/cleared focus. Within-list navigation creates nothing. Dialog
title and submission share the same captured optional intent. Generic observation
and repeated completion cannot attach ambient selection. Existing/resumed agents
retain authoritative focus. Cancellation, decline and failure cannot leak story
intent into later generic creation; observation/focus races and duplicate story
completion preserve explicit targets and newer revisions.

Three generic-creation, four completion and three coordinated board-entry
scenarios retain 1:1 scenario/test/implementation mappings. All ten behavioral
acceptance tests pass. Original causal RED and cancellation RED remain in prior
evidence. No production Rust or test source changed in this closure session.

## Current executed Rust validation

The already-completed fresh isolated runs, not a redundant rerun here, establish:

| Receipt | Result |
| --- | --- |
| `tui-acceptance-regressions.log` | 161 passed, 0 failed, 15 targets, exit 0; includes all 10 acceptance tests |
| `persistence-regressions.log` | 12 passed, 0 failed, 2 targets, exit 0 |
| Total executed scoped tests | **173 passed, 0 failed** |
| `package-check.log` | Affected packages compile, exit 0 |
| `strict-clippy.log` | Applicable library/test checks with `-D warnings`, exit 0 |
| `library-build.log` | Applicable library build, exit 0 |
| `changed-file-format.log` | Changed Rust files pass rustfmt, exit 0 |

Exact argv, isolated environment roots, per-stage source manifests and raw outputs
are in `BUG-280-281-closure-raw-evidence.tar.gz`. Runs use offline/locked ci-test,
debug0, -j2, original workspace and existing dedicated Cargo cache. Synthetic
user/config/credential/data/session roots isolate real user state.
Every current stage manifest has SHA256
`b7b716f409875da1ac3c812b8e3e9b882a029e805ed1d890583878814e055ce5`.
`source-and-history-provenance.log` preserves handoff and historical provenance.

## Native Fspec closure checks

All commands use the native Fspec tool, not a shell CLI. The archived
`native-scoped-validation.json` transcribes exact scoped validation/audit payloads
and explicitly labels summaries. Results:

- Syntax: all 9 applicable updated specifications valid, zero syntax errors.
- Acceptance tags: all 3 focused features pass, before and after `@done` addition.
- Other scoped tags: 5 pass; legacy `rust-mux-mode` retains 5 baseline placement
  errors for unrelated @BUG-164/@BUG-165 scenario tags. No global pass claimed.
- Coverage: all 10 acceptance scenarios fully covered (3/4/3); all 8 applicable
  coverage audits pass, with 191 mapped file instances found, no links removed.
- Completion: registered `@done` added only to the three focused acceptance
  features, formatted natively and independently revalidated; tag lists confirm it.
- Both cards have no children or blocking dependencies; native lifecycle hooks
  and card virtual hooks are empty. No configured quality command is pending.

## Limitations, denials and safety

The historical unrelated package-format failure remains baseline; changed-file
formatting passes. Global tests/format/tags are not represented as all-green.
The standalone `--bin fspec` build is denied, not pending, and is not a configured
completion gate. The legacy architecture-note patch and denied temporary issue
plan remain denied and were never retried/re-expressed. The denied compound shell
audit was not replayed; its exact preserved receipt is in the new archive.

No agents/sessions were created or managed, model changed, previous failed session
resumed/recovered/compacted/cleared, history edited, credentials read, source
reimplemented, assertions weakened, standalone binary built, install/restart,
commit/push, or unrelated cleanup performed. No denial is being treated as success.
No pending/uncertain action is claimed. Installed-daemon interactive validation is
not claimed; deployment remains outside this source closure.

BUG-282 remains source-fix Done. Its second-Linux-failure JSON was attached
unchanged with SHA256
`cc146bb2f60c397f9e903d063d967dcdfd25f758b2c9360e751e5d906d3dcfaa`.
Recurrence in the unchanged installed daemon is preserved evidence, not proof of
live repair or a new source regression; its authoritative source review remains
`spec/attachments/BUG-282/bug282-review-final-validation.md`.

The per-card `BUG-281-closure-hashes.sha256` seals this report, the raw archive,
standalone native-check receipt and retained research. Final native status is
verified after ordinary Done transitions, without skip/force options.
