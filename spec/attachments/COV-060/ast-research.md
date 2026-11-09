# COV-060 AST research

## fspec astgrep --pattern 'use codelet::$$$X;' --lang rust --path rust/tests
rust/tests/integration_wiring_test.rs:92:5:use codelet::rig;


## fspec astgrep --pattern 'assert_eq!(provider.context_window(), $N)' --lang rust --path rust/tests
rust/tests/codex_provider_test.rs:84:5:assert_eq!(provider.context_window(), 272_000)
rust/tests/rig_provider_streaming_test.rs:222:5:assert_eq!(provider.context_window(), 200_000)
rust/tests/openai_provider_test.rs:41:5:assert_eq!(provider.context_window(), 128_000)


## Supporting greps (non-AST)

- `grep -n '^use codelet::' rust/tests/codex_provider_test.rs` → lines 7-8
  (`use codelet::agent::{…}`, `use codelet::providers::{CodexProvider, LlmProvider}`).
- `grep -n '^\[package\]' rust/Cargo.toml` → no match: the root manifest is a virtual workspace,
  so nothing compiles `rust/tests/*.rs` or `rust/examples/*.rs`.
- No workspace member is named `codelet` (`grep 'name = "codelet"' rust/Cargo.lock` → 0).

## Findings

- `rust/tests/codex_provider_test.rs:84,87` assert `context_window() == 272_000` and
  `max_output_tokens() == 4096`, the defaults BUG-330 removed; its header names
  `spec/features/codex-provider-chatgpt-backend-api-with-oauth.feature`, which does not exist.
- The other 29 files in `rust/tests/` (7937 lines in total) and the 6 files in `rust/examples/`
  are unbuilt for the same reason; they are outside this card and are tracked separately.
