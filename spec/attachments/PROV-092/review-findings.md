# Epic Review: scriptable-subscription-providers (PROV-085 → PROV-092)

**Date:** 2026-04-19
**Reviewer:** Claude Code (fspec review-skill)
**Work Units Reviewed:** 8 (PROV-085, PROV-086, PROV-087, PROV-088, PROV-089, PROV-090, PROV-091, PROV-092)
**Scope:** All uncommitted changes covering the scriptable-subscription-providers epic keystone.

## Summary

| Work Unit | Title | Status | 🔴 Critical | 🟡 Warnings | 🟢 Obs |
|-----------|-------|--------|------------|-------------|--------|
| PROV-085  | Remove BUILTIN_PROVIDER_NAMES guard | WARN | 0 | 5 | 4 |
| PROV-086  | Add cred:: Rhai namespace | WARN | 0 | 6 | 6 |
| PROV-087  | Browser loopback + PKCE flow | **FAIL** | **5** | 10 | 4 |
| PROV-088  | Device flow + auto-refresh middleware | **FAIL** | **4** | 6 | 6 |
| PROV-089  | StreamChunk::ReasoningDelta plumbing | PASS | 0 | 3 | 7 |
| PROV-090  | thinking_config in Rhai build_request | PASS | 0 | 3 | 5 |
| PROV-091  | Multimodal image content bridge | PASS | 0 | 5 | 10 |
| PROV-092  | Complete create_rig_agent keystone | PASS | 0 | 6 | 6 |

**Overall:** 2 work units (PROV-087 & PROV-088) have **critical** implementation gaps that must move back to `implementing`. The other 6 have warnings (mostly DRY/SOLID hygiene, Gherkin ordering, file size).

---

## 🔴 Critical Issues Requiring Rework

### PROV-087 — Browser loopback + PKCE flow (5 critical)

1. **NAPI bindings not re-exported.** `codelet/napi/src/lib.rs:24` declares `mod custom_oauth;` but missing `pub use custom_oauth::*;`. `index.d.ts` contains zero `customOauth*` exports → TypeScript cannot reach the NAPI. Rule [0] violated end-to-end.
2. **Rule [1] not implemented.** `custom_oauth_authorize_start` returns only the script URL + PKCE verifier; never spawns `OAuthCallbackServer`, never binds a listener, never waits for callback. Browser flow is fully delegated to TypeScript — contradicts feature doc and attachment plan.
3. **Scenario 1 test is theatre.** `scriptable_oauth_napi_bridges_tests.rs:139-193` simulates persistence by hand with `std::fs::write`, never driving `OAuthCallbackServer`. 100% coverage reported but effective behavioral coverage ≈ 60%.
4. **Bypasses CredentialStore.** `custom_oauth.rs:93-129` writes tokens with ad-hoc `std::fs::write` on `<fspec_home>/oauth/<provider>.json` — parallel storage to Claude/Codex/Copilot. Architecture note [2] says CredentialStore; reality says otherwise.
5. **DRY violation in `script_provider_aliases.rs:28-134`.** Four near-identical dispatcher blocks; each duplicates the pattern already in `script_provider.rs:114-225`. ~170 lines of duplication collapsible to ~30 with a `invoke_map_fn(name, args)` helper.

### PROV-088 — Device flow + auto-refresh middleware (4 critical)

1. **Architecture doc string lies about path.** Feature line 8 claims `codelet/providers/src/custom/script_refreshing_client.rs`; actual file is `codelet/providers/src/oauth/scripted_refreshing_client.rs`.
2. **"Wraps reqwest::Client" claim is false.** `ScriptedRefreshingClient` (lines 45-48) holds `&ScriptedOAuthProvider` + `provider_name` only — no HTTP client, no request interception. Not middleware in any architectural sense.
3. **Middleware is UNWIRED in production.** `ScriptedRefreshingClient`, `resolve_refresh_middleware`, `RefreshMiddleware::Custom` have zero references outside the test file. `RhaiCustomProvider` never calls `ensure_fresh_if_needed`. ACDD principle "IMPLEMENTATION = CREATION + CONNECTION" violated.
4. **Rule #5 dispatcher never dispatches.** `resolve_refresh_middleware` returns Custom/Builtin but no production code branches on it. Scenario 5 passes vacuously.

---

## 🟡 Warnings (Consolidated)

### Cross-cutting DRY violations

- **`fspec_home()` duplicated 3+ places** — `oauth/cred_module.rs:32-39`, `claude_auth.rs:31-40`, `copilot/auth.rs:154-163`, `custom/discovery.rs:44-45`. Consolidate into a single shared helper.
- **OAuth script dispatcher bodies** — `script_provider_aliases.rs` and `script_provider.rs` have near-identical spawn_blocking→engine_arc→call_fn→cast patterns for every OAuth method. Extract `invoke_map_fn` helper.
- **`LoginImplementation` vs `RefreshMiddleware`** — two 1:1 identical enums (`custom_oauth.rs:45-52` and `scripted_refreshing_client.rs:27-42`). Collapse.
- **JSON-envelope wrapper pattern** duplicated between `custom_oauth.rs` and `custom_oauth_device_json.rs`. Extract `call_and_serialize` helper.
- **`invoke_build_request` vs `invoke_build_stream_request`** (`provider.rs` + `provider_stream.rs`) — shared 3-line prologue with `request_to_rhai`. Extract `build_request_dyn` helper.
- **Dispatch arms in `session_manager.rs`** — custom arm (5461-5506) duplicates MCP wiring + `RigAgent::with_default_depth` + `run_agent_stream_with_images` from openai/claude/codex arms. Extract `run_built_rig_agent` helper.
- **`handle_text` vs `handle_reasoning`** (`stream_convert.rs:66-94`) — identical except variant constructor. Extract `extract_text_from_map` helper.

### Gherkin ordering (preconditions as And-after-Then)

- PROV-085: feature file lines 52, 59 — `FSPEC_DISABLE_SCRIPT_SHADOWING` env var placed as And-after-Then (should be Given).
- PROV-086: feature file lines 56, 64-65, 72-73, 92-93, 100-101 — engine binding + prior write preconditions placed after Then.

### File size creeping past 300 lines

- `provider.rs` — 291 lines (9 from cap)
- `custom_provider.rs` — 287 lines
- `custom_oauth.rs` — 293 lines (7 from cap)
- `rig_model.rs` — 346 lines (over cap)
- `adapter.rs` — 332 lines (pre-existing, over cap)
- `manager.rs` — 2274 lines (pre-existing, far over cap, not PROV-085's regression)

### `expect()` / `unimplemented!()` in production hot paths

- `rig_model.rs:184` — `unimplemented!("RhaiCustomProviderModel must be constructed via …")` in rig's `make()` factory. Unreachable by design; replace with `Err(CompletionError::...)` for defensiveness.
- `custom_provider.rs:219` — `.expect("non-empty after empty-check above")`. Replace with `let Some(first) = iter.next() else { return agent_builder.build(); };`.
- `rig_model.rs:163` — `OneOrMany::many(items).expect(...)`. Use `OneOrMany::one(...)` on the fallback branch instead.
- `adapter.rs:158-163` vs `rig_message_convert.rs:114-117` — assistant-side images: hard error in one path, silent drop in the other. Pick one policy.

### Security & correctness

- **PROV-087 no state-parameter validation** — `callback_server.rs::validate_state` exists but `custom_oauth.rs` never calls it. CSRF-hardening gap.
- **PROV-087 PKCE verifier supplied 100% by script** — no entropy audit, a malicious/misbehaving script can break PKCE.
- **PROV-086 `HOME` env var fallback to `/tmp`** — world-writable dir. Either error or document.
- **PROV-091 magic `image/png` default** when `media_type: None` — risk of wrong Content-Type.

### Dead code / scope drift

- PROV-087: `load_scripted_provider_for` drops every `ProviderConfig` field except `name`, `display_name`, `script`, `auth_url` (hardcodes rest to None).
- PROV-088: `auth_poll_or_legacy` missing from `script_provider_aliases.rs` — inconsistent with other `auth_*_or_legacy` pattern.
- PROV-092: `uses_rhai_system_prompt_facade()` returns hardcoded `true` — holdover from opaque-shim era; simplify or remove.

### Stale documentation

- PROV-085: `config.rs:268` doc-comment still says "name pattern, built-in conflict, script existence" — "built-in conflict" phrase is stale.
- PROV-088: feature doc string at line 8 has wrong path.
- PROV-090: example [3] says "without thinking_config being passed further (wiring parity only)" but implementation threads it into `additional_params` and back via `extract_thinking_config`.

---

## Remediation Plan

### Phase A — Critical Rework (PROV-087, PROV-088)

These work units must move `done → implementing`. Two patterns of fix:

**Pattern 1: Either implement the claim or update the spec.**

For PROV-087 rule [1]: "NAPI layer reuses callback_server.rs and opens browser" — EITHER:
- (A) Actually implement `browser_loopback_authorize()` in Rust that spawns `OAuthCallbackServer`, opens browser, awaits callback. This is the architecturally correct choice and matches Claude/Codex/Copilot conventions.
- (B) Update feature doc + rule [1] + architecture notes to explicitly say "TypeScript owns the browser + callback server; Rust only produces URL + verifier" and add a new scenario covering the split-responsibility contract.

Same for PROV-088 rule #4 ("middleware wraps reqwest::Client"): either make it real middleware (wrap `RefreshingHttpClient`) or demote to "ensure-fresh helper invoked manually" and fix the feature doc string.

**Pattern 2: Wire up production paths.**

- PROV-087: Add `pub use custom_oauth::*;` to `codelet/napi/src/lib.rs`; regenerate `index.d.ts`.
- PROV-088: Either call `ScriptedRefreshingClient::ensure_fresh_if_needed` from `RhaiCustomProvider::complete_with_tools*` / NAPI dispatcher, OR remove the unwired code and re-scope PROV-088 to be the standalone-helper version.
- PROV-087: Replace ad-hoc file I/O with `CredentialStore` (or justify the deviation in architecture notes).

### Phase B — DRY/SOLID Hygiene (all 8 work units)

Batch refactors that touch many files:

1. Create `codelet/providers/src/paths.rs` — single `fspec_home()` that `claude_auth`, `copilot/auth`, `oauth/cred_module`, `custom/discovery` all call.
2. Create `codelet/providers/src/oauth/script_invoke.rs` — `invoke_map_fn(provider, name, args, fallback_name)` helper; collapse `script_provider.rs` + `script_provider_aliases.rs` duplicates.
3. Merge `LoginImplementation` and `RefreshMiddleware` into a single enum.
4. Extract `build_request_dyn(messages, tools, thinking_config)` helper in `custom/provider.rs` to serve both `invoke_build_request` and `invoke_build_stream_request`.
5. Extract `extract_text_from_map(map, key)` helper in `stream_convert.rs`.
6. Extract `run_built_rig_agent<M>(agent, session, mcp_wrappers, …)` helper in `session_manager.rs` to serve custom / openai / claude / codex arms.

### Phase C — Gherkin & Documentation Cleanup

1. Reorder And-after-Then preconditions to Given blocks in PROV-085 and PROV-086 feature files.
2. Fix stale doc-comment in `config.rs:268` (remove "built-in conflict").
3. Fix feature doc string path in PROV-088 feature file (oauth/ not custom/).
4. Update PROV-090 example [3] to reflect actual scope overlap with PROV-092.

### Phase D — Production Hardening

1. Replace `unimplemented!()` in `rig_model.rs:184` with `Err(CompletionError::InvalidRequest(...))` for defensiveness.
2. Replace `.expect(...)` in `custom_provider.rs:219` and `rig_model.rs:163` with let-else pattern.
3. Unify assistant-side image policy (hard-error everywhere, or silent-drop everywhere with documentation).
4. Add state-parameter validation to `custom_oauth.rs` exchange flow.
5. Add explicit error for `image_to_source` failure modes (don't silently drop images).

---

## Validation Results

All feature files pass `fspec validate`. Tag registry has 273 files with tag violations (pre-existing across the project; not introduced by this epic).

Cargo tests (per work unit):

- PROV-085: 5/5 pass
- PROV-086: 9/9 pass
- PROV-087: 5/5 pass (but scenarios 1 and 3 are theatre per C3)
- PROV-088: 5/5 pass (but scenarios 4 and 5 are unwired per C3/C4)
- PROV-089: 6/6 pass
- PROV-090: 4/4 pass
- PROV-091: 8/8 pass
- PROV-092: 8/8 pass

Full `codelet-providers` suite: 317/318 pass. The single failure `custom_provider_is_unavailable_when_required_env_var_is_unset` is pre-existing in credentials.rs (not introduced by this epic).

---

## Next Steps

1. Move PROV-087 and PROV-088 back to `implementing`.
2. Execute Phase A fixes with new tests where needed.
3. Execute Phase B refactors in a single cohesive pass (all 8 work units touched).
4. Execute Phase C Gherkin cleanup.
5. Execute Phase D hardening.
6. Re-verify tests, coverage, validation.
7. Move PROV-087 and PROV-088 back through `validating → done`.
