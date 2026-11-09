//! Audit sessions (AUDIT-004): `fspec audit --start`, `--status`,
//! `--continue`, `--reset` and `--complete` walk the problems that need
//! judgment step by step, the way `fspec reverse` walks its gaps.
//!
//! Feature: spec/features/audit-sessions-walk-the-judgment-problems-step-by-step.feature
//!
//! `--start` audits the scope (`--feature`, `--work-unit`, `--area`) and
//! creates a session whose steps are its guided problems in report order;
//! an explicit `--area meaning` adds one review step per live scenario and an
//! explicit `--area spec` one stale-text review step per feature. Each step
//! prints exact facts and the fspec commands to use; fspec decides nothing
//! that needs judgment and runs no tests. `--continue` on a problem step
//! re-runs the audit of that step's feature and area and stays while a
//! problem with the same area, code, feature, scenario and location remains;
//! on a review step it records when the step was attested. `--complete`
//! deletes the session only when every step was continued past.
//!
//! The session file lives in the temp directory, keyed by the project root
//! and the scope ([`crate::types::guided_session`]), so agents auditing
//! different scopes in one tree never share a session.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::impl_ranges::compact;
use super::{detect_into, feature_name, parse_areas, Area, AuditArgs, AuditedFeature, FixKind};
use super::{Problem, COMMAND};
use crate::error::FspecCoreError;
use crate::io::time::iso8601_now;
use crate::types::coverage::{ImplLines, TestMapping};
use crate::types::guided_session;

/// The review area that adds one step per live scenario.
pub(super) const MEANING: &str = "meaning";

/// The session command `fspec audit` was given.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Action {
    Start,
    Status,
    Continue,
    Reset,
    Complete,
}

impl Action {
    /// The one session flag of `args`, if any; more than one is an error.
    pub(super) fn of(args: &AuditArgs) -> Result<Option<Action>, FspecCoreError> {
        let flags = [
            (args.start, Action::Start),
            (args.status, Action::Status),
            (args.r#continue, Action::Continue),
            (args.reset, Action::Reset),
            (args.complete, Action::Complete),
        ];
        let set: Vec<Action> = flags
            .into_iter()
            .filter(|(on, _)| *on)
            .map(|(_, action)| action)
            .collect();
        match set.as_slice() {
            [] => Ok(None),
            [action] => Ok(Some(*action)),
            _ => Err(invalid(
                "use only one of --start, --status, --continue, --reset and --complete".to_string(),
            )),
        }
    }
}

fn invalid(reason: String) -> FspecCoreError {
    FspecCoreError::InvalidArgs {
        command: COMMAND,
        reason,
    }
}

/// A session's scope: the `--area`, `--feature` and `--work-unit` set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Scope {
    areas: Vec<String>,
    features: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    work_unit_id: Option<String>,
}

impl Scope {
    fn of(args: &AuditArgs) -> Self {
        let mut areas = args.area.clone();
        areas.sort();
        areas.dedup();
        let mut features: Vec<String> =
            args.feature.iter().map(|name| feature_name(name)).collect();
        features.sort();
        features.dedup();
        Scope {
            areas,
            features,
            work_unit_id: args.work_unit_id.clone(),
        }
    }

    /// The session key's scope parts.
    fn key(&self) -> Vec<String> {
        self.areas
            .iter()
            .map(|area| format!("area={area}"))
            .chain(self.features.iter().map(|name| format!("feature={name}")))
            .chain(self.work_unit_id.iter().map(|id| format!("work-unit={id}")))
            .collect()
    }

    /// The scope's flags, as `--status`, `--continue` and the rest take them.
    fn flags(&self) -> String {
        let flags: Vec<String> = self
            .areas
            .iter()
            .map(|area| format!("--area {area}"))
            .chain(self.features.iter().map(|name| format!("--feature {name}")))
            .chain(
                self.work_unit_id
                    .iter()
                    .map(|id| format!("--work-unit {id}")),
            )
            .collect();
        flags.join(" ")
    }

    fn describe(&self) -> String {
        if self.key().is_empty() {
            "the whole project".to_string()
        } else {
            self.flags()
        }
    }

    /// `fspec audit --<flag>` with the scope's flags.
    fn command(&self, flag: &str) -> String {
        let mut command = String::from("fspec audit");
        command.push_str(" --");
        command.push_str(flag);
        let flags = self.flags();
        if !flags.is_empty() {
            command.push(' ');
            command.push_str(&flags);
        }
        command
    }
}

/// What a step asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum StepKind {
    /// A guided problem: done when the audit no longer reports it.
    Problem,
    /// A meaning review of one scenario: done when attested.
    Meaning,
    /// A stale-text review of one feature: done when attested.
    StaleText,
}

/// One step of an audit session.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Step {
    kind: StepKind,
    /// `<area> <code> <feature> ["<scenario>"] [<location>]` for a problem,
    /// `meaning <feature> "<scenario>"` or `stale-text <feature>` for a
    /// review.
    heading: String,
    feature: String,
    /// A problem step's identity, with `feature`, `scenario` and `location`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    area: Option<Area>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    scenario: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    location: Option<String>,
    /// The exact facts and commands the step prints.
    details: Vec<String>,
    /// When a review step was attested by `--continue`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    attested_at: Option<String>,
}

impl Step {
    fn of_problem(problem: &Problem) -> Self {
        let repair = problem.fix.step.clone().unwrap_or_default();
        Step {
            kind: StepKind::Problem,
            heading: problem.heading(),
            feature: problem.feature.clone(),
            area: Some(problem.area),
            code: Some(problem.code.to_string()),
            scenario: problem.scenario.clone(),
            location: problem.location.clone(),
            details: vec![
                format!("Fact: {}", problem.fact),
                format!("Repair with: {repair}"),
                "Decide the values the command needs from the facts above and the files they name; --continue checks this problem again and advances only when the audit no longer reports it.".to_string(),
            ],
            attested_at: None,
        }
    }

    fn review(kind: StepKind, heading: String, feature: &str, details: Vec<String>) -> Self {
        Step {
            kind,
            heading,
            feature: feature.to_string(),
            area: None,
            code: None,
            scenario: None,
            location: None,
            details,
            attested_at: None,
        }
    }

    /// Whether `problem` is this problem step's problem.
    fn is(&self, problem: &Problem) -> bool {
        self.area == Some(problem.area)
            && self.code.as_deref() == Some(problem.code)
            && self.feature == problem.feature
            && self.scenario == problem.scenario
            && self.location == problem.location
    }
}

/// The session file's content.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuditSession {
    scope: Scope,
    created_at: String,
    current_step: u64,
    total_steps: u64,
    steps: Vec<Step>,
}

impl AuditSession {
    fn every_step_done(&self) -> bool {
        guided_session::every_step_done(self.current_step, self.total_steps)
    }

    fn current(&self) -> Option<&Step> {
        let index = usize::try_from(self.current_step.checked_sub(1)?).ok()?;
        self.steps.get(index)
    }

    /// `Step N of M: <heading>`, the step's details and the next command.
    fn render_current(&self) -> String {
        let Some(step) = self.current() else {
            return format!("Every step is done: run {}", self.scope.command("complete"));
        };
        let mut lines = vec![format!(
            "Step {} of {}: {}",
            self.current_step, self.total_steps, step.heading
        )];
        lines.extend(step.details.iter().map(|detail| format!("  {detail}")));
        lines.push(String::new());
        lines.push(format!(
            "When this step is done, run: {}",
            self.scope.command("continue")
        ));
        lines.join("\n")
    }
}

/// The result envelope of a session command.
fn envelope(output: String, exit_code: u8) -> Result<String, FspecCoreError> {
    serde_json::to_string(&json!({"output": output, "exitCode": exit_code}))
        .map_err(|error| invalid(format!("failed to serialize result: {error}")))
}

/// Run session command `action` for `args`' scope.
pub(super) fn run(
    project_root: &Path,
    args: &AuditArgs,
    action: Action,
) -> Result<String, FspecCoreError> {
    if args.fix || args.json || args.changed {
        return Err(invalid(
            "--start, --status, --continue, --reset and --complete cannot be combined with --fix, --json or --changed".to_string(),
        ));
    }
    let scope = Scope::of(args);
    // The scope's areas are checked by every session command alike.
    detection_areas(&scope)?;
    let path = guided_session::session_path(COMMAND, project_root, &scope.key());
    let loaded: Option<AuditSession> = guided_session::load(&path);
    let missing = || {
        invalid(format!(
            "No audit session for {}: start one with {}",
            scope.describe(),
            scope.command("start")
        ))
    };
    let save = |session: &AuditSession| {
        guided_session::save(&path, session).map_err(|source| FspecCoreError::Io {
            command: COMMAND,
            source,
        })
    };
    match action {
        Action::Start => {
            if path.exists() {
                return Err(invalid(format!(
                    "An audit session for {} already exists: continue it with {} or delete it with {}",
                    scope.describe(),
                    scope.command("continue"),
                    scope.command("reset")
                )));
            }
            let (steps, others) = start_steps(project_root, args, &scope)?;
            let other_note = if others == 0 {
                String::new()
            } else {
                format!(
                    "\n{others} other problem(s) need no judgment and are not steps: {}",
                    audit_command(&scope)
                )
            };
            if steps.is_empty() {
                return envelope(
                    format!(
                        "There is nothing to walk: no problem in {} needs judgment and no review area was asked for, so no session was created{other_note}",
                        scope.describe()
                    ),
                    0,
                );
            }
            let session = AuditSession {
                scope: scope.clone(),
                created_at: iso8601_now(),
                current_step: 1,
                total_steps: steps.len() as u64,
                steps,
            };
            save(&session)?;
            envelope(
                format!(
                    "Audit session started for {}: {} step(s){other_note}\n\n{}",
                    scope.describe(),
                    session.total_steps,
                    session.render_current()
                ),
                0,
            )
        }
        Action::Status => {
            let Some(session) = loaded else {
                return envelope(format!("No audit session for {}", scope.describe()), 0);
            };
            envelope(render_status(&session), 0)
        }
        Action::Continue => {
            let mut session = loaded.ok_or_else(missing)?;
            if session.every_step_done() {
                return Err(invalid(format!(
                    "Every step is done: run {}",
                    scope.command("complete")
                )));
            }
            let index = session.current_step.saturating_sub(1) as usize;
            let step = session.steps.get(index).cloned().ok_or_else(|| {
                invalid(format!("the session file {} is damaged", path.display()))
            })?;
            if step.kind == StepKind::Problem {
                if let Some(problem) = still_reported(project_root, &step)? {
                    return envelope(
                        format!(
                            "The problem persists: {}\n\n{}",
                            problem.line(),
                            session.render_current()
                        ),
                        1,
                    );
                }
            } else if let Some(current) = session.steps.get_mut(index) {
                current.attested_at = Some(iso8601_now());
            }
            let done = session.current_step;
            session.current_step = guided_session::next_step(Some(done));
            save(&session)?;
            envelope(
                format!(
                    "Step {done} of {} done: {}\n\n{}",
                    session.total_steps,
                    step.heading,
                    session.render_current()
                ),
                0,
            )
        }
        Action::Reset => {
            let existed = loaded.is_some() || path.exists();
            guided_session::delete(&path);
            let output = if existed {
                format!("Audit session for {} reset", scope.describe())
            } else {
                format!("No audit session for {}", scope.describe())
            };
            envelope(output, 0)
        }
        Action::Complete => {
            let session = loaded.ok_or_else(missing)?;
            if !session.every_step_done() {
                return Err(invalid(format!(
                    "Cannot complete: not every step is done (step {} of {}); run {}",
                    session.current_step,
                    session.total_steps,
                    scope.command("continue")
                )));
            }
            guided_session::delete(&path);
            envelope(
                format!(
                    "Audit session for {} complete: {} step(s) done",
                    scope.describe(),
                    session.total_steps
                ),
                0,
            )
        }
    }
}

/// `fspec audit` over the scope's detection areas (without the review area).
fn audit_command(scope: &Scope) -> String {
    let flags: Vec<String> = scope
        .areas
        .iter()
        .filter(|area| area.as_str() != MEANING)
        .map(|area| format!("--area {area}"))
        .chain(
            scope
                .features
                .iter()
                .map(|name| format!("--feature {name}")),
        )
        .chain(
            scope
                .work_unit_id
                .iter()
                .map(|id| format!("--work-unit {id}")),
        )
        .collect();
    if flags.is_empty() {
        "fspec audit".to_string()
    } else {
        format!("fspec audit {}", flags.join(" "))
    }
}

/// The detection areas of the scope: every area when none is named, none
/// when only the meaning area is named.
fn detection_areas(scope: &Scope) -> Result<Vec<Area>, FspecCoreError> {
    let names: Vec<String> = scope
        .areas
        .iter()
        .filter(|area| area.as_str() != MEANING)
        .cloned()
        .collect();
    if names.is_empty() && !scope.areas.is_empty() {
        return Ok(Vec::new());
    }
    parse_areas(&names)
}

/// The steps `--start` creates, and the number of the scope's problems that
/// need no judgment.
fn start_steps(
    project_root: &Path,
    args: &AuditArgs,
    scope: &Scope,
) -> Result<(Vec<Step>, usize), FspecCoreError> {
    let meaning = scope.areas.iter().any(|area| area == MEANING);
    let stale_text = scope.areas.iter().any(|area| area == Area::Spec.name());
    let test_command = if meaning {
        Some(test_command(project_root)?)
    } else {
        None
    };
    let areas = detection_areas(scope)?;
    let detect_args = AuditArgs {
        feature: args.feature.clone(),
        work_unit_id: args.work_unit_id.clone(),
        ..AuditArgs::default()
    };
    let mut audited = Vec::new();
    let detected = detect_into(project_root, &detect_args, &areas, true, Some(&mut audited))?;
    let (guided, others): (Vec<&Problem>, Vec<&Problem>) = detected
        .problems
        .iter()
        .partition(|problem| problem.fix.kind == FixKind::Guided);
    let mut steps: Vec<Step> = guided.into_iter().map(Step::of_problem).collect();
    if let Some(command) = &test_command {
        for feature in &audited {
            steps.extend(meaning_steps(feature, command));
        }
    }
    if stale_text {
        steps.extend(audited.iter().map(stale_text_step));
    }
    Ok((steps, others.len()))
}

/// The configured `tools.test.command`; the meaning area needs it.
fn test_command(project_root: &Path) -> Result<String, FspecCoreError> {
    let path = crate::io::spec_paths::config(project_root)?;
    let not_configured = || {
        invalid(
            "the meaning area needs the project's test command and none is configured: run fspec configure-tools --test-command <command>".to_string(),
        )
    };
    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Err(not_configured()),
        Err(error) => return Err(invalid(format!("cannot read {}: {error}", path.display()))),
    };
    let config: Value = serde_json::from_str(&raw)
        .map_err(|error| invalid(format!("cannot parse {}: {error}", path.display())))?;
    config
        .pointer("/tools/test/command")
        .and_then(Value::as_str)
        .filter(|command| !command.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(not_configured)
}

/// `file:lines` of an implementation mapping.
fn impl_location(file: &str, lines: &ImplLines) -> String {
    match lines {
        ImplLines::Array(lines) => format!("{file}:{}", compact(lines)),
        ImplLines::String(lines) => format!("{file}:{lines}"),
    }
}

/// One meaning review step per live scenario of `feature`.
fn meaning_steps(feature: &AuditedFeature, test_command: &str) -> Vec<Step> {
    let name = &feature.name;
    feature
        .live_scenarios
        .iter()
        .map(|scenario| {
            let mappings: Vec<&TestMapping> = feature
                .coverage
                .iter()
                .flat_map(|coverage| &coverage.scenarios)
                .filter(|entry| &entry.name == scenario)
                .flat_map(|entry| &entry.test_mappings)
                .collect();
            let tests: Vec<String> = mappings
                .iter()
                .map(|mapping| format!("{}:{}", mapping.file, mapping.lines))
                .collect();
            let implementations: Vec<String> = mappings
                .iter()
                .flat_map(|mapping| &mapping.impl_mappings)
                .map(|implementation| impl_location(&implementation.file, &implementation.lines))
                .collect();
            let listed = |items: &[String]| {
                if items.is_empty() {
                    "none mapped".to_string()
                } else {
                    items.join(", ")
                }
            };
            let details = vec![
                format!("Test mappings: {}", listed(&tests)),
                format!("Implementation mappings: {}", listed(&implementations)),
                format!("Test command (tools.test.command): {test_command}"),
                "Check that the scenario's test needs its implementation (fspec runs no tests):".to_string(),
                "  1. git worktree add --detach <scratch-dir>".to_string(),
                "  2. Bring the uncommitted changes over: git diff HEAD --binary | git -C <scratch-dir> apply, and copy in each untracked file listed above".to_string(),
                "  3. In <scratch-dir>, remove or stub the implementation lines listed above".to_string(),
                format!("  4. In <scratch-dir>, run the scenario's test with: {test_command}"),
                "  5. Confirm the test fails because the behaviour is missing, not because the build broke".to_string(),
                "  6. git worktree remove --force <scratch-dir>".to_string(),
                format!(
                    "If the test still passes, or fails only because the build broke, fix the test, or the mappings with fspec link-coverage {name} --scenario \"{scenario}\" --test-file <file> --test-lines <lines> --replace, then check again."
                ),
            ];
            Step::review(
                StepKind::Meaning,
                format!("meaning {name} \"{scenario}\""),
                name,
                details,
            )
        })
        .collect()
}

/// The stale-text review step of `feature`.
fn stale_text_step(feature: &AuditedFeature) -> Step {
    let name = &feature.name;
    let mut test_files: Vec<&str> = Vec::new();
    let mut impl_files: Vec<&str> = Vec::new();
    for mapping in feature
        .coverage
        .iter()
        .flat_map(|coverage| &coverage.scenarios)
        .flat_map(|scenario| &scenario.test_mappings)
    {
        if !test_files.contains(&mapping.file.as_str()) {
            test_files.push(&mapping.file);
        }
        for implementation in &mapping.impl_mappings {
            if !impl_files.contains(&implementation.file.as_str()) {
                impl_files.push(&implementation.file);
            }
        }
    }
    let listed = |files: &[&str]| {
        if files.is_empty() {
            "none mapped".to_string()
        } else {
            files.join(", ")
        }
    };
    let details = vec![
        format!("Feature file: spec/features/{name}.feature"),
        format!("Implementation files: {}", listed(&impl_files)),
        format!("Test files: {}", listed(&test_files)),
        "Read the feature's description, rules and scenarios next to these files. Where the text no longer says what the code does, change it with fspec commands:".to_string(),
        format!("  fspec update-feature-text {name} --find \"<old text>\" --replace \"<new text>\""),
        format!("  fspec update-scenario {name} \"<old name>\" \"<new name>\""),
        format!("  fspec update-step {name} \"<scenario>\" \"<current step>\" --text \"<new text>\""),
    ];
    Step::review(
        StepKind::StaleText,
        format!("stale-text {name}"),
        name,
        details,
    )
}

/// The problem a problem step names, if the audit of its feature and area
/// still reports it.
fn still_reported(project_root: &Path, step: &Step) -> Result<Option<Problem>, FspecCoreError> {
    let Some(area) = step.area else {
        return Ok(None);
    };
    let args = AuditArgs {
        feature: vec![step.feature.clone()],
        ..AuditArgs::default()
    };
    let detected = detect_into(project_root, &args, &[area], false, None)?;
    Ok(detected
        .problems
        .into_iter()
        .find(|problem| step.is(problem)))
}

/// `--status`: the scope, every step with its state, and the current step.
fn render_status(session: &AuditSession) -> String {
    let progress = if session.every_step_done() {
        format!(
            "every step is done; run {}",
            session.scope.command("complete")
        )
    } else {
        format!("step {} of {}", session.current_step, session.total_steps)
    };
    let mut lines = vec![format!(
        "Audit session for {} (started {}): {progress}",
        session.scope.describe(),
        session.created_at
    )];
    for (index, step) in session.steps.iter().enumerate() {
        let number = index as u64 + 1;
        let state = match (&step.attested_at, number.cmp(&session.current_step)) {
            (Some(at), _) => format!(" (attested {at})"),
            (None, std::cmp::Ordering::Less) => " (resolved)".to_string(),
            (None, std::cmp::Ordering::Equal) => " (current)".to_string(),
            (None, std::cmp::Ordering::Greater) => String::new(),
        };
        lines.push(format!("  {number}. {}{state}", step.heading));
    }
    if !session.every_step_done() {
        lines.push(String::new());
        lines.push(session.render_current());
    }
    lines.join("\n")
}
