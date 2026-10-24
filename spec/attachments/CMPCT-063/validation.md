# CMPCT-063 validation

Completed on 2026-10-17. Scope: explicit lossless original-envelope retrieval,
shared core reader, both SessionSearch handlers, bounded wire responses, and DAG
builder instructions. Existing search/show preview behavior remains unchanged.

## ACDD evidence

- Feature: `spec/features/lossless-original-envelope-retrieval.feature`.
- Seven scenario tests: `rust/agent-loop/tests/cmpct063_original_retrieval.rs`.
- Initial red run: `/tmp/cmpct063-tests.log`; all seven scenarios failed before
  implementation because the originals action and builder contract were absent.
- Final green run: `/tmp/cmpct063-final-targeted.log`; seven scenarios and three
  Codex reload regression tests pass, including property-based pagination checks.
- Removed seven obsolete duplicate test mappings left by the interrupted session
  and linked builder instruction coverage. Every scenario has one test mapping
  with implementation coverage.

## Passing checks

From `rust/`:

```sh
cargo test --profile ci-test -p codelet-agent-loop --test cmpct063_original_retrieval --test prov149_codex_reload --no-fail-fast
cargo test --profile ci-test -p codelet-providers --test prov149_codex_history
cargo test --profile ci-test -p codelet-core --test blob_store_lifted_test --test provider_extension_persistence --no-fail-fast
cargo test --profile ci-test -p codelet-tools --lib session_search
cargo check -p codelet-tools -p codelet-core -p codelet-providers -p codelet-cli -p codelet-sessions -p codelet-agent-loop -p codelet-napi
cargo clippy -p codelet-tools -p codelet-core -p codelet-providers -p codelet-cli -p codelet-sessions -p codelet-agent-loop -p codelet-napi
```

These test targets pass 96 tests. Logs are `/tmp/cmpct063-final-{targeted,
provider-tests,persistence,tools,check,clippy}.log`.

All eleven Rust files in this card pass targeted rustfmt checks with
`--edition 2021 --config skip_children=true,color=Never`. `git diff --check`
passes. Feature formatting, Gherkin validation, scoped tag validation, and the
feature coverage audit pass.

## Existing failures outside this card

- `cmpct045_generate_compaction_behavioral`: 24 pass, one fails. Its source-string
  assertion requires `DeepSearchTool` directly in `codex/mod.rs`, but the inherited
  Codex refactor wires tools in `codex/agent.rs`. Log:
  `/tmp/cmpct063-final-tests.log`. No code in that provider slice was changed
  for CMPCT-063.
- The previous session's `rpc080_agent_loop_persistence` run reports 11 pass,
  one fail. The test requires a single-line `persist_token_state` call, while
  HEAD already formats that call across multiple lines. Log:
  `/tmp/cmpct063-neighbor-tests.log`.
- Repository-wide tag validation reports 526 unrelated invalid feature files;
  this card's feature passes scoped validation.
- Package-wide `cargo fmt --check` reports unrelated existing formatting issues;
  the files touched by this card pass the targeted check.

No unrelated repairs, additional cards, commits, or branches were made.
