//! `fspec audit`'s exact @step block search for drifted test mappings
//! (COV-078).
//!
//! A scenario's exact @step block in a file is a line range whose @step
//! comments, in line order, are exactly the scenario's steps in order
//! ([`exact_blocks`], the extractor link-coverage uses). For a scenario whose
//! test mapping is drifted, out of range or points at a missing file, the
//! block is looked for first in the scenario's mapped test files, then in the
//! files the working tree adds (untracked, or staged as added) that no other
//! feature maps. Found exactly once, `--fix` relinks the scenario to it;
//! otherwise the problem names the count and the candidate ranges. Only
//! comment text is read, so it works for any language.

use std::cell::OnceCell;
use std::collections::BTreeSet;
use std::path::Path;

use super::fix::Repair;
use super::source::Source;
use super::Sidecars;
use crate::commands::link_coverage::step::{block_count_fact, exact_blocks, parsed_scenario_steps};
use crate::error::FspecCoreError;
use crate::types::coverage::CoverageScenario;

/// What the block search decided for one scenario.
#[derive(Clone)]
pub(crate) enum Relink {
    /// The block occurs once: the repair that relinks the scenario to it.
    Once(Repair),
    /// The block does not occur exactly once (or cannot be placed): the fact
    /// to add to the problem.
    NotOnce(String),
    /// The block occurs once, exactly where the scenario already maps it.
    Unchanged,
}

/// The files the working tree adds: untracked files and files staged as
/// added (COV-077: under --changed, only the files staged as added), read
/// once per run and only when a search needs them.
#[derive(Default)]
pub(crate) struct AddedFiles(OnceCell<Vec<String>>);

impl AddedFiles {
    fn get(&self, project_root: &Path, source: &Source) -> Result<&[String], FspecCoreError> {
        if let Some(files) = self.0.get() {
            return Ok(files);
        }
        let files = added_files(project_root, source)?;
        Ok(self.0.get_or_init(|| files))
    }
}

/// Untracked and staged-as-added files of the git working tree rooted at
/// `project_root`; none when the project root is not a git working tree.
fn added_files(project_root: &Path, source: &Source) -> Result<Vec<String>, FspecCoreError> {
    if !project_root.join(".git").exists() {
        return Ok(Vec::new());
    }
    let git_error = |error: codelet_git::GitError| FspecCoreError::InvalidArgs {
        command: "audit",
        reason: format!("cannot list the files the working tree adds: {error}"),
    };
    let mut files: BTreeSet<String> = BTreeSet::new();
    if let Source::WorkTree = source {
        files.extend(codelet_git::get_untracked_files(project_root).map_err(git_error)?);
    }
    files.extend(
        codelet_git::get_staged_files_with_change_type(project_root)
            .map_err(git_error)?
            .into_iter()
            .filter(|file| file.change_type == codelet_git::ChangeType::Added)
            .map(|file| file.path),
    );
    Ok(files.into_iter().collect())
}

/// A candidate block: its file and (first, last) @step line.
type Candidate = (String, (usize, usize));

/// The exact blocks of `steps` in each of `files` (those that exist), as
/// `(file, (first, last))`.
fn search(
    project_root: &Path,
    source: &Source,
    steps: &[String],
    files: &[&str],
) -> Result<Vec<Candidate>, FspecCoreError> {
    let mut candidates = Vec::new();
    for file in files {
        let Some(bytes) = source.read_bytes(project_root, file)? else {
            continue;
        };
        let text = String::from_utf8_lossy(&bytes);
        candidates.extend(
            exact_blocks(steps, &text)
                .into_iter()
                .map(|range| (file.to_string(), range)),
        );
    }
    Ok(candidates)
}

/// The block search for `scenario` of feature `name`.
pub(crate) fn relink(
    project_root: &Path,
    name: &str,
    feature: &gherkin::Feature,
    scenario: &CoverageScenario,
    sidecars: &Sidecars,
) -> Result<Relink, FspecCoreError> {
    let Some(steps) = parsed_scenario_steps(feature, &scenario.name) else {
        return Ok(Relink::NotOnce(
            "the scenario is not in the feature file".to_string(),
        ));
    };
    let mut mapped: Vec<&str> = Vec::new();
    for mapping in &scenario.test_mappings {
        if !mapped.contains(&mapping.file.as_str()) {
            mapped.push(mapping.file.as_str());
        }
    }
    let source = &sidecars.source;
    let mut candidates = search(project_root, source, &steps, &mapped)?;
    if candidates.is_empty() {
        let added: Vec<&str> = sidecars
            .added
            .get(project_root, source)?
            .iter()
            .map(String::as_str)
            .filter(|file| !mapped.contains(file))
            .filter(|file| sidecars.other_features(file, name).is_empty())
            .collect();
        candidates = search(project_root, source, &steps, &added)?;
    }
    let [(new_file, (first, last))] = candidates.as_slice() else {
        return Ok(Relink::NotOnce(block_count_fact(&candidates)));
    };
    let new_lines = format!("{first}-{last}");
    // The mappings replaced: those of the file holding the block, else (a
    // move) those of the scenario's only mapped file.
    let old_file = if mapped.contains(&new_file.as_str()) {
        new_file.as_str()
    } else if let [only] = mapped.as_slice() {
        only
    } else {
        return Ok(Relink::NotOnce(format!(
            "{}, but the scenario maps {} test files, so the mapping to move is not one file",
            block_count_fact(&candidates),
            mapped.len()
        )));
    };
    let old_lines: Vec<String> = scenario
        .test_mappings
        .iter()
        .filter(|mapping| mapping.file == old_file)
        .map(|mapping| mapping.lines.clone())
        .collect();
    if old_file == new_file && old_lines == [new_lines.as_str()] {
        return Ok(Relink::Unchanged);
    }
    Ok(Relink::Once(Repair::Test {
        scenario: scenario.name.clone(),
        old_file: old_file.to_string(),
        old_lines,
        new_file: new_file.clone(),
        new_lines,
    }))
}
