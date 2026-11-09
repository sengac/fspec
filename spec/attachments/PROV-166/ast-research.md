# PROV-166 AST research

## fspec astgrep --pattern 'SessionModel { $$$F }' --lang rust --path rust/sessions/src
No matches found

## fspec astgrep --pattern 'SessionModel { $$$F }' --lang rust --path rust/rpc/src
No matches found

## fspec astgrep --pattern 'SessionModel { $$$F }' --lang rust --path rust/core/src
No matches found

## fspec astgrep --pattern 'codelet_rpc_types::ModelInfo { $$$F }' --lang rust --path rust/sessions/src
rust/sessions/src/cloud_models.rs:264:24:codelet_rpc_types::ModelInfo {
rust/sessions/src/cloud_models.rs:271:17:codelet_rpc_types::ModelInfo {


## fspec astgrep --pattern 'wire_limit($$$A)' --lang rust --path rust/sessions/src
No matches found

## fspec astgrep --pattern 'size_badge_label($$$A)' --lang rust --path rust/fspec-tui/src
rust/fspec-tui/src/views/agent/header_build.rs:74:30:size_badge_label(m.compaction_threshold, m.context_window)


## Struct literals (astgrep does not match these struct-expression patterns; found with grep)

- `SessionModel {` — rust/sessions/src/handle_impl/handle_impl_1.rs:234 (live session) and :242
  (unknown session, all 0); rust/rpc/src/service_implementation/session_control.rs:121 (no handle,
  all 0); rust/core/src/session_manager_handle/stub/stub_handle_1.rs:196,203 and
  handle_trait_2.rs:71 (trait default / stub, all 0); test constructions in fspec-tui and rpc-types.
- `wire_limit` is called by path (`crate::model_resolution::wire_limit`) at handle_impl_1.rs:236-240
  and handle_impl_4.rs:217-218 — the PROV-165 adapter this card removes.

## Findings

- Every default `SessionModel` uses `0` for all three limits; with Option they become `None`.
- `ModelInfo` docs (rust/rpc-types/src/chrome.rs:27-32) define `compaction_threshold == 0` as
  "unknown"; `size_badge_label` (header_build.rs:227-238) decodes both 0s.
