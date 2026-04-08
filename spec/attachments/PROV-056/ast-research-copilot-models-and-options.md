# PROV-056 AST Research: Copilot Model Catalog, Provider Options & Reasoning Effort

**Date:** 2026-04-07
**Purpose:** Identify existing Rust patterns to mirror when implementing the Copilot model catalog service, the per-family reasoning-effort variants facade, and the Copilot small-model facade.

---

## 1. Existing Reasoning / Thinking Facade Pattern (templates to mirror)

### 1.1 `ThinkingConfigFacade` trait — `codelet/tools/src/facade/thinking_config.rs:76`

```rust
pub trait ThinkingConfigFacade {
    /// Returns the provider identifier (e.g., "gemini-3", "claude")
    fn provider(&self) -> &'static str;

    /// Generates the request configuration JSON for the specified thinking level
    fn request_config(&self, level: ThinkingLevel) -> Value;

    /// Checks if a response part contains thinking content
    fn is_thinking_part(&self, part: &Value) -> bool;

    /// Extracts thinking text from a response part (if it's a thinking part)
    fn extract_thinking_text(&self, part: &Value) -> Option<String>;
}
```

This is the canonical "single-level → single-config JSON" facade. Per slice 3 §2.3 it does **not** match the "menu of variants per model" shape we need for Copilot — Copilot returns `{ low: {...}, medium: {...}, high: {...}, xhigh?: {...} }` keyed by effort. We will introduce a new pure-function facade `CopilotProviderOptionsFacade::build_reasoning_variants(model)` rather than retrofitting `ThinkingConfigFacade`.

### 1.2 `CopilotBehaviorFacade` (PROV-055) — `codelet/providers/src/copilot/behavior_facade.rs`

Already in tree from PROV-055. Provides `family() -> &'static str` ("gpt" | "claude" | "gemini") and a `select_copilot_behavior_facade(model_id)` dispatcher. PROV-056 will compose this dispatcher (call it from `build_reasoning_variants`) so the per-family branches stay in one place.

```rust
// existing — PROV-055
pub fn select_copilot_behavior_facade(model_id: &str) -> BoxedCopilotBehaviorFacade {
    if model_id.starts_with("gpt-") { Box::new(CopilotGptBehaviorFacade) }
    else if model_id.starts_with("claude-") { Box::new(CopilotClaudeBehaviorFacade) }
    else if model_id.starts_with("gemini-") { Box::new(CopilotGeminiBehaviorFacade) }
    else { Box::new(CopilotGptBehaviorFacade) }
}
```

### 1.3 `CodexProvider::build_reasoning_params` — `codelet/providers/src/codex/mod.rs:255`

```rust
fn build_reasoning_params(thinking_config: Option<&serde_json::Value>) -> serde_json::Value {
    let mut params = serde_json::json!({
        "store": false,
        "include": ["reasoning.encrypted_content"],
    });
    // ... merge user effort or default to high
    params
}
```

This is the existing precedent for the **`store: false`** + **`include: ["reasoning.encrypted_content"]`** pair. We will reuse the exact same JSON shape inside `CopilotProviderOptionsFacade::build_reasoning_variants` for the GPT branch — except we will emit one entry per supported `effort` instead of a single aggregated JSON.

The Codex facade test pattern (`build_reasoning_params_with_high_effort_config`, `build_reasoning_params_defaults_to_high_when_none`, etc.) is a good template for the per-family unit tests we will add to `CopilotProviderOptionsFacade`.

---

## 2. Existing Model Type — `codelet/providers/src/models/types.rs:38`

```rust
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub family: Option<String>,
    pub release_date: Option<String>,
    pub attachment: bool,
    pub reasoning: bool,
    pub tool_call: bool,
    pub temperature: bool,
    pub interleaved: Option<InterleavedConfig>,
    pub modalities: Option<Modalities>,
    pub cost: Option<CostInfo>,
    pub limit: LimitInfo,
    pub status: Option<ModelStatus>,
    pub experimental: Option<bool>,
    pub options: HashMap<String, serde_json::Value>,
    pub headers: HashMap<String, String>,
}
```

**Gaps for PROV-056:**

1. No `variants` field — we cannot attach the per-effort variant map to a `ModelInfo` row.
   **Resolution:** keep the variants in a separate `HashMap<String, CopilotReasoningVariant>` returned by `CopilotProviderOptionsFacade::build_reasoning_variants(model: &ModelInfo) -> HashMap<String, CopilotReasoningVariant>`. No NAPI surface change needed for PROV-056 — this stays Rust-internal until a future TUI integration ticket.

2. No `api.npm` per-model field — fspec stores `npm` on `ProviderInfo`, not on the model. The opencode `transform.ts` switches on `model.api.npm`; we already dispatch on **family prefix** via `select_copilot_behavior_facade(model.id)` (PROV-055), which is the morally-equivalent axis. No model type change needed.

3. fspec already has `release_date: Option<String>` so the date-gated `xhigh` rule (`release_date >= "2025-12-04"`) maps cleanly with a lexical string compare on the existing field — no parser extension needed.

---

## 3. Catalog Fetch Pattern — `codelet/providers/src/models/cache.rs:96`

The existing `fetch_and_cache()` shows the canonical reqwest pattern fspec uses:

```rust
async fn fetch_and_cache(&self) -> Result<ModelsDevResponse, ProviderError> {
    let client = reqwest::Client::new();
    let response = client
        .get(MODELS_DEV_URL)
        .header("User-Agent", "codelet/0.1")
        // ...
}
```

`CopilotModelCatalogService::fetch_models(base_url, token)` will follow this pattern but:

- URL is `${base_url}/models` (caller-supplied — github.com or enterprise base)
- Adds `Authorization: Bearer ${token}` header
- Uses a `Client::builder().timeout(Duration::from_millis(5000))` (per slice 3 §1.1: 5000 ms single attempt, no retry)
- Parses the Copilot-specific schema (NOT `ModelsDevResponse`) into a new local type `CopilotModelsResponse { data: Vec<CopilotModelEntry> }` mirroring the Zod shape from slice 3 §1.1

The catalog response shape and the merge result are **different types** — the catalog is the over-the-wire schema, `ModelInfo` is the merged in-memory representation. The merge function bridges them.

---

## 4. Test Patterns to Mirror

### 4.1 PROV-055 wiremock integration test pattern — `codelet/providers/tests/copilot_http_middleware_routing_test.rs`

PROV-055 uses pure-Rust unit tests (no HTTP) for endpoint selection / classifier / header building / behavior dispatch. PROV-056 will use the same pattern: most scenarios are **pure logic** that can be tested by constructing a `CopilotModelsResponse` directly from `serde_json::from_str(...)` and asserting the merge result. No wiremock needed for the 9 PROV-056 scenarios because they all start from "the merged catalog contains..." or "the /models endpoint returns..." which we can simulate at the JSON-parse boundary.

### 4.2 Codex reasoning facade unit-test pattern — `codelet/providers/src/codex/mod.rs:557`

```rust
fn build_reasoning_params_with_high_effort_config() {
    // @step Given I have a thinking_config with reasoning effort "high"
    let config = serde_json::json!({"reasoning": {"effort": "high", "summary": "auto"}});

    // @step When I call build_reasoning_params with the thinking_config
    let params = CodexProvider::build_reasoning_params(Some(&config));

    // @step Then the params should contain reasoning.effort "high"
    assert_eq!(params["reasoning"]["effort"], "high");
    // ...
}
```

This is the exact `// @step` comment style that maps each Gherkin step to its assertion. We will mirror this in `tests/copilot_models_and_options_test.rs` per scenario in `github-copilot-model-catalog-provider-options-reasoning-effort.feature`.

---

## 5. Files to Create

| Path | Purpose | Expected size |
|---|---|---|
| `codelet/providers/src/copilot/models.rs` | `CopilotModelsResponse`, `CopilotModelEntry`, `CopilotModelCatalogService::merge_with_existing(...)`, `parse_release_date(id, version)`, `build_model_from_remote(...)` | ~250 lines |
| `codelet/providers/src/copilot/provider_options.rs` | `CopilotProviderOptionsFacade::build_reasoning_variants(model)`, `apply_store_false(options)`, `CopilotReasoningVariant` struct | ~200 lines |
| `codelet/providers/src/copilot/small_model.rs` | `CopilotSmallModelFacade::small_options(model)`, `small_model_priority(provider_id)`, `resolve_small_model(catalog, override)` | ~150 lines |
| `codelet/providers/tests/copilot_models_and_options_test.rs` | One test per scenario in `github-copilot-model-catalog-provider-options-reasoning-effort.feature`, with `// @step` comments | ~350 lines |

## 6. Module Wiring

Append to `codelet/providers/src/copilot/mod.rs`:

```rust
pub mod models;
pub mod provider_options;
pub mod small_model;

pub use models::{
    CopilotModelCatalogService, CopilotModelEntry, CopilotModelsResponse, CopilotModelCapabilities,
    CopilotModelLimits, CopilotModelSupports,
};
pub use provider_options::{
    CopilotProviderOptionsFacade, CopilotReasoningVariant,
};
pub use small_model::{
    CopilotSmallModelFacade,
};
```

No changes to `codelet/providers/src/manager.rs` are required for PROV-056 — the Copilot module surface stays internal to the `copilot` namespace until a follow-up ticket wires it into `ProviderType` and the model registry.
