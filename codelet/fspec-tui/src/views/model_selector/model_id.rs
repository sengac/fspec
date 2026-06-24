//! PROV-117 root cause #1 — registry-id normalization.
//!
//! Faithful port of the TypeScript `extractModelIdForRegistry`
//! (`src/tui/services/modelInitializationService.ts:42-49`): strips a
//! trailing `-YYYYMMDD` date suffix from a model id so the registry-normalized
//! family id (`claude-sonnet-4`) and the dated catalog id
//! (`claude-sonnet-4-20250514`) compare equal.
//!
//! The TS helper lives in the TUI services layer, so the Rust equivalent
//! belongs in the model-selector view layer (NOT the provider/registry layer):
//! it is consumed only by the `(current)` marker / cursor-seed comparison
//! sites (`state.rs`, `rows.rs`, `rows_render.rs`). The RAW dated id continues
//! to flow untouched through `Action::ModelSelected` → `set_session_model` →
//! `ProviderManager::select_model` so the actual model API call still receives
//! the dated id (TS `apiModelId`).

/// Strip a trailing `-YYYYMMDD` (exactly 8 digits) date suffix from a model id.
/// Returns the input unchanged when no such suffix is present.
///
/// Mirrors TS `extractModelIdForRegistry` regex `^(.+)-(\d{8})$` → group 1.
pub(crate) fn extract_model_id_for_registry(model_id: &str) -> &str {
    if let Some((head, tail)) = model_id.rsplit_once('-') {
        if tail.len() == 8 && tail.bytes().all(|b| b.is_ascii_digit()) && !head.is_empty() {
            return head;
        }
    }
    model_id
}

/// Whether two model ids refer to the same model after registry
/// normalization (both sides date-suffix-stripped). Mirrors TS
/// `extractModelIdForRegistry(m.id) === normalizedModelId`
/// (modelInitializationService.ts:133-136), but normalizes BOTH operands so a
/// dated row id matches a normalized current id AND vice-versa.
pub(crate) fn model_ids_match(a: &str, b: &str) -> bool {
    a == b || extract_model_id_for_registry(a) == extract_model_id_for_registry(b)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn strips_trailing_eight_digit_date_suffix() {
        // @step Given a dated catalog id
        // @step Then the -YYYYMMDD suffix is stripped (TS extractModelIdForRegistry parity)
        assert_eq!(
            extract_model_id_for_registry("claude-sonnet-4-20250514"),
            "claude-sonnet-4"
        );
    }

    #[test]
    fn leaves_suffix_free_id_unchanged() {
        assert_eq!(
            extract_model_id_for_registry("prov117-plain-alpha"),
            "prov117-plain-alpha"
        );
    }

    #[test]
    fn does_not_strip_non_eight_digit_or_non_numeric_tails() {
        // 4-digit tail (not a date) is preserved.
        assert_eq!(extract_model_id_for_registry("gpt-4o"), "gpt-4o");
        // 7 digits — wrong length.
        assert_eq!(extract_model_id_for_registry("model-1234567"), "model-1234567");
        // 9 digits — wrong length.
        assert_eq!(
            extract_model_id_for_registry("model-123456789"),
            "model-123456789"
        );
        // alpha tail.
        assert_eq!(extract_model_id_for_registry("o3-mini"), "o3-mini");
    }

    #[test]
    fn match_is_symmetric_across_normalization() {
        assert!(model_ids_match(
            "claude-sonnet-4",
            "claude-sonnet-4-20250514"
        ));
        assert!(model_ids_match(
            "claude-sonnet-4-20250514",
            "claude-sonnet-4"
        ));
        assert!(model_ids_match("gpt-4o", "gpt-4o"));
        assert!(!model_ids_match("claude-sonnet-4", "claude-opus-4"));
    }
}
