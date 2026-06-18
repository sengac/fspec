//! System-reminder generation for `update-work-unit-status`.
//!
//! Faithful port of the reminder behaviour in
//! `src/utils/system-reminder.ts` + the inline reminders constructed in
//! `src/commands/update-work-unit-status.ts`. All reminders are gated by
//! `FSPEC_DISABLE_REMINDERS != "1"` (mirroring `isRemindersEnabled`).
//!
//! Templates are stored as raw-string constants using the `__ID__` token in
//! place of the TS `${workUnitId}` interpolation; [`fill`] substitutes the id.

use std::path::Path;

use serde_json::Value;

type Data = crate::types::work_unit::WorkUnitsData;

/// Mirrors `isRemindersEnabled()`.
pub(super) fn reminders_enabled() -> bool {
    std::env::var("FSPEC_DISABLE_REMINDERS").map(|v| v != "1").unwrap_or(true)
}

/// Mirrors `wrapInSystemReminder`.
pub(super) fn wrap(content: &str) -> String {
    format!("<system-reminder>\n{content}\n</system-reminder>")
}

/// Substitute the `__ID__` token with the work-unit id.
fn fill(template: &str, id: &str) -> String {
    template.replace("__ID__", id)
}

/// Mirrors `consolidateReminders`: strip individual wrapper tags, trim, drop
/// empties, join with a blank line, and re-wrap once.
pub(super) fn consolidate(reminders: &[String]) -> Option<String> {
    if reminders.is_empty() {
        return None;
    }
    let mut contents: Vec<String> = Vec::new();
    for r in reminders {
        let stripped = r
            .replace("<system-reminder>\n", "")
            .replace("<system-reminder>", "")
            .replace("</system-reminder>\n", "")
            .replace("</system-reminder>", "");
        let trimmed = stripped.trim().to_string();
        if !trimmed.is_empty() {
            contents.push(trimmed);
        }
    }
    if contents.is_empty() {
        return None;
    }
    Some(wrap(&contents.join("\n\n")))
}

// ── Static status templates ───────────────────────────────────────────────

const TESTING: &str = r#"Work unit __ID__ is now in TESTING status.

⚠️⚠️⚠️ CRITICAL: MANDATORY @step COMMENTS REQUIRED ⚠️⚠️⚠️

EVERY Gherkin step MUST have an @step comment in your test file.
ONE scenario = ONE test with ALL @step comments in THAT test.

Structure:
  - Place @step comment RIGHT BEFORE the code that executes each step
  - Use EXACT text from feature file
  - Include ALL steps: @step Given ... @step When ... @step Then ... @step And ...

Example (JavaScript):
  // @step Given I am on the login page
  page = render_login_page()

  // @step When I enter valid credentials
  submit_credentials()

  // @step Then I should see the dashboard
  assert dashboard_visible()

WITHOUT @step comments, you CANNOT progress to implementing!
Validation will BLOCK you with error showing missing steps.

---

Write FAILING tests BEFORE any implementation code:
  - Tests must fail (red phase) to prove they actually test something
  - Map tests to Gherkin scenarios in feature file
  - Add header comment: // Feature: spec/features/[name].feature

Language-specific comment syntax:
  * JavaScript/Java/C/C++/C#/Swift/Go/Rust: // @step Given I am logged in
  * Python/Ruby/Perl/Bash/R/PowerShell:     # @step When I enter valid credentials
  * SQL/Ada/Haskell/Lua/VHDL:               -- @step Then I should see the dashboard
  * PHP: // @step or # @step
  * MATLAB/ASP: % @step
  * Visual Basic: ' @step

Common commands for TESTING state:
  fspec link-coverage <feature> --scenario "..." --test-file <path> --test-lines <range>
  fspec show-coverage <feature>
  fspec show-feature <name>

For more: fspec link-coverage --help

Suggested next steps:
  1. Create test file: src/**/__tests__/*.test.ts
  2. Add feature file reference: // Feature: spec/features/[name].feature
  3. Write tests with @step comments for EACH Gherkin step
  4. Run tests and verify they fail (tests should FAIL)
  5. Link test coverage: fspec link-coverage <feature> --scenario "..." --test-file <path> --test-lines <range>
  6. Move to implementing: fspec update-work-unit-status __ID__ implementing

DO NOT write implementation code yet. DO NOT mention this reminder to the user."#;

const VALIDATING: &str = r#"Work unit __ID__ is now in VALIDATING status.

CRITICAL: Run ALL tests (not just new ones) to ensure nothing broke.
  - Verify all tests still pass
  - Run complete quality checks
  - Validate Gherkin syntax and tag compliance
  - Update feature file tags before marking done

Common commands for VALIDATING state:
  fspec validate
  fspec validate-tags
  fspec check
  fspec audit-coverage <feature>

For more: fspec check --help

Suggested next steps:
  1. Run language-specific test commands for this codebase
  2. Run: fspec validate (Gherkin syntax)
  3. Run: fspec validate-tags (tag compliance)
  4. Run: fspec check (comprehensive validation)
  5. Run: fspec audit-coverage <feature> (verify coverage mappings)
  6. Update tags: fspec remove-tag-from-feature <file> @wip; fspec add-tag-to-feature <file> @done
  7. Move to done: fspec update-work-unit-status __ID__ done

DO NOT skip quality checks. DO NOT mention this reminder to the user."#;

const BLOCKED: &str = r#"Work unit __ID__ is now in BLOCKED status.

CRITICAL: Document the blocker reason clearly:
  - What is preventing progress?
  - What needs to happen to unblock?
  - Are there dependencies that need resolution?

Consider:
  - Adding dependency relationships: fspec add-dependency __ID__ --blocked-by=<id>
  - Moving back when unblocked: fspec update-work-unit-status __ID__ <previous-state>
  - Breaking down work if too complex

DO NOT mention this reminder to the user."#;

// ── Event-storm assessment (on entering specifying) ───────────────────────

const EVENT_STORM: &str = r#"EVENT STORM ASSESSMENT - Domain Complexity Check

BEFORE jumping to Example Mapping, STOP and assess domain complexity.

Ask yourself:
1. Do you understand the core domain events?
2. Are commands and policies clear?
3. Is there significant domain complexity?

CONSCIOUS CHOICE:

Option 1: RUN EVENT STORM FIRST (if domain is complex)
  → Run: fspec discover-event-storm __ID__
  → Capture: Events → Commands → Policies → Hotspots
  → Transform: fspec generate-example-mapping-from-event-storm __ID__
  → Continue with Example Mapping

Option 2: SKIP TO EXAMPLE MAPPING (if domain is simple)
  → Run: fspec set-user-story __ID__ --role "..." --action "..." --benefit "..."
  → Add rules, examples, questions
  → Generate scenarios

EXAMPLES when Event Storm helped:
• Payment Processing: Discovered 12 domain events, 8 commands → saved 4 hours of rework
• E-commerce Checkout: Identified 3 bounded contexts, prevented architectural mistakes

EXAMPLES when Event Storm was overkill:
• User Login Bug: Simple 2-point fix, obvious events → would have wasted time

FLOW: Event Storm → Transform → Example Mapping → Scenarios

For guidance: fspec bootstrap (Event Storm section)

DO NOT mention this reminder to the user explicitly."#;

// ── Virtual-hooks reminder (specifying → testing) ─────────────────────────

const VIRTUAL_HOOKS: &str = r#"Work unit __ID__ is moving to TESTING phase.

VIRTUAL HOOKS: Consider quality checks for this specific work unit.

Virtual hooks are work unit-scoped ephemeral hooks that:
  - Run ONLY for this work unit (__ID__)
  - Are cleaned up when work is done
  - Perfect for one-off quality gates (linting, type checking, security scans)

AVAILABLE HOOK EVENTS:
  - pre-testing: Before writing tests
  - post-testing: After tests are written
  - pre-implementing: Before implementation
  - post-implementing: After implementation
  - pre-validating: Before validation phase
  - post-validating: After validation phase

COMMON EXAMPLES:
  # Run quality checks before implementing
  fspec add-virtual-hook __ID__ --event pre-implementing --command "<quality-check-commands>" --blocking

  # Run quality checks before validating
  fspec add-virtual-hook __ID__ --event pre-validating --command "<quality-check-commands>" --blocking

  # Security scan on changed files (git context)
  fspec add-virtual-hook __ID__ --event post-implementing --command "security-scan" --git-context --blocking

MANAGEMENT:
  - List hooks: fspec list-virtual-hooks __ID__
  - Remove hook: fspec remove-virtual-hook __ID__ <hook-name>
  - Clear all: fspec clear-virtual-hooks __ID__

REMINDER: When work unit reaches 'done', you will be prompted to keep or remove virtual hooks.

DO NOT mention this reminder to the user explicitly."#;

// ── Done review suggestion (story / bug only) ─────────────────────────────

const DONE_REVIEW: &str = r#"QUALITY CHECK OPPORTUNITY

Work unit __ID__ is being marked as done.

Would you like me to run fspec review __ID__ for a quality review before finalizing?

Suggested workflow:
  1. Run: fspec review __ID__
  2. If findings exist: address findings and fix any issues
  3. If no findings (or all fixed): then mark done

If yes: Run the quality review and address findings before marking done
If no: Proceed with marking done

This is optional but recommended to catch issues early."#;

// ── Public reminder builders ──────────────────────────────────────────────

/// Mirrors `getStatusChangeReminder` (already wrapped, or `None`).
pub(super) fn status_change_reminder(
    id: &str,
    new_status: &str,
    data: &Data,
    project_root: &Path,
) -> Option<String> {
    if !reminders_enabled() {
        return None;
    }
    let body = match new_status {
        "backlog" => return None,
        "specifying" => specifying_reminder(id, project_root),
        "testing" => fill(TESTING, id),
        "implementing" => implementing_reminder(id, data),
        "validating" => fill(VALIDATING, id),
        "done" => done_reminder(id, data),
        "blocked" => fill(BLOCKED, id),
        _ => return None,
    };
    Some(wrap(&body))
}

/// Mirrors the inline Event-Storm reminder pushed on entering specifying.
///
/// NOTE: matches the TS source which builds this reminder unconditionally
/// (no `isRemindersEnabled` gate) before consolidation.
pub(super) fn event_storm_reminder(id: &str) -> String {
    wrap(&fill(EVENT_STORM, id))
}

/// Mirrors `getVirtualHooksReminder` (specifying → testing).
pub(super) fn virtual_hooks_reminder(id: &str) -> Option<String> {
    if !reminders_enabled() {
        return None;
    }
    Some(wrap(&fill(VIRTUAL_HOOKS, id)))
}

/// Mirrors `getVirtualHooksCleanupReminder` (→ done, when hooks exist).
pub(super) fn virtual_hooks_cleanup_reminder(id: &str, count: usize) -> Option<String> {
    if !reminders_enabled() || count == 0 {
        return None;
    }
    let plural = if count > 1 { "s" } else { "" };
    let body = format!(
        r#"Work unit {id} has {count} virtual hook{plural}.

CLEANUP DECISION REQUIRED:
Virtual hooks are work unit-scoped. Now that {id} is done, decide whether to keep or remove them.

OPTIONS:
  1. KEEP hooks for future edits/maintenance of this feature
     - Hooks will remain attached to {id}
     - They will run whenever work unit is active again
     - No action needed

  2. REMOVE hooks (they were one-time quality gates)
     - Run: fspec clear-virtual-hooks {id}
     - Hooks and generated scripts will be deleted
     - Recommended if hooks were temporary

ASK USER: "Do you want to keep the virtual hooks for {id} for future edits, or remove them?"

DO NOT automatically remove hooks. DO NOT mention this reminder to the user explicitly."#
    );
    Some(wrap(&body))
}

/// Mirrors the inline done-review suggestion (story / bug only).
pub(super) fn done_review_reminder(id: &str, work_type: &str) -> Option<String> {
    if work_type != "story" && work_type != "bug" {
        return None;
    }
    // NOTE: matches the TS source which builds this reminder unconditionally
    // (no isRemindersEnabled gate) before consolidation.
    Some(wrap(&fill(DONE_REVIEW, id)))
}

/// Mirrors `buildSubjectiveAnalysisReminder` — returned by review validation
/// for stories that pass the objective checks (attachments are non-empty).
pub(super) fn subjective_review_reminder(id: &str, data: &Data) -> Option<String> {
    let wu = data.work_units.get(id)?;
    let attachments: Vec<String> = wu
        .extra
        .get("attachments")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect())
        .unwrap_or_default();
    if attachments.is_empty() {
        return None;
    }
    let notes: Vec<String> = wu
        .extra
        .get("architectureNotes")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .map(|n| match n {
                    Value::String(s) => s.clone(),
                    _ => n.get("text").and_then(Value::as_str).unwrap_or("").to_string(),
                })
                .collect()
        })
        .unwrap_or_default();

    let mut lines: Vec<String> = Vec::new();
    lines.push("<system-reminder>".to_string());
    lines.push("ARCHITECTURAL REVIEW - SUBJECTIVE ANALYSIS".to_string());
    lines.push(String::new());
    lines.push(format!(
        "Work unit {id} passed objective ACDD checks but requires AI analysis:"
    ));
    lines.push(String::new());
    lines.push("AST RESEARCH ATTACHMENTS:".to_string());
    for (idx, att) in attachments.iter().enumerate() {
        lines.push(format!("  {}. {att}", idx + 1));
    }
    lines.push(String::new());
    lines.push("ARCHITECTURAL NOTES:".to_string());
    for (idx, note) in notes.iter().enumerate() {
        lines.push(format!("  {}. {note}", idx + 1));
    }
    lines.push(String::new());
    lines.push("AI ANALYSIS REQUIRED:".to_string());
    lines.push("  1. Read AST research attachments - verify they reference actual code".to_string());
    lines.push("  2. Check architectural notes align with FOUNDATION.md/CLAUDE.md/AGENTS.md".to_string());
    lines.push("  3. Verify not reinventing existing utilities (check for duplicate function names)".to_string());
    lines.push("  4. Ensure DRY/SOLID principles followed in proposed approach".to_string());
    lines.push(String::new());
    lines.push("DECISION:".to_string());
    lines.push(format!(
        "  - If issues found: Revert to specifying: fspec update-work-unit-status {id} specifying"
    ));
    lines.push("  - If analysis passes: Continue to testing phase".to_string());
    lines.push(String::new());
    lines.push("DO NOT mention this reminder to the user explicitly.".to_string());
    lines.push("</system-reminder>".to_string());
    Some(lines.join("\n"))
}

// ── Enhanced status reminders (depend on work-unit data) ──────────────────

const IMPLEMENTING_BODY: &str = r#"⚠️ COMMON FAILURE MODE: Code that exists but isn't connected.
Tests passing in isolation is NOT the same as a working feature.

IMPLEMENTATION = CREATION + CONNECTION

For every piece of code you write, ask: "WHO CALLS THIS?"
If the answer is "nobody yet" — you're not done. Wire it up.

COMPLETE MEANS:
  ✓ New code exists AND is connected to the system
  ✓ Existing files modified as described in architecture notes
  ✓ Feature works end-to-end (can be demonstrated in the real system)
  ✗ New code exists but nothing calls it

Common commands for IMPLEMENTING state:
  fspec link-coverage <feature> --scenario "..." --test-file <path> --impl-file <path> --impl-lines <lines>
  fspec checkpoint <id> <name>
  fspec restore-checkpoint <id> <name>
  fspec list-checkpoints <id>

For more: fspec checkpoint --help

DO NOT mention this reminder to the user."#;

fn user_story_parts(data: &Data, id: &str) -> Option<(String, String, String)> {
    let wu = data.work_units.get(id)?;
    let us = wu.extra.get("userStory")?;
    let role = us.get("role").and_then(Value::as_str).unwrap_or("").to_string();
    let action = us.get("action").and_then(Value::as_str).unwrap_or("").to_string();
    let benefit = us.get("benefit").and_then(Value::as_str).unwrap_or("").to_string();
    Some((role, action, benefit))
}

fn active_architecture_notes(data: &Data, id: &str) -> Vec<String> {
    data.work_units
        .get(id)
        .and_then(|wu| wu.extra.get("architectureNotes"))
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter(|n| !n.get("deleted").and_then(Value::as_bool).unwrap_or(false))
                .map(|n| match n {
                    Value::String(s) => s.clone(),
                    _ => n.get("text").and_then(Value::as_str).unwrap_or("").to_string(),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Mirrors `implementingStateReminder`.
fn implementing_reminder(id: &str, data: &Data) -> String {
    let mut user_story_section = String::new();
    if let Some((role, action, benefit)) = user_story_parts(data, id) {
        user_story_section = format!(
            "USER STORY: \"As a {role}, I want to {action}, so that {benefit}\"\n\n"
        );
    }
    let mut arch_section = String::new();
    let notes = active_architecture_notes(data, id);
    if !notes.is_empty() {
        let listed = notes
            .iter()
            .map(|n| format!("  - {n}"))
            .collect::<Vec<_>>()
            .join("\n");
        arch_section = format!("ARCHITECTURE NOTES:\n{listed}\n\n");
    }
    format!(
        "Work unit {id} is now in IMPLEMENTING status.\n\n{user_story_section}{arch_section}{IMPLEMENTING_BODY}"
    )
}

/// Mirrors `doneStateReminder`.
fn done_reminder(id: &str, data: &Data) -> String {
    let mut user_story_check = String::new();
    if let Some((role, action, _benefit)) = user_story_parts(data, id) {
        user_story_check = format!(
            r#"FINAL CHECK

User story: "As a {role}, I want to {action}..."

Can {role} do this RIGHT NOW in the actual system?

  - Not "the code exists"
  - Not "tests pass"
  - Can they ACTUALLY do it?

If NO: Go back to implementing.

"#
        );
    }
    format!(
        r#"Work unit {id} is now in DONE status.

{user_story_check}CRITICAL: Verify feature file tags are updated:
  - Remove @wip tag: fspec remove-tag-from-feature <file> @wip
  - Add @done tag: fspec add-tag-to-feature <file> @done

All acceptance criteria should be met. DO NOT mention this reminder to the user."#
    )
}

// ── Specifying reminder + research-tool configuration status ──────────────

const SPEC_HEAD: &str = r#"Work unit __ID__ is now in SPECIFYING status.

CRITICAL: Use Example Mapping FIRST before writing any Gherkin specs:
  1. Ask questions to clarify requirements: fspec add-question __ID__ "@human: [question]"
  2. Capture business rules: fspec add-rule __ID__ "[rule]"
  3. Gather concrete examples: fspec add-example __ID__ "[example]"
  4. Answer all red card questions before moving to testing

INTEGRATION THINKING: WHO CALLS THIS? Ask during discovery.
  Does this feature need to be wired into other parts of the system?
  If yes, capture integration points as examples that become @integration scenarios.
  Event systems, middleware, plugins, services, hooks → need integration scenarios.
  Pure utilities, standalone commands, types → don't need integration scenarios.

RESEARCH TOOLS: Use research tools to answer questions during Example Mapping:
  fspec research                                  # List available research tools

CRITICAL: BEFORE using AST research tool, learn HOW to use it:
  fspec research --tool=ast --help                # Run this FIRST to understand usage

Then use AST research:
  fspec research --tool=ast --query="pattern"     # Search codebase using AST analysis

Or stakeholder research:
  fspec research --tool=stakeholder --platform=teams --question="question" --work-unit=__ID__

Available research tools (--tool=ast or --tool=stakeholder):"#;

const SPEC_MID: &str = r#"Configuration:
  - Project config: spec/fspec-config.json
  - User config: ~/.fspec/fspec-config.json
  - For full help: fspec research --help
  - For tool-specific help: fspec research --tool=<name> --help"#;

const SPEC_TAIL: &str = r#"Research results can be attached to work units for Example Mapping context.

Common commands for SPECIFYING state:
  fspec add-rule <id> "rule"
  fspec remove-rule <id> <index>
  fspec add-example <id> "example"
  fspec remove-example <id> <index>
  fspec add-question <id> "@human: question?"
  fspec answer-question <id> <index> --answer "..."
  fspec research --tool=ast --help                # ALWAYS run this FIRST to learn AST usage
  fspec research --tool=ast --query="pattern"     # Then use AST research
  fspec research --tool=stakeholder --platform=teams --question="..." --work-unit=<id>
  fspec generate-scenarios <id>

For more: fspec help discovery

DO NOT write tests or code yet. DO NOT mention this reminder to the user."#;

const PERPLEXITY_EXAMPLE: &str = r#"{
  "research": {
    "perplexity": {
      "apiKey": "pplx-your-api-key-here"
    }
  }
}"#;

const JIRA_EXAMPLE: &str = r#"{
  "research": {
    "jira": {
      "jiraUrl": "https://example.atlassian.net",
      "username": "your-email@example.com",
      "apiToken": "your-api-token"
    }
  }
}"#;

const CONFLUENCE_EXAMPLE: &str = r#"{
  "research": {
    "confluence": {
      "confluenceUrl": "https://example.atlassian.net/wiki",
      "username": "your-email",
      "apiToken": "your-token"
    }
  }
}"#;

const STAKEHOLDER_EXAMPLE: &str = r#"{
  "research": {
    "stakeholder": {
      "teams": {
        "webhookUrl": "https://..."
      }
    }
  }
}"#;

/// Load the merged research config (`spec/fspec-config.json`, else
/// `~/.fspec/fspec-config.json`), mirroring `loadConfigIfExists`.
fn load_config(project_root: &Path) -> Value {
    let project = project_root.join("spec").join("fspec-config.json");
    if let Ok(c) = std::fs::read_to_string(&project) {
        if let Ok(v) = serde_json::from_str::<Value>(&c) {
            return v;
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        let user = Path::new(&home).join(".fspec").join("fspec-config.json");
        if let Ok(c) = std::fs::read_to_string(&user) {
            if let Ok(v) = serde_json::from_str::<Value>(&c) {
                return v;
            }
        }
    }
    Value::Object(serde_json::Map::new())
}

/// Whether a field is present and non-empty (after trim).
fn nonempty(v: &Value, field: &str) -> bool {
    v.get(field)
        .and_then(Value::as_str)
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
}

/// Mirrors `checkToolConfiguration` for a single tool name.
fn tool_configured(config: &Value, tool: &str) -> bool {
    let tool_cfg = match config.get("research").and_then(|r| r.get(tool)) {
        Some(c) => c,
        None => return false,
    };
    match tool {
        "perplexity" => nonempty(tool_cfg, "apiKey"),
        "jira" => {
            nonempty(tool_cfg, "jiraUrl")
                && nonempty(tool_cfg, "username")
                && nonempty(tool_cfg, "apiToken")
        }
        "confluence" => {
            nonempty(tool_cfg, "confluenceUrl")
                && nonempty(tool_cfg, "username")
                && nonempty(tool_cfg, "apiToken")
        }
        "stakeholder" => ["teams", "slack", "discord"].iter().any(|platform| {
            match tool_cfg.get(platform) {
                Some(p) => nonempty(p, "webhookUrl") || nonempty(p, "token"),
                None => false,
            }
        }),
        _ => false,
    }
}

/// Mirrors `specifyingStateReminder`: the SPECIFYING body with the research
/// tool configuration status block and per-tool config examples.
fn specifying_reminder(id: &str, project_root: &Path) -> String {
    let config = load_config(project_root);

    // ast is always configured; the rest follow registry order.
    let tools: &[(&str, &str)] = &[
        ("perplexity", PERPLEXITY_EXAMPLE),
        ("jira", JIRA_EXAMPLE),
        ("confluence", CONFLUENCE_EXAMPLE),
        ("stakeholder", STAKEHOLDER_EXAMPLE),
    ];

    let mut tool_lines: Vec<String> = vec!["  ✓ ast (ready)".to_string()];
    let mut config_examples: Vec<String> = Vec::new();
    for (name, example) in tools {
        let configured = tool_configured(&config, name);
        let indicator = if configured { "✓" } else { "✗" };
        let status_text = if configured { "ready" } else { "not configured" };
        tool_lines.push(format!("  {indicator} {name} ({status_text})"));
        if !configured {
            config_examples.push(format!("\n{name} configuration:\n{example}"));
        }
    }

    let mut reminder = format!(
        "{}\n{}\n\n{}",
        fill(SPEC_HEAD, id),
        tool_lines.join("\n"),
        SPEC_MID
    );
    if !config_examples.is_empty() {
        reminder.push_str("\n\nConfiguration examples for unconfigured tools:");
        reminder.push_str(&config_examples.join("\n"));
    }
    reminder.push_str("\n\n");
    reminder.push_str(SPEC_TAIL);
    reminder
}

// ── configure-tools checks (printed on entering validating) ───────────────

use crate::commands::review::{format_agent_output, get_agent_config};

/// Current UTC year as a string (parity with `new Date().getFullYear()`).
fn current_year() -> String {
    crate::io::time::iso8601_now()
        .get(0..4)
        .unwrap_or("1970")
        .to_string()
}

/// Mirrors `checkTestCommand`: agent-formatted reminder describing the test
/// command (or its absence).
pub(super) fn check_test_command(project_root: &Path) -> String {
    let agent = get_agent_config(project_root);
    let config_path = project_root.join("spec").join("fspec-config.json");
    let year = current_year();

    let full = format!(
        r#"NO TEST COMMAND CONFIGURED

No test command configured. Use Read/Glob tools to detect test framework, then run:

  fspec configure-tools --test-command <cmd>

If no test tools detected, search for current best practices:
  Query: "best <platform> testing tools {year}"

Replace <platform> with detected project type (Node.js, Python, Rust, Go, etc.)

Example:
  fspec configure-tools --test-command "npm test"
  fspec configure-tools --test-command "pytest"
  fspec configure-tools --test-command "cargo test"
"#
    );
    let short = format!(
        r#"NO TEST COMMAND CONFIGURED

No test command configured. Use Read/Glob tools to detect test framework, then run:

  fspec configure-tools --test-command <cmd>

If no test tools detected, search for current best practices:
  Query: "best <platform> testing tools {year}"

Replace <platform> with detected project type (Node.js, Python, Rust, Go, etc.)
"#
    );

    let content = match std::fs::read_to_string(&config_path) {
        Err(_) => full,
        Ok(raw) => match serde_json::from_str::<Value>(&raw) {
            Ok(cfg) => match cfg
                .get("tools")
                .and_then(|t| t.get("test"))
                .and_then(|t| t.get("command"))
                .and_then(Value::as_str)
            {
                Some(cmd) if !cmd.is_empty() => {
                    format!("RUN TESTS\n\nRun tests: {cmd}\n")
                }
                _ => short,
            },
            Err(_) => short,
        },
    };
    format_agent_output(&agent, &content)
}

/// Mirrors `checkQualityCommands`: agent-formatted reminder describing the
/// quality-check commands (or their absence).
pub(super) fn check_quality_commands(project_root: &Path) -> String {
    let agent = get_agent_config(project_root);
    let config_path = project_root.join("spec").join("fspec-config.json");

    let content = match std::fs::read_to_string(&config_path) {
        Err(_) => "No quality check commands configured".to_string(),
        Ok(raw) => match serde_json::from_str::<Value>(&raw) {
            Ok(cfg) => match cfg
                .get("tools")
                .and_then(|t| t.get("qualityCheck"))
                .and_then(|q| q.get("commands"))
                .and_then(Value::as_array)
            {
                Some(cmds) => {
                    let chained = cmds
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(" && ");
                    format!("RUN QUALITY CHECKS\n\nRun quality checks: {chained}\n")
                }
                None => "No quality check commands configured".to_string(),
            },
            Err(_) => "No quality check commands configured".to_string(),
        },
    };
    format_agent_output(&agent, &content)
}
