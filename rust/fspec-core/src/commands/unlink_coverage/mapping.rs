//! Mapping removal operations for `unlink-coverage`.

use serde_json::Value;

use super::invalid;
use crate::error::FspecCoreError;

/// Remove one implementation mapping from a test-mapping row.
pub(super) fn remove_impl_from_row(
    tm: &mut Value,
    imf: &str,
    scenario: &str,
) -> Result<String, FspecCoreError> {
    // TS: testMapping.implMappings.findIndex — throws if implMappings
    // is undefined.
    let impl_mappings = tm.get_mut("implMappings").and_then(Value::as_array_mut);
    let impl_mappings = match impl_mappings {
        Some(ims) => ims,
        None => {
            return Err(invalid(
                "Cannot read properties of undefined (reading 'findIndex')".to_string(),
            ));
        }
    };
    let impl_pos = impl_mappings
        .iter()
        .position(|im| im.get("file").and_then(Value::as_str) == Some(imf));
    let impl_pos = match impl_pos {
        Some(p) => p,
        None => {
            return Err(invalid(format!(
                "Implementation file not found in test mapping: {imf}\nSuggestion: Run fspec show-coverage to see current mappings"
            )));
        }
    };
    impl_mappings.remove(impl_pos);
    Ok(format!(
        "\u{2713} Removed implementation mapping {imf} from scenario \"{scenario}\""
    ))
}

/// The scenario's `testMappings` array; `reading` names the TS accessor that
/// would throw when the array is absent.
pub(super) fn test_mappings<'a>(
    scenario_entry: &'a mut Value,
    reading: &str,
) -> Result<&'a mut Vec<Value>, FspecCoreError> {
    scenario_entry
        .get_mut("testMappings")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| {
            invalid(format!(
                "Cannot read properties of undefined (reading '{reading}')"
            ))
        })
}

/// Legacy first-match row position for `tf`.
pub(super) fn first_row(test_mappings: &[Value], tf: &str) -> Result<usize, FspecCoreError> {
    test_mappings
        .iter()
        .position(|tm| tm.get("file").and_then(Value::as_str) == Some(tf))
        .ok_or_else(|| {
            invalid(format!(
                "Test file not found in scenario mappings: {tf}\nSuggestion: Run fspec show-coverage to see current mappings"
            ))
        })
}

pub(super) fn removed_row_message(tf: &str, scenario: &str) -> String {
    format!(
        "\u{2713} Removed test mapping {tf} (and all its implementation mappings) from scenario \"{scenario}\""
    )
}

/// BUG-502: position of the first mapping of `tf` whose implementation
/// mappings include `imf`.
pub(super) fn carrying_row(test_mappings: &[Value], tf: &str, imf: &str) -> Option<usize> {
    test_mappings.iter().position(|tm| {
        tm.get("file").and_then(Value::as_str) == Some(tf)
            && tm
                .get("implMappings")
                .and_then(Value::as_array)
                .is_some_and(|ims| {
                    ims.iter()
                        .any(|im| im.get("file").and_then(Value::as_str) == Some(imf))
                })
    })
}
