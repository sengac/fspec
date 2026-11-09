//! `update-feature-text` — BUG-507.
//!
//! Replaces exactly one occurrence of a text anywhere in a feature file
//! outside its steps and scenario headers: the Feature title, the description,
//! `#` comments, Rule blocks and data table / Examples cells. The text may span
//! several lines.
//!
//! It refuses (writing nothing):
//!   * an empty `find`, a text that is not in the file, or one that occurs
//!     more than once (the error names the count);
//!   * a match that touches a step line or a Scenario / Scenario Outline /
//!     Background header — `update-step` and `update-scenario` change those
//!     and keep `@step` comments and coverage in sync;
//!   * a result that is not valid Gherkin, or that changes any scenario name
//!     or step.
//!
//! Two-front-doors: the dispatcher and the fspec CLI (`fspec
//! update-feature-text <feature> --find <text> --replace <text>`) both call
//! [`run`].

use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::json;

use crate::error::FspecCoreError;
use crate::io::gherkin::parse_feature_lenient;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateFeatureTextArgs {
    feature: String,
    find: String,
    replace: String,
}

fn invalid(reason: String) -> FspecCoreError {
    FspecCoreError::InvalidArgs {
        command: "update-feature-text",
        reason,
    }
}

/// Dispatcher / CLI entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: UpdateFeatureTextArgs = serde_json::from_str(args_json)
        .map_err(|e| invalid(format!("failed to parse args: {e}")))?;
    if args.find.is_empty() {
        return Err(invalid("--find must not be empty".to_string()));
    }

    let path = resolve_feature_path(project_root, &args.feature);
    let content = std::fs::read_to_string(&path).map_err(|error| {
        invalid(format!(
            "cannot read feature file {}: {error}",
            path.display()
        ))
    })?;
    let feature = parse_feature_lenient(&content)
        .map_err(|e| invalid(format!("{} is not valid Gherkin: {e}", path.display())))?;

    let matches: Vec<usize> = content
        .match_indices(&args.find)
        .map(|(offset, _)| offset)
        .collect();
    let offset = match matches.as_slice() {
        [offset] => *offset,
        [] => {
            return Err(invalid(format!(
                "text not found in {}: '{}'",
                path.display(),
                args.find
            )))
        }
        several => {
            return Err(invalid(format!(
                "found {} matches of the text in {}; make --find unique",
                several.len(),
                path.display()
            )))
        }
    };

    // 1-based lines the match touches.
    let first_line = content[..offset].matches('\n').count() + 1;
    let last_line = first_line + args.find.matches('\n').count();
    let protected = protected_lines(&feature);
    if let Some((line, kind)) = (first_line..=last_line).find_map(|line| {
        protected
            .iter()
            .find(|(protected_line, _)| *protected_line == line)
            .copied()
    }) {
        let (what, command) = match kind {
            Protected::Step => ("a step line", "fspec update-step"),
            Protected::Header => ("a scenario or background header", "fspec update-scenario"),
        };
        return Err(invalid(format!(
            "the text touches {what} (line {line}); use {command}, which keeps @step comments and coverage in sync"
        )));
    }

    let updated = format!(
        "{}{}{}",
        &content[..offset],
        args.replace,
        &content[offset + args.find.len()..]
    );
    let reparsed = parse_feature_lenient(&updated)
        .map_err(|e| invalid(format!("the result is not valid Gherkin: {e}")))?;
    if outline(&reparsed) != outline(&feature) {
        return Err(invalid(
            "the result changes a scenario name or step; use fspec update-scenario or fspec update-step"
                .to_string(),
        ));
    }

    std::fs::write(&path, &updated).map_err(|source| FspecCoreError::Io {
        command: "update-feature-text",
        source,
    })?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&args.feature)
        .to_string();
    serde_json::to_string(&json!({
        "success": true,
        "message": format!("✓ Updated text on line {first_line} of {file_name}"),
    }))
    .map_err(|e| invalid(format!("failed to serialise response: {e}")))
}

#[derive(Clone, Copy)]
enum Protected {
    Step,
    Header,
}

/// 1-based lines holding a step or a scenario / background header.
fn protected_lines(feature: &gherkin::Feature) -> Vec<(usize, Protected)> {
    let mut lines = Vec::new();
    let add_background = |background: &gherkin::Background, lines: &mut Vec<_>| {
        lines.push((background.position.line, Protected::Header));
        for step in &background.steps {
            lines.push((step.position.line, Protected::Step));
        }
    };
    let add_scenario = |scenario: &gherkin::Scenario, lines: &mut Vec<_>| {
        lines.push((scenario.position.line, Protected::Header));
        for step in &scenario.steps {
            lines.push((step.position.line, Protected::Step));
        }
    };
    if let Some(background) = &feature.background {
        add_background(background, &mut lines);
    }
    for scenario in &feature.scenarios {
        add_scenario(scenario, &mut lines);
    }
    for rule in &feature.rules {
        if let Some(background) = &rule.background {
            add_background(background, &mut lines);
        }
        for scenario in &rule.scenarios {
            add_scenario(scenario, &mut lines);
        }
    }
    lines
}

/// Every scenario name with its steps, and the background steps, in order.
fn outline(feature: &gherkin::Feature) -> Vec<String> {
    let mut entries = Vec::new();
    let mut push = |text: String| entries.push(text);
    let scenarios = feature
        .scenarios
        .iter()
        .chain(feature.rules.iter().flat_map(|rule| rule.scenarios.iter()));
    for step in feature.background.iter().flat_map(|b| &b.steps) {
        push(format!("background: {}{}", step.keyword, step.value));
    }
    for scenario in scenarios {
        push(format!("scenario: {}", scenario.name));
        for step in &scenario.steps {
            push(format!("step: {}{}", step.keyword, step.value));
        }
    }
    entries
}

fn resolve_feature_path(project_root: &Path, feature: &str) -> PathBuf {
    if feature.ends_with(".feature") || feature.starts_with("spec/features/") {
        project_root.join(feature)
    } else {
        crate::io::spec_paths::features_dir_of(project_root).join(format!("{feature}.feature"))
    }
}
