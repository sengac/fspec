//! PROV-139 — parse/format helpers extracted from `profile_form.rs` to keep
//! that module under the 300-LoC ceiling.
//!
//! Feature: spec/features/provider-settings-profile-form.feature
//!
//! Pure functions shared by `ProfileForm::from_definition` (formatting stored
//! values back into editable strings) and `ProfileForm::build_definition`
//! (parsing typed strings into the wire shape). No form state lives here.

use codelet_rpc_types::ProfileDefinition;

use crate::views::model_selector::form::parse_compaction_trigger;

/// TS `compactionThresholdParser.ts` range constants (lines 15-21). Mirrored on
/// the profile save path only — the shared `parse_compaction_trigger` and the
/// model_selector custom-model form stay range-free (TS does not range-check
/// the custom-model form).
const MIN_PERCENTAGE: u32 = 1;
const MAX_PERCENTAGE: u32 = 100;
const MIN_TOKEN_THRESHOLD: u32 = 1000;

/// Format an optional numeric field back into its editable string.
pub(super) fn opt_num(value: Option<u32>) -> String {
    value.map(|n| n.to_string()).unwrap_or_default()
}

/// Profile-scoped compaction-trigger parse: split via the shared
/// [`parse_compaction_trigger`], then enforce the TS range rules (percentage
/// 1..=100 inclusive, tokens >= 1000). Out-of-range → `(None, None)` so the
/// field is omitted from the saved profile, matching TS
/// `parseCompactionThreshold`. The shared splitter — and therefore the
/// model_selector custom-model form — is left range-free.
pub(super) fn profile_compaction_trigger(raw: &str) -> (Option<String>, Option<u32>) {
    let (kind, value) = parse_compaction_trigger(raw);
    match (kind.as_deref(), value) {
        (Some("percentage"), Some(n)) if (MIN_PERCENTAGE..=MAX_PERCENTAGE).contains(&n) => {
            (kind, value)
        }
        (Some("tokens"), Some(n)) if n >= MIN_TOKEN_THRESHOLD => (kind, value),
        _ => (None, None),
    }
}

/// Render a stored compaction threshold back into its raw editable string
/// (`percentage` → `"80%"`, `tokens` → `"200000"`, otherwise empty).
pub(super) fn render_threshold(kind: Option<&str>, value: Option<u32>) -> String {
    match (kind, value) {
        (Some("percentage"), Some(v)) => format!("{v}%"),
        (Some("tokens"), Some(v)) => v.to_string(),
        _ => String::new(),
    }
}

/// PROV-142: parse the Auto-Continue form field's raw string into the wire
/// value. Empty ⇒ `None` (off, today's behavior); `"0"` ⇒ `Some(0)` (the
/// explicit-off sentinel); `"n"` (n >= 1) ⇒ `Some(n)` (on with budget n).
/// Non-numeric input is an `Err` with a user-facing hint mirroring
/// `/continue`'s invalid-argument rejection.
pub(super) fn parse_auto_continue(raw: &str) -> Result<Option<u32>, String> {
    match raw.trim() {
        "" => Ok(None),
        text => text.parse::<u32>().map(Some).map_err(|_| {
            "Auto-Continue must be 0 (off) or a positive integer budget (e.g. 300)".to_string()
        }),
    }
}

/// PROV-144: parse the Max Images form field's raw string into the wire value.
/// Empty ⇒ `None` (absent on disk ⇒ the tool-layer default of 4); `"0"` ⇒
/// `Some(0)` (the no-vision sentinel — the Read tool fails image reads);
/// `"n"` (n >= 1) ⇒ `Some(n)` (a single Read result returns at most n images).
/// Non-numeric input is an `Err` with a user-facing hint.
pub(super) fn parse_max_images(raw: &str) -> Result<Option<u32>, String> {
    match raw.trim() {
        "" => Ok(None),
        text => text.parse::<u32>().map(Some).map_err(|_| {
            "Max Images must be a whole number (0 = no vision, 4 = default)".to_string()
        }),
    }
}

/// PROV-145: parse the Loop Window form field's raw string into the wire
/// value. Empty ⇒ `None` (absent on disk ⇒ the RIG-014 default window of 160
/// words); `"n"` ⇒ `Some(n)`. Non-numeric input (including floats like
/// `"1.5"`) is an `Err` with a user-facing hint.
pub(super) fn parse_loop_detection_window(raw: &str) -> Result<Option<u32>, String> {
    match raw.trim() {
        "" => Ok(None),
        text => text
            .parse::<u32>()
            .map(Some)
            .map_err(|_| "Loop Window must be a whole number of words (160 = default)".to_string()),
    }
}

/// PROV-145: parse the Loop Repeat form field's raw string into the wire
/// value. Empty ⇒ `None` (absent on disk ⇒ the RIG-014 default threshold of
/// 10); `"n"` ⇒ `Some(n)`. Non-numeric input is an `Err` with a user-facing
/// hint.
pub(super) fn parse_loop_detection_max_repeats(raw: &str) -> Result<Option<u32>, String> {
    match raw.trim() {
        "" => Ok(None),
        text => text
            .parse::<u32>()
            .map(Some)
            .map_err(|_| {
                "Loop Repeat must be a whole number (10 = default)".to_string()
            }),
    }
}

/// PROV-145: parse the Loop Retries form field's raw string into the wire
/// value. Empty ⇒ `None` (absent on disk ⇒ the RIG-014 default cap of 10);
/// `"0"` ⇒ `Some(0)` (the never-auto-retry sentinel); `"n"` ⇒ `Some(n)`.
/// Non-numeric input is an `Err` with a user-facing hint.
pub(super) fn parse_loop_detection_max_retries(raw: &str) -> Result<Option<u32>, String> {
    match raw.trim() {
        "" => Ok(None),
        text => text
            .parse::<u32>()
            .map(Some)
            .map_err(|_| {
                "Loop Retries must be a whole number (0 = never retry, 10 = default)".to_string()
            }),
    }
}

/// Build a [`ProfileDefinition`] from the current form values.
///
/// Moved from `profile_form.rs` to keep that file under the 300-LoC
/// ceiling. Returns `Err(hint)` when the save must be REJECTED with a
/// visible hint (PROV-142 non-numeric Auto-Continue; PROV-144
/// non-numeric Max Images; PROV-145 non-numeric loop-detection fields);
/// `Ok(None)` when base URL, API key, or the trimmed name is empty (TS
/// `handleSave` guard — the form stays open silently); `Ok(Some(def))`
/// on success.
pub fn build_definition(
    form: &super::profile_form::ProfileForm,
) -> Result<Option<ProfileDefinition>, String> {
    if form.base_url.is_empty() || form.api_key.is_empty() || form.name.trim().is_empty() {
        return Ok(None);
    }
    // PROV-142: parse the Auto-Continue field. Empty ⇒ None (off, today's
    // behavior); "0" ⇒ Some(0) (explicit-off sentinel); "n" (n >= 1) ⇒
    // Some(n) (on with budget n); non-numeric ⇒ reject with a hint.
    let auto_continue = parse_auto_continue(&form.auto_continue)?;
    // PROV-144: parse the Max Images field. Empty ⇒ None (absent ⇒ default
    // 4); "0" ⇒ Some(0) (no-vision sentinel); "n" (n >= 1) ⇒ Some(n)
    // (cap of n images per Read result); non-numeric ⇒ reject with a hint.
    let max_images = parse_max_images(&form.max_images)?;
    // PROV-145: parse the loop-detection numeric fields. Empty ⇒ None
    // (absent ⇒ the RIG-014 defaults 160 / 10 / 10); "n" ⇒ Some(n);
    // non-numeric ⇒ reject with a hint naming the field.
    let loop_detection_window = parse_loop_detection_window(&form.loop_window)?;
    let loop_detection_max_repeats = parse_loop_detection_max_repeats(&form.loop_repeat)?;
    let loop_detection_max_retries = parse_loop_detection_max_retries(&form.loop_retries)?;
    let (compaction_threshold_type, compaction_threshold_value) =
        profile_compaction_trigger(&form.compaction_threshold);
    Ok(Some(ProfileDefinition {
        base_url: form.base_url.clone(),
        api_key: form.api_key.clone(),
        context_window: form.context_window.trim().parse::<u32>().ok(),
        max_output_tokens: form.max_output_tokens.trim().parse::<u32>().ok(),
        compaction_threshold_type,
        compaction_threshold_value,
        streaming: Some(form.streaming),
        auto_continue,
        // PROV-143: always carry the explicit toggle so the on-disk
        // profile reflects the form (true ⇒ preserved, false ⇒ stripped).
        preserve_thinking: Some(form.preserve_thinking),
        // PROV-144: carry the parsed Max Images limit (empty ⇒ None so the
        // persistence read-modify-write REMOVES the key ⇒ default 4).
        max_images,
        // PROV-145: always carry the explicit loop-detection toggle so
        // the on-disk profile reflects the form (true ⇒ detector on,
        // false ⇒ detector off).
        loop_detection_enabled: Some(form.loop_detection),
        // PROV-145: carry the parsed loop-detection numeric fields
        // (empty ⇒ None so the persistence read-modify-write REMOVES the
        // keys ⇒ the RIG-014 defaults apply).
        loop_detection_window,
        loop_detection_max_repeats,
        loop_detection_max_retries,
    }))
}
