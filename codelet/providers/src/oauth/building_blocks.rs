//! Rhai Building Block Modules (PROV-060 / PROV-086)
//!
//! Registers `http::`, `crypto::`, `json::`, `oauth::`, and (optionally)
//! `cred::` modules for use in Rhai scripts. Each module exposes
//! provider-agnostic primitives that custom OAuth scripts can use.
//!
//! The provider-scoped `cred::` module is kept in a separate file to
//! stay within the 300-line limit; `pub use` re-exports are provided
//! below so existing callers that import from `oauth::building_blocks`
//! continue to work unchanged.

use rhai::{Dynamic, Map, Module};

use super::engine::RhaiModule;

pub use super::cred_module::{build_cred_module, fspec_home};
pub use super::json_convert::{dynamic_to_json_value, json_value_to_dynamic};

/// Register all default PROV-060 building block modules.
///
/// Returns a Vec of `RhaiModule` for use with `build_sandboxed_engine`.
///
/// This set does **not** include the `cred::` module — the `cred::`
/// namespace is scoped to a specific provider name and is only added
/// by [`register_all_modules_for_provider`] (PROV-086).
pub fn register_all_modules() -> Vec<RhaiModule> {
    vec![
        build_http_module(),
        build_crypto_module(),
        build_json_module(),
        build_oauth_module(),
        build_log_module(),
    ]
}

/// Build the `log` module with `warn`, `info`, `debug`, `error`, and
/// `trace` functions that forward to Rust's `tracing` facility.
///
/// Rhai scripts can call `log::warn("message")` to emit diagnostics
/// that flow through the TypeScript log bridge into `~/.fspec/fspec.log`,
/// which is invaluable for debugging the rhai dispatch pipeline.
///
/// Signatures accept either a single string or a key/value map
/// (rendered as `key=value, ...` pairs) so scripts can emit
/// structured-ish entries without a real format() API.
fn build_log_module() -> RhaiModule {
    let mut module = Module::new();

    fn fmt_map(m: &Map) -> String {
        let mut parts = Vec::with_capacity(m.len());
        for (k, v) in m {
            parts.push(format!("{k}={v:?}"));
        }
        parts.join(", ")
    }

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

/// Register the default building block modules plus a provider-scoped
/// `cred::` module (PROV-086).
///
/// The returned module list is intended for
/// [`super::engine::build_provider_engine`]. The `cred::` module binds
/// the given `provider_name` at build time; any call to `cred::read`,
/// `cred::write`, `cred::delete`, or `cred::path` that passes a
/// different name is rejected with an access-denied runtime error —
/// preventing one provider's script from reading another provider's
/// credential file.
pub fn register_all_modules_for_provider(provider_name: &str) -> Vec<RhaiModule> {
    let mut modules = register_all_modules();
    modules.push(build_cred_module(provider_name.to_string()));
    modules
}

/// Build the `http` module with `post` and `get` functions.
///
/// Uses `ureq` for synchronous HTTP (Rhai is sync-only).
fn build_http_module() -> RhaiModule {
    let mut module = Module::new();

    // http::post(url, body, headers) -> Map { status, body }
    module.set_native_fn(
        "post",
        |url: String, body: String, headers: Map| -> Result<Dynamic, Box<rhai::EvalAltResult>> {
            let mut req = ureq::post(&url);
            for (key, value) in &headers {
                if let Ok(v) = value.clone().into_string() {
                    req = req.set(key.as_str(), &v);
                }
            }
            let response = req.send_string(&body).map_err(|e| {
                Box::new(rhai::EvalAltResult::ErrorRuntime(
                    format!("HTTP POST failed: {e}").into(),
                    rhai::Position::NONE,
                ))
            })?;
            let status = response.status();
            let resp_body = response.into_string().map_err(|e| {
                Box::new(rhai::EvalAltResult::ErrorRuntime(
                    format!("Failed to read response body: {e}").into(),
                    rhai::Position::NONE,
                ))
            })?;
            let mut result = Map::new();
            result.insert("status".into(), Dynamic::from(status as i64));
            result.insert("body".into(), Dynamic::from(resp_body));
            Ok(Dynamic::from_map(result))
        },
    );

    // http::get(url, headers) -> Map { status, body }
    module.set_native_fn(
        "get",
        |url: String, headers: Map| -> Result<Dynamic, Box<rhai::EvalAltResult>> {
            let mut req = ureq::get(&url);
            for (key, value) in &headers {
                if let Ok(v) = value.clone().into_string() {
                    req = req.set(key.as_str(), &v);
                }
            }
            let response = req.call().map_err(|e| {
                Box::new(rhai::EvalAltResult::ErrorRuntime(
                    format!("HTTP GET failed: {e}").into(),
                    rhai::Position::NONE,
                ))
            })?;
            let status = response.status();
            let resp_body = response.into_string().map_err(|e| {
                Box::new(rhai::EvalAltResult::ErrorRuntime(
                    format!("Failed to read response body: {e}").into(),
                    rhai::Position::NONE,
                ))
            })?;
            let mut result = Map::new();
            result.insert("status".into(), Dynamic::from(status as i64));
            result.insert("body".into(), Dynamic::from(resp_body));
            Ok(Dynamic::from_map(result))
        },
    );

    RhaiModule {
        name: "http".to_string(),
        module,
    }
}

/// Build the `crypto` module with `sha256` and `base64url_encode`.
fn build_crypto_module() -> RhaiModule {
    let mut module = Module::new();

    // crypto::sha256(data) -> hex string
    module.set_native_fn(
        "sha256",
        |data: String| -> Result<Dynamic, Box<rhai::EvalAltResult>> {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(data.as_bytes());
            let hash = hasher.finalize();
            Ok(Dynamic::from(format!("{hash:x}")))
        },
    );

    // crypto::base64url_encode(data) -> base64url string (no padding)
    module.set_native_fn(
        "base64url_encode",
        |data: String| -> Result<Dynamic, Box<rhai::EvalAltResult>> {
            use base64::Engine;
            let encoded =
                base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data.as_bytes());
            Ok(Dynamic::from(encoded))
        },
    );

    RhaiModule {
        name: "crypto".to_string(),
        module,
    }
}

/// Build the `json` module with `parse` and `stringify`.
fn build_json_module() -> RhaiModule {
    let mut module = Module::new();

    // json::parse(s) -> Dynamic map/array
    module.set_native_fn(
        "parse",
        |s: String| -> Result<Dynamic, Box<rhai::EvalAltResult>> {
            let value: serde_json::Value = serde_json::from_str(&s).map_err(|e| {
                Box::new(rhai::EvalAltResult::ErrorRuntime(
                    format!("JSON parse failed: {e}").into(),
                    rhai::Position::NONE,
                ))
            })?;
            Ok(json_value_to_dynamic(&value))
        },
    );

    // json::stringify(value) -> string
    module.set_native_fn(
        "stringify",
        |value: Dynamic| -> Result<Dynamic, Box<rhai::EvalAltResult>> {
            let json_val = dynamic_to_json_value(&value);
            let s = serde_json::to_string(&json_val).map_err(|e| {
                Box::new(rhai::EvalAltResult::ErrorRuntime(
                    format!("JSON stringify failed: {e}").into(),
                    rhai::Position::NONE,
                ))
            })?;
            Ok(Dynamic::from(s))
        },
    );

    RhaiModule {
        name: "json".to_string(),
        module,
    }
}

/// Build the `oauth` module with `generate_pkce` and `generate_state`.
fn build_oauth_module() -> RhaiModule {
    let mut module = Module::new();

    // oauth::generate_pkce() -> Map { verifier, challenge, challenge_method }
    module.set_native_fn(
        "generate_pkce",
        || -> Result<Dynamic, Box<rhai::EvalAltResult>> {
            let pkce = crate::oauth_crypto::generate_pkce();
            let mut result = Map::new();
            result.insert("verifier".into(), Dynamic::from(pkce.verifier));
            result.insert("challenge".into(), Dynamic::from(pkce.challenge));
            result.insert(
                "challenge_method".into(),
                Dynamic::from(pkce.challenge_method),
            );
            Ok(Dynamic::from_map(result))
        },
    );

    // oauth::generate_state() -> random state string
    module.set_native_fn(
        "generate_state",
        || -> Result<Dynamic, Box<rhai::EvalAltResult>> {
            use rand::Rng;
            let state: String = rand::thread_rng()
                .sample_iter(&rand::distributions::Alphanumeric)
                .take(32)
                .map(char::from)
                .collect();
            Ok(Dynamic::from(state))
        },
    );

    // oauth::urlencoded(s) -> percent-encoded string
    module.set_native_fn(
        "urlencoded",
        |s: String| -> Result<Dynamic, Box<rhai::EvalAltResult>> {
            Ok(Dynamic::from(crate::oauth_crypto::urlencoded(&s)))
        },
    );

    RhaiModule {
        name: "oauth".to_string(),
        module,
    }
}
