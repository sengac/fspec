//! Rhai-scriptable system prompt facade for custom providers (PROV-065).
//!
//! Implements [`SystemPromptFacade`](codelet_tools::facade::SystemPromptFacade)
//! and delegates to three optional Rhai functions defined by the custom
//! provider script:
//!
//! - `identity_prefix(config)` — returns a String or unit
//! - `transform_preamble(config, preamble, fspec_guidance)` — returns a String
//! - `format_system_prompt(config, preamble, fspec_guidance)` — returns a
//!   String (plain) or a Map `#{ format: "array", blocks: [...] }` for
//!   Claude-style structured output with `cache_control` metadata.
//!
//! When an optional function is missing or raises a runtime error, the
//! facade falls back to sensible defaults built around
//! [`prepend_fspec_guidance`](codelet_tools::facade::prepend_fspec_guidance).
//! Facade methods never panic.
//!
//! # `'static` lifetime handling
//!
//! The [`SystemPromptFacade`] trait requires `provider()` and
//! `identity_prefix()` to return `&'static str`. Because Rhai produces
//! owned `String` values, we resolve the values lazily and leak them via
//! `Box::leak`, caching the resulting `&'static str` inside a
//! [`OnceCell`]. Providers are created once per process so the leak is
//! bounded and intentional.

use std::sync::Arc;

use once_cell::sync::OnceCell;
use rhai::{Dynamic, Engine, Map, Scope, AST};
use serde_json::Value;

use codelet_tools::facade::{prepend_fspec_guidance, SystemPromptFacade};
use codelet_tools::FSPEC_WORKFLOW_GUIDANCE;

use super::conversion::dynamic_to_json_value;

/// Rhai script function names this facade recognises.
const FN_IDENTITY_PREFIX: &str = "identity_prefix";
const FN_TRANSFORM_PREAMBLE: &str = "transform_preamble";
const FN_FORMAT_SYSTEM_PROMPT: &str = "format_system_prompt";

/// System prompt facade backed by an optional Rhai script.
///
/// See the module-level documentation for the semantics of each optional
/// script function and the fallback behaviour.
pub struct RhaiSystemPromptFacade {
    provider_name: String,
    engine: Arc<Engine>,
    ast: Arc<AST>,
    config: Dynamic,
    /// Lazily-populated cache for `identity_prefix()`. `None` means the
    /// script does not provide a prefix; `Some(leaked)` means the prefix
    /// has been leaked as `&'static str`.
    identity_prefix: OnceCell<Option<&'static str>>,
    /// Lazily-populated cache for `provider()` — leaked `provider_name`.
    provider_static: OnceCell<&'static str>,
}

impl RhaiSystemPromptFacade {
    /// Build a new facade.
    ///
    /// Infallible by design: no Rhai functions are invoked at construction
    /// time. Evaluation happens lazily on the first call to each trait
    /// method, and any Rhai errors are logged and fall back to defaults.
    pub fn new(
        provider_name: String,
        engine: Arc<Engine>,
        ast: Arc<AST>,
        config: Dynamic,
    ) -> Self {
        Self {
            provider_name,
            engine,
            ast,
            config,
            identity_prefix: OnceCell::new(),
            provider_static: OnceCell::new(),
        }
    }

    /// Return `true` if the compiled AST defines a function with
    /// `fn_name`. Arity is intentionally not compared so scripts may use
    /// whichever signature is convenient — arity mismatches surface as
    /// Rhai runtime errors and trigger the fallback path.
    fn has_script_fn(&self, fn_name: &str) -> bool {
        self.ast.iter_functions().any(|f| f.name == fn_name)
    }

    /// Invoke a Rhai function and return the resulting [`Dynamic`],
    /// logging and swallowing any error. Callers are responsible for
    /// interpreting the returned value.
    fn call_script_fn(
        &self,
        fn_name: &str,
        args: impl rhai::FuncArgs,
    ) -> Option<Dynamic> {
        let mut scope = Scope::new();
        match self
            .engine
            .call_fn::<Dynamic>(&mut scope, &self.ast, fn_name, args)
        {
            Ok(value) => Some(value),
            Err(e) => {
                tracing::warn!(
                    provider = %self.provider_name,
                    function = %fn_name,
                    error = %e,
                    "Rhai system prompt function failed; using default"
                );
                None
            }
        }
    }

    /// Resolve the identity prefix lazily, leaking the result for
    /// `'static` lifetime. A missing function, a unit return, an empty
    /// string, or any runtime error all resolve to `None`.
    fn resolve_identity_prefix(&self) -> Option<&'static str> {
        *self.identity_prefix.get_or_init(|| {
            if !self.has_script_fn(FN_IDENTITY_PREFIX) {
                return None;
            }
            let value = self.call_script_fn(FN_IDENTITY_PREFIX, (self.config.clone(),))?;
            if value.is_unit() {
                return None;
            }
            let text = match value.into_string() {
                Ok(s) => s,
                Err(typ) => {
                    tracing::warn!(
                        provider = %self.provider_name,
                        actual_type = %typ,
                        "identity_prefix() must return a string or (); using None"
                    );
                    return None;
                }
            };
            if text.is_empty() {
                return None;
            }
            let leaked: &'static str = Box::leak(text.into_boxed_str());
            Some(leaked)
        })
    }

    /// Convert the return value of `format_system_prompt` into a
    /// [`serde_json::Value`] according to the documented protocol. Returns
    /// `None` when the value shape is not recognised; callers fall back to
    /// the default plain-string format.
    fn format_value_from_dynamic(&self, value: Dynamic) -> Option<Value> {
        // Rhai strings first (Dynamic reports is_map()=false for strings).
        if value.is_string() {
            if let Ok(s) = value.into_string() {
                return Some(Value::String(s));
            }
            return None;
        }

        if !value.is_map() {
            tracing::warn!(
                provider = %self.provider_name,
                "format_system_prompt must return a string or an array-format map; using default"
            );
            return None;
        }

        let map = value.try_cast::<Map>()?;
        let format_tag = map
            .get("format")
            .and_then(|v| v.clone().into_string().ok())
            .unwrap_or_default();
        if format_tag != "array" {
            tracing::warn!(
                provider = %self.provider_name,
                format = %format_tag,
                "format_system_prompt map must have format='array'; using default"
            );
            return None;
        }

        let blocks = map.get("blocks")?.clone();
        if !blocks.is_array() {
            tracing::warn!(
                provider = %self.provider_name,
                "format_system_prompt map 'blocks' must be an array; using default"
            );
            return None;
        }
        let block_list = blocks.into_typed_array::<Dynamic>().ok()?;
        let json_blocks: Vec<Value> = block_list
            .iter()
            .map(dynamic_to_json_value)
            .collect();
        Some(Value::Array(json_blocks))
    }
}

impl SystemPromptFacade for RhaiSystemPromptFacade {
    fn provider(&self) -> &'static str {
        self.provider_static.get_or_init(|| {
            let owned = self.provider_name.clone();
            Box::leak(owned.into_boxed_str())
        })
    }

    fn identity_prefix(&self) -> Option<&'static str> {
        self.resolve_identity_prefix()
    }

    fn transform_preamble(&self, preamble: &str) -> String {
        if !self.has_script_fn(FN_TRANSFORM_PREAMBLE) {
            return prepend_fspec_guidance(preamble);
        }
        let args = (
            self.config.clone(),
            preamble.to_string(),
            FSPEC_WORKFLOW_GUIDANCE.to_string(),
        );
        match self.call_script_fn(FN_TRANSFORM_PREAMBLE, args) {
            Some(value) => value
                .into_string()
                .unwrap_or_else(|_| prepend_fspec_guidance(preamble)),
            None => prepend_fspec_guidance(preamble),
        }
    }

    fn format_for_api(&self, preamble: &str) -> Value {
        if !self.has_script_fn(FN_FORMAT_SYSTEM_PROMPT) {
            return Value::String(prepend_fspec_guidance(preamble));
        }
        let args = (
            self.config.clone(),
            preamble.to_string(),
            FSPEC_WORKFLOW_GUIDANCE.to_string(),
        );
        match self
            .call_script_fn(FN_FORMAT_SYSTEM_PROMPT, args)
            .and_then(|value| self.format_value_from_dynamic(value))
        {
            Some(value) => value,
            None => Value::String(prepend_fspec_guidance(preamble)),
        }
    }
}

impl std::fmt::Debug for RhaiSystemPromptFacade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RhaiSystemPromptFacade")
            .field("provider_name", &self.provider_name)
            .field(
                "identity_prefix_cached",
                &self.identity_prefix.get().copied(),
            )
            .finish_non_exhaustive()
    }
}
