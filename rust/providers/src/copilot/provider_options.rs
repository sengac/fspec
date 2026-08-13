//! Copilot provider-options helpers (PROV-056 architecture note [3]).
//!
//! Zero-retention enforcement and other provider-level option mutations that
//! are applied to **every** Copilot request regardless of the selected model.
//!
//! Separation of concerns: model-catalog fetching lives in
//! [`crate::copilot::models`]; request-time option mutations live here. The
//! two concerns share no state.

use serde_json::Value;
use std::collections::HashMap;

/// Force `store: false` in the given provider-options map.
///
/// Called for **every** Copilot request irrespective of model id, family, or
/// any other attribute (PROV-056 rules [3] and [4]). The function takes only
/// the options map — its signature deliberately cannot inspect the model, so
/// there is no path by which a per-model branch could accidentally be
/// introduced.
pub fn apply_store_false(options: &mut HashMap<String, Value>) {
    options.insert("store".to_string(), Value::Bool(false));
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn apply_store_false_sets_store_to_false() {
        let mut opts: HashMap<String, Value> = HashMap::new();
        apply_store_false(&mut opts);
        assert_eq!(opts.get("store"), Some(&Value::Bool(false)));
    }

    #[test]
    fn apply_store_false_overrides_existing_true_value() {
        let mut opts: HashMap<String, Value> = HashMap::new();
        opts.insert("store".to_string(), Value::Bool(true));
        apply_store_false(&mut opts);
        assert_eq!(opts.get("store"), Some(&Value::Bool(false)));
    }

    #[test]
    fn apply_store_false_is_model_agnostic() {
        // The function signature CANNOT inspect any model id — it only takes
        // the options map. This is the compile-time guarantee of rule [4].
        let mut opts: HashMap<String, Value> = HashMap::new();
        apply_store_false(&mut opts);
        assert_eq!(opts.get("store"), Some(&Value::Bool(false)));
    }
}
