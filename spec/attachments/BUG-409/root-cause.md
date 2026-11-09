# BUG-409 root cause

`sessions_ordered_by_most_recently_updated_first` (rust/sessions/tests/tui099_session_list_ordering.rs) creates real
sessions with default model `anthropic/claude-sonnet-4` in a fresh temp data directory. Nothing seeds the models.dev
cache or an anthropic credential there, so `resolve_provider_manager -> ModelRegistry::new -> ModelCache::get()` fails
(the BUG-186 pattern the RPC-422 and RPC-002 tests already fix), the legacy `create_session` turns the error into an
empty id (BUG-399), and the first `expect("session A should exist")` panics. The sibling scenarios never check the
ids they create, so they pass.

Fix: `make_temp_data_dir` in tests/tui099_session_list_ordering_support/mod.rs seeds `cache/models.json` from
tests/fixtures/rpc422_models.json (which knows claude-sonnet-4) and sets a dummy `ANTHROPIC_API_KEY`, as RPC-422 does.
