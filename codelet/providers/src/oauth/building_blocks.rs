//! Rhai Building Block Modules (PROV-060)
//!
//! Registers http::, crypto::, json::, and oauth:: modules for use
//! in Rhai scripts. Each module exposes provider-agnostic primitives
//! that custom OAuth scripts can use.

use rhai::{Dynamic, Map, Module};

use super::engine::RhaiModule;

/// Register all default PROV-060 building block modules.
///
/// Returns a Vec of `RhaiModule` for use with `build_sandboxed_engine`.
pub fn register_all_modules() -> Vec<RhaiModule> {
    vec![
        build_http_module(),
        build_crypto_module(),
        build_json_module(),
        build_oauth_module(),
    ]
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

/// Convert `serde_json::Value` to Rhai `Dynamic`.
fn json_value_to_dynamic(value: &serde_json::Value) -> Dynamic {
    match value {
        serde_json::Value::Null => Dynamic::UNIT,
        serde_json::Value::Bool(b) => Dynamic::from(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Dynamic::from(i)
            } else if let Some(f) = n.as_f64() {
                Dynamic::from(f)
            } else {
                Dynamic::UNIT
            }
        }
        serde_json::Value::String(s) => Dynamic::from(s.clone()),
        serde_json::Value::Array(arr) => {
            let items: Vec<Dynamic> = arr.iter().map(json_value_to_dynamic).collect();
            Dynamic::from_array(items)
        }
        serde_json::Value::Object(obj) => {
            let mut map = Map::new();
            for (k, v) in obj {
                map.insert(k.clone().into(), json_value_to_dynamic(v));
            }
            Dynamic::from_map(map)
        }
    }
}

/// Convert Rhai `Dynamic` to `serde_json::Value`.
fn dynamic_to_json_value(value: &Dynamic) -> serde_json::Value {
    if value.is_unit() {
        serde_json::Value::Null
    } else if let Ok(b) = value.as_bool() {
        serde_json::Value::Bool(b)
    } else if let Ok(i) = value.as_int() {
        serde_json::Value::Number(serde_json::Number::from(i))
    } else if let Ok(f) = value.as_float() {
        serde_json::Number::from_f64(f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null)
    } else if let Ok(s) = value.clone().into_string() {
        serde_json::Value::String(s)
    } else if value.is_array() {
        let arr = value.clone().into_typed_array::<Dynamic>().unwrap_or_default();
        serde_json::Value::Array(arr.iter().map(dynamic_to_json_value).collect())
    } else if value.is_map() {
        let map = value.clone().cast::<Map>();
        let mut obj = serde_json::Map::new();
        for (k, v) in &map {
            obj.insert(k.to_string(), dynamic_to_json_value(v));
        }
        serde_json::Value::Object(obj)
    } else {
        serde_json::Value::Null
    }
}
