//! `show-coverage` — Rust port of `src/commands/show-coverage.ts` (RPC-300).
//!
//! Loads a `.feature.coverage` JSON sidecar (per-feature mode) or every
//! sidecar under `spec/features/` (project-wide mode), enriches each entry
//! with missing-file warnings, synthesizes a `stats` block when the legacy
//! file omits it, and renders either markdown or 2-space JSON.
//!
//! Two-front-doors invariant (RPC-003 §7/§11): the LLM dispatcher AND the
//! standalone clap CLI both call this single `run` function — the CLI
//! bridge contains zero coverage logic.
//!
//! ## Behaviour parity with TypeScript (`src/commands/show-coverage.ts`)
//!
//! * Per-feature mode (positional `featureName` present):
//!   * Resolves `<root>/spec/features/<name>.feature.coverage`; tolerates
//!     a trailing `.feature` on the bare name.
//!   * Missing → `FspecCoreError::Io` whose `source` message starts with
//!     `Coverage file not found: <name>.feature.coverage` and ends with a
//!     create-feature suggestion line.
//!   * Invalid JSON → `FspecCoreError::InvalidArgs` whose reason begins
//!     `Invalid JSON in coverage file: <name>.feature.coverage` and ends
//!     with `Suggestion: Validate the JSON or recreate the file`.
//!   * Markdown sections: title, coverage line, `## Summary` (with line
//!     order `Total Scenarios, Covered, Uncovered, Test Files,
//!     Implementation Files, Test Lines, Implementation Lines, Total
//!     Lines`), optional `## Warnings`, `## Scenarios`, optional
//!     `## ⚠️  Coverage Gaps`.
//!   * JSON: top-level keys `fileName, scenarios, stats, warnings` in
//!     declaration order; each scenario has an appended `coverageStatus`.
//!
//! * Project-wide mode (no `featureName`):
//!   * Reads every `*.feature.coverage` under `spec/features/`.
//!   * Missing dir → `FspecCoreError::Io` with `Features directory not
//!     found: spec/features/` + suggestion.
//!   * Empty dir → `FspecCoreError::Io` with `No coverage files found in
//!     spec/features/` + suggestion.
//!   * Invalid JSON files are silently skipped.
//!   * Markdown: `# Project Coverage Report`, overall coverage,
//!     `## Project Summary`, `## Features Overview` (✅/⚠️/❌ by
//!     percentage band), `## Detailed Coverage by Feature`.
//!   * JSON: top-level `{aggregated: {totalFeatures, totalScenarios,
//!     coveredScenarios, coveragePercent}, features: [{fileName,
//!     coverage}, ...]}`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::FspecCoreError;
use crate::types::coverage::{
    calculate_stats, CoverageFile, CoverageScenario, CoverageStats, ImplLines,
};

mod mapping_json;
mod project;
mod single;
mod single_json;

use mapping_json::show_mapping_json;
use project::*;
use single::*;
use single_json::*;

/// CLI / dispatcher arguments accepted by `show-coverage`.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ShowCoverageArgs {
    /// Feature basename (e.g. `"user-login"`), with or without `.feature`.
    #[serde(default)]
    feature_name: Option<String>,
    /// `"markdown"` (default) or `"json"`. Accept `"text"` as an alias for
    /// `"markdown"` since the TS Commander.js shape used both at various
    /// points.
    #[serde(default)]
    format: Option<String>,
    /// Optional output file path (project-root-relative). When `Some`,
    /// the rendered content is also written to `<project_root>/<output>`
    /// after a successful render — mirrors the parity behaviour of
    /// `commands::show_feature::run`. The CLI bridge prints a confirmation
    /// line ("✓ Coverage report written to <path>") instead of stdout.
    #[serde(default)]
    output: Option<String>,
    /// REFAC-020: explicit machine readback of ordered mapping rows and
    /// canonical scenario hashes. Requires `featureName`; ignores `format`.
    #[serde(default)]
    mapping_json: bool,
}

/// Coverage status of a single scenario, per the TS `CoverageStatus` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CoverageStatusKind {
    FullyCovered,
    PartiallyCovered,
    Uncovered,
}

impl CoverageStatusKind {
    fn symbol(self) -> &'static str {
        match self {
            CoverageStatusKind::FullyCovered => "✅",
            CoverageStatusKind::PartiallyCovered => "⚠️",
            CoverageStatusKind::Uncovered => "❌",
        }
    }
    fn label(self) -> &'static str {
        match self {
            CoverageStatusKind::FullyCovered => "FULLY COVERED",
            CoverageStatusKind::PartiallyCovered => "PARTIALLY COVERED",
            CoverageStatusKind::Uncovered => "UNCOVERED",
        }
    }
    fn json_str(self) -> &'static str {
        match self {
            CoverageStatusKind::FullyCovered => "fully-covered",
            CoverageStatusKind::PartiallyCovered => "partially-covered",
            CoverageStatusKind::Uncovered => "uncovered",
        }
    }
}

fn coverage_status(scenario: &CoverageScenario) -> CoverageStatusKind {
    if scenario.test_mappings.is_empty() {
        return CoverageStatusKind::Uncovered;
    }
    let has_impl = scenario
        .test_mappings
        .iter()
        .any(|tm| !tm.impl_mappings.is_empty());
    if has_impl {
        CoverageStatusKind::FullyCovered
    } else {
        CoverageStatusKind::PartiallyCovered
    }
}

/// Dispatcher entry point. Two-front-doors invariant.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: ShowCoverageArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "show-coverage",
            reason: format!("failed to parse args: {e}"),
        })?;

    if args.mapping_json {
        return show_mapping_json(project_root, args.feature_name.as_deref());
    }

    let format = args.format.as_deref().unwrap_or("markdown");
    let format = match format {
        "json" => OutputFormat::Json,
        // TS defaults to markdown; "text" used as alias by some callers.
        _ => OutputFormat::Markdown,
    };

    let rendered = match args.feature_name.as_deref() {
        Some(name) => show_single_feature(project_root, name, format),
        None => show_project_wide(project_root, format),
    }?;

    // Optional writeback (parity with show-feature). On success only.
    if let Some(out_rel) = args.output.as_deref() {
        let out_abs = project_root.join(out_rel);
        if let Some(parent) = out_abs.parent() {
            std::fs::create_dir_all(parent).map_err(|source| FspecCoreError::Io {
                command: "show-coverage",
                source,
            })?;
        }
        std::fs::write(&out_abs, rendered.as_bytes()).map_err(|source| FspecCoreError::Io {
            command: "show-coverage",
            source,
        })?;
    }

    Ok(rendered)
}

#[derive(Debug, Clone, Copy)]
enum OutputFormat {
    Markdown,
    Json,
}

#[cfg(test)]
mod tests;
