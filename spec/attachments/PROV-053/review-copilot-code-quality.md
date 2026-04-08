# Code Quality Review: codelet/providers/src/copilot/

**Scope:** 15 source files in `codelet/providers/src/copilot/` (plus `models/` subdirectory) and 4 integration test files in `codelet/providers/tests/`.

**Reviewer:** Rust code-quality sub-agent (read-only)
**Date:** 2026-04-07

---

## Summary

| Metric | Value |
|---|---|
| Total files reviewed | 15 src + 4 tests |
| Total lines (src + tests) | 4,853 |
| Files over 300-line limit | **3** (`provider.rs` 468, `oauth.rs` 364, `refreshing_client.rs` 331) |
| `allow(dead_code)` markers | 1 (`provider.rs:134`) |
| `unwrap()` / `expect()` in non-test code | 0 ✅ |
| `todo!()` / `unimplemented!()` / `panic!()` in non-test code | 0 ✅ |
| `TODO` / `FIXME` / `HACK` / `XXX` comments | 0 ✅ |
| Critical issues | 15 |
| Warnings | 26 |
| Observations | 8 |

---

## 🔴 Critical Issues (Must Fix)

### File Size Violations (>300 lines)

**1. `provider.rs:1-468` (468 lines) — 🔴 OVER LIMIT by 168 lines**

Single file mixes at least 5 distinct responsibilities:
- (a) `CopilotBaseUrl` newtype
- (b) `CopilotChatCompletionsSystemPromptFacade` type alias
- (c) `CopilotResponsesSystemPromptFacade` struct + trait impl
- (d) `CopilotProvider` struct + state
- (e) `base_url_for` URL computation
- (f) `system_prompt_facade_for_endpoint` selector
- (g) `list_models` async fetch wrapper
- (h) `new()` constructor (45 lines)
- (i) free function `rig_response_to_completion`
- (j) `LlmProvider` trait impl
- (k) extensive `#[cfg(test)]` block

**Fix:** Split per "Proposed Refactoring" section below.

**2. `oauth.rs:1-364` (364 lines) — 🔴 OVER LIMIT by 64 lines**

Bundles RFC 8628 device-code flow, polling state machine, normalize-domain helper, multiple config structs, response DTOs, and the orchestrator. The 110-line `poll_device_token` function alone is a maintenance hazard.

**Fix:** Split per "Proposed Refactoring" section below.

**3. `refreshing_client.rs:1-331` (331 lines) — 🔴 OVER LIMIT by 31 lines**

Borderline; the structure is clean but the 130 lines of in-file tests + three near-duplicate `send` / `send_multipart` / `send_streaming` impls push it over. The misleading `refreshing_client.rs` name (the comment in lines 8–11 explicitly says it does NOT refresh) compounds the smell.

**Fix:** Rename file and split per "Proposed Refactoring" section below.

### Dead Code / Suppressed Warnings

**4. `provider.rs:134` — `#[allow(dead_code)]` on `rig_client` field**

A struct field carrying an expensive dependency (`openai::CompletionsClient<CopilotHttpClient>`) is held only because something *might* use it later. The `completion_model` already wraps it (line 254-255: `CompletionModel::new(rig_client.clone(), …)`). Holding it twice doubles storage and hides the fact that it's never read after construction. **Grep confirms no accessor method exists.**

**Fix:** Either drop the field and stop cloning, or document the *real* reason it must be kept and remove the `allow`. If genuinely dead, delete it.

### Misleading Naming

**5. `refreshing_client.rs:1-23` — File and type both named `refreshing_client` / `CopilotHttpClient` is misleading**

The doc comment explicitly states (line 8): *"this client does not need a refresh loop, a write-lock state machine, or a token endpoint."* The name is a lie. Anyone grepping for "refreshing" will find this file and assume it does what `RefreshingClaudeClient` / `RefreshingCodexClient` do.

**Fix:** Rename file to `http_client.rs` and module path to `crate::copilot::http_client`.

### DRY Violations

**6. `header_facade.rs:96` vs `constants.rs:25-27` — User-Agent built in two places with two different sources of truth**

`header_facade.rs` builds the User-Agent inline:
```rust
if let Ok(ua) = HeaderValue::from_str(&format!("codelet/{}", env!("CARGO_PKG_VERSION"))) {
```
while `constants.rs::copilot_user_agent()` already exists *for exactly this purpose* and is unused by production. The constants module's doc-comment (lines 21–24) says: *"Compile-time helper: the full default User-Agent string … so test code can assert exact equality"*. Production code ignores it; tests use `starts_with("codelet/")`. Two sources of truth → guaranteed drift.

**Fix:** `header_facade.rs:96` must call `crate::copilot::constants::copilot_user_agent()`.

**7. `header_facade.rs:101`, `models/fetch.rs:57` — Authorization Bearer header built in 2+ places with inconsistent header capitalization**

- `header_facade.rs:101` uses the typed `AUTHORIZATION` constant (lowercase via http crate)
- `models/fetch.rs:57` uses the literal string `"Authorization"` (mixed case)

Both are DRY violations *and* a correctness smell — a future case-sensitive http stack will route these differently.

**Fix:** Add a `bearer_header_value(token: &str) -> HeaderValue` helper in `constants.rs` (or new `headers.rs`), and `models/fetch.rs` should use the typed `AUTHORIZATION` constant.

**8. `models/fetch.rs:44, 60, 66, 72` — `ProviderError::Api` repeated 4× with identical structure**

Four near-identical `.map_err(|e| ProviderError::Api { provider: COPILOT_PROVIDER_ID.to_string(), message: format!("…: {e}") })` blocks. ~20 lines of boilerplate in a 78-line file.

**Fix:** Add a `fn copilot_api_err(msg: impl Into<String>) -> ProviderError` helper, or `.map_err(api_err("/models request failed"))` combinator.

**9. `provider.rs:228, 234, 249, 298, 326, 375` — `"github-copilot"` literal appears 6 times despite a constant existing**

`crate::copilot::constants::COPILOT_PROVIDER_ID` is *exactly* this string. The constants file documents (line 7): *"so that test, catalog, middleware, and provider code all reference exactly one source of truth"*. `provider.rs` ignores its own constant.

**Fix:** Replace every literal with `COPILOT_PROVIDER_ID`.

### Hardcoded URLs / Magic Strings

**10. `provider.rs:161` — `"https://api.githubcopilot.com"` is a magic string in the middle of `base_url_for`**

Should be `pub const COPILOT_GITHUBCOM_API_BASE: &str = "https://api.githubcopilot.com"` in `constants.rs`. Same for the enterprise template at line 164 — `"copilot-api"` subdomain prefix is policy, not a string.

**11. `oauth.rs:159, 201` — endpoint paths `/login/device/code`, `/login/oauth/access_token` are magic strings**

They appear nowhere else and should be `const COPILOT_DEVICE_CODE_PATH` / `const COPILOT_DEVICE_TOKEN_PATH` so integration test fixtures can reference the same constants instead of hardcoding.

### Dangerous Defaults

**12. `auth.rs:87` — Silently defaulting HOME to `/tmp`**

```rust
let home = std::env::var("HOME").unwrap_or_else(|_| String::from("/tmp"));
```

**DANGEROUS** for a credential file — `/tmp` is world-writable on shared systems and gets cleaned. The mode 0600 enforcement is meaningless if the file lives in a tmpfs anyone can `rm`. This should be a hard error: *"HOME is not set; cannot determine credential directory"*.

**13. `auth.rs:136-149` — TOCTOU smell in `write_copilot_auth`**

```rust
tokio::fs::write(&path, content).await?;  // line 144 — file created with umask default
enforce_mode_0600(&path).await?;            // line 147 — chmod afterwards
```

Between `fs::write` and `enforce_mode_0600` the file briefly exists with the umask-default permissions. On a multi-user system another user could `cat` it during that window.

**Fix:** Use `OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&path)` to set the mode at creation time.

### Test Fixture Duplication

**14. `tests/copilot_models_catalog_test.rs` — 9 nearly-identical catalog JSON blocks**

The same `capabilities/limits/supports/streaming/tool_calls/vision` JSON structure repeated 9+ times (lines 77, 150, 213, 270, 318, 361, 438, 499, 564). Each is 15-25 lines of identical scaffolding. The `make_entry` helper inside `models/builder.rs:129-156` does this internally — it should be moved to shared test fixtures.

**Fix:** Extract `make_copilot_model_json(id, version, picker_enabled, reasoning_effort) -> serde_json::Value` into `tests/fixtures/mod.rs`.

**15. `auth.rs:192-216` duplicates `FspecHomeGuard` from `tests/fixtures/mod.rs:73-97`**

Two test guards, identical shape, identical drop logic, identical env-var contract. The src/ copy is strictly duplication.

**Fix:** Move the in-source-tree guard to a shared `crate::test_helpers` module behind `#[cfg(test)]`, or have the unit tests use `tests/fixtures::setup_fspec_home`.


---

## 🟡 Warnings

### Magic Numbers in Time Conversions

**16. `oauth.rs:207` — `device_code.interval * 1000`**

The literal `1000` is a magic number for "seconds → ms". Should be `const MS_PER_SECOND: u64 = 1000;` or better, use `Duration::from_secs(device_code.interval).as_millis() as u64`.

**17. `oauth.rs:256` — `server_interval_secs * 1000`**

Same magic number, second occurrence in the same function.

### Diagnostic Loss

**18. `oauth.rs:175` — `let body = response.text().await.unwrap_or_default();`**

Silently turning a network read failure into an empty string when reporting an HTTP error produces `"Device code request failed with status 500: "` (empty body).

**Fix:** `.unwrap_or_else(|e| format!("<failed to read body: {e}>"))`.

### Long Functions

**19. `oauth.rs:197-306` — `poll_device_token` is 110 lines**

Far above the 50-line target. Nested matches inside an async loop inside a timeout. Should be decomposed:
- `compute_polling_intervals(config, device_code) -> PollingTimings`
- `handle_pending(current_interval_ms, safety_margin) -> Duration`
- `handle_slow_down(response, current, increment, override_set) -> u64`
- `parse_poll_response(json) -> PollOutcome`

**20. `provider.rs:221-265` — `CopilotProvider::new` is 45 lines, mixes 4 concerns**

Validation (226-237), http client construction (240), rig client builder (242-252), completion model wrap (254-255), struct assembly. Should be split into `validate_inputs` + `build_rig_client` + `assemble`.

### Open/Closed Violations

**21. `behavior_facade.rs:148-162` — `select_copilot_behavior_facade` uses `if/else if/else if/else`**

With 3 prefixes today (gpt/claude/gemini) and an unknown-fallback, a future Mistral/Cohere addition will require *editing this function*. Better:
```rust
const FAMILY_DISPATCH: &[(&str, fn() -> BoxedCopilotBehaviorFacade)] = &[
    ("gpt-",    || Box::new(CopilotGptBehaviorFacade)),
    ("claude-", || Box::new(CopilotClaudeBehaviorFacade)),
    ("gemini-", || Box::new(CopilotGeminiBehaviorFacade)),
];
```
New families add a row, no editing of dispatch logic.

**22. `behavior_facade.rs:62-125` — Three facade impls + selector in one file (230 lines)**

Edge of acceptable. The moment a fourth family arrives (or `mutate_chat_params` starts doing work for GPT-5) the file will jump past 300 lines. Pre-emptively split.

### Dead/Phantom API Surface

**23. `behavior_facade.rs:33` — `fn mutate_chat_params(&self, _params: &mut Value) {}`**

Default trait method that all 3 implementations leave as the default. Grep confirms **zero callers anywhere**. Either *no* implementation needs it (delete it) or *some future* implementation will need it (add a test).

**24. `constants.rs:13-16, 19` — `COPILOT_USER_AGENT_PREFIX` and `COPILOT_OPENAI_INTENT_VALUE` are unused**

Defined `pub const` but `header_facade.rs` does NOT import them; it inlines `"codelet/"` and `HeaderValue::from_static("conversation-edits")`. The constants are unused by the production code they were created to centralize.

**Fix:** Either delete them or wire them up at `header_facade.rs:35, 96`.

### Test Code Concerns Leaking Into Production

**25. `oauth.rs:248-258` — Branch for "if test override is set, scale proportionally"**

```rust
let server_interval_ms = if config.poll_interval_override_ms.is_some() {
    current_interval_ms
} else {
    server_interval_secs * 1000
};
```

Production behaviour depends on whether a test injected an override. Minor SoC violation.

**Fix:** Take a `IntervalScaling::Production | IntervalScaling::Test { factor }` enum.

### Lint Suppression Style

**26. 8× `#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]` in test modules**

All in-file tests use the same allow. Should be hoisted to workspace-level `[lints]` in `Cargo.toml` or `clippy.toml`.

### mod.rs Organization

**27. `mod.rs:53, 61` — `CONTEXT_WINDOW` / `MAX_OUTPUT_TOKENS` constants in mod.rs**

These belong in `constants.rs`, not in `mod.rs`. Magic numbers with extensive justifying comments — at minimum they should sit next to `COPILOT_PROVIDER_ID` so all dispatch-layer fallbacks live together.

**28. `mod.rs:63-85` — 20+ re-exports, hard to scan**

Adding a new symbol means picking the right `pub use`. Could be replaced with `pub use auth::*; pub use behavior_facade::*; …` since sub-modules are all internal.

### Silent Header Drops

**29. `header_facade.rs:96-103` — `if let Ok(ua) = HeaderValue::from_str(...)`**

If `CARGO_PKG_VERSION` contained an invalid header byte, the User-Agent header would be silently dropped. Same for the Bearer token — a token containing whitespace would silently produce a request without an Authorization header.

**Fix:** For a security-critical header, use `expect("Bearer token must be header-safe")` or propagate via `HeaderValue::try_from(...).map_err(|e| ProviderError::config(...))?`.

### Unusable Default Impl

**30. `refreshing_client.rs:67-77` — `Default` impl creates a client with empty access token**

The doc says: *"attempting to use it for a real request will fail at the API layer with an authentication error"*. Deferred panic — the type system invites construction of an invalid value.

**Fix:** Either remove the `Default` impl (force callers to provide a token) or return `Result<Self, ConfigError>`.

### Redundant Defensive Code

**31. `models/fetch.rs:51` — `trim_end_matches('/')` of base URL**

Indicates the base URL contract is unclear. `CopilotBaseUrl` should be guaranteed-no-trailing-slash by construction (in `provider.rs::base_url_for`), making this trim redundant. SoC: URL-shape policy belongs to the producer, not the consumer.

### Type Conversions

**32. `provider.rs:367` — `.max_tokens(crate::copilot::MAX_OUTPUT_TOKENS as u64)`**

`MAX_OUTPUT_TOKENS` is `usize`. The `as u64` never actually needs to truncate but is not explicit.

**Fix:** Use `u64::try_from(...).unwrap_or(u64::MAX)` for explicit safety, or change `MAX_OUTPUT_TOKENS` to `u64`.

### Allocation Smells

**33. `models/builder.rs:117` — `let prefix = format!("{id}-")`**

Allocates a String just to call `strip_prefix`. Could be `version.strip_prefix(id).and_then(|r| r.strip_prefix('-'))` — zero allocations.

### Delegation Stubs

**34. `provider.rs:96-114` — `CopilotResponsesSystemPromptFacade` is a delegate-only stub**

Every method delegates to `OpenAISystemPromptFacade.<same_method>`. The only difference is `provider()` returns `"copilot-responses"`. Three of four methods construct a new `OpenAISystemPromptFacade` per call (lines 104, 108, 112).

**Fix:** Use `Deref<Target = OpenAISystemPromptFacade>`, or better, take a generic `WithProviderName<F: SystemPromptFacade>` wrapper to avoid the stub-class proliferation when the next endpoint family appears.

### Test Helper Duplication

**35. `tests/copilot_http_middleware_routing_test.rs:41-43, 168-169, 228-229, 277-278, 323-324` — `build_headers` + `classify_body` called identically 5 times**

```rust
let classification = CopilotRequestClassifier::classify(&body);
let headers = CopilotHeaderFacade::build_headers(&classification, access_token);
```

**Fix:** Extract `fn build_request_headers(body: &Value, token: &str) -> HeaderMap` helper.

### External Coupling via mod.rs Constants

**36. `manager.rs:619` references `crate::copilot::CONTEXT_WINDOW` directly**

Fine in itself, but illustrates that those constants are part of the *public* API contract — yet they live in `mod.rs`, not `constants.rs`. Coupling smell.

### Test Assertion Brittleness

**37. `copilot_oauth_device_flow_test.rs:107` — brittle substring check**

```rust
assert!(msgs[0].contains("ABCD-1234"), "Should display user_code")
```

`contains` could pass for many wrong shapes. Should split on `|` and assert each field.

### Tests Use `panic!` Where `assert!(matches!)` is Cleaner

**38. `copilot_oauth_device_flow_test.rs:288, 369` — `panic!("Expected CopilotPollResult::Success, got: {other:?}")`**

OK because tests, but cleaner as `assert!(matches!(...))` or `unreachable!()`.

### Fixture `.unwrap()` Without Context

**39. `models/builder.rs:155` — `serde_json::from_value(value).unwrap()`**

In a test helper, fine, but `.expect("test fixture must deserialize")` gives better failure output.

### Inline Test Data Bloat

**40. `tests/copilot_models_catalog_test.rs` — 636 lines of which ~500 are inline JSON**

9 `mount_models_response(&server, json!({...}))` blocks, 15-25 lines each. Crosses 600 lines just for catalog scenarios.

**Fix:** Extract `make_models_response_with_one(model)` and `make_models_response_with_many(models)` builders into `tests/fixtures/mod.rs`.

### Inconsistent Error Handling Across Tests

**41. Tests mix `expect()` with failure messages and bare `unwrap()`** across the 4 integration test files. Should be consistent — prefer `.expect("meaningful reason")` everywhere.

---

## 🟢 Observations (Positive Findings)

**42.** ✅ No `panic!`, `todo!`, or `unimplemented!` in production code.

**43.** ✅ No `TODO`, `FIXME`, `HACK`, or `XXX` comments anywhere.

**44.** ✅ All public APIs have `///` doc comments. Docs are high-quality and include cross-references and rule numbers.

**45.** ✅ `classifier.rs` is **exemplary** — 175 lines, single responsibility, pure functions, zero IO, comprehensive in-file unit tests, clean Gherkin-shape-handling.

**46.** ✅ `endpoint.rs` is **exemplary** — 151 lines, pure dispatch, well-tested, includes doc-tests.

**47.** ✅ `provider_options.rs` is **exemplary** — 52 lines, single function with a deliberately constrained signature enforcing the rule *at compile time*. The doc comment explaining *why* the signature cannot accept a model id is a gem.

**48.** ✅ `models/fetch.rs` (78 lines), `models/schema.rs` (77 lines), `models/builder.rs` (240 lines) — well factored, single-concern per file, clear separation between wire-format DTOs / HTTP IO / pure mapping.

**49.** ✅ `constants.rs` has the *intent* right (centralize magic strings) — just needs to be actually used by its would-be consumers.

---

## Proposed Refactoring

For each over-300-line file, here is a concrete split:

### 1. `provider.rs` (468 lines → split into 5 files)

| New file | Lines | Content |
|---|---|---|
| **`provider.rs`** | ~180 | `CopilotProvider` struct, `new()` (split into helpers), `Debug` impl, `LlmProvider` trait impl only |
| **`base_url.rs`** | ~40 | `CopilotBaseUrl` newtype + `base_url_for()` function + tests |
| **`system_prompt.rs`** | ~90 | `CopilotChatCompletionsSystemPromptFacade` alias, `CopilotResponsesSystemPromptFacade` struct + `SystemPromptFacade` impl, `system_prompt_facade_for_endpoint()` selector + tests |
| **`response_adapter.rs`** | ~60 | `rig_response_to_completion()` free function + `StopReason` mapping + tests |
| **`list_models.rs`** | ~30 | `CopilotProvider::list_models` as a standalone helper function (or moved onto the existing `models/fetch.rs::fetch_models` facade) |

Reasoning: each new file is <200 lines, each has one SoC-clear purpose, and the `CopilotResponsesSystemPromptFacade` is no longer hidden inside a file named "provider".

### 2. `oauth.rs` (364 lines → split into 4 files)

| New file | Lines | Content |
|---|---|---|
| **`oauth/mod.rs`** | ~30 | Re-exports and top-level `copilot_device_auth_login()` orchestrator only (moved to its own file below, this mod.rs just re-exports) |
| **`oauth/types.rs`** | ~110 | `CopilotDeploymentType`, `CopilotDeviceCodeResponse`, `CopilotPollResult`, `CopilotDisplayCallback`, `CopilotPollConfig`, `CopilotDeviceAuthConfig`, `DevicePollResponse`, constants |
| **`oauth/device_code.rs`** | ~70 | `normalize_enterprise_domain()`, `request_device_code()` + tests |
| **`oauth/polling.rs`** | ~120 | `poll_device_token()` (broken into `compute_polling_intervals`, `handle_pending`, `handle_slow_down`, `parse_poll_response` helpers) + tests |
| **`oauth/login.rs`** | ~60 | `copilot_device_auth_login()` orchestrator + tests |

Reasoning: separates types from operations, makes the 110-line polling monster manageable, each file is <130 lines.

### 3. `refreshing_client.rs` (331 lines → rename + split into 3 files)

First, **rename** `refreshing_client.rs` → `http_client.rs` (the "refreshing" name is a lie).

| New file | Lines | Content |
|---|---|---|
| **`http_client.rs`** | ~120 | `CopilotHttpClient` struct, `new()`, `Default` (or removed), accessors, `HttpClientExt` trait impl dispatching to helpers |
| **`http_client/body_classify.rs`** | ~50 | `classify_body()` private helper + tests |
| **`http_client/header_inject.rs`** | ~60 | `inject_copilot_headers()` private helper + tests |
| **`http_client/tests.rs`** | ~130 | Move all unit tests out of the main file into a cfg(test) sibling module |

Or, more modestly: keep a single file but lift the 130 lines of tests into `tests/copilot_http_client_test.rs` as an integration test file. That alone drops the file to ~200 lines.

### 4. `behavior_facade.rs` (230 lines — pre-emptive split for O/C compliance)

While technically under 300 lines, a fourth family will push this over. Pre-emptive split:

| New file | Lines | Content |
|---|---|---|
| **`behavior_facade/mod.rs`** | ~60 | `CopilotBehaviorFacade` trait, `BoxedCopilotBehaviorFacade` alias, `select_copilot_behavior_facade()` (using dispatch-table pattern from Warning #21) |
| **`behavior_facade/gpt.rs`** | ~70 | `CopilotGptBehaviorFacade` struct + impl + reasoning_opaque round-trip tests |
| **`behavior_facade/claude.rs`** | ~35 | `CopilotClaudeBehaviorFacade` struct + impl + tests |
| **`behavior_facade/gemini.rs`** | ~35 | `CopilotGeminiBehaviorFacade` struct + impl + tests |
| **`behavior_facade/selector_tests.rs`** | ~30 | Tests for the prefix dispatcher |

Reasoning: adding a Mistral facade becomes "create `mistral.rs`, add a row to the dispatch table" — zero editing of existing files. **O/C compliant.**

### 5. New `constants.rs` additions

Expand `constants.rs` to absorb every scattered magic string flagged in this review:

```rust
// Provider identity
pub const COPILOT_PROVIDER_ID: &str = "github-copilot";
pub const COPILOT_NPM_KEY: &str = "@ai-sdk/github-copilot";

// URL bases
pub const COPILOT_GITHUBCOM_API_BASE: &str = "https://api.githubcopilot.com";
pub const COPILOT_ENTERPRISE_API_SUBDOMAIN: &str = "copilot-api";

// Endpoint paths
pub const COPILOT_MODELS_PATH: &str = "/models";
pub const COPILOT_CHAT_COMPLETIONS_PATH: &str = "/chat/completions";
pub const COPILOT_RESPONSES_PATH: &str = "/responses";
pub const COPILOT_DEVICE_CODE_PATH: &str = "/login/device/code";
pub const COPILOT_DEVICE_TOKEN_PATH: &str = "/login/oauth/access_token";

// Header values
pub const COPILOT_USER_AGENT_PREFIX: &str = "codelet/";
pub const COPILOT_OPENAI_INTENT_VALUE: &str = "conversation-edits";

// Dispatch-layer fallbacks (moved from mod.rs)
pub const COPILOT_FALLBACK_CONTEXT_WINDOW: usize = 200_000;
pub const COPILOT_FALLBACK_MAX_OUTPUT_TOKENS: usize = 4_096;

// Timing
pub const COPILOT_AUTH_PENDING_SAFETY_MARGIN_MS: u64 = 3_000;
pub const COPILOT_SLOW_DOWN_INCREMENT_MS: u64 = 5_000;
pub const MS_PER_SECOND: u64 = 1_000;

// Helpers
#[must_use]
pub fn copilot_user_agent() -> String { /* unchanged */ }

#[must_use]
pub fn bearer_header_value(token: &str) -> Result<HeaderValue, InvalidHeaderValue> {
    HeaderValue::from_str(&format!("Bearer {token}"))
}

#[must_use]
pub fn copilot_api_err(message: impl Into<String>) -> ProviderError {
    ProviderError::Api {
        provider: COPILOT_PROVIDER_ID.to_string(),
        message: message.into(),
    }
}
```

### 6. New `tests/fixtures/mod.rs` additions

```rust
/// Build a Copilot /models response JSON with one entry.
pub fn make_copilot_model_json(
    id: &str,
    version: &str,
    picker_enabled: bool,
    reasoning_effort: Option<Vec<&str>>,
) -> serde_json::Value { /* ... */ }

/// Build a Copilot /models response body wrapping N entries.
pub fn make_models_response(entries: Vec<serde_json::Value>) -> serde_json::Value { /* ... */ }

/// Install a fake copilot_auth.json into the active (temp) FSPEC_HOME.
pub async fn install_fake_copilot_credential(enterprise_url: Option<String>) { /* ... */ }

/// Helper combining classify + build_headers.
pub fn build_request_headers(body: &serde_json::Value, token: &str) -> http::HeaderMap { /* ... */ }
```

This single file would eliminate ~300 lines of duplication across the 4 integration test files.

---

## Priority Order for Fixes

If addressing incrementally, fix in this order for maximum SoC payoff with minimum rework:

1. **Issues 4, 5** — drop dead `rig_client` field, rename `refreshing_client.rs` → `http_client.rs` (mechanical)
2. **Issues 12, 13** — fix dangerous HOME fallback and TOCTOU in credential write (security)
3. **Issues 6, 7, 9, 10** — expand `constants.rs` and replace literals (prevents drift)
4. **Issue 1** — split `provider.rs` (biggest SoC payoff)
5. **Issue 2** — split `oauth.rs` and decompose `poll_device_token`
6. **Issue 3** — split or test-extract `refreshing_client.rs`
7. **Issues 14, 15, 35, 40** — consolidate test fixtures
8. **Issue 22, 23** — pre-emptive `behavior_facade.rs` split + delete dead `mutate_chat_params`
9. Remaining warnings in priority as encountered.

---

## Final Verdict

The copilot module is **well-structured at the micro level** — pure functions are pure, IO is isolated, tests are comprehensive, no panics or TODOs leak through. The dominant issues are:

1. **One oversized `provider.rs` that smuggles 5 responsibilities** (the most consequential violation)
2. **`oauth.rs` approaching structural debt** from a single 110-line polling function
3. **`constants.rs` exists but is bypassed by its own consumers** — a trap for future drift
4. **`refreshing_client.rs` is mis-named** in a way that will actively mislead future maintainers
5. **Test fixture duplication** that will compound as new scenarios are added

None of the findings are architectural dead-ends — every one has a straightforward, mechanical fix. The module is **close to excellent** but needs discipline about SoC at the file-organization level and DRY at the constants/fixtures level.
