//! Lenient serde deserializers for tool arguments.
//!
//! Lower-powered LLMs sometimes send numeric parameters as strings (e.g., `"10"` instead of `10`)
//! or boolean parameters as strings (e.g., `"true"` instead of `true`).
//!
//! These custom deserializers accept both the native type and its string representation,
//! preventing hard deserialization failures that would otherwise surface as opaque API errors.
//!
//! Usage on Args structs:
//! ```ignore
//! #[serde(default, deserialize_with = "crate::serde_coerce::deser_option_usize")]
//! pub limit: Option<usize>,
//! ```

use serde::{Deserialize, Deserializer};
use serde_json::Value;

/// Deserialize `Option<usize>` accepting both a JSON number and a numeric string.
pub fn deser_option_usize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<usize>, D::Error> {
    let v = Option::<Value>::deserialize(d)?;
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => Ok(n.as_u64().map(|n| n as usize)),
        Some(Value::String(s)) => Ok(s.trim().parse::<usize>().ok()),
        _ => Ok(None),
    }
}

/// Deserialize `Option<u64>` accepting both a JSON number and a numeric string.
pub fn deser_option_u64<'de, D: Deserializer<'de>>(d: D) -> Result<Option<u64>, D::Error> {
    let v = Option::<Value>::deserialize(d)?;
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => Ok(n.as_u64()),
        Some(Value::String(s)) => Ok(s.trim().parse::<u64>().ok()),
        _ => Ok(None),
    }
}

/// Deserialize `Option<u32>` accepting both a JSON number and a numeric string.
pub fn deser_option_u32<'de, D: Deserializer<'de>>(d: D) -> Result<Option<u32>, D::Error> {
    let v = Option::<Value>::deserialize(d)?;
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => Ok(n.as_u64().and_then(|n| u32::try_from(n).ok())),
        Some(Value::String(s)) => Ok(s.trim().parse::<u32>().ok()),
        _ => Ok(None),
    }
}

/// Deserialize `Option<f32>` accepting both a JSON number and a numeric string.
pub fn deser_option_f32<'de, D: Deserializer<'de>>(d: D) -> Result<Option<f32>, D::Error> {
    let v = Option::<Value>::deserialize(d)?;
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => Ok(n.as_f64().map(|n| n as f32)),
        Some(Value::String(s)) => Ok(s.trim().parse::<f32>().ok()),
        _ => Ok(None),
    }
}

/// Deserialize `Option<bool>` accepting a JSON boolean, `"true"`/`"false"` strings,
/// and numeric `0`/`1`.
pub fn deser_option_bool<'de, D: Deserializer<'de>>(d: D) -> Result<Option<bool>, D::Error> {
    let v = Option::<Value>::deserialize(d)?;
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(b)),
        Some(Value::String(s)) => match s.trim().to_lowercase().as_str() {
            "true" | "1" | "yes" => Ok(Some(true)),
            "false" | "0" | "no" => Ok(Some(false)),
            _ => Ok(None),
        },
        Some(Value::Number(n)) => Ok(n.as_u64().map(|n| n != 0)),
        _ => Ok(None),
    }
}

/// Deserialize a required `usize` accepting both a JSON number and a numeric string.
pub fn deser_usize<'de, D: Deserializer<'de>>(d: D) -> Result<usize, D::Error> {
    let v = Value::deserialize(d)?;
    match &v {
        Value::Number(n) => n
            .as_u64()
            .map(|n| n as usize)
            .ok_or_else(|| serde::de::Error::custom("expected non-negative integer")),
        Value::String(s) => s
            .trim()
            .parse::<usize>()
            .map_err(|_| serde::de::Error::custom(format!("cannot parse \"{s}\" as an unsigned integer"))),
        _ => Err(serde::de::Error::custom(format!(
            "expected number or numeric string, got {v}"
        ))),
    }
}

/// Deserialize a required `bool` accepting a JSON boolean, `"true"`/`"false"` strings,
/// and numeric `0`/`1`.
pub fn deser_bool<'de, D: Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    let v = Value::deserialize(d)?;
    match &v {
        Value::Bool(b) => Ok(*b),
        Value::String(s) => match s.trim().to_lowercase().as_str() {
            "true" | "1" | "yes" => Ok(true),
            "false" | "0" | "no" => Ok(false),
            _ => Err(serde::de::Error::custom(format!(
                "cannot parse \"{s}\" as boolean (expected true/false/yes/no/0/1)"
            ))),
        },
        Value::Number(n) => n
            .as_u64()
            .map(|n| n != 0)
            .ok_or_else(|| serde::de::Error::custom("expected 0 or 1 for boolean")),
        _ => Err(serde::de::Error::custom(format!(
            "expected boolean or boolean string, got {v}"
        ))),
    }
}

/// Deserialize `Vec<usize>` where each element accepts both a JSON number and a numeric string.
pub fn deser_vec_usize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<usize>, D::Error> {
    let v = Value::deserialize(d)?;
    match v {
        Value::Array(arr) => {
            let mut result = Vec::with_capacity(arr.len());
            for (i, item) in arr.iter().enumerate() {
                let n = match item {
                    Value::Number(n) => n
                        .as_u64()
                        .map(|n| n as usize)
                        .ok_or_else(|| serde::de::Error::custom(format!("element [{i}]: expected non-negative integer"))),
                    Value::String(s) => s
                        .trim()
                        .parse::<usize>()
                        .map_err(|_| serde::de::Error::custom(format!("element [{i}]: cannot parse \"{s}\" as unsigned integer"))),
                    _ => Err(serde::de::Error::custom(format!(
                        "element [{i}]: expected number or numeric string, got {item}"
                    ))),
                }?;
                result.push(n);
            }
            Ok(result)
        }
        _ => Err(serde::de::Error::custom(format!(
            "expected array, got {v}"
        ))),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use serde_json::json;

    #[derive(Deserialize, Debug)]
    struct TestOptionUsize {
        #[serde(default, deserialize_with = "deser_option_usize")]
        val: Option<usize>,
    }

    #[derive(Deserialize, Debug)]
    struct TestOptionBool {
        #[serde(default, deserialize_with = "deser_option_bool")]
        val: Option<bool>,
    }

    #[derive(Deserialize, Debug)]
    struct TestOptionU64 {
        #[serde(default, deserialize_with = "deser_option_u64")]
        val: Option<u64>,
    }

    #[derive(Deserialize, Debug)]
    struct TestOptionU32 {
        #[serde(default, deserialize_with = "deser_option_u32")]
        val: Option<u32>,
    }

    #[derive(Deserialize, Debug)]
    struct TestOptionF32 {
        #[serde(default, deserialize_with = "deser_option_f32")]
        val: Option<f32>,
    }

    #[derive(Deserialize, Debug)]
    struct TestRequiredUsize {
        #[serde(deserialize_with = "deser_usize")]
        val: usize,
    }

    #[derive(Deserialize, Debug)]
    struct TestRequiredBool {
        #[serde(deserialize_with = "deser_bool")]
        val: bool,
    }

    #[derive(Deserialize, Debug)]
    struct TestVecUsize {
        #[serde(deserialize_with = "deser_vec_usize")]
        val: Vec<usize>,
    }

    // ── Option<usize> ──────────────────────────────

    #[test]
    fn option_usize_from_number() {
        let t: TestOptionUsize = serde_json::from_value(json!({"val": 42})).unwrap();
        assert_eq!(t.val, Some(42));
    }

    #[test]
    fn option_usize_from_string() {
        let t: TestOptionUsize = serde_json::from_value(json!({"val": "42"})).unwrap();
        assert_eq!(t.val, Some(42));
    }

    #[test]
    fn option_usize_from_null() {
        let t: TestOptionUsize = serde_json::from_value(json!({"val": null})).unwrap();
        assert_eq!(t.val, None);
    }

    #[test]
    fn option_usize_missing() {
        let t: TestOptionUsize = serde_json::from_value(json!({})).unwrap();
        assert_eq!(t.val, None);
    }

    #[test]
    fn option_usize_unparseable_string() {
        let t: TestOptionUsize = serde_json::from_value(json!({"val": "abc"})).unwrap();
        assert_eq!(t.val, None);
    }

    // ── Option<bool> ──────────────────────────────

    #[test]
    fn option_bool_from_bool() {
        let t: TestOptionBool = serde_json::from_value(json!({"val": true})).unwrap();
        assert_eq!(t.val, Some(true));
    }

    #[test]
    fn option_bool_from_string_true() {
        let t: TestOptionBool = serde_json::from_value(json!({"val": "true"})).unwrap();
        assert_eq!(t.val, Some(true));
    }

    #[test]
    fn option_bool_from_string_false() {
        let t: TestOptionBool = serde_json::from_value(json!({"val": "false"})).unwrap();
        assert_eq!(t.val, Some(false));
    }

    #[test]
    fn option_bool_from_number_1() {
        let t: TestOptionBool = serde_json::from_value(json!({"val": 1})).unwrap();
        assert_eq!(t.val, Some(true));
    }

    #[test]
    fn option_bool_from_number_0() {
        let t: TestOptionBool = serde_json::from_value(json!({"val": 0})).unwrap();
        assert_eq!(t.val, Some(false));
    }

    #[test]
    fn option_bool_missing() {
        let t: TestOptionBool = serde_json::from_value(json!({})).unwrap();
        assert_eq!(t.val, None);
    }

    // ── Option<u64> ──────────────────────────────

    #[test]
    fn option_u64_from_number() {
        let t: TestOptionU64 = serde_json::from_value(json!({"val": 24})).unwrap();
        assert_eq!(t.val, Some(24));
    }

    #[test]
    fn option_u64_from_string() {
        let t: TestOptionU64 = serde_json::from_value(json!({"val": "24"})).unwrap();
        assert_eq!(t.val, Some(24));
    }

    // ── Option<u32> ──────────────────────────────

    #[test]
    fn option_u32_from_number() {
        let t: TestOptionU32 = serde_json::from_value(json!({"val": 5})).unwrap();
        assert_eq!(t.val, Some(5));
    }

    #[test]
    fn option_u32_from_string() {
        let t: TestOptionU32 = serde_json::from_value(json!({"val": "5"})).unwrap();
        assert_eq!(t.val, Some(5));
    }

    // ── Option<f32> ──────────────────────────────

    #[test]
    fn option_f32_from_number() {
        let t: TestOptionF32 = serde_json::from_value(json!({"val": 0.75})).unwrap();
        assert!((t.val.unwrap() - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn option_f32_from_string() {
        let t: TestOptionF32 = serde_json::from_value(json!({"val": "0.75"})).unwrap();
        assert!((t.val.unwrap() - 0.75).abs() < f32::EPSILON);
    }

    // ── Required usize ──────────────────────────────

    #[test]
    fn required_usize_from_number() {
        let t: TestRequiredUsize = serde_json::from_value(json!({"val": 10})).unwrap();
        assert_eq!(t.val, 10);
    }

    #[test]
    fn required_usize_from_string() {
        let t: TestRequiredUsize = serde_json::from_value(json!({"val": "10"})).unwrap();
        assert_eq!(t.val, 10);
    }

    #[test]
    fn required_usize_from_bad_string() {
        let r = serde_json::from_value::<TestRequiredUsize>(json!({"val": "abc"}));
        assert!(r.is_err());
    }

    // ── Required bool ──────────────────────────────

    #[test]
    fn required_bool_from_bool() {
        let t: TestRequiredBool = serde_json::from_value(json!({"val": true})).unwrap();
        assert!(t.val);
    }

    #[test]
    fn required_bool_from_string() {
        let t: TestRequiredBool = serde_json::from_value(json!({"val": "true"})).unwrap();
        assert!(t.val);
    }

    // ── Vec<usize> ──────────────────────────────

    #[test]
    fn vec_usize_from_numbers() {
        let t: TestVecUsize = serde_json::from_value(json!({"val": [1, 2, 3]})).unwrap();
        assert_eq!(t.val, vec![1, 2, 3]);
    }

    #[test]
    fn vec_usize_from_strings() {
        let t: TestVecUsize = serde_json::from_value(json!({"val": ["1", "2", "3"]})).unwrap();
        assert_eq!(t.val, vec![1, 2, 3]);
    }

    #[test]
    fn vec_usize_mixed() {
        let t: TestVecUsize = serde_json::from_value(json!({"val": [1, "2", 3]})).unwrap();
        assert_eq!(t.val, vec![1, 2, 3]);
    }
}
