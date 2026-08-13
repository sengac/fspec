@done
@rust
@bootstrap
@cli
@RPC-200
Feature: Port bootstrap command to Rust
  """
  File layout: core impl rust/fspec-core/src/commands/bootstrap.rs (rewrite stub, signature gains project_root); embedded static doc asset adjacent to it via include_str! (e.g. bootstrap_doc.txt, byte-exact capture of `node dist/index.js bootstrap` in an empty dir); help config rust/fspec-core/src/help/configs/bootstrap.rs; CLI bridge rust/fspec/src/bootstrap.rs; core tests rust/fspec-core/tests/bootstrap.rs; CLI tests rust/fspec/tests/cli_bootstrap.rs; help fixture rust/fspec/tests/fixtures/help/bootstrap.txt. Two feature files: bootstrap-rust-port.feature (dispatcher contract) + bootstrap-cli-subcommand.feature (clap surface).
  Strategy: do NOT re-port the ~4000 lines of TS string-building (17 slashCommandSections + 6 display*Help bodies). Capture the byte-exact static output once and embed via include_str!; run() applies ONLY the two config string-replacements (<test-command>, <quality-check-commands>) and appends the event-storm reminder. Reuse existing Rust precedents: configure_tools.rs for spec/fspec-config.json (tools.test.command, tools.qualityCheck.commands), board.rs / generate_foundation_md.rs for foundation.json eventStorm.items, io store helpers for work-units.json (FOUND- id / title contains 'event storm' / status != done), and the inline wrapInSystemReminder byte format used in discover_event_storm.rs.
  Async assessment: NONE. Pure blocking std::fs reads (config, foundation, work-units) + string replacement + in-memory concatenation. No network, no child process, no real tokio .await — fully compatible with poll_sync_future.
  SHARED-FILE CHANGES (supervisor, Phase C): (1) dispatch.rs bootstrap arm -> commands::bootstrap::run(args_json, project_root).await, remove stub arm (1-arg -> 2-arg); (2) canonical.rs add 'bootstrap' to PORTED_COMMANDS; (3) help/configs/mod.rs register pub mod bootstrap; (4) main.rs add mod bootstrap, Mode::Bootstrap clap variant (no positional args, no flags), forward! arm, and --help intercept arm; (5) commands/mod.rs stub already registered — verify only; (6) capture asset bootstrap_doc.txt via node build — supervisor/cargo-runner.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. bootstrap takes no arguments and no flags; invoking it always emits the complete documentation
  #   2. Output is assembled in a fixed order: header (slash-command template) + complete workflow documentation (17 sections joined by newline) + a Step 12 Complete Command Reference explainer + the six help-topic strings
  #   3. The six help-topic sections are appended in this exact order separated by blank lines: specs, work, discovery, metrics, setup, hooks; each is plain ANSI-stripped text
  #   4. If spec/fspec-config.json has tools.test.command, every <test-command> placeholder is globally replaced with that value; otherwise the placeholder is left intact
  #   5. If spec/fspec-config.json has tools.qualityCheck.commands, they are joined with " && " and every <quality-check-commands> placeholder is globally replaced; otherwise the placeholder is left intact
  #   6. A missing or unparseable spec/fspec-config.json leaves both placeholders intact and bootstrap still succeeds
  #   7. No Event Storm reminder is appended when spec/foundation.json is absent, when its eventStorm.items is non-empty, or when any file read/parse errors
  #   8. When foundation.json exists with empty eventStorm and a non-done work unit whose id starts with FOUND- and whose lowercased title contains "event storm" exists, the work-unit-variant BIG PICTURE EVENT STORMING NEEDED system-reminder naming that work unit is appended
  #   9. When foundation.json exists with empty eventStorm and no matching FOUND- work unit, the no-work-unit-variant BIG PICTURE EVENT STORMING NEEDED system-reminder is appended
  #   10. The CLI prints the result to stdout and exits 0; on a thrown error it prints "Error running bootstrap: <message>" and exits 1
  #   11. Both front doors (LLM dispatcher JSON and standalone clap CLI) converge on the same fspec_core::commands::bootstrap::run(args_json, project_root) function; the CLI bridge does JSON marshalling only
  #
  # EXAMPLES:
  #   1. Dispatching bootstrap in an empty project returns the complete documentation (over 10000 chars) with <test-command> and <quality-check-commands> placeholders intact and no Event Storm reminder
  #   2. The output contains the header marker and the strings ACDD, Example Mapping, Story Point Estimation, and Kanban
  #   3. The output contains all six help-section markers
  #   4. With tools.test.command "cargo test", every <test-command> is replaced and none remain
  #   5. With tools.qualityCheck.commands ["cargo clippy", "cargo fmt --check"], every <quality-check-commands> is replaced with the joined string
  #   6. With a malformed fspec-config.json the command still returns successfully and placeholders remain intact
  #   7. With foundation.json present, empty eventStorm, and FOUND-001 "Conduct Foundation Event Storm" (specifying), the output ends with a reminder naming FOUND-001
  #   8. With foundation.json present, empty eventStorm, and no matching work unit, the output ends with the no-work-unit reminder
  #   9. With foundation.json whose eventStorm.items has entries, no reminder is appended
  #
  # ========================================
  Background: User Story
    As a fspec maintainer porting the CLI to Rust
    I want to run `bootstrap` in the Rust binary
    So that I get byte-for-byte the same complete fspec documentation with config and event-storm transforms applied as the TypeScript command

  Scenario: Dispatch returns the complete documentation for an empty project
    Given a project root tempdir with no fspec-config.json and no foundation.json
    When I dispatch bootstrap
    Then the dispatcher returns success=true
    Then the rendered output is longer than 10000 characters
    Then the rendered output contains the substring "<test-command>"
    Then the rendered output contains the substring "<quality-check-commands>"
    Then the rendered output does not contain the substring "BIG PICTURE EVENT STORMING NEEDED"

  Scenario: Output contains the header marker and core workflow strings
    Given a project root tempdir with no fspec-config.json and no foundation.json
    When I dispatch bootstrap
    Then the rendered output contains the substring "# fspec Command - Kanban-Based Project Management"
    Then the rendered output contains the substring "ACDD"
    Then the rendered output contains the substring "Example Mapping"
    Then the rendered output contains the substring "Story Point Estimation"
    Then the rendered output contains the substring "Kanban"

  Scenario: Output contains all six help-section markers in order
    Given a project root tempdir with no fspec-config.json and no foundation.json
    When I dispatch bootstrap
    Then the rendered output contains the substring "GHERKIN SPECIFICATIONS"
    Then the rendered output contains the substring "create-story"
    Then the rendered output contains the substring "add-rule"
    Then the rendered output contains the substring "query-metrics"
    Then the rendered output contains the substring "discover-foundation"
    Then the rendered output contains the substring "LIFECYCLE HOOKS"

  Scenario: Config test-command placeholder is replaced
    Given a project root tempdir whose spec/fspec-config.json sets tools.test.command to "cargo test"
    When I dispatch bootstrap
    Then the dispatcher returns success=true
    Then the rendered output contains the substring "cargo test"
    Then the rendered output does not contain the substring "<test-command>"

  Scenario: Config quality-check-commands placeholder is replaced with the joined string
    Given a project root tempdir whose spec/fspec-config.json sets tools.qualityCheck.commands to ["cargo clippy", "cargo fmt --check"]
    When I dispatch bootstrap
    Then the dispatcher returns success=true
    Then the rendered output contains the substring "cargo clippy && cargo fmt --check"
    Then the rendered output does not contain the substring "<quality-check-commands>"

  Scenario: Malformed config leaves placeholders intact and still succeeds
    Given a project root tempdir whose spec/fspec-config.json contains invalid JSON
    When I dispatch bootstrap
    Then the dispatcher returns success=true
    Then the rendered output contains the substring "<test-command>"
    Then the rendered output contains the substring "<quality-check-commands>"

  Scenario: Event Storm reminder names the matching FOUND work unit
    Given a project root tempdir whose foundation.json has an empty eventStorm and a non-done FOUND-001 work unit titled "Conduct Foundation Event Storm"
    When I dispatch bootstrap
    Then the dispatcher returns success=true
    Then the rendered output contains the substring "BIG PICTURE EVENT STORMING NEEDED"
    Then the rendered output contains the substring "FOUND-001"

  Scenario: Event Storm reminder suggests creating a work unit when none matches
    Given a project root tempdir whose foundation.json has an empty eventStorm and no matching FOUND work unit
    When I dispatch bootstrap
    Then the dispatcher returns success=true
    Then the rendered output contains the substring "BIG PICTURE EVENT STORMING NEEDED"
    Then the rendered output contains the substring "fspec create-task FOUND"

  Scenario: No reminder when the event storm is already populated
    Given a project root tempdir whose foundation.json eventStorm.items already has entries
    When I dispatch bootstrap
    Then the dispatcher returns success=true
    Then the rendered output does not contain the substring "BIG PICTURE EVENT STORMING NEEDED"

  Scenario: CLI and dispatcher converge on the same fspec_core run function
    Given a project root tempdir with no fspec-config.json and no foundation.json
    When I dispatch bootstrap and also run the CLI subcommand fspec bootstrap against an equivalent project root
    Then both paths produce output containing "# fspec Command - Kanban-Based Project Management"
    Then the CLI bridge module rust/fspec/src/bootstrap.rs contains no documentation-building or transform logic — its only computation is JSON arg marshalling
