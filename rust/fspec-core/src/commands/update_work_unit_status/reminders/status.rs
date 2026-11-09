//! Static status templates and the status-transition reminder builders.

use std::path::Path;

use super::enhanced::{done_reminder, implementing_reminder};
use super::specifying::specifying_reminder;
use super::{fill, reminders_enabled, wrap, Data};

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
  fspec audit --work-unit __ID__

For more: fspec audit --help

Suggested next steps:
  1. Run language-specific test commands for this codebase
  2. Run: fspec validate (Gherkin syntax)
  3. Run: fspec validate-tags (tag compliance)
  4. Run: fspec audit --work-unit __ID__ (Gherkin, tags, format, coverage and spec)
  5. Check each scenario's test needs its implementation: fspec audit --start --area meaning --work-unit __ID__
  6. Update tags: fspec remove-tag-from-feature <file> @wip; fspec add-tag-to-feature <file> @done
  7. Move to done: fspec update-work-unit-status __ID__ done

DO NOT skip quality checks. DO NOT mention this reminder to the user."#;

const BLOCKED: &str = r#"Work unit __ID__ is now in BLOCKED status.

CRITICAL: Document the blocker reason clearly:
  - What is preventing progress?
  - What needs to happen to unblock?
  - Are there dependencies that need resolution?

Consider:
  - Adding dependency relationships: fspec add-dependency __ID__ --blocked-by=<workUnitId>
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
  fspec add-virtual-hook __ID__ pre-implementing "<quality-check-commands>" --blocking

  # Run quality checks before validating
  fspec add-virtual-hook __ID__ pre-validating "<quality-check-commands>" --blocking

  # Security scan on changed files (git context)
  fspec add-virtual-hook __ID__ post-implementing "security-scan" --git-context --blocking

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
pub(in crate::commands::update_work_unit_status) fn status_change_reminder(
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
pub(in crate::commands::update_work_unit_status) fn event_storm_reminder(id: &str) -> String {
    wrap(&fill(EVENT_STORM, id))
}

/// Mirrors `getVirtualHooksReminder` (specifying → testing).
pub(in crate::commands::update_work_unit_status) fn virtual_hooks_reminder(
    id: &str,
) -> Option<String> {
    if !reminders_enabled() {
        return None;
    }
    Some(wrap(&fill(VIRTUAL_HOOKS, id)))
}

/// Mirrors `getVirtualHooksCleanupReminder` (→ done, when hooks exist).
pub(in crate::commands::update_work_unit_status) fn virtual_hooks_cleanup_reminder(
    id: &str,
    count: usize,
) -> Option<String> {
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
pub(in crate::commands::update_work_unit_status) fn done_review_reminder(
    id: &str,
    work_type: &str,
) -> Option<String> {
    if work_type != "story" && work_type != "bug" {
        return None;
    }
    // NOTE: matches the TS source which builds this reminder unconditionally
    // (no isRemindersEnabled gate) before consolidation.
    Some(wrap(&fill(DONE_REVIEW, id)))
}
