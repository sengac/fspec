# AST Research — PROV-101 selection fallbacks

AST-confirmed locations of the selection fallbacks to remove (matches the
DeepSearch inventory in no-fallback-policy.md).

## #1/#2 handle_impl.rs default-to-anthropic
Pattern: `self.get_default_model().unwrap_or_else(|| $X)` (lang=rust)
- codelet/sessions/src/handle_impl.rs:86  (create_session)
- codelet/sessions/src/handle_impl.rs:816 (create_isolated_session)

## #3 manager.rs detect_default_provider priority chain
Pattern: `fn detect_default_provider($$$ARGS) -> $RET { $$$BODY }` (lang=rust)
- codelet/providers/src/manager.rs:718
Call sites (grep): manager.rs:287 (new), manager.rs:405 (with_model_support),
manager.rs:767 (detect_default_provider_for_test shim).

## #4/#5 model_selector first_selectable_or_zero
Pattern: `pub(crate) fn first_selectable_or_zero($$$ARGS) -> $RET { $$$BODY }` (lang=rust)
- codelet/fspec-tui/src/views/model_selector/rows.rs:137 (definition)
Call sites (grep) in mod.rs: 138 (set_providers else-branch), 164 (adjust_scroll),
278 (toggle_expansion), 449/460/467 (filter handlers), 534 (Home), plus test refs.

## #6/#7/#8 hardcoded anthropic/claude catalog
- #6 codelet/providers/src/models/fallback_models.json — grep across the whole
  repo shows it is referenced ONLY by a doc-comment in
  codelet/sessions/tests/rpc343_mid_session_model_reresolution.rs:14. No Rust
  `include_str!`, no loader. => dead file, safe to delete.
- #7 registry.rs:284-347 and #8 cache.rs:186-208 are both inside
  `#[cfg(test)] mod tests` (create_test_response / cache_content fixture). They
  are NOT runtime selection defaults => test fixtures, left as-is per policy doc
  NOT-fallbacks guidance.

## Offline test data source (confirmed)
ModelCache::new() -> get_data_dir()/cache/models.json. Tests set
`codelet_common::set_data_directory(tempdir)`. To run select_model offline the
temp cache must be seeded with a real models.json (proven: rpc343 test FAILS
when network is blocked AND temp cache is empty; passes when network reachable).
PROV-101 tests seed the temp cache from the on-disk user cache fixture, so they
are fully offline.
