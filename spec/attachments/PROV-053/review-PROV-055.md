# Review: PROV-055 — GitHub Copilot HTTP middleware, facades & endpoint routing

## Status: **FAIL**

The five facade modules under review are individually clean, well-documented, well-tested, and pass `cargo test` + `cargo build` without warnings. **However, three binding rules from the work unit and feature file are not satisfied by the code on disk**, and the work unit is marked `done`. PROV-055's title literally says "**HTTP middleware**" — the middleware does not exist.

## 🔴 Critical Issues (Must Fix)

1. **`CopilotHttpClient` middleware (`refreshing_client.rs`) does not exist.**
   - Feature file architecture block (`spec/features/github-copilot-http-middleware-and-routing.feature:9`) explicitly lists `refreshing_client.rs (CopilotHttpClient middleware)` in the module layout.
   - **Rule 2** (`PROV-055` example map): "CopilotHttpClient implements `rig::http_client::HttpClientExt` as a middleware layer that wraps every outgoing request, mirroring the RefreshingClaudeClient / RefreshingCodexClient pattern."
   - Work unit description: "**CopilotHttpClient middleware implementing rig::http_client::HttpClientExt** to inject required headers..."
   - Reality: `Grep refreshing_client` in `codelet/providers/src/copilot/` returns **only one hit — a dangling rustdoc link in `provider.rs:92`** (`[CopilotHttpClient](super::refreshing_client)`) pointing at a module that does not exist. `ls codelet/providers/src/copilot/` confirms no `refreshing_client.rs`.
   - The phrase **"when that module lands"** in `provider.rs:90-93` is the author's own admission this is unfinished.
   - **Impact**: this is the headline deliverable of PROV-055. There is no broken-feature-file middleware-test scenario to catch this because the integration test mentioned in architecture note [3] (using wiremock/httpmock) was also never written.

2. **`CopilotProvider` does not implement the `LlmProvider` trait.**
   - **Rule 0**: "CopilotProvider implements the LlmProvider trait and is registered as ProviderType::GitHubCopilot in codelet/providers/src/manager.rs."
   - `Grep "impl LlmProvider for CopilotProvider"` → **No matches found.**
   - All five sibling providers have it: `claude.rs:676`, `codex/mod.rs:474`, `gemini.rs:291`, `openai.rs:471`, `zai.rs:320`.
   - `CopilotProvider` (`provider.rs:94`) is a stateless marker `pub struct CopilotProvider;` with three associated functions only — no `complete`, no `complete_with_tools`, no `name`.
   - **Impact**: a TUI user who selects `/github-copilot` cannot send a chat message — there is no end-to-end code path. The "feature works end-to-end" requirement from CLAUDE.md is not met.

3. **`ProviderManager::get_copilot()` factory method is missing from `manager.rs`.**
   - `Grep "get_copilot|get_github_copilot"` in `manager.rs` → **No matches found.**
   - Every other provider has one: `get_claude`, `get_openai`, `get_codex`, `get_gemini`, `get_zai`.
   - Enum-level wiring exists (`ProviderType::GitHubCopilot`, `from_str`, `as_str`, `has_credentials`, `detect_default_provider`, `context_window`, `max_output_tokens`) — but **no instantiation path**.
   - **Impact**: even if the LlmProvider impl existed, the manager has no way to construct a `CopilotProvider` instance for the dispatcher.

4. **`CopilotHeaderFacade::build_headers` has zero production call sites.**
   - `Grep "CopilotHeaderFacade::build_headers"` returns **only test/doctest/module-doc references** — no real consumer.
   - The whole point of the facade was for the (non-existent) `CopilotHttpClient` middleware to call it per request. With no middleware, the facade is dead code from the runtime's perspective. The unit tests inside `header_facade.rs:117-167` exercise it, but it never affects an actual request.
   - This is a direct consequence of issue #1 but worth flagging on its own because it explains why the test file passes despite the middleware being missing: **the tests call the facades directly, never through an HTTP client**, so they cannot detect the absence of integration.

5. **`CopilotResponsesSystemPromptFacade` is a stub that does not implement the real `BoxedSystemPromptFacade` trait.**
   - **Rule 8**: "CopilotResponsesSystemPromptFacade is a new implementation of `BoxedSystemPromptFacade` used only for /responses endpoint models; chat/completions models use the existing OpenAISystemPromptFacade."
   - Reality (`provider.rs:55-88`): `CopilotSystemPromptFacade` is a **local trait** with one method (`fn provider(&self) -> &'static str`) — explicitly **not** the real `codelet_tools::facade::SystemPromptFacade`. The doc comment at `provider.rs:48-54` confesses: *"This is a **local** interface, not the full `codelet_tools::facade::SystemPromptFacade` trait — we intentionally keep it tiny so PROV-055 does not have to cross the crate boundary. The full integration ... lands in PROV-056."*
   - The Gherkin scenarios (line 64, line 73) assert against `system_prompt_facade.provider()` returning the strings `"copilot-responses"` / `"openai"` — this is satisfied by the stub, **but the assertion only proves the local marker trait works, not that the real SystemPromptFacade is wired anywhere**. The feature scenarios are therefore only superficially passing.
   - This is a deliberate scope deferral to PROV-056, but it means **Rule 8 is unmet** in PROV-055 even though the work unit closed. Either:
     - (a) The rule should have been deferred explicitly to PROV-056, or
     - (b) Rule 8 should not have been marked done.

6. **Rule 7 (OpenAIFspecFacade reuse) has no evidence in the code.**
   - **Rule 7**: "Copilot reuses existing OpenAI-compatible tool facades from `codelet_tools::facade` (OpenAIFspecFacade, openai_bridge_tool, etc.) without defining a new tool facade family..."
   - `Grep "OpenAIFspecFacade|openai_bridge_tool"` in `codelet/providers/src/copilot/` → **No matches found.**
   - There is no test scenario asserting tool-facade selection either. Rule 7 is essentially an unimplemented architectural promise — there's no way to verify it from the artifacts under review.

---

## 🟡 Warnings (Should Fix)

1. **Dangling rustdoc intra-doc link in `provider.rs:92`.**
   `[CopilotHttpClient](super::refreshing_client)` — the target module does not exist. `cargo doc` will emit a broken-link warning. Either delete the comment, gate it behind `#[doc(hidden)]`, or land the module.

2. **`provider.rs:55-88` defines a duplicate `CopilotSystemPromptFacade` trait.**
   This shadows / duplicates the already-existing `BoxedSystemPromptFacade` family in `codelet/tools/src/facade/system_prompt.rs`. Two traits doing morally the same thing in two crates is exactly the kind of separation-of-concerns leak the SOLID review checklist asks about. Pick one — either cross the crate boundary now or wait until PROV-056 to add the responses variant. Don't ship a parallel hierarchy.

3. **Header building duplication is widespread in the providers crate (DRY violation across modules).**
   Per cross-cutting search:
   - `Bearer {token}` is re-implemented in **at least 6 places**: `copilot/header_facade.rs:101`, `claude.rs:274`, `claude.rs:374`, `claude_refreshing_client.rs:204`, `codex/refreshing_client.rs:222`, `openai.rs:303`.
   - The "strip stale Authorization, then inject fresh Bearer" middleware is **byte-for-byte identical** in `claude_refreshing_client.rs:201-206` and `codex/refreshing_client.rs:219-224`.
   - `claude.rs` duplicates its own 3-header block twice (`Authorization`/`User-Agent`/`x-app`) at lines 269-289 and 371-384.
   - Codex builds its `originator: codelet` + `ChatGPT-Account-Id` triple in two different files (`codex/refreshing_client.rs:225-230` and `codex/codex_oauth.rs:131,184-185`).
   - PROV-055 was the natural moment to introduce a `bearer(token) -> HeaderValue` helper or a `replace_authorization(&mut HeaderMap, token)` helper — instead the new `CopilotHeaderFacade` re-implements `Bearer` formatting once more (`header_facade.rs:101`). This propagates the duplication rather than fixing it.

4. **`CopilotBehaviorFacade::family()` breaks naming convention with sibling traits.**
   - `SystemPromptFacade` exposes `fn provider(&self) -> &'static str` returning `"claude"`, `"openai"`, `"gemini"`.
   - `ThinkingConfigFacade` also uses `provider()`-style identifiers.
   - `CopilotBehaviorFacade::family()` (`behavior_facade.rs:24`) is the odd one out. The semantic argument (Copilot is the provider, GPT/Claude/Gemini are families) is defensible, but it prevents any future generic helper from treating these traits uniformly. Either rename, or introduce a shared supertrait `trait NamedFacade { fn name(&self) -> &'static str; }`.

5. **`select_copilot_behavior_facade` defaults unknown prefixes to GPT silently.**
   `behavior_facade.rs:151-153` has `_ => Box::new(CopilotGptBehaviorFacade)`. This is documented (line 139) as "Unknown prefixes default to GPT because GPT is the lowest-common-denominator behaviour set." However:
   - There is **no warning log, no telemetry**, and the unit test (`behavior_selector_dispatches_by_prefix`) actively asserts `mistral-8b → gpt`, locking in silent fallback as correct.
   - The Copilot model catalog comes from a live `/models` endpoint (PROV-056) — if that endpoint adds a new family (e.g. `mistral-*`), every request will silently degrade to GPT semantics with no operator visibility. Recommend at minimum a `tracing::warn!` on the default branch.

6. **`CopilotEndpointFacade::select` digit-prefix parsing edge case.**
   `endpoint.rs:84-91` does `rest.chars().take_while(char::is_ascii_digit).collect()` then `parse::<u32>`. This means:
   - `gpt-` (no digits) → empty string → parse fails → ChatCompletions ✅
   - `gpt-50` → `50 >= 5` → Responses ✅ (probably wrong — `gpt-50` doesn't exist, but if it ever does and is unrelated to the GPT-5 family, this misclassifies it)
   - `gpt-4.5` → digits = `"4"` (`.` halts) → 4 → ChatCompletions ✅
   - `gpt-5-turbo` → digits = `"5"` → 5 → Responses ✅
   - `gpt-5-mini` → handled by literal exclusion ✅

   The logic works for the current Copilot model line, but the "N >= 5" rule is fragile against forward versions like `gpt-50`/`gpt-100`. Recommend an explicit allowlist (`gpt-5*` only) rather than `>= 5`. There is also no test for `gpt-50` or `gpt-100`.

7. **`_shield_unused_imports` in test file is a code smell.**
   `tests/copilot_http_middleware_routing_test.rs:445-453` has `#[allow(dead_code)] fn _shield_unused_imports()` to silence warnings for imports the tests don't actually exercise (`CopilotGptBehaviorFacade`, `CopilotClaudeBehaviorFacade`, `CopilotBaseUrl`, `RequestClassification`, `CopilotBehaviorFacade`). Comment says *"until impl lands"*. Since impl is supposedly done, the shield should be deletable — the fact that it isn't, and that the imports really *are* unused by the actual scenarios, suggests several behaviors that should be tested are not.

8. **Coverage line ranges in `.coverage` file are inflated/inaccurate.**
   `show-coverage` reports `header_facade.rs:60-117` for the first scenario, but lines 60-117 cover the entire `CopilotHeaderFacade` struct + its `build_headers` method including doc comments (line 60 is `pub struct CopilotHeaderFacade;`). The real scenario only exercises a subset of the build path. Same pattern for `endpoint.rs:63-95` and `provider.rs:82-118`. This isn't strictly wrong (the scenario does call into all that code), but the line ranges look auto-generated from "whole function" rather than actual coverage analysis.

9. **Background section of feature file is invalid Gherkin.**
   `spec/features/github-copilot-http-middleware-and-routing.feature:41-44` uses a `Background:` block to put the user story:
   ```gherkin
   Background: User Story
     As a codelet user authenticated with GitHub Copilot
     I want to send chat requests to Copilot models from the codelet TUI
     So that every request is correctly authorized, classified, and routed...
   ```
   `Background:` in Gherkin is meant to hold `Given`/`When`/`Then` steps that run before every scenario, not free-form prose. This passes `fspec validate` (because @cucumber/gherkin tolerates titled prose in Background), but it's a mis-use of the Gherkin construct. The user story should live in a doc string, not a Background.

10. **No agent-mode signaling contract documented in code.**
    `classifier.rs:91-96` decides agent mode by `body.metadata.mode == "agent"`. But:
    - **Where in the codebase is this metadata field actually injected on the way out?** No producer is shown in PROV-055.
    - The test file comment at line 314 says *"signaled via body metadata.mode = 'agent' — slice2 memo §6"*, but the slice2 memo is an attachment, not code. There is no integration showing that any part of codelet actually sets this field when the autonomous agent mode is active.
    - **Result**: scenario 6 ("Agent-mode workflow") proves the *classifier* recognizes the marker, but proves nothing about whether codelet ever *produces* the marker. End-to-end this scenario does not work.

---

## 🟢 Observations (Nice to Have)

1. **`#[must_use]` is consistent on the new facades** (good): `endpoint.rs:74`, `header_facade.rs:76`, `behavior_facade.rs:143`, `provider.rs:104, 122`. By contrast, `select_claude_facade` in `system_prompt.rs:427` is **not** `#[must_use]`. Consider backporting consistency.

2. **`HeaderName::from_static` constants** in `header_facade.rs:23-44` are well-chosen — avoids per-request allocation of header names. Good pattern.

3. **All five facade files are well under the 300-line limit** (max is `behavior_facade.rs` at 222 lines including tests and docs). No refactoring pressure.

4. **Per-file unit tests are present and pure** as architecture note [2] required. Good adherence to that note specifically.

5. **Doc comments are exemplary** — every public type and function has a JSDoc-equivalent rustdoc comment with arguments, returns, and (often) examples. This is the high-water mark in the codelet crate.

6. **Step count between feature file and test file matches exactly**: 43 `@step` comments in the test file, 43 step lines in the feature file. Step text appears to match verbatim on every scenario I sampled.

7. **Test file header references the correct feature file** (`copilot_http_middleware_routing_test.rs:2`).

8. **Edge-case guard tests** (`endpoint_selector_edge_cases`, `behavior_selector_dispatches_by_prefix`, `classifier_text_only_request`, `classifier_detects_responses_api_input_image`) at lines 355-443 add value beyond the Gherkin scenarios — these are exactly the kind of regression-pin tests architecture note [2] asks for.

9. **`CopilotEndpoint::path()`** (`endpoint.rs:29-34`) is a const fn returning `&'static str` — clean, zero-allocation, idiomatic.

10. **`detect_vision_content`** (`classifier.rs:52-83`) handles all three body shapes (OpenAI chat, OpenAI responses, Anthropic messages) with one walker — that's the kind of single-responsibility purity SOLID asks for.

---

## Coverage Verification

- **Feature file**: `spec/features/github-copilot-http-middleware-and-routing.feature` — **OK** (6 scenarios, all required tags present: `@done`, `@rust`, `@authentication`, `@providers`, `@PROV-055`; doc string present at lines 8-13; no prefill placeholders; user story misplaced in Background — see Warning #9).
- **Test file**: `codelet/providers/tests/copilot_http_middleware_routing_test.rs` — **OK** (43 `@step` comments matching feature file 1:1, header comment references feature file, all 6 scenarios + 6 edge-case guard tests pass).
- **Implementation files**:
  - `header_facade.rs` — **OK** (well-formed, but never called from production — see Critical #4)
  - `endpoint.rs` — **OK**
  - `classifier.rs` — **OK**
  - `behavior_facade.rs` — **OK**
  - `provider.rs` — **ISSUE**: dangling rustdoc link, duplicate local trait, no `LlmProvider` impl
- **Scenario coverage (Gherkin → tests)**: **6/6 scenarios** mapped via `@step` comments and `link-coverage` records.
- **Scenario coverage (Gherkin → end-to-end runtime)**: **0/6 scenarios** actually exercise the HTTP middleware that the feature title promises, because no middleware exists. All 6 tests are facade-level unit tests that bypass the missing middleware entirely.

## Build & Test Verification

- `cargo build -p codelet-providers` → **EXIT=0** (clean)
- `cargo test -p codelet-providers --test copilot_http_middleware_routing_test` → **12 passed; 0 failed; 0 ignored**

Both pass — but neither catches the missing middleware, missing `LlmProvider` impl, missing `get_copilot()` factory, or unused `CopilotHeaderFacade`. The green test suite is **misleading**: it certifies that pure functions return correct values, not that PROV-055 is shippable.

## Files Reviewed

1. `spec/features/github-copilot-http-middleware-and-routing.feature` (99 lines)
2. `codelet/providers/tests/copilot_http_middleware_routing_test.rs` (453 lines)
3. `codelet/providers/src/copilot/header_facade.rs` (167 lines)
4. `codelet/providers/src/copilot/endpoint.rs` (151 lines)
5. `codelet/providers/src/copilot/provider.rs` (185 lines)
6. `codelet/providers/src/copilot/behavior_facade.rs` (222 lines)
7. `codelet/providers/src/copilot/classifier.rs` (175 lines)
8. `codelet/providers/src/copilot/mod.rs` (73 lines, for context)
9. `codelet/providers/src/copilot/oauth.rs` (lines 1-152, for normalize_enterprise_domain comparison)
10. `codelet/providers/src/manager.rs` (relevant excerpts: lines 1-100, 340-385, 555-577)
11. `codelet/providers/src/cache_optimization.rs` (lines 80-160, for CacheOptimizationFacade::build_headers comparison)
12. `codelet/tools/src/facade/system_prompt.rs` (lines 420-445, for select_claude_facade comparison)

---

## Recommendation

**Move PROV-055 status back from `done` to `implementing`** (or open a follow-up `PROV-055-FOLLOWUP` story under PROV-053) and complete:

1. Create `codelet/providers/src/copilot/refreshing_client.rs` with `CopilotHttpClient` implementing `rig::http_client::HttpClientExt`, calling `CopilotHeaderFacade::build_headers` per request (Rule 2 — Critical #1, #4).
2. Add `impl LlmProvider for CopilotProvider` (Rule 0 — Critical #2).
3. Add `ProviderManager::get_copilot()` factory method (Rule 0, second half — Critical #3).
4. Either implement the real `BoxedSystemPromptFacade` for `CopilotResponsesSystemPromptFacade` or explicitly defer Rule 8 to PROV-056 (Critical #5).
5. Either implement Rule 7 (OpenAIFspecFacade reuse) with a test, or explicitly defer it (Critical #6).
6. Add a wiremock/httpmock integration test as architecture note [3] required — **this is the test that would have caught all the above missing pieces** (Critical #1).

The five facade files themselves are high-quality work and should not be rewritten — they just need to be **wired into a real HTTP request path**.
