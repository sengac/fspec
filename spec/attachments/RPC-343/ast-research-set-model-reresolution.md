# RPC-343 — AST research: mid-session model re-resolution call sites

AST analysis (AstGrep, language=rust) of the code paths involved in the
mid-session `set_model` fix. Confirms the exact functions to modify and reuse.

## The path to fix

| Symbol | Location | Role |
|---|---|---|
| `fn set_model(&self, session_id, provider_id, model_id) -> Result<(), String>` | `codelet/sessions/src/handle_impl.rs:1008` | **MODIFY** — mid-session entrypoint; currently only swaps label strings |
| `fn get_session_model(&self, session_id) -> SessionModel` | `codelet/sessions/src/handle_impl.rs:178` | READ — returns cached_context_window / cached_max_output_tokens / cached_compaction_threshold (the values that go stale) |
| `pub fn set_model(&self, provider_id, model_id)` | `codelet/sessions/src/background_session.rs:722` | label-only string swap (the cosmetic update) |
| `pub fn set_model_limits(&self, context_window: u32, max_output_tokens: u32, compaction_threshold: u32)` | `codelet/sessions/src/background_session.rs:729` | **CALL** — writes the three cached limits that get_session_model reads |

## What to reuse (creation-time resolution to mirror)

| Symbol | Location | Role |
|---|---|---|
| `session.set_model_limits(...)` (creation) | `codelet/sessions/src/session_manager.rs:557`, `:803` | the creation-time caching the fix must mirror |
| `resolve_compaction_threshold(...)` | `codelet/sessions/src/session_manager.rs:551`, `:797` | recompute compaction threshold from ctx/out/model_id |
| `pub fn provider_manager_mut(&mut self) -> &mut ProviderManager` | `codelet/cli/src/session/mod.rs:148` | reach the inner request-issuing manager to re-select the model |
| `pub fn select_model(&mut self, model_string: &str) -> Result<&ModelInfo, ProviderError>` | `codelet/providers/src/manager.rs:437` | re-resolve provider+model+limits in place (re-detects credentials at :446, sets current_provider at :491) |

## Findings

1. `set_model` (handle_impl.rs:1008) currently delegates to BackgroundSession::set_model
   (background_session.rs:722) which swaps two String fields only — **no re-resolution**.
2. `set_model_limits` (background_session.rs:729) is the single sink for the three cached
   limit fields and is called ONLY at creation (session_manager.rs:557, :803), never from set_model.
3. The inner request-issuing manager is reachable via `provider_manager_mut()`
   (session/mod.rs:148); `select_model` (manager.rs:437) switches provider in place and
   re-detects credentials — no manager rebuild required, no async needed.
4. Fix: in handle_impl.rs:1008, after the label swap, lock inner, call
   `provider_manager_mut().select_model("provider/model")` (or set_model_direct for
   profile/codex/custom), read context_window()/max_output_tokens(), recompute via
   resolve_compaction_threshold(), and call session.set_model_limits(...). On error, return
   Err and leave prior state intact.
