//! Paired indexed selector: `testMappingIndex` + `expectedScenarioHash`.

use serde_json::Value;

use super::{invalid, scenario_hash};
use crate::error::FspecCoreError;

/// A validated exact row selector. Both members are mandatory together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Selector {
    pub index: usize,
    pub expected_hash: String,
}

impl Selector {
    /// Parse the optional raw selector members. Returns `Ok(None)` only when
    /// both are absent (legacy mode); any partial, negative, fractional or
    /// malformed selector is rejected before the sidecar is read.
    pub(crate) fn parse(
        index: Option<&Value>,
        hash: Option<&Value>,
        command: &'static str,
    ) -> Result<Option<Self>, FspecCoreError> {
        let (index, hash) =
            match (index, hash) {
                (None, None) => return Ok(None),
                (Some(index), Some(hash)) => (index, hash),
                _ => return Err(invalid(
                    command,
                    "--test-mapping-index and --expected-scenario-hash must be supplied together"
                        .to_string(),
                )),
            };
        let index = index
            .as_u64()
            .and_then(|index| usize::try_from(index).ok())
            .ok_or_else(|| {
                invalid(
                    command,
                    format!(
                        "--test-mapping-index must be a non-negative integer, received {index}"
                    ),
                )
            })?;
        let expected_hash = hash
            .as_str()
            .filter(|hash| {
                hash.len() == 64
                    && hash
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            })
            .ok_or_else(|| {
                invalid(
                    command,
                    format!(
                        "--expected-scenario-hash must be a lowercase 64-character SHA-256 hash, received {hash}"
                    ),
                )
            })?
            .to_string();
        Ok(Some(Self {
            index,
            expected_hash,
        }))
    }

    /// Verify, inside the sidecar transaction, that the selected scenario is
    /// unchanged and the selected row exists for exactly `test_file`.
    pub(crate) fn verify(
        &self,
        scenario: &Value,
        test_file: &str,
        command: &'static str,
    ) -> Result<(), FspecCoreError> {
        let actual = scenario_hash(scenario);
        if actual != self.expected_hash {
            return Err(invalid(
                command,
                format!(
                    "Stale scenario hash: expected {}, current {actual}\nSuggestion: re-read rows with fspec show-coverage <feature> --mapping-json",
                    self.expected_hash
                ),
            ));
        }
        let rows = scenario
            .get("testMappings")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                invalid(
                    command,
                    "Selected scenario has no testMappings array".into(),
                )
            })?;
        let row = rows.get(self.index).ok_or_else(|| {
            invalid(
                command,
                format!(
                    "--test-mapping-index {} is out of bounds for {} test mappings",
                    self.index,
                    rows.len()
                ),
            )
        })?;
        match row.get("file").and_then(Value::as_str) {
            Some(file) if file == test_file => Ok(()),
            other => Err(invalid(
                command,
                format!(
                    "Test mapping {} belongs to {}, not --test-file {test_file}",
                    self.index,
                    other.unwrap_or("<no file>")
                ),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use serde_json::json;

    #[test]
    fn rejects_unpaired_and_malformed_selectors() {
        let hash = json!("a".repeat(64));
        assert_eq!(Selector::parse(None, None, "t").unwrap(), None);
        assert!(Selector::parse(Some(&json!(1)), None, "t").is_err());
        assert!(Selector::parse(None, Some(&hash), "t").is_err());
        for index in [json!(-1), json!(1.5), json!("1")] {
            assert!(Selector::parse(Some(&index), Some(&hash), "t").is_err());
        }
        for bad in [json!("A".repeat(64)), json!("a".repeat(63)), json!(1)] {
            assert!(Selector::parse(Some(&json!(0)), Some(&bad), "t").is_err());
        }
        let parsed = Selector::parse(Some(&json!(2)), Some(&hash), "t").unwrap();
        assert_eq!(parsed.map(|s| s.index), Some(2));
    }
}
