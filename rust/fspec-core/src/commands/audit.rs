//! `audit` — the one deterministic drift check (AUDIT-003).
//!
//! Feature: spec/features/fspec-audit-is-the-one-deterministic-drift-check.feature
//!
//! Reports only exact facts, over five areas:
//!
//! - **gherkin**: the feature parses, plus validate's content checks (the
//!   same [`crate::commands::validate::check_content`] `fspec validate` runs);
//! - **tags**: registered and correctly placed tags (the same
//!   [`crate::commands::validate_tags::content_errors`] `fspec validate-tags`
//!   runs);
//! - **format**: the feature file equals the formatter's output byte for byte;
//! - **coverage**: missing test or implementation files, unreadable or
//!   out-of-file ranges (BUG-501/515/525), steps without an exactly matching
//!   @step comment in the mapped lines (BUG-375/509), test files another
//!   feature maps (BUG-357), uncovered non-`@deprecated` scenarios, and the
//!   coverage file out of sync with the feature (BUG-485/557);
//! - **spec**: features without scenarios, coverage files without their
//!   feature, and features whose every mapped test file is missing.
//!
//! Every coverage sidecar is read once per run; the shared-test-file check
//! uses that index. Each problem is one line,
//! `<area> <code> <feature> ["<scenario>"] [<location>]: <fact> -> <command>`,
//! and `--json` prints `{problems, exitCode}` with the same problems.
//!
//! AUDIT-004: a problem whose repair needs judgment has fix kind `guided`
//! and names `fspec audit --start --feature <f>`; the session commands
//! (`--start`, `--status`, `--continue`, `--reset`, `--complete`) are in
//! [`session`].
//!
//! Both front doors (the dispatcher and `fspec audit`) call [`run`], which
//! returns `{output, problems, exitCode}`: exit 0 means no problems, 1
//! problems; an `Err` is exit 2.

mod carry;
mod coverage;
pub(crate) mod drift;
mod fix;
mod history;
pub(crate) mod impl_ranges;
mod relink;
mod session;
pub(crate) mod source;
mod stats;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::types::coverage::CoverageFile;

const COMMAND: &str = "audit";

/// Arguments of `fspec audit [--area <name>]... [--feature <name>]...
/// [--work-unit <workUnitId>] [--json]`.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
struct AuditArgs {
    area: Vec<String>,
    feature: Vec<String>,
    work_unit_id: Option<String>,
    json: bool,
    /// COV-079: apply the exact mechanical repairs, then report what is left.
    fix: bool,
    /// COV-077: audit the staged changes, reading the git index.
    changed: bool,
    /// AUDIT-004: the audit session commands.
    start: bool,
    status: bool,
    #[serde(rename = "continue")]
    r#continue: bool,
    reset: bool,
    complete: bool,
}

/// The areas `fspec audit` checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Area {
    Gherkin,
    Tags,
    Format,
    Coverage,
    Spec,
}

impl Area {
    const ALL: [Area; 5] = [
        Area::Gherkin,
        Area::Tags,
        Area::Format,
        Area::Coverage,
        Area::Spec,
    ];

    pub(crate) fn name(self) -> &'static str {
        match self {
            Area::Gherkin => "gherkin",
            Area::Tags => "tags",
            Area::Format => "format",
            Area::Coverage => "coverage",
            Area::Spec => "spec",
        }
    }
}

/// How a problem is repaired: `fspec audit --fix` (an exact mechanical
/// repair), another concrete command, or (AUDIT-004) a step of an audit
/// session, for a repair that needs judgment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum FixKind {
    Mechanical,
    Command,
    Guided,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Fix {
    pub(crate) kind: FixKind,
    pub(crate) command: String,
    /// AUDIT-004: a guided problem's repair command, which the session step
    /// names (it holds a `<placeholder>`, or the choice between candidates
    /// needs judgment).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) step: Option<String>,
}

/// One exact problem.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Problem {
    pub(crate) area: Area,
    pub(crate) code: &'static str,
    pub(crate) feature: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) scenario: Option<String>,
    /// `file:lines` (or a comma-separated list of them), or a file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) lines: Option<String>,
    pub(crate) fact: String,
    pub(crate) fix: Fix,
    /// The exact repair `--fix` applies (mechanical problems only).
    #[serde(skip)]
    pub(crate) repair: Option<fix::Repair>,
}

impl Problem {
    pub(crate) fn new(area: Area, code: &'static str, feature: &str, fact: String) -> Self {
        Problem {
            area,
            code,
            feature: feature.to_string(),
            scenario: None,
            location: None,
            file: None,
            lines: None,
            fact,
            fix: Fix {
                kind: FixKind::Command,
                command: String::new(),
                step: None,
            },
            repair: None,
        }
    }

    /// A problem `--fix` repairs with `repair`.
    pub(crate) fn mechanical(mut self, repair: fix::Repair) -> Self {
        self.fix = Fix {
            kind: FixKind::Mechanical,
            command: format!("fspec audit --fix --feature {}", self.feature),
            step: None,
        };
        self.repair = Some(repair);
        self
    }

    /// AUDIT-004: a problem whose repair needs judgment: its command becomes
    /// the session step's command and the problem names the audit session.
    pub(crate) fn guided(mut self) -> Self {
        if self.fix.kind == FixKind::Guided {
            return self;
        }
        let step = std::mem::take(&mut self.fix.command);
        self.fix = Fix {
            kind: FixKind::Guided,
            command: format!("fspec audit --start --feature {}", self.feature),
            step: Some(step),
        };
        self.repair = None;
        self
    }

    pub(crate) fn scenario(mut self, scenario: &str) -> Self {
        self.scenario = Some(scenario.to_string());
        self
    }

    /// The location: one `file:lines` (or a bare file) also fills `file`
    /// and `lines`; a list of them stays only in `location`.
    pub(crate) fn at(mut self, location: &str) -> Self {
        if !location.contains(", ") {
            match location.rsplit_once(':') {
                Some((file, lines))
                    if !lines.is_empty()
                        && lines
                            .chars()
                            .all(|c| c.is_ascii_digit() || c == '-' || c == ',') =>
                {
                    self.file = Some(file.to_string());
                    self.lines = Some(lines.to_string());
                }
                _ => self.file = Some(location.to_string()),
            }
        }
        self.location = Some(location.to_string());
        self
    }

    pub(crate) fn fix(mut self, command: String) -> Self {
        self.fix.command = command;
        self
    }

    /// `<area> <code> <feature> ["<scenario>"] [<location>]`.
    pub(crate) fn heading(&self) -> String {
        let mut heading = format!("{} {} {}", self.area.name(), self.code, self.feature);
        if let Some(scenario) = &self.scenario {
            heading.push_str(&format!(" \"{scenario}\""));
        }
        if let Some(location) = &self.location {
            heading.push_str(&format!(" {location}"));
        }
        heading
    }

    /// `<area> <code> <feature> ["<scenario>"] [<location>]: <fact> -> <command>`.
    pub(crate) fn line(&self) -> String {
        format!("{}: {} -> {}", self.heading(), self.fact, self.fix.command)
    }
}

/// The `--json` report.
#[derive(Serialize)]
struct Report<'a> {
    /// The repairs `--fix` made.
    #[serde(skip_serializing_if = "<[String]>::is_empty")]
    fixed: &'a [String],
    problems: &'a [Problem],
    #[serde(rename = "exitCode")]
    exit_code: u8,
}

/// Every coverage sidecar of the project, read once: its text by feature
/// name, and the features mapping each test and implementation file.
/// COV-077: they, and every other file of the run, are read from `source`.
pub(crate) struct Sidecars {
    raw: BTreeMap<String, String>,
    test_file_features: BTreeMap<String, BTreeSet<String>>,
    impl_file_features: BTreeMap<String, BTreeSet<String>>,
    pub(crate) source: source::Source,
    /// COV-078: the files the working tree adds, listed when first needed.
    added: relink::AddedFiles,
    /// BUG-591: the git answers that show a legacy mapping did not move.
    history: history::History,
}

impl Sidecars {
    fn load(project_root: &Path, source: source::Source) -> Result<Self, FspecCoreError> {
        let mut raw = BTreeMap::new();
        let mut test_file_features: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut impl_file_features: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for path in source.spec_files(project_root, ".feature.coverage")? {
            let name = feature_name(path.trim_end_matches(".coverage"));
            let Some(text) = source.read(project_root, &path)? else {
                continue;
            };
            // A sidecar that does not parse is reported by the coverage area.
            if let Ok(coverage) = serde_json::from_str::<Value>(&text) {
                let mappings = coverage
                    .get("scenarios")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(|scenario| scenario.get("testMappings").and_then(Value::as_array))
                    .flatten();
                for mapping in mappings {
                    if let Some(file) = mapping.get("file").and_then(Value::as_str) {
                        test_file_features
                            .entry(file.to_string())
                            .or_default()
                            .insert(name.clone());
                    }
                    let impl_files = mapping
                        .get("implMappings")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                        .filter_map(|implementation| implementation.get("file"))
                        .filter_map(Value::as_str);
                    for file in impl_files {
                        impl_file_features
                            .entry(file.to_string())
                            .or_default()
                            .insert(name.clone());
                    }
                }
            }
            raw.insert(name, text);
        }
        Ok(Sidecars {
            raw,
            test_file_features,
            impl_file_features,
            source,
            added: relink::AddedFiles::default(),
            history: history::History::default(),
        })
    }

    /// The features other than `feature` that map `test_file`.
    pub(crate) fn other_features(&self, test_file: &str, feature: &str) -> Vec<String> {
        self.test_file_features
            .get(test_file)
            .into_iter()
            .flatten()
            .filter(|name| name.as_str() != feature)
            .cloned()
            .collect()
    }
}

/// Dispatcher entry point; `project_root` is supplied by both front doors.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let invalid = |reason: String| FspecCoreError::InvalidArgs {
        command: COMMAND,
        reason,
    };
    let args: AuditArgs = serde_json::from_str(args_json)
        .map_err(|error| invalid(format!("failed to parse args: {error}")))?;
    // AUDIT-004: the audit session commands.
    if let Some(action) = session::Action::of(&args)? {
        return session::run(project_root, &args, action);
    }
    if args.area.iter().any(|area| area == session::MEANING) {
        return Err(invalid(
            "the meaning area is accepted only with --start and the session commands (--status, --continue, --reset, --complete); fspec audit --start --area meaning creates its review steps".to_string(),
        ));
    }
    let areas = parse_areas(&args.area)?;
    if args.changed && (!args.feature.is_empty() || args.work_unit_id.is_some()) {
        return Err(invalid(
            "--changed sets the scope to the staged changes and cannot be combined with --feature or --work-unit"
                .to_string(),
        ));
    }
    let mut detected = detect(project_root, &args, &areas, true)?;
    let mut fixed = Vec::new();
    if args.fix {
        fixed = fix::apply(project_root, &detected.problems)?;
        if !fixed.is_empty() {
            // COV-078: a named feature whose orphan coverage file --fix
            // deleted is gone from the scope.
            detected = detect(project_root, &args, &areas, false)?;
        }
    }
    let Detected {
        problems,
        features: feature_count,
    } = detected;

    let exit_code: u8 = if problems.is_empty() { 0 } else { 1 };
    let output = if args.json {
        serde_json::to_string(&Report {
            fixed: &fixed,
            problems: &problems,
            exit_code,
        })
        .map_err(|error| invalid(format!("failed to serialize report: {error}")))?
    } else {
        let report = render(&problems, feature_count);
        if fixed.is_empty() {
            report
        } else {
            format!(
                "{}

{report}",
                fixed.join("\n")
            )
        }
    };
    serde_json::to_string(&json!({
        "output": output,
        "fixed": fixed,
        "problems": problems,
        "exitCode": exit_code,
    }))
    .map_err(|error| invalid(format!("failed to serialize result: {error}")))
}

/// What [`detect`] found.
struct Detected {
    problems: Vec<Problem>,
    /// The number of features audited.
    features: usize,
}

/// AUDIT-005: one feature of a work unit as the workflow gates see it.
pub(crate) struct AuditedFeature {
    pub(crate) name: String,
    /// The feature's non-`@deprecated` scenarios, in feature order.
    pub(crate) live_scenarios: Vec<String>,
    /// The coverage file, when it exists and parses.
    pub(crate) coverage: Option<CoverageFile>,
}

/// AUDIT-005: the audit of one work unit's features (`fspec audit
/// --work-unit <workUnitId> --area <area>...`) for the workflow gates: its problems
/// and each feature that parses, with the coverage the audit read.
pub(crate) struct WorkUnitAudit {
    pub(crate) problems: Vec<Problem>,
    pub(crate) features: Vec<AuditedFeature>,
}

/// AUDIT-005: audit `areas` of work unit `id`'s features (its linked
/// features, else the features tagged with its ID), exactly as `fspec audit
/// --work-unit <workUnitId>` does.
pub(crate) fn work_unit_audit(
    project_root: &Path,
    id: &str,
    areas: &[Area],
) -> Result<WorkUnitAudit, FspecCoreError> {
    let args = AuditArgs {
        work_unit_id: Some(id.to_string()),
        ..AuditArgs::default()
    };
    let mut audited = Vec::new();
    let detected = detect_into(project_root, &args, areas, true, Some(&mut audited))?;
    Ok(WorkUnitAudit {
        problems: detected.problems,
        features: audited,
    })
}

/// Every problem of the scope, and the number of features audited.
/// With `strict`, a `--feature` name without a feature or coverage file is
/// an error; otherwise it is left out.
fn detect(
    project_root: &Path,
    args: &AuditArgs,
    areas: &[Area],
    strict: bool,
) -> Result<Detected, FspecCoreError> {
    detect_into(project_root, args, areas, strict, None)
}

/// [`detect`], also collecting each feature that parses into `audited`.
fn detect_into(
    project_root: &Path,
    args: &AuditArgs,
    areas: &[Area],
    strict: bool,
    mut audited: Option<&mut Vec<AuditedFeature>>,
) -> Result<Detected, FspecCoreError> {
    let source = if args.changed {
        source::Source::staged(project_root)?
    } else {
        source::Source::WorkTree
    };
    let features: BTreeSet<String> = source
        .spec_files(project_root, ".feature")?
        .iter()
        .map(|file| feature_name(file))
        .collect();
    let sidecars = Sidecars::load(project_root, source)?;
    let scope = scope(project_root, args, &features, &sidecars, strict)?;

    let registry = if areas.contains(&Area::Tags) {
        Some(tag_registry(project_root, &sidecars.source)?)
    } else {
        None
    };

    let mut problems: Vec<Problem> = Vec::new();
    for name in &scope {
        let feature_file = format!("spec/features/{name}.feature");
        let sidecar = sidecars.raw.get(name).map(String::as_str);
        if !features.contains(name) {
            if areas.contains(&Area::Spec) && sidecar.is_some() {
                let coverage_file = format!("{feature_file}.coverage");
                problems.push(
                    Problem::new(
                        Area::Spec,
                        "orphan-coverage",
                        name,
                        "the coverage file has no feature file".to_string(),
                    )
                    .at(&coverage_file)
                    .mechanical(fix::Repair::DeleteCoverage),
                );
            }
            continue;
        }
        let content = sidecars
            .source
            .read(project_root, &feature_file)?
            .ok_or_else(|| FspecCoreError::InvalidArgs {
                command: COMMAND,
                reason: format!("{feature_file} disappeared during the audit"),
            })?;
        let (parsed, errors) = crate::commands::validate::check_content(&content);
        if areas.contains(&Area::Gherkin) {
            for error in errors {
                problems.push(
                    Problem::new(Area::Gherkin, "invalid", name, error.message)
                        .at(&format!("{feature_file}:{}", error.line))
                        .fix(format!("fspec validate {feature_file}")),
                );
            }
        }
        if let Some((registry, work_units)) = &registry {
            let errors = crate::commands::validate_tags::content_errors(
                &feature_file,
                &content,
                registry,
                work_units.as_ref(),
            );
            for error in errors {
                problems.push(
                    Problem::new(Area::Tags, "invalid-tag", name, error.message)
                        .at(&feature_file)
                        .fix(format!("fspec validate-tags {feature_file}")),
                );
            }
        }
        let Some(feature) = parsed else {
            // The gherkin area reports why; AUDIT-005: the coverage area
            // names the fact that the coverage was not checked, so an audit
            // of coverage alone never passes a feature that does not parse.
            if areas.contains(&Area::Coverage) {
                problems.push(
                    Problem::new(
                        Area::Coverage,
                        "unparsed-feature",
                        name,
                        "the feature does not parse, so its coverage is not checked".to_string(),
                    )
                    .at(&feature_file)
                    .fix(format!("fspec validate {feature_file}")),
                );
            }
            continue;
        };
        if areas.contains(&Area::Format)
            && crate::io::gherkin_format::format_feature(&feature, &content) != content
        {
            problems.push(
                Problem::new(
                    Area::Format,
                    "needs-formatting",
                    name,
                    "the file differs from the formatter's output".to_string(),
                )
                .at(&feature_file)
                .fix(format!("fspec format {feature_file}")),
            );
        }
        if areas.contains(&Area::Spec) && feature.scenarios.is_empty() && feature.rules.is_empty() {
            problems.push(
                Problem::new(
                    Area::Spec,
                    "no-scenarios",
                    name,
                    "the feature has no scenarios".to_string(),
                )
                .fix(format!("fspec add-scenario {name} \"<scenario>\""))
                .guided(),
            );
        }
        let coverage = match sidecar.map(serde_json::from_str::<CoverageFile>) {
            Some(Ok(coverage)) => Some(coverage),
            _ => None,
        };
        if areas.contains(&Area::Coverage) {
            coverage::problems(
                &coverage::FeatureCoverage {
                    project_root,
                    name,
                    feature_file: &feature_file,
                    content: &content,
                    feature: &feature,
                    sidecar,
                    coverage: coverage.as_ref(),
                },
                &sidecars,
                &mut problems,
            )?;
        }
        if areas.contains(&Area::Spec) {
            if let Some(coverage) = &coverage {
                let test_files: BTreeSet<&str> = coverage
                    .scenarios
                    .iter()
                    .flat_map(|scenario| &scenario.test_mappings)
                    .map(|mapping| mapping.file.as_str())
                    .collect();
                if !test_files.is_empty()
                    && test_files
                        .iter()
                        .all(|file| !sidecars.source.exists(project_root, file))
                {
                    problems.push(
                        Problem::new(
                            Area::Spec,
                            "all-tests-missing",
                            name,
                            format!("all {} mapped test file(s) are missing", test_files.len()),
                        )
                        .fix(format!(
                            "fspec unlink-coverage {name} --scenario \"<scenario>\" --all"
                        ))
                        .guided(),
                    );
                }
            }
        }
        if let Some(audited) = audited.as_deref_mut() {
            let deprecated =
                crate::types::coverage::deprecated_scenario_names(&content).unwrap_or_default();
            audited.push(AuditedFeature {
                name: name.clone(),
                live_scenarios: feature
                    .scenarios
                    .iter()
                    .map(|scenario| scenario.name.clone())
                    .filter(|scenario| !deprecated.contains(scenario))
                    .collect(),
                coverage,
            });
        }
    }

    Ok(Detected {
        problems,
        features: scope.len(),
    })
}

/// `--area` names; none means every area. An unknown name is an error.
fn parse_areas(names: &[String]) -> Result<Vec<Area>, FspecCoreError> {
    if names.is_empty() {
        return Ok(Area::ALL.to_vec());
    }
    names
        .iter()
        .map(|name| {
            Area::ALL
                .into_iter()
                .find(|area| area.name() == name)
                .ok_or_else(|| FspecCoreError::InvalidArgs {
                    command: COMMAND,
                    reason: format!(
                        "unknown area '{name}'; use gherkin, tags, format, coverage or spec (or meaning with --start)"
                    ),
                })
        })
        .collect()
}

/// `spec/features/a/b.feature` (or `a/b.feature`, `a/b`) → `a/b`.
fn feature_name(file: &str) -> String {
    let name = file.strip_prefix("spec/features/").unwrap_or(file);
    name.strip_suffix(".feature").unwrap_or(name).to_string()
}

/// The feature names to audit: `--feature` names and `--work-unit`'s
/// features, else every feature and every coverage file.
fn scope(
    project_root: &Path,
    args: &AuditArgs,
    features: &BTreeSet<String>,
    sidecars: &Sidecars,
    strict: bool,
) -> Result<BTreeSet<String>, FspecCoreError> {
    if args.changed {
        return changed_scope(project_root, features, sidecars);
    }
    if args.feature.is_empty() && args.work_unit_id.is_none() {
        return Ok(features
            .iter()
            .cloned()
            .chain(sidecars.raw.keys().cloned())
            .collect());
    }
    let mut scope = BTreeSet::new();
    for name in args.feature.iter().map(|name| feature_name(name)) {
        if !features.contains(&name) && !sidecars.raw.contains_key(&name) {
            if !strict {
                continue;
            }
            return Err(FspecCoreError::InvalidArgs {
                command: COMMAND,
                reason: format!("no feature or coverage file named '{name}' in spec/features"),
            });
        }
        scope.insert(name);
    }
    if let Some(id) = &args.work_unit_id {
        scope.extend(work_unit_features(project_root, id, features)?);
    }
    Ok(scope)
}

/// COV-077: the features a commit touches: those whose feature or coverage
/// file is staged (added, modified or deleted), and every feature mapping a
/// staged test or implementation file.
fn changed_scope(
    project_root: &Path,
    features: &BTreeSet<String>,
    sidecars: &Sidecars,
) -> Result<BTreeSet<String>, FspecCoreError> {
    let staged = codelet_git::get_staged_files_with_change_type(project_root).map_err(|error| {
        FspecCoreError::InvalidArgs {
            command: COMMAND,
            reason: format!("cannot list the staged changes: {error}"),
        }
    })?;
    let mut scope = BTreeSet::new();
    for file in staged.iter().map(|file| file.path.as_str()) {
        if let Some(spec) = file.strip_prefix("spec/features/") {
            if let Some(name) = spec
                .strip_suffix(".feature")
                .or_else(|| spec.strip_suffix(".feature.coverage"))
            {
                if features.contains(name) || sidecars.raw.contains_key(name) {
                    scope.insert(name.to_string());
                }
            }
        }
        for mapping in [&sidecars.test_file_features, &sidecars.impl_file_features] {
            scope.extend(mapping.get(file).into_iter().flatten().cloned());
        }
    }
    Ok(scope)
}

/// The tag registry and work unit IDs `validate-tags` checks against, read
/// from `source` (COV-077: the staged spec/tags.json and work-units.json).
fn tag_registry(
    project_root: &Path,
    source: &source::Source,
) -> Result<
    (
        crate::commands::validate_tags::Registry,
        Option<std::collections::HashSet<String>>,
    ),
    FspecCoreError,
> {
    use crate::commands::validate_tags::{
        load_registry, load_work_units, registry_of, work_unit_ids,
    };
    if let source::Source::WorkTree = source {
        return Ok((load_registry(project_root)?, load_work_units(project_root)?));
    }
    let tags = match source.read(project_root, "spec/tags.json")? {
        Some(raw) => serde_json::from_str(&raw).map_err(|error| FspecCoreError::InvalidArgs {
            command: COMMAND,
            reason: format!("cannot parse the staged spec/tags.json: {error}"),
        })?,
        None => crate::types::tags::TagsData::initial(),
    };
    let work_units = source
        .read(project_root, "spec/work-units.json")?
        .and_then(|raw| work_unit_ids(&raw));
    Ok((registry_of(&tags), work_units))
}

/// A work unit's linked features, else the features carrying its tag at
/// feature level (and those that cannot be read or parsed).
fn work_unit_features(
    project_root: &Path,
    id: &str,
    features: &BTreeSet<String>,
) -> Result<Vec<String>, FspecCoreError> {
    let invalid = |reason: String| FspecCoreError::InvalidArgs {
        command: COMMAND,
        reason,
    };
    let path = crate::io::spec_paths::work_units(project_root)?;
    let raw = std::fs::read_to_string(&path)
        .map_err(|error| invalid(format!("cannot read {}: {error}", path.display())))?;
    let data: Value = serde_json::from_str(&raw)
        .map_err(|error| invalid(format!("cannot parse {}: {error}", path.display())))?;
    let work_unit = data
        .get("workUnits")
        .and_then(|units| units.get(id))
        .ok_or_else(|| invalid(format!("work unit '{id}' not found")))?;
    let linked: Vec<String> = work_unit
        .get("linkedFeatures")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(feature_name)
        .collect();
    if !linked.is_empty() {
        return Ok(linked);
    }
    // AUDIT-005 (BUG-360): a feature file that cannot be read as text or
    // parsed may carry the tag, so it stays in the scope, where the audit
    // reports it instead of passing it.
    let mut tagged = Vec::new();
    for name in features {
        let file = project_root.join(format!("spec/features/{name}.feature"));
        let tags = std::fs::read_to_string(file)
            .ok()
            .and_then(|content| crate::io::gherkin::parse_feature_lenient(&content).ok())
            .map(|feature| feature.tags);
        let in_scope =
            tags.is_none_or(|tags| tags.iter().any(|tag| tag.trim_start_matches('@') == id));
        if in_scope {
            tagged.push(name.clone());
        }
    }
    Ok(tagged)
}

/// The text report: one line per problem and a summary line.
fn render(problems: &[Problem], feature_count: usize) -> String {
    if problems.is_empty() {
        return format!("No problems found in {feature_count} feature(s)");
    }
    let affected: BTreeSet<&str> = problems.iter().map(|p| p.feature.as_str()).collect();
    let mut lines: Vec<String> = problems.iter().map(Problem::line).collect();
    lines.push(String::new());
    lines.push(format!(
        "{} problem(s) in {} of {feature_count} feature(s)",
        problems.len(),
        affected.len()
    ));
    lines.join("\n")
}
