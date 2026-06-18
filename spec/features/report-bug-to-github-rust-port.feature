@done
@rust
@report-bug-to-github
@cli
@RPC-285
Feature: Port report-bug-to-github command to Rust

  """
  Core impl target: codelet/fspec-core/src/commands/report_bug_to_github.rs. Signature changes from the
  current stub run(_args_json) to the canonical run(args_json, project_root). Args mirror the TS
  ReportBugOptions CLI surface registered at src/commands/report-bug-to-github.ts:359-413:
  {projectRoot:Option<String>, bugDescription:Option<String>, expectedBehavior:Option<String>,
  actualBehavior:Option<String>, interactive:bool}. Returns a JSON envelope String the CLI bridge decodes to
  render TS-equivalent stdout/stderr. Envelope shape: {title, markdown, context, url, previewShown,
  cancelled, browserOpened}.

  ⚠️ SCOPE FLAG (poll_sync_future / async-process safety) — see WORKER report to supervisor.
  The TS command has a deterministic gather/format/URL core plus two NON-deterministic side surfaces:
    (A) DETERMINISTIC CORE (specified here, 100% poll_sync_future-SAFE):
        - gatherContext: blocking fs reads (package.json→pinned const 0.9.3, error-logs json), OS from
          std::env::consts::OS, work-unit context from work-units.json + feature-file scan, and git
          branch/status via BLOCKING std::process::Command::new("git").output() — the exact pattern already
          documented poll_sync_future-safe in update_work_unit_status.rs:479.
        - formatBugReportMarkdown: pure string assembly.
        - constructGitHubURL: percent-encoding (hand-rolled encodeURIComponent — no url crate available).
    (B) DEFERRED pending supervisor scope decision (the research EXECUTE-mode analogue):
        - openInBrowser(url): launches the system browser via the `open` npm package (no-ops in test env,
          fire-and-forget detached spawn). Launching a GUI browser from the LLM dispatcher / a headless CI
          context is an undesirable real side effect, so the dispatcher/core path returns browserOpened=false
          and surfaces the url in the envelope instead. Best-effort launch on the standalone-binary path is a
          supervisor decision.
        - Interactive stdin prompts (prompt/confirm/editTitle/editBody): real stdin blocking, NOT
          poll_sync_future-safe, and never wired by the Commander action handler anyway (the handler passes
          interactive:true without any callbacks, so --interactive only sets previewShown).

  SHARED-FILE REQUESTS to supervisor (deferred to Phase C): (1) canonical.rs PORTED_COMMANDS add
  'report-bug-to-github'; (2) dispatch.rs move from run_stub to run_ported with run(args_json, project_root);
  (3) main.rs Mode::ReportBugToGithub clap variant {project_root, bug_description, expected_behavior,
  actual_behavior, interactive} + forward! arm + intercept_ts_help arm + mod report_bug_to_github;
  (4) help/configs/mod.rs register report_bug_to_github::CONFIG. Worker owns: commands/report_bug_to_github.rs
  (rewrite stub), help/configs/report_bug_to_github.rs, fspec/src/report_bug_to_github.rs bridge, tests, fixture.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. With no work-unit/git/error context the command still produces a complete bug report: title
  #      'Bug: <first 60 chars of description>' and markdown with Description, Expected Behavior, Actual
  #      Behavior, Steps to Reproduce, Environment and Additional Context sections.
  #   2. The Environment section reports the pinned fspec version (0.9.3), the OS platform, and—only when a
  #      git branch resolves—the current git branch line.
  #   3. When a most-recently-updated non-done work unit exists, its id/title/status are included in
  #      Additional Context, and the first feature file tagged @<id> is reported as the Feature File.
  #   4. The GitHub URL is https://github.com/sengac/fspec/issues/new with title, body and
  #      labels=bug,needs-triage percent-encoded exactly like JS encodeURIComponent (unreserved set
  #      A-Za-z0-9 and - _ . ! ~ * ' ( )).
  #   5. Context gathering, markdown formatting and URL construction perform only blocking file reads/writes
  #      plus a blocking git subprocess, so the command is safe under the single-poll sync dispatcher and never
  #      returns Poll::Pending.
  #   6. Automatic system-browser launch and real interactive stdin prompts are DEFERRED (dispatcher-only, not
  #      implemented) — same class as research EXECUTE; every path returns the url with browserOpened=false.
  #   7. The Environment line emits the fspec version + OS/arch from std::env::consts (the Rust port has no
  #      Node runtime, so process.version is not reproduced); tests assert the line is present + well-formed.
  #   8. Work-unit context is gathered via the faithful TS side-effect: ensure_work_units_file creates
  #      spec/work-units.json with the canonical initial structure when missing (parity with TS
  #      ensureWorkUnitsFile → findOrCreateSpecDirectory + readJSON default).
  #
  # EXAMPLES:
  #   1. Empty project, no git, no work units: dispatching with bug-description 'crash on save' returns a
  #      report whose markdown contains '## Environment', 'fspec version: 0.9.3' and the OS, and a GitHub URL
  #      beginning https://github.com/sengac/fspec/issues/new?title=.
  #   2. A project with one in-progress work unit AUTH-001 tagged in user-login.feature: the generated
  #      markdown's Additional Context names 'AUTH-001', its title/status and
  #      '**Feature File**: spec/features/user-login.feature'.
  #   3. Title/body containing spaces and '#' round-trip through encodeURIComponent so the URL has %20 for
  #      spaces and %23 for '#' and contains no raw spaces.
  #
  # SCOPE RULINGS (supervisor, RPC-285):
  #   - Node-version line: emit fspec version + OS/arch from std::env::consts (no Node runtime); behaviour-
  #     level assertion, not byte-exact Node string.
  #   - Browser launch + interactive prompts: DEFERRED — print/return the URL, never open a browser.
  #   - Work-units file: replicate TS side-effect faithfully via ensure_work_units_file (creates the file).
  #
  # ========================================

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want to port the report-bug-to-github command's deterministic context-gathering, markdown-formatting and
    GitHub-URL-construction surface to Rust as a parity port
    So that the standalone Rust binary and the dispatcher can produce a pre-filled GitHub bug-report URL
    without falling back to TypeScript and without an undesirable browser side effect on the dispatcher path

  Scenario: Default flow produces a complete bug report with all markdown sections
    Given an empty project root tempdir with no git repo and no work units
    When I dispatch report-bug-to-github with bug-description "crash on save"
    Then the dispatcher returns success
    And the report markdown contains the section "## Description"
    And the report markdown contains the section "## Expected Behavior"
    And the report markdown contains the section "## Actual Behavior"
    And the report markdown contains the section "## Steps to Reproduce"
    And the report markdown contains the section "## Environment"
    And the report markdown contains the section "## Additional Context"
    And the report title starts with "Bug: crash on save"

  Scenario: Environment section reports the pinned fspec version and OS
    Given an empty project root tempdir with no git repo and no work units
    When I dispatch report-bug-to-github with bug-description "crash on save"
    Then the dispatcher returns success
    And the report markdown contains "fspec version: 0.9.3"
    And the report markdown contains the current OS platform line

  Scenario: Constructed GitHub URL targets the sengac/fspec issues endpoint with encoded labels
    Given an empty project root tempdir with no git repo and no work units
    When I dispatch report-bug-to-github with bug-description "crash on save"
    Then the dispatcher returns success
    And the result url starts with "https://github.com/sengac/fspec/issues/new?title="
    And the result url contains "labels=bug%2Cneeds-triage"

  Scenario: URL encoding escapes spaces and special characters like encodeURIComponent
    Given an empty project root tempdir with no git repo and no work units
    When I dispatch report-bug-to-github with bug-description "fix #42 now"
    Then the dispatcher returns success
    And the result url contains "%23" for the hash character
    And the result url contains no raw space characters

  Scenario: Work unit context is included when a non-done work unit exists
    Given a project root tempdir whose work-units.json has an in-progress work unit "AUTH-001" titled "Login"
    And a feature file spec/features/user-login.feature tagged "@AUTH-001"
    When I dispatch report-bug-to-github with bug-description "login broken"
    Then the dispatcher returns success
    And the report markdown contains "AUTH-001"
    And the report markdown contains "**Feature File**: spec/features/user-login.feature"

  Scenario: Gathering context faithfully replicates the TS work-units file side-effect and is sync-safe
    Given an empty project root tempdir with no spec subdirectory
    When I dispatch report-bug-to-github with bug-description "crash on save"
    Then the dispatcher returns success
    And spec/work-units.json is created in the project root with the canonical initial structure
    And the dispatcher does not return an error
    And the result reports browserOpened as false

  Scenario: Both front doors converge on the same fspec-core function
    Given an empty project root tempdir with no git repo and no work units
    When I dispatch report-bug-to-github with bug-description "crash on save" via the dispatcher
    Then the dispatcher returns success
    And the result url starts with "https://github.com/sengac/fspec/issues/new?title="
