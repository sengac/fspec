# PROV-166 design: model limits on the wire are Option

## Problem

PROV-165 (commit 3ef07f2c) made unavailable model limits `None` inside the daemon but kept the wire
types integer, so `sessions::model_resolution::wire_limit` (model_resolution.rs) encoded `None` as 0
for `get_session_model` (handle_impl_1.rs:234-240) and `get_model_info` (handle_impl_4.rs:217-218).
Every default `SessionModel` (handle_impl_1.rs:242, rpc session_control.rs:121, core stub
stub_handle_1.rs:196/203, handle_trait_2.rs:71) also reports 0, and `ModelInfo` documents 0 as
"unknown" (rpc-types/src/chrome.rs:27-32), decoded again by `size_badge_label`
(fspec-tui/src/views/agent/header_build.rs:227-238).

## Design

- `codelet_rpc_types::SessionModel { context_window: Option<i64>, max_output_tokens: Option<i64>, compaction_threshold: Option<i64> }`.
- `codelet_rpc_types::ModelInfo { context_window: Option<u32>, compaction_threshold: Option<u32>, .. }`.
- `get_session_model` maps the cached `ResolvedModelLimits` with `Option::map(i64::from)`; an unknown
  session, a missing handle and the stub/trait defaults report `None`.
- `resolve_model_info(registry, provider, model, context_window: Option<u32>, compaction_threshold: Option<u32>)`.
- `wire_limit` is deleted.
- `size_badge_label(compaction_threshold: Option<u32>, context_window: Option<u32>) -> Option<String>`:
  `compaction_threshold.or(context_window).map(format_context_window)`; no badge when both are
  unavailable (the header never renders 0).
- The MCP dispatch schemas are generated from the types (schemars), so they now allow `null`.

## Alternatives rejected

- **Keep integers and document 0** — the sentinel PROV-165 set out to remove.
- **A separate `limits_available: bool`** — two fields that can disagree.

## Wire / API changes

`SessionModel` and `ModelInfo` limit fields become nullable (JSON `null`). No backward compatibility
is kept (user rule).
