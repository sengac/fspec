# COV-062 AST research

## fspec astgrep --pattern 'async fn $NAME() -> TestResult { $$$BODY }' --lang rust --path rust/sessions/tests/codex_catalog_limit_resolution.rs
rust/sessions/tests/codex_catalog_limit_resolution.rs:89:1:async fn creating_a_codex_session_resolves_limits_from_its_source_catalog() -> TestResult {
rust/sessions/tests/codex_catalog_limit_resolution.rs:104:1:async fn switching_codex_models_refreshes_limits_from_the_source_catalog() -> TestResult {
rust/sessions/tests/codex_catalog_limit_resolution.rs:119:1:async fn an_uncatalogued_codex_selection_does_not_inherit_guessed_limits() -> TestResult {
rust/sessions/tests/codex_catalog_limit_resolution.rs:133:1:async fn a_partial_codex_model_name_does_not_authorize_catalog_capacity() -> TestResult {


## fspec astgrep --pattern 'assert_limits($$$A)' --lang rust --path rust/sessions/tests/codex_catalog_limit_resolution.rs
rust/sessions/tests/codex_catalog_limit_resolution.rs:42:5:assert_limits(&session, "codex/gpt-6.1-sol", Some(400_000), Some(128_000))
rust/sessions/tests/codex_catalog_limit_resolution.rs:99:5:assert_limits(&session, "codex/gpt-6.1-sol", Some(400_000), Some(128_000))
rust/sessions/tests/codex_catalog_limit_resolution.rs:114:5:assert_limits(&session, "codex/gpt-4o", Some(128_000), Some(16_384))
rust/sessions/tests/codex_catalog_limit_resolution.rs:128:5:assert_limits(&session, "codex/unknown-model", None, None)
rust/sessions/tests/codex_catalog_limit_resolution.rs:142:5:assert_limits(&session, "codex/gpt-4", None, None)


## fspec astgrep --pattern 'session.set_model_limits($A)' --lang rust --path rust/sessions/src
rust/sessions/src/session_creation_helper.rs:240:5:session.set_model_limits(resolved)


## Findings

- Four scenario tests (lines 89/104/119/133); `assert_limits` (line 58) is the single
  assertion helper checking transport identity, active limits, cached limits and the
  compaction threshold, so every Then/And step of each scenario is proved by its call.
- Creation caches limits at session_creation_helper.rs:240; a model switch caches them in
  `SessionManagerHandle::set_model` (handle_impl/handle_impl_6.rs:5-95, `set_model_limits`
  at line 74, which the pattern above does not match because the receiver is the session
  handle bound in a nested scope). Both read `ProviderManager::refresh_runtime_options` /
  `selection_model_info` (providers/src/manager_runtime.rs:65-113).
