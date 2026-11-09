# BUG-284 mechanical streaming-loop extraction

This is the separately verified mechanical stage authorized by the human on
2026-10-28 at 22:01:24 UTC. It is not an admission repair or live deployment.
The earlier approval denial and preparation evidence remain unchanged.

## Source and structure

The original 2,564-line source is preserved byte-for-byte at
`/tmp/fspec-bug284/mechanical/stream_loop.before.rs`, SHA-256
`3cbfd1d44f21d7b54515d8b4f6b68e320a150584a1bd8083d2c96474abe7790e`.
The existing two public streaming entrypoints retain their signatures.
The root is now a 102-line facade, with 21 typed child modules. Each child is
under 300 lines; the largest is the 228-line driver. There is one control loop,
no textual include fragments, and no oversized relocated function.

The source inventory follows the approved proposal: annotations; driver;
preflight; telemetry; explicit context/state; polling and interrupted settlement;
assistant, tool-result and usage handling; final response/thinking/completion;
compaction restart; ordered error recovery, truncation, network and terminal
errors; stream end; post-loop finish. Request handoff remains in the driver until
the separately tested semantic admission stage.

An independent read-only DeepSearch review compared every branch to the saved
original and found no material difference in ordering, resets, branching,
stream drop/settlement, cancellation, request-history length, retry budgets or
token accounting. It specifically verified the preserved reset asymmetries:
truncation does not clear accumulated reasoning, whereas network and compaction
recovery do. Interruption and accepted completion explicitly drop the stream
before settlement; ordinary stop paths do not gain a new pre-finish drop.

## Verification

Receipts live under `/tmp/fspec-bug284/receipts/`:

- `mechanical-check-final`: scoped candidate compilation passed.
- `mechanical-clippy-verified`: strict CLI library Clippy passed.
- `mechanical-final-acceptance`: BUG-276 5/5 and BUG-282 3/3 passed; BUG-284
  remained exactly nine compiled tests, seven behavioral failures and two
  passing controls, exit 101. Four actual selected-Codex lineage failures,
  two runtime/cold mismatches and the acknowledged-pending crash loss remain.
  No admission semantics changed to turn these failures green.
- `mechanical-stream-regressions`: 26 tests passed across seven scoped targets,
  with one failure in `stream_recovery_compaction_builder`.
- `mechanical-baseline-regression`: that same exact test,
  `recovery_admits_the_resume_prompt_before_committing_a_replacement`, also
  fails against the byte-identical original source in an isolated diagnostic
  workspace. Both return `builder failure must stop recovery`. It is not
  concealed as a passing regression or attributed to the extraction.
- `mechanical-rollback`: all four provider attempt-authority tests passed.
- Direct scoped Rust formatting check passed.
- `mechanical-protected-bug283`: protected BUG-283 manifest verification passed.
- `mechanical-daemon.sha256`: unchanged running PID 922084 executable remains
  `d2c910442adbe130e69e921b151c4613b6fb445f87e05ecd3857f83c7064eb4a`.

Earlier initial-check and Clippy failures are retained: the extraction initially
needed an explicit callback-ID type and an inlined constant format argument.
They were corrected without weakening lints. Final receipts supersede them.

## Remaining work and metadata boundaries

BUG-284 is still Testing. The normal Implementing transition rejected the five
literal helper-file paths in coverage, despite their single Cargo integration
root. Native validation has no Rust module-target awareness. The supported
solution is a genuine executable root-level scenario facade with exact step
comments and native unlink/link migration, not detached comments or skip flags.
The five outstanding acceptance scenarios still need real tests.

The extraction affects 53 existing feature sidecars that reference the old root
coordinates. Their exact paths are listed in
`/tmp/fspec-bug284/mechanical/affected-coverage-files.txt`. This is an explicit
additional metadata coordination report: none has been silently rewritten.
Migration must preserve scenario/test provenance and unrelated mappings,
coordinate concurrent ownership, and use native coverage commands before Done.

The next scoped proof needs a numeric-loopback-only, test-support-gated Codex
factory redirect, without touching credentials, Cargo or RPC interfaces. The
reported proposed provider files are `rust/providers/src/codex/mod.rs`,
`rust/providers/src/codex/constructors.rs`, and new
`rust/providers/src/codex/provider_test_endpoint.rs`. No such provider edit has
yet occurred. The actual queued fresh-input proof must use normal production
provider construction/dispatch and selected history, not a stub or a direct
replacement for the task loop.

No install, restart, original-history mutation, native-session capacity change,
commit, branch or push occurred. Nothing here certifies the missing admission,
durability, crash-window or repeated-automatic-compaction acceptance matrix.
