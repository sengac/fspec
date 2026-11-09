# PROV-167 design: provider objects report undeclared limits as None

## Problem

After PROV-165/166 the only remaining 0-sentinel for model limits is the provider object API:
`LlmProvider::context_window(&self) -> usize` and `max_output_tokens(&self) -> usize`
(rust/providers/src/lib.rs:134-138). `CodexProvider` answers `0` for both
(rust/providers/src/codex/completion.rs:62-71, BUG-330), meaning "no declared limit".

## Design

- `LlmProvider::context_window(&self) -> Option<usize>` and `max_output_tokens(&self) -> Option<usize>`.
- Providers that declare limits return `Some(..)`: Claude (`claude.rs:700-710`), OpenAI
  (`openai.rs:518-524`), Gemini, Z.AI, Copilot, custom (`custom/provider.rs:228-234`) and the stub
  provider. `CodexProvider` returns `None`.
- No production code reads these methods (the session and request paths read `ProviderManager`, see
  PROV-165 ast-research), so only tests and the trait implementations change.

## Alternatives rejected

- **Remove the methods from `LlmProvider`** — they are part of the provider API exercised by tests;
  removing them is a separate API decision, not the sentinel fix.

## Wire / API changes

Public Rust API: the two `LlmProvider` methods return `Option<usize>`. No RPC change.
