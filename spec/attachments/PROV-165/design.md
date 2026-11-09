# PROV-165 design: unavailable model limits are a typed state, not 0

## Problem

BUG-330 (commit 1b64044c) made Codex declare no static limits by returning `0`:

- `rust/providers/src/codex/mod.rs:123-131` — `default_context_window()` / `default_max_output_tokens()` return `0`.
- `rust/providers/src/manager/impl_4.rs:176-181` — the manager's Codex `ConstantResolver` uses `default_ctx: 0, default_out: 0`.
- `rust/providers/src/model_limits.rs:79-101` — `resolve_model_limits(.., provider_default: usize) -> usize` returns that 0 as if it were a limit.

`0` then flows on as a number:

- `cli/src/interactive/stream_loop/preflight.rs:31-55` and `cli/src/provider_runtime.rs:3-8` decode it again with `> 0`;
- `cli/src/compaction_threshold.rs:87-100`, `cli/src/compaction_publication.rs:19-35`,
  `agent-loop/src/turn_compaction.rs:27-60` and `sessions/src/model_resolution.rs:231-244` compute a
  compaction threshold from it — and `core/src/compaction_hook.rs:131,160` compacts whenever
  `total > threshold`, i.e. on every request when the threshold is 0;
- `sessions/src/background_session` caches it in `AtomicU32`s whose initial value is also 0, and
  the wire builders (`handle_impl_1.rs:236-238`, `handle_impl_4.rs:217-218`) publish it.
- `ProviderType::Custom` guesses `128_000` / `4096` when no user value exists (`impl_4.rs:195-202`).

## Design

### Types (codelet-providers)

- `ModelLimitsResolver::default_context_window(&self) -> Option<usize>` and
  `default_max_output_tokens(&self) -> Option<usize>`: `None` means the provider declares no
  static limit. Codex returns `None`; providers with constants return `Some(CONST)`; OpenAI returns
  `Some(self.context_window)`.
- `resolve_model_limits(registry, user, max, provider_default: Option<usize>) -> Option<usize>`,
  `resolve_context_window(..) -> Option<usize>`, `resolve_max_output_tokens(..) -> Option<usize>`.
  Priority is unchanged (user → catalog → declared default, each clamped by the ceiling).
- `ConstantResolver { default_ctx: Option<usize>, default_out: Option<usize> }`; Codex `None`,
  `Custom(_)` `None` (no guessed defaults — the user value already outranks the default, so the
  former `unwrap_or(128_000)` only ever produced a guess).
- `ProviderManager::context_window(&self) -> Option<usize>` and `max_output_tokens(&self) -> Option<usize>`.
  `raw_model_context_window/raw_model_max_output_tokens` keep their meaning (catalog/user data exists).
- `ProviderManager::get_openai` fails with `ProviderError::config("openai", "max output tokens unavailable for <model>")` when the output limit is `None`.

### Error for work that needs limits (codelet-cli)

New module `codelet_cli::model_limits`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelLimitKind { ContextWindow, MaxOutputTokens }

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("{limit} unavailable for {model}: set it in the model settings or select a catalogued model")]
pub struct ModelLimitUnavailable { pub limit: ModelLimitKind, pub model: String }

pub struct RequiredLimits { pub context_window: u64, pub max_output_tokens: u64 }

pub fn required_limits(manager: &ProviderManager) -> Result<RequiredLimits, ModelLimitUnavailable>;
```

`model` is the manager's `selected_model_string()`, or its provider name when no model is selected.
Callers: prompt preflight (the error stops the turn before any request; the downstream stream loop
keeps `u64` because a prompt never runs without both limits), `CompactionPublication::new`
(now `Result`), and `TurnCompaction::capture` (the error is kept and returned by `submit`, like the
existing evidence error).

`provider_runtime::output_limit` returns `Option<u64>` straight from the manager (no `> 0` decode).
`generate_compaction_handler::Builder` already holds `Option<usize>` limits and takes the manager's
`Option` directly.

### Compaction threshold

`resolve_compaction_threshold(context_window: Option<u64>, max_output: Option<u64>, ..) -> Option<u64>`
— `None` unless both limits are available. Callers that already hold `RequiredLimits` pass `Some`.

### Session cache (codelet-sessions)

`ResolvedModelLimits { context_window: Option<u32>, max_output_tokens: Option<u32>, compaction_threshold: Option<u32> }`.
`BackgroundSession`'s three `AtomicU32` caches become one `RwLock<ResolvedModelLimits>` (initially all
`None`) read through `model_limits()`; `set_model_limits(ResolvedModelLimits)`.
`committed_publication` requires the cached window and threshold and fails with a clear error when
either is unavailable.

### Split (reviewable cards)

| Card | Scope |
|---|---|
| **PROV-165** (this) | resolver, resolution functions, ProviderManager, threshold, cli/agent-loop consumers, session cache. |
| **PROV-166** | wire: `SessionModel.{context_window,max_output_tokens,compaction_threshold}` and `ModelInfo.{context_window,compaction_threshold}` become `Option`; TUI renders "unavailable". Until then `handle_impl_1::get_session_model` and `handle_impl_4::get_model_info` are the only places that encode `None` as the wire's existing 0 — one marked, transitional adapter (`wire_limit`), removed by PROV-166. |
| **PROV-167** | `LlmProvider::context_window()/max_output_tokens()` → `Option<usize>` (Codex `None`), all implementations and callers. No production code reads them today, which is why they can move separately. |

## Alternatives rejected

- **`ModelLimit` enum `{ Known(u64), Unavailable }`** — isomorphic to `Option<u64>` without the std
  combinators; every consumer would re-implement `map`/`ok_or`.
- **`Result` from `ProviderManager::context_window`** — "unavailable" is a valid, expected state
  (an uncatalogued Codex model is selectable, codex-catalog-limit-resolution); only work that needs
  a limit turns it into an error, so the error lives in `required_limits`.
- **One card for everything including the wire and LlmProvider** — ~180 edited call sites across
  eight crates in one review.
- **Treat `None` as "never compact" and keep sending prompts** — admission would still refuse them
  (`request_admission.rs:13-29` `MissingContextCapacity`); failing in preflight with the limit and
  model named is the explicit version of the same outcome.

## Wire / API changes

- Public Rust API: the resolver trait methods, resolution functions, `ProviderManager` limit
  accessors and `resolve_compaction_threshold` change signature as above;
  `BackgroundSession::{cached_context_window, cached_max_output_tokens, cached_compaction_threshold}`
  are replaced by `model_limits()` / `set_model_limits(ResolvedModelLimits)`.
- No RPC wire change in this card (PROV-166).

## Out of scope (reported)

- `custom/config.rs:47-54` serde defaults (200000 / 4096) for custom provider model config and the
  `OPENAI_CONTEXT_WINDOW` / `OPENAI_MAX_OUTPUT_TOKENS` env parsing that silently falls back to the
  constants (`impl_4.rs:155-162`) are guesses of the same kind; they need their own cards.
