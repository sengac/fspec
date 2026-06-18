//! `reverse` help configuration — Rust port of `src/commands/reverse-help.ts`
//! (RPC-294).
//!
//! Byte-for-byte parity with `node dist/index.js reverse --help` piped to a
//! non-TTY. Field order and content mirror the TS `config` object verbatim so
//! `format_command_help` reproduces the documented output.

use super::super::{CommonError, CommandExample, CommandHelpConfig, CommandOption, CommonPatternEntry};

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "--strategy=<A|B|C|D>",
        description: "Choose reverse ACDD strategy: A=Spec Gap Filling (tests → features), B=Test Gap Filling (features → tests), C=Coverage Mapping (link existing), D=Full Reverse ACDD (code → everything)",
        default_value: None,
    },
    CommandOption {
        flag: "--continue",
        description: "Continue to next step in current strategy execution. Increments step counter and emits guidance for next file/scenario.",
        default_value: None,
    },
    CommandOption {
        flag: "--status",
        description: "Show current session status: phase, chosen strategy, progress (step N of M), gaps detected, files remaining.",
        default_value: None,
    },
    CommandOption {
        flag: "--reset",
        description: "Delete session file and start fresh. Use when you want to abandon current session and re-analyze project.",
        default_value: None,
    },
    CommandOption {
        flag: "--complete",
        description: "Mark session as complete and delete session file. Validates all steps finished before completing.",
        default_value: None,
    },
    CommandOption {
        flag: "--dry-run",
        description: "Preview gap analysis without creating session file. Shows what would be detected and suggested strategy.",
        default_value: None,
    },
];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Bullet("Strategy A (Spec Gap Filling): Tests exist → Create features from test assertions → Link coverage with --skip-validation"),
    CommonPatternEntry::Bullet("Strategy B (Test Gap Filling): Features exist → Create test skeletons → Link coverage with --skip-validation"),
    CommonPatternEntry::Bullet("Strategy C (Coverage Mapping): Both exist → Link existing tests to scenarios using fspec link-coverage"),
    CommonPatternEntry::Bullet("Strategy D (Full Reverse ACDD): Raw code only → Analyze implementation → Create features → Create tests → Create work units"),
    CommonPatternEntry::Bullet("Check progress anytime: fspec reverse --status shows current phase, strategy, and step progress"),
    CommonPatternEntry::Bullet("Made a mistake: fspec reverse --reset deletes session and lets you start fresh"),
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec reverse",
        description: Some("Initial analysis: detect gaps and suggest strategy"),
        output: Some("<system-reminder>\nGap analysis complete.\nDetected: 3 test files without features\nSuggested: Strategy A (Spec Gap Filling)\nTo choose this strategy, run: fspec reverse --strategy=A\n</system-reminder>\n\nFound 3 test files without feature files\nSuggested Strategy: A (Spec Gap Filling)\nEstimated Effort: 6-9 story points"),
    },
    CommandExample {
        command: "fspec reverse --strategy=A",
        description: Some("Choose Strategy A and receive first step guidance"),
        output: Some("<system-reminder>\nStep 1 of 3\nStrategy: A (Spec Gap Filling)\nAfter completing this step, run: fspec reverse --continue\n</system-reminder>\n\nRead test file: src/__tests__/auth.test.ts\nThen create feature file\nThen run fspec link-coverage with --skip-validation"),
    },
    CommandExample {
        command: "fspec reverse --continue",
        description: Some("Move to next step after completing current one"),
        output: Some("<system-reminder>\nStep 2 of 3\nProcess file: src/__tests__/user.test.ts\nAfter completing this step, run: fspec reverse --continue\n</system-reminder>\n\nProcess test file: src/__tests__/user.test.ts..."),
    },
    CommandExample {
        command: "fspec reverse --status",
        description: Some("Check current session status and progress"),
        output: Some("Phase: executing\nStrategy: A (Spec Gap Filling)\nProgress: Step 2 of 3\nGaps Detected: 3 test files without features\nFiles:\n  [✓] src/__tests__/auth.test.ts (completed)\n  [→] src/__tests__/user.test.ts (in progress)\n  [ ] src/__tests__/product.test.ts (pending)"),
    },
    CommandExample {
        command: "fspec reverse --complete",
        description: Some("Finalize session after completing all steps"),
        output: Some("<system-reminder>\nSession completed successfully.\nAll gaps filled.\n</system-reminder>\n\n✓ Reverse ACDD session complete"),
    },
    CommandExample {
        command: "fspec reverse --dry-run",
        description: Some("Preview analysis without creating session"),
        output: Some("DRY-RUN MODE: Analysis complete, no session created.\nDetected: 3 test files without features\nSuggested: Strategy A (Spec Gap Filling)\nEstimated Effort: 6-9 story points"),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Error: Existing reverse session detected",
        fix: "Use: fspec reverse --continue (to continue), fspec reverse --status (to check progress), fspec reverse --reset (to start over), or fspec reverse --complete (to finalize)",
    },
    CommonError {
        error: "Error: No active reverse session",
        fix: "Start a new session by running: fspec reverse (analyzes project and suggests strategy)",
    },
    CommonError {
        error: "Error: Cannot complete: not all steps are finished",
        fix: "Continue working through remaining steps with: fspec reverse --continue, then try --complete again",
    },
];

const RELATED: &[&str] = &[
    "create-feature",
    "add-scenario",
    "link-coverage",
    "unlink-coverage",
    "generate-coverage",
    "create-story",
    "create-bug",
    "create-task",
];

const PREREQUISITES: &[&str] = &[
    "Project must have at least some code (tests, features, or implementation)",
    "For Strategy A: Test files must exist",
    "For Strategy B: Feature files must exist",
    "For Strategy C: Both tests and features must exist",
    "For Strategy D: Implementation files must exist",
];

const NOTES: &[&str] = &[
    "This is a GUIDANCE tool, not an execution tool - AI must use other fspec commands to do the work",
    "Session state is stored in OS temp directory using project-specific hash",
    "Sessions are ephemeral - OS cleans up temp files automatically",
    "Only one session can be active per project at a time",
    "Use --dry-run to preview analysis without committing to a session",
    "Strategies are mutually exclusive - choose one, complete it, then run reverse again for other gaps",
    "Use --skip-validation flag with link-coverage when linking forward-planned coverage",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "reverse",
    description: "Interactive reverse ACDD strategy planning command. Analyzes project state, detects gaps (missing features, tests, coverage), suggests strategic approaches, and guides AI step-by-step through gap-filling workflow.",
    usage: Some("fspec reverse [options]"),
    arguments: &[],
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some("Use this command when: 1) You have an existing codebase without fspec specifications, 2) Features exist but tests are missing, 3) Tests exist but feature files are missing, 4) Coverage mappings are incomplete. This is a planning/guidance tool that helps AI choose the right reverse ACDD strategy."),
    when_not_to_use: Some("Do not use for: 1) New projects (use forward ACDD: create-feature → tests → implementation), 2) Executing reverse ACDD work directly (this command only guides, AI must execute using other fspec commands). Not for projects already using full ACDD workflow."),
    prerequisites: PREREQUISITES,
    common_patterns: COMMON_PATTERNS,
    typical_workflow: Some("1. Run fspec reverse (analyze and detect gaps) → 2. Run fspec reverse --strategy=A (choose strategy) → 3. Do the work for step 1 using other fspec commands → 4. Run fspec reverse --continue (get next step) → 5. Repeat until all steps done → 6. Run fspec reverse --complete (finalize)"),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
