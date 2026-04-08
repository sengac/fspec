# Epic Review: PROV-053 — Add GitHub Copilot provider

**Date:** 2026-04-07
**Reviewer:** Claude Code (fspec review skill)
**Work Units Reviewed:** 4 (PROV-053, PROV-054, PROV-055, PROV-056)

## Summary

| Work Unit | Status | Critical | Warnings | Observations |
|-----------|--------|----------|----------|--------------|
| PROV-053 (parent) | WARN | 6 | 6 | 6 |
| PROV-054 (OAuth) | WARN | 0 | 12 | 10 |
| PROV-055 (middleware) | **FAIL** | 6 | 10 | 10 |
| PROV-056 (models) | WARN | 3 | 7 | 15 |
| **TOTAL** | **FAIL** | **15** | **35** | **41** |

## Consolidated Critical Issues (Must Fix)

### Missing HTTP Middleware Layer (PROV-053 #2, PROV-055 #1, #4)
- `codelet/providers/src/copilot/refreshing_client.rs` does NOT exist
- `CopilotHttpClient` implementing `rig::http_client::HttpClientExt` is missing
- `CopilotHeaderFacade::build_headers` has zero production callers
- Integration is broken end-to-end — facades are dead code in the runtime
- Dangling rustdoc link in `provider.rs:92` to non-existent module

### Missing LlmProvider Trait Implementation (PROV-053 #1, #3, PROV-055 #2)
- `impl LlmProvider for CopilotProvider` does NOT exist
- `CopilotProvider` is a unit struct with only associated functions (code smell)
- No `complete()`, `complete_with_tools()`, or streaming methods for Copilot path
- TUI user selecting `/github-copilot` cannot send a chat message

### Missing ProviderManager Factory (PROV-053 #4, PROV-055 #3)
- `ProviderManager::get_github_copilot()` missing from `manager.rs`
- All other providers have this accessor: `get_claude`, `get_openai`, `get_codex`, `get_gemini`, `get_zai`

### Spec/Implementation Mismatches
- PROV-055: `CopilotResponsesSystemPromptFacade` is a stub local trait, not the real `BoxedSystemPromptFacade`
- PROV-055: Rule 7 (OpenAIFspecFacade reuse) has zero evidence in code
- PROV-056: `copilot/provider_options.rs` doesn't exist — `apply_store_false` lives in `models.rs`
- PROV-056: Doc-comments cite non-existent rule numbers `[13]` and `[8]`
- PROV-056: Dual entry-points `fetch_models` and `CopilotModelCatalogService::fetch`

### Coverage/Test Issues
- PROV-053: Coverage line ranges point at `manager.rs` enum code for scenarios that exercise facade files
- PROV-053: Test header claims `ProviderManager::get_github_copilot` coverage for a nonexistent method

## Consolidated DRY Violations

### OAuth Device Flow Duplication (PROV-054 W3)
- `copilot/oauth.rs` and `codex/codex_device_auth.rs` share ~150 lines of byte-identical code
- Same `SLOW_DOWN_INCREMENT_MS`, `DisplayCallback`, `DevicePollResponse` struct, polling loop, error strings
- Architecture note claimed "not reusable" but inspection shows otherwise

### OAuth Token Storage Duplication (PROV-053 W1, PROV-054 W4, W5)
- `claude_auth.rs` and `copilot/auth.rs` are ~80% byte-identical
- `get_fspec_home()` is byte-identical between both files
- `claude_auth.rs` is missing mode 0600 enforcement

### HTTP Fetch + Bearer Auth Duplication (PROV-055 W3, PROV-056 W2)
- `Bearer {token}` pattern re-implemented in at least 6 files
- "Strip+inject Authorization" middleware byte-identical in `claude_refreshing_client.rs` and `codex/refreshing_client.rs`
- Catalog HTTP fetch scaffolding duplicated across `copilot/models.rs`, `openai.rs`, `models/cache.rs`

## Consolidated File Size Violations (>300 lines)

- `codelet/providers/src/copilot/oauth.rs` — 364 lines
- `codelet/providers/src/copilot/models.rs` — 438 lines

## Detailed Per-Work-Unit Findings

See individual review files:
- [review-PROV-053.md](./review-PROV-053.md)
- [review-PROV-054.md](./review-PROV-054.md)
- [review-PROV-055.md](./review-PROV-055.md)
- [review-PROV-056.md](./review-PROV-056.md)

## Fix Plan

### Phase A: Move work units backward (done → implementing)
All 4 work units must go back through the workflow.

### Phase B: Extract shared abstractions (addresses most DRY violations)
1. Create `codelet/providers/src/credentials_store.rs` — generic OAuth credential file persistence (trait-based)
2. Create `codelet/providers/src/oauth_device_flow.rs` — shared RFC 8628 device flow (dialect-parameterised)
3. Create `codelet/providers/src/http_helpers.rs` — shared Bearer header + JSON fetch helpers

### Phase C: Refactor per-file
1. Split `copilot/models.rs` into `copilot/models/{schema,fetch,builder}.rs`
2. Create `copilot/provider_options.rs` with `apply_store_false`
3. Split `copilot/oauth.rs` (after extraction to shared device flow)
4. Remove duplicate local trait `CopilotSystemPromptFacade` from `provider.rs`

### Phase D: Build missing pieces
1. Create `copilot/refreshing_client.rs` — `CopilotHttpClient` middleware
2. Add `impl LlmProvider for CopilotProvider` — the real trait impl
3. Add `ProviderManager::get_github_copilot()` factory method
4. Wire `CopilotHeaderFacade` through the middleware
5. Wire `CopilotResponsesSystemPromptFacade` to real `BoxedSystemPromptFacade`

### Phase E: Test/coverage fixes
1. Re-link coverage with correct impl line ranges for PROV-053
2. Fix test file header comment in `copilot_provider_manager_integration_test.rs`
3. Add integration test using wiremock for HTTP middleware
4. Fix test coverage gaps in PROV-054 (W7, W8)

### Phase F: Cleanup
1. Fix stale rule-number citations in `models.rs`
2. Remove `clamp_u64_to_u32` (use `u32::try_from`)
3. Move user story out of `Background:` block in PROV-055 feature file
4. Add `tracing::warn!` on silent GPT fallback in behavior selector
5. Remove `_shield_unused_imports` workaround
6. Remove broken rustdoc intra-doc link in `provider.rs:92`

### Phase G: Verification
1. `cargo test -p codelet-providers` — all tests pass
2. `cargo build -p codelet-providers` — clean build
3. `fspec validate` — all features valid
4. `fspec validate-tags` — all tags valid
5. `fspec show-coverage` — 100% on all 4 feature files
6. Move work units forward: implementing → validating → done
