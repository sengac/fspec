//! Rhai `log::` building block module.
//!
//! Provides `warn`, `info`, `debug`, `error`, and `trace` functions
//! that forward to Rust's `tracing` facility. Scripts can emit
//! diagnostics via `log::warn("message")` or pass a `Map` whose entries
//! are rendered as `key=value, ...` pairs.
//!
//! Extracted from `building_blocks.rs` to keep that file under the
//! 300-line project limit.

use rhai::{Dynamic, Map, Module};

use super::engine::RhaiModule;

/// Format a Rhai `Map` as a `key=value, ...` string for structured-ish
/// log output.
fn fmt_map(m: &Map) -> String {
    let mut parts = Vec::with_capacity(m.len());
    for (k, v) in m {
        parts.push(format!("{k}={v:?}"));
    }
    parts.join(", ")
}

/// Build the `log` module with `warn`, `info`, `debug`, `error`, and
/// `trace` functions that forward to Rust's `tracing` facility.
///
/// Signatures accept either a single string or a key/value map
/// (rendered as `key=value, ...` pairs) so scripts can emit
/// structured-ish entries without a real `format()` API.
pub fn build_log_module() -> RhaiModule {
    let mut module = Module::new();

    module.set_native_fn(
        "warn",
        |msg: String| -> Result<Dynamic, Box<rhai::EvalAltResult>> {
            tracing::warn!(target: "rhai_script", source = "rhai", "{}", msg);
            Ok(Dynamic::UNIT)
        },
    );
    module.set_native_fn(
        "warn",
        |label: String, data: Map| -> Result<Dynamic, Box<rhai::EvalAltResult>> {
            tracing::warn!(target: "rhai_script", source = "rhai", "{} {{ {} }}", label, fmt_map(&data));
            Ok(Dynamic::UNIT)
        },
    );
    module.set_native_fn(
        "info",
        |msg: String| -> Result<Dynamic, Box<rhai::EvalAltResult>> {
            tracing::info!(target: "rhai_script", source = "rhai", "{}", msg);
            Ok(Dynamic::UNIT)
        },
    );
    module.set_native_fn(
        "info",
        |label: String, data: Map| -> Result<Dynamic, Box<rhai::EvalAltResult>> {
            tracing::info!(target: "rhai_script", source = "rhai", "{} {{ {} }}", label, fmt_map(&data));
            Ok(Dynamic::UNIT)
        },
    );
    module.set_native_fn(
        "debug",
        |msg: String| -> Result<Dynamic, Box<rhai::EvalAltResult>> {
            tracing::debug!(target: "rhai_script", source = "rhai", "{}", msg);
            Ok(Dynamic::UNIT)
        },
    );
    module.set_native_fn(
        "debug",
        |label: String, data: Map| -> Result<Dynamic, Box<rhai::EvalAltResult>> {
            tracing::debug!(target: "rhai_script", source = "rhai", "{} {{ {} }}", label, fmt_map(&data));
            Ok(Dynamic::UNIT)
        },
    );
    module.set_native_fn(
        "error",
        |msg: String| -> Result<Dynamic, Box<rhai::EvalAltResult>> {
            tracing::error!(target: "rhai_script", source = "rhai", "{}", msg);
            Ok(Dynamic::UNIT)
        },
    );
    module.set_native_fn(
        "error",
        |label: String, data: Map| -> Result<Dynamic, Box<rhai::EvalAltResult>> {
            tracing::error!(target: "rhai_script", source = "rhai", "{} {{ {} }}", label, fmt_map(&data));
            Ok(Dynamic::UNIT)
        },
    );
    module.set_native_fn(
        "trace",
        |msg: String| -> Result<Dynamic, Box<rhai::EvalAltResult>> {
            tracing::trace!(target: "rhai_script", source = "rhai", "{}", msg);
            Ok(Dynamic::UNIT)
        },
    );

    RhaiModule {
        name: "log".to_string(),
        module,
    }
}
