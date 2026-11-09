//! Canonical whole-scenario hashing: object keys are sorted recursively while
//! every array keeps its order and every unknown member participates.

use serde_json::Value;
use sha2::{Digest, Sha256};

/// Lowercase hex SHA-256 of the canonical compact JSON of `scenario`.
pub(crate) fn scenario_hash(scenario: &Value) -> String {
    hex::encode(Sha256::digest(canonical(scenario).to_string().as_bytes()))
}

fn canonical(value: &Value) -> Value {
    match value {
        Value::Object(object) => {
            let mut members: Vec<(&String, &Value)> = object.iter().collect();
            members.sort_by_key(|(key, _)| *key);
            Value::Object(
                members
                    .into_iter()
                    .map(|(key, value)| (key.clone(), canonical(value)))
                    .collect(),
            )
        }
        Value::Array(values) => Value::Array(values.iter().map(canonical).collect()),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use serde_json::json;

    #[test]
    fn key_order_is_irrelevant_but_array_order_is_not() {
        let left = json!({"b": [1, 2], "a": {"y": 1, "x": 2}});
        let right = json!({"a": {"x": 2, "y": 1}, "b": [1, 2]});
        assert_eq!(scenario_hash(&left), scenario_hash(&right));
        assert_ne!(
            scenario_hash(&left),
            scenario_hash(&json!({"a": {"x": 2, "y": 1}, "b": [2, 1]}))
        );
        assert_eq!(scenario_hash(&left).len(), 64);
    }
}
