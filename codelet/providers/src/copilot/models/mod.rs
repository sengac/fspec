//! Copilot model catalog (PROV-056).
//!
//! The GitHub Copilot `/models` endpoint is the **sole source of truth** for
//! the catalog. There is intentionally:
//!
//! - **No** hardcoded model id, family, version, or release date in this
//!   module
//! - **No** fallback to `models.dev` or any other registry
//! - **No** date-gating or id-pattern heuristic for reasoning effort tiers
//! - **No** merge with a previously cached catalog
//! - **No** per-model special-casing of `store: false` — that lives in
//!   [`crate::copilot::provider_options`]
//!
//! Every field on every returned [`crate::models::ModelInfo`] is derived
//! directly from the current `/models` response. Each call to [`fetch_models`]
//! fully replaces whatever the caller previously held.
//!
//! ## Module layout
//!
//! | File            | Responsibility                                         |
//! |-----------------|--------------------------------------------------------|
//! | `schema.rs`     | Wire-format DTOs for the `/models` response            |
//! | `fetch.rs`      | Async HTTP fetch with timeout + JSON parse             |
//! | `builder.rs`    | Pure `CopilotModelEntry → ModelInfo` mapping           |
//!
//! Reasoning variants per model are stored verbatim in
//! `ModelInfo.options["reasoning_variants"]` as a JSON array of strings,
//! copied directly from `capabilities.supports.reasoning_effort`. If the
//! field is missing or empty, the array is empty — no inference.

pub mod builder;
pub mod fetch;
pub mod schema;

pub use builder::{
    build_catalog_from_response, build_model_info, derive_release_date, NPM_OPTION_KEY,
    PROVIDER_ID_OPTION_KEY, REASONING_VARIANTS_KEY,
};
pub use fetch::{fetch_models, COPILOT_FETCH_TIMEOUT, COPILOT_MODELS_PATH};
pub use schema::{
    CopilotModelCapabilities, CopilotModelEntry, CopilotModelLimits, CopilotModelSupports,
    CopilotModelsResponse,
};
