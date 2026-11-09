//! Selected single-feature generation (`featureName`, REFAC-020).
//!
//! Resolves exactly one feature under `spec/features` (never falling back to
//! the global scan), plans additions/removals losslessly against the raw
//! sidecar JSON, and only then writes — inside the shared sidecar transaction.
//! A dry run reports the identical plan without taking the lock or writing.

use std::path::{Component, PathBuf};

use super::super::coverage_lineage::SidecarTransaction;
use super::plan::{plan, Plan, PlanStatus};
use super::*;

/// One resolved selected feature and its sidecar.
struct Target {
    relative: String,
    feature_path: PathBuf,
    coverage_path: PathBuf,
}

pub(super) fn run_selected(
    project_root: &Path,
    feature_name: &str,
    dry_run: bool,
) -> Result<String, FspecCoreError> {
    let target = resolve(project_root, feature_name)?;
    let _transaction = if dry_run {
        None
    } else {
        Some(SidecarTransaction::acquire(
            &target.coverage_path,
            "generate-coverage",
        )?)
    };
    let planned = plan(&target.feature_path, &target.coverage_path)?;
    if !dry_run {
        if planned.status == PlanStatus::Blocked {
            return Err(invalid(format!(
                "Selected coverage generation blocked for {}:\n  - {}",
                target.relative,
                planned.blocked_reasons.join("\n  - ")
            )));
        }
        if let Some(next) = &planned.next {
            write_json_atomic(&target.coverage_path, next)?;
        }
    }
    render(&target, &planned, dry_run)
}

fn render(target: &Target, planned: &Plan, dry_run: bool) -> Result<String, FspecCoreError> {
    let coverage = format!("{}.coverage", target.relative);
    serde_json::to_string_pretty(&json!({
        "success": planned.status != PlanStatus::Blocked,
        "dryRun": dry_run,
        "plan": {
            "featureFile": target.relative,
            "coverageFile": coverage,
            "status": planned.status.as_str(),
            "addedScenarios": planned.added,
            "removedScenarios": planned.removed,
            "blockedReasons": planned.blocked_reasons,
        },
    }))
    .map_err(|e| invalid(format!("failed to serialize plan: {e}")))
}

/// Resolve `feature_name` to exactly one `.feature` file under
/// `spec/features`: a relative path when it contains `/`, otherwise a unique
/// basename anywhere beneath that directory.
fn resolve(project_root: &Path, feature_name: &str) -> Result<Target, FspecCoreError> {
    let rejected = |why: &str| invalid(format!("Invalid feature name {feature_name:?}: {why}"));
    if feature_name.trim().is_empty() {
        return Err(rejected("a feature name is required"));
    }
    if feature_name.contains('\\') {
        return Err(rejected("use '/' separators"));
    }
    let requested = Path::new(feature_name);
    if requested
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(rejected("must be a relative path without '.' or '..'"));
    }
    let with_extension = if feature_name.ends_with(".feature") {
        feature_name.to_string()
    } else {
        format!("{feature_name}.feature")
    };
    let features_dir = crate::io::spec_paths::features_dir_of(project_root);
    let relative = if feature_name.contains('/') {
        if !features_dir.join(&with_extension).is_file() {
            return Err(rejected("no such feature file under spec/features"));
        }
        with_extension
    } else {
        let mut found = Vec::new();
        find_named(&features_dir, &features_dir, &with_extension, &mut found)?;
        match found.len() {
            1 => found.remove(0),
            0 => return Err(rejected("no such feature file under spec/features")),
            count => {
                return Err(rejected(&format!(
                    "ambiguous; {count} matches ({}) — pass the path relative to spec/features",
                    found.join(", ")
                )))
            }
        }
    };
    let feature_path = features_dir.join(&relative);
    let mut coverage_path = feature_path.clone().into_os_string();
    coverage_path.push(".coverage");
    Ok(Target {
        relative: format!("spec/features/{relative}"),
        feature_path,
        coverage_path: PathBuf::from(coverage_path),
    })
}

fn find_named(
    root: &Path,
    directory: &Path,
    file_name: &str,
    found: &mut Vec<String>,
) -> Result<(), FspecCoreError> {
    let entries = std::fs::read_dir(directory)
        .map_err(|e| invalid(format!("Failed to read features directory: {e}")))?;
    let mut paths: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            find_named(root, &path, file_name, found)?;
        } else if path.file_name().and_then(|name| name.to_str()) == Some(file_name)
            && path.is_file()
        {
            if let Ok(relative) = path.strip_prefix(root) {
                let parts: Vec<String> = relative
                    .components()
                    .map(|part| part.as_os_str().to_string_lossy().into_owned())
                    .collect();
                found.push(parts.join("/"));
            }
        }
    }
    Ok(())
}
