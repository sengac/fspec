//! Review validation for `update-work-unit-status` (Example Mapping, architecture notes, AST research)
//! and the scenario-existence check.

use std::path::Path;

use super::prefill::linked_feature_files;
use super::queries::{
    array_len, check_unanswered_questions, has_estimate, has_examples, is_parent_work_unit,
    soft_dependency_warning, string_array, Data,
};
use super::reminders;
use super::temporal::check_temporal_ordering;
use super::transitions::msg;
use crate::error::FspecCoreError;

/// Two-level review validation (REMIND-014). Mirrors `performReviewValidation`
/// in `src/utils/review-validation.ts`: ordered Level-1 hard blocks for stories
/// (Example Mapping → architectural notes → AST research). Bugs are exempt.
pub(super) fn perform_review_validation(
    data: &Data,
    id: &str,
    work_type: &str,
) -> Result<Option<String>, FspecCoreError> {
    if work_type == "bug" {
        return Ok(None);
    }
    if array_len(data, id, "rules")? == 0 || array_len(data, id, "examples")? == 0 {
        return Err(msg(
            "Cannot transition to testing - Example Mapping incomplete. \
Use: fspec add-rule <workUnitId> \"<rule>\" and fspec add-example <workUnitId> \"<example>\" to complete Example Mapping.",
        ));
    }
    if array_len(data, id, "architectureNotes")? == 0 {
        return Err(msg(
            "Cannot transition to testing - no architectural notes documented. \
Use: fspec add-architecture-note <workUnitId> <note> to add architectural notes explaining implementation approach and alignment with existing codebase.",
        ));
    }
    let has_ast_research = string_array(data, id, "attachments")?
        .iter()
        .any(|a| a.contains("ast-research"));
    if !has_ast_research {
        return Err(msg(ast_research_error_message(is_in_capture_mode())));
    }
    // Level 2: subjective analysis reminder (non-blocking) for stories.
    Ok(reminders::subjective_review_reminder(id, data))
}

/// Mirrors `isInCaptureMode()` (`src/utils/output.ts`). In TS, capture mode is
/// active when output is being buffered for a codelet agent invoked via the
/// NAPI callback; direct CLI invocation is never in capture mode. The Rust
/// core's NAPI delegation (TOOL-019) sets `FSPEC_CAPTURE_MODE=1` when it serves
/// an agent request, so the CLI binary defaults to `false` — matching the TS
/// CLI default exactly. Delegates to the shared [`crate::utils::mode::in_capture_mode`].
pub(super) fn is_in_capture_mode() -> bool {
    crate::utils::mode::in_capture_mode()
}

/// Mirrors `buildASTResearchErrorMessage()` (`src/utils/review-validation.ts`).
/// In capture mode the agent has the AstGrep tool natively, so it is directed
/// there; in CLI mode the user is directed to `fspec astgrep` (CLI-015).
pub(super) fn ast_research_error_message(capture: bool) -> &'static str {
    if capture {
        "Cannot transition to testing - no AST research performed during discovery. \
Use the AstGrep tool to analyze relevant code in the codebase. \
Save output to file matching pattern: ast-research-<description>.json or ast-research-<description>.md. \
Then attach: fspec add-attachment <workUnitId> spec/attachments/<workUnitId>/ast-research-<description>.{json|md}"
    } else {
        "Cannot transition to testing - no AST research performed during discovery. \
FIRST run: fspec astgrep --help (to learn HOW to use the astgrep tool). \
THEN use: fspec astgrep --pattern <pattern> --lang <language> [--path <path>] to analyze relevant code. \
Save output to file matching pattern: ast-research-<description>.json or ast-research-<description>.md. \
Then attach: fspec add-attachment <workUnitId> spec/attachments/<workUnitId>/ast-research-<description>.{json|md}"
    }
}

/// Assert at least one feature file is tagged with `@id` (mirrors
/// `checkScenariosExist`, which treats tag presence as scenario existence).
pub(super) fn check_scenarios_exist(project_root: &Path, id: &str) -> Result<(), FspecCoreError> {
    let features = linked_feature_files(project_root, id)
        .map_err(|err| msg(format!("Cannot find the scenarios of {id}: {err}")))?;
    if features.is_empty() {
        return Err(msg(format!(
            "No Gherkin scenarios found for work unit {id}. At least one scenario must be tagged with @{id}. Use 'fspec generate-scenarios {id}' or manually tag scenarios."
        )));
    }
    Ok(())
}

/// The specifying -> testing gate: review validation, a bug's linked feature,
/// answered questions, existing scenarios and temporal ordering. Adds
/// warnings for missing examples, estimate and unfinished soft dependencies.
/// Returns the subjective review reminder (REMIND-014).
pub(super) fn check_testing_entry(
    project_root: &Path,
    data: &Data,
    id: &str,
    work_type: &str,
    skip_temporal: bool,
    warnings: &mut Vec<String>,
) -> Result<Option<String>, FspecCoreError> {
    // Review validation (Example Mapping + architectural notes + AST research).
    let review_reminder = perform_review_validation(data, id, work_type)?;

    // Bugs must link to an existing feature file.
    if work_type == "bug" {
        let has_linked = array_len(data, id, "linkedFeatures")? > 0;
        let has_scenarios = !linked_feature_files(project_root, id)
            .map_err(|err| msg(format!("Cannot find the feature files of {id}: {err}")))?
            .is_empty();
        if !has_linked && !has_scenarios {
            return Err(msg(format!(
                "Bugs must link to existing feature file before moving to testing.\n\n\
Use: fspec add-tag-to-feature spec/features/<feature-name>.feature @{id}\n\
Or tag scenarios in feature files with @{id}\n\n\
If the feature has no spec, create a story instead of a bug."
            )));
        }
    }

    // Unanswered questions block the transition.
    check_unanswered_questions(data, id, "specifying", "testing")?;

    // Warn if no examples captured.
    if !has_examples(data, id)? {
        warnings.push(
            "No examples captured in Example Mapping. Consider adding examples with 'fspec add-example' before testing."
                .to_string(),
        );
    }

    // Scenarios must exist (skip for parent work units with children).
    if !is_parent_work_unit(data, id)? {
        check_scenarios_exist(project_root, id)?;
    }

    // Temporal validation: feature file created after entering specifying.
    if !skip_temporal {
        check_temporal_ordering(project_root, data, id, "testing")?;
    }

    // Warn if no estimate.
    if !has_estimate(data, id) {
        warnings.push(
            "No estimate assigned. Consider adding estimate with --estimate=<points>".to_string(),
        );
    }

    // Warn about soft dependencies (dependsOn) that aren't done.
    if let Some(w) = soft_dependency_warning(data, id)? {
        warnings.push(w);
    }

    Ok(review_reminder)
}
