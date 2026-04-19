//! Rhai ↔ JSON conversion helpers for the custom provider (PROV-063).
//!
//! These helpers are pulled into the `custom` module so the provider can
//! convert between `serde_json::Value` (used by reqwest bodies) and
//! `rhai::Dynamic` (the currency of the Rhai engine) without depending on
//! the private copies inside `oauth::building_blocks`.

use rhai::{Dynamic, Map};

/// Convert a `serde_json::Value` into a Rhai `Dynamic`.
///
/// Mapping:
/// - `Null` → unit
/// - `Bool` → bool
/// - `Number` → i64 when it fits, otherwise f64 (NaN/∞ safe)
/// - `String` → Rhai string
/// - `Array` → Rhai array
/// - `Object` → Rhai map
pub(crate) fn json_value_to_dynamic(value: &serde_json::Value) -> Dynamic {
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

/// Convert a Rhai `Dynamic` into a `serde_json::Value`.
///
/// Unknown / unhandled Dynamic types serialise as `Null`.
pub(crate) fn dynamic_to_json_value(value: &Dynamic) -> serde_json::Value {
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
        let arr = value
            .clone()
            .into_typed_array::<Dynamic>()
            .unwrap_or_default();
        serde_json::Value::Array(arr.iter().map(dynamic_to_json_value).collect())
    } else if value.is_map() {
        // `cast` would panic on type mismatch. `try_cast` returns Option
        // so we can fall back to Null instead.
        match value.clone().try_cast::<Map>() {
            Some(map) => {
                let mut obj = serde_json::Map::new();
                for (k, v) in &map {
                    obj.insert(k.to_string(), dynamic_to_json_value(v));
                }
                serde_json::Value::Object(obj)
            }
            None => serde_json::Value::Null,
        }
    } else {
        serde_json::Value::Null
    }
}
