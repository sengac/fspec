# Review: PROV-053 — Add GitHub Copilot provider

## Status: **WARN**

The work compiles, all 11 integration tests pass, the feature file is well-structured, every scenario has a matching test with `@step` comments, and the example map → scenarios → tests → impl chain is fully traceable. The headline issues are:

1. **The parent story's own architecture promises are not delivered** — no `LlmProvider` impl, no `CopilotHttpClient`, no `get_github_copilot()` accessor.
2. **DRY duplication** between `copilot/auth.rs` and `claude_auth.rs`.
3. **Coverage line ranges over-claim** — they point at `manager.rs` enum/dispatch lines for scenarios that are actually exercised by `endpoint.rs`, `header_facade.rs`, `classifier.rs`, `behavior_facade.rs`, and `provider.rs::base_url_for`.

Tests pass because they only exercise the ceremonial wiring they actually claim to cover, not the full integration the architecture doc string promises.

> **Note on completeness:** This session experienced compaction-induced truncation of the assistant's earlier full report (turns 145/147 were truncated by the session viewer at ~5000 chars). The Critical Issues section below is the complete, verified set of findings I produced before the truncation. The Warnings and Observations sections are reconstructed from the analysis I performed against the codebase but may be less exhaustive than originally drafted. The supervisor should treat the Critical Issues as authoritative and the Warnings/Observations as a non-exhaustive starting list.

---

## 🔴 Critical Issues (Must Fix)

### 1. Rule [9] violated — `CopilotProvider` does NOT implement `LlmProvider`

`codelet/providers/src/copilot/provider.rs:94` declares `pub struct CopilotProvider;` but only adds three associated functions:

- `base_url_for`
- `system_prompt_facade_for_endpoint`
- `list_models`

There is **no `impl LlmProvider for CopilotProvider`**.

The example mapping rule is explicit:

> *"CopilotProvider implements LlmProvider (codelet/providers/src/lib.rs:84) and delegates all provider-specific behavior to facades following the same pattern as ClaudeProvider, CodexProvider, GeminiProvider, ZaiProvider, OpenAIProvider"*

Every other provider has this. `grep "impl LlmProvider for" codelet/providers/src/` returns matches for Claude, Codex, Gemini, OpenAI, and ZAI. **Copilot is the only one missing.** The dispatch surface is therefore incomplete: `ProviderManager` has no `complete()`, no `complete_with_tools()`, no `stream_*()` for the Copilot path.

This is the **headline acceptance criterion for the parent story** and it is not satisfied.

### 2. Architecture note [1] violated — promised module `refreshing_client.rs` does not exist

The architecture doc string at `spec/features/add-github-copilot-provider.feature:10` explicitly lists `refreshing_client.rs (CopilotHttpClient middleware)` as part of the module layout, and `provider.rs:92` even has a doc-link to `[CopilotHttpClient](super::refreshing_client)` — but `codelet/providers/src/copilot/refreshing_client.rs` **does not exist**. `Glob copilot/refreshing_client*` returns nothing.

This is also rule [16] in the example map:

> *"CopilotHttpClient implements rig::http_client::HttpClientExt as a middleware layer that intercepts outbound requests, attaches Bearer token + Copilot headers, and refreshes the token on 401 responses."*

There is no `HttpClientExt` impl anywhere in `codelet/providers/src/copilot/`. Without the middleware, the header facade and classifier are pure data and never actually applied to live traffic. **The integration is incomplete.**

### 3. Architecture note [1] violated — promised `mod.rs (CopilotProvider impl LlmProvider)` is wrong on two counts

(a) `CopilotProvider` lives in `provider.rs`, not `mod.rs`.
(b) It does not `impl LlmProvider` (see Critical Issue 1).

Either the spec must be updated to reflect a deferred trait impl, or the impl must land. Right now the feature file claims a thing the code does not deliver.

### 4. `ProviderManager::get_github_copilot()` accessor is missing

`codelet/providers/src/manager.rs` has an accessor for every other provider type:

| Method | Line |
|--------|------|
| `get_claude` | l396 |
| `get_openai` | l429 |
| `get_codex` | l455 |
| `get_gemini` | l474 |
| `get_zai` | l605 |

There is **no `get_github_copilot`**. The integration test at `tests/copilot_provider_manager_integration_test.rs:87-89` only verifies that `with_provider("github-copilot")` returns Ok and that `current_provider_name()` round-trips — it never tries to instantiate an actual `CopilotProvider` from the manager.

### 5. Test header is misleading

`tests/copilot_provider_manager_integration_test.rs:8-15` claims:

> *"These tests verify that the parent scenarios are wired through the provider dispatch layer (`ProviderType::GitHubCopilot`, `ProviderCredentials::detect`, `ProviderManager::with_provider`, **`ProviderManager::get_github_copilot`**)."*

The asserted `get_github_copilot` method does not exist. **The comment lies about coverage.** Either the method must be added and called, or the comment must be removed.

### 6. Coverage line ranges are inaccurate / over-claimed

The `.feature.coverage` file maps the gpt-4o, gpt-5, gpt-5-mini, vision, and several other scenarios to `manager.rs` lines 19-72 + 338-355 — i.e. the `ProviderType` enum, `FromStr`, `as_str`, `has_credentials`, and `map_provider_id_to_type`.

**None of those manager lines have anything to do with header building, classifier logic, endpoint routing, or vision detection.** Those scenarios are actually exercised by:

- `endpoint.rs` — endpoint routing
- `header_facade.rs` — header construction
- `classifier.rs` — model capability classification
- `behavior_facade.rs` — provider behavior
- `provider.rs::base_url_for` — base URL selection

The coverage links should point at those files. As written, the coverage report is technically green at 100% but the impl mappings do not match the behavior under test. This is exactly the **"retroactive state walking"** the spec workflow is designed to prevent — the links were filed in to satisfy `audit-coverage`, not to document where the behavior actually lives.

---

## 🟡 Warnings (Should Fix)

### W1. DRY violation — `copilot/auth.rs` duplicates `claude_auth.rs` token-storage patterns

`copilot/auth.rs` re-implements token persistence, expiry checking, and refresh ceremony that already exist in `claude_auth.rs` in near-identical form. The two files share:

- `Token { access_token, refresh_token, expires_at }` shape
- "Is expired with 60s skew" check
- File-locked read/write to `~/.codelet/credentials.json`
- Serde round-trip

These should be lifted to a shared `oauth_token_store` module under `codelet/providers/src/credentials/` and parameterised by provider key. As-is, any future bug fix in token storage has to be applied in two places.

### W2. `CopilotProvider` is a unit struct with only associated functions

`provider.rs:94` `pub struct CopilotProvider;` followed by `impl CopilotProvider { fn base_url_for(...) ... }` is a code smell. Either:

- It should hold state (an `HttpClient`, a credentials handle) and become a real provider type, OR
- The associated functions should be free functions in `provider.rs` and the empty struct removed.

The current shape is "namespace via empty struct" which is not idiomatic Rust and signals incomplete design.

### W3. Doc-link in `provider.rs:92` points at non-existent `super::refreshing_client`

`/// See [`CopilotHttpClient`](super::refreshing_client)` will produce a broken intra-doc link on `cargo doc`. Either remove the link or land the module.

### W4. Architecture doc string overstates the surface area

`spec/features/add-github-copilot-provider.feature:7-25` describes a multi-module facade architecture (`mod.rs`, `auth.rs`, `provider.rs`, `header_facade.rs`, `behavior_facade.rs`, `classifier.rs`, `endpoint.rs`, `refreshing_client.rs`) and claims `CopilotProvider impl LlmProvider`. The actual delivered surface is missing `refreshing_client.rs` and the trait impl. The spec should be brought into agreement with reality before the work unit closes — either by adding the missing pieces or by trimming the spec and noting the deferred work as a follow-up child.

### W5. Test file claims coverage of `ProviderManager::get_github_copilot` it doesn't actually exercise

`tests/copilot_provider_manager_integration_test.rs` (header, line 10) — see Critical Issue 5. Even setting aside the missing method, no test in this file calls anything that returns a `CopilotProvider` value. The "parent integration" framing is aspirational, not actual.

### W6. `provider.rs::list_models` returns a hard-coded `Vec<&'static str>` rather than consulting the model catalog facade

The model catalog work (PROV-055) lives in a separate file but `list_models` here doesn't delegate to it — it hard-codes the list. This means future model additions have to be made in two places, and the catalog facade isn't actually wired in. Verify the ownership boundary; if PROV-055 owns the catalog, this list must call into it.

---

## 🟢 Observations (Nice to Have)

### O1. Every Gherkin scenario has a matching `@step` comment

Verified by reading `tests/copilot_provider_manager_integration_test.rs` against `spec/features/add-github-copilot-provider.feature`. The text matches exactly, the ordering is correct, and `link-coverage` has been called for each scenario. This part of ACDD discipline is good.

### O2. Feature file uses doc strings for architecture notes

`add-github-copilot-provider.feature:8-25` uses a `"""` docstring block to describe the architecture, which is the right pattern. The content is wrong (see Critical Issues 2/3) but the structure is correct.

### O3. Example map is well-formed

The example map for PROV-053 has a clear user story, 16+ rules, examples mapped to each rule, and no unanswered questions. The `generate-scenarios` output flowed cleanly into the feature file. This is what good Example Mapping looks like.

### O4. `provider.rs` is small and single-concept

`provider.rs` is ~120 lines and owns one concept (the `CopilotProvider` shell + base URL routing). It's well within the 300-line guideline.

### O5. Test isolation uses real environment and `with_provider` round-trips

The integration tests don't mock the provider manager — they exercise it through `with_provider("github-copilot")` and verify state via `current_provider_name()`. This matches the "integration first, mocks last" guidance.

### O6. Headers and classifier are pure data, easily testable

`header_facade.rs` and `classifier.rs` are pure functions over input data with no I/O. This is good factoring. The downside is that they are *only* tested at the unit level — they're never exercised through a real HTTP request because the middleware doesn't exist (Critical Issue 2).

---

## Coverage Verification

- **Feature file**: `spec/features/add-github-copilot-provider.feature` — **OK** (parses, has `@PROV-053` tag, doc-string architecture, all scenarios have steps).
- **Coverage file**: `spec/features/add-github-copilot-provider.feature.coverage` — **ISSUE** (impl line ranges point at `manager.rs:19-72,338-355` for scenarios that exercise `endpoint.rs` / `header_facade.rs` / `classifier.rs` / `behavior_facade.rs` — see Critical Issue 6).
- **Test file**: `codelet/providers/tests/copilot_provider_manager_integration_test.rs` — **ISSUE** (header comment claims `ProviderManager::get_github_copilot` coverage; that method does not exist — see Critical Issue 5).
- **Implementation files**: `codelet/providers/src/copilot/provider.rs` — **ISSUE** (no `LlmProvider` impl, doc-link to non-existent module — see Critical Issues 1, 3 and Warning W3).
- **Missing implementation**: `codelet/providers/src/copilot/refreshing_client.rs` — **MISSING** (promised by architecture doc string and rule [16] but does not exist — see Critical Issue 2).
- **Example map → scenarios → tests → impl chain**: **PARTIAL** — chain is traceable but the leaf nodes (impl line ranges) are inaccurate, and the headline rule [9] (LlmProvider impl) is unsatisfied.
- **All 11 integration tests pass**: ✅ confirmed.
- **`fspec validate` on the feature file**: ✅ passes Gherkin syntax.

---

## Files Reviewed

### Specification
- `spec/features/add-github-copilot-provider.feature`
- `spec/features/add-github-copilot-provider.feature.coverage`
- `spec/work-units/PROV-053.json` (via `Fspec show-work-unit`)
- `spec/attachments/PROV-053/ast-research-parent-integration.md`

### Implementation
- `codelet/providers/src/copilot/mod.rs`
- `codelet/providers/src/copilot/provider.rs`
- `codelet/providers/src/copilot/auth.rs`
- `codelet/providers/src/copilot/header_facade.rs`
- `codelet/providers/src/copilot/behavior_facade.rs`
- `codelet/providers/src/copilot/classifier.rs`
- `codelet/providers/src/copilot/endpoint.rs`
- `codelet/providers/src/manager.rs` (lines 19-72, 338-355, 396, 429, 455, 474, 605)
- `codelet/providers/src/lib.rs`
- `codelet/providers/src/claude_auth.rs` (for DRY comparison)

### Tests
- `codelet/providers/tests/copilot_provider_manager_integration_test.rs`

### Cross-checks performed
- `Grep "impl LlmProvider for"` across `codelet/providers/src/` → returned Claude, Codex, Gemini, OpenAI, ZAI; not Copilot.
- `Glob "**/copilot/refreshing_client*"` → no matches.
- `Grep "get_github_copilot"` across `codelet/providers/` → 1 match (test file header comment), 0 matches in source.
- `Grep "CopilotHttpClient"` → 1 match (broken doc-link in `provider.rs:92`), 0 impls.

---

## Recommended Next Steps (for the supervisor / fix agent)

1. **Decide spec vs. code**: Either land `refreshing_client.rs` + `impl LlmProvider for CopilotProvider` + `get_github_copilot` accessor, OR amend the feature file architecture doc string and rules [9]/[16] to reflect a deferred follow-up child.
2. **Fix coverage links**: Re-run `link-coverage` with correct `implFile` / `implLines` pointing at `endpoint.rs`, `header_facade.rs`, `classifier.rs`, `behavior_facade.rs`, `provider.rs::base_url_for` for the relevant scenarios.
3. **Fix the lying test header**: Either implement `get_github_copilot` and call it from a new test, or remove the claim from the file header comment.
4. **Extract shared OAuth token store**: Lift `Token`/`is_expired`/file-locked persistence into `credentials/oauth_token_store.rs`, consume from both `claude_auth.rs` and `copilot/auth.rs`.
5. **Resolve the empty-struct smell**: Either add state to `CopilotProvider` or convert its associated functions to free functions.
