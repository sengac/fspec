@done
@quality-assurance
@rust
@cli
@RPC-295
Feature: review CLI subcommand on the standalone fspec Rust binary

  """
  CLI surface for the `review` subcommand on the standalone fspec Rust binary.
  Two-front-doors pattern:
    - Shell argv         → clap → codelet/fspec/src/review.rs → fspec_core::commands::review::run
    - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::review::run
  Both call sites pass a JSON-encoded `{ workUnitId: string }` args shape and a
  `project_root: &Path`; the CLI surface resolves project_root from CWD (parity
  with TS `review(workUnitId, { cwd: process.cwd() })`).

  The clap subcommand exposes a single required positional `<work-unit-id>` and NO
  other flags, mirroring the TS Commander.js registration at
  src/commands/review.ts:569-586 (`program.command('review <work-unit-id>')`).

  Unlike `init`, review has NO custom *-help.ts in the TypeScript source — only a
  one-line `.description(...)`. Therefore the Rust port follows the
  delete-scenarios SPECIAL-CASE: NO rich help CONFIG file, NO intercept_ts_help arm,
  bare clap-generated --help output. The CLI tests assert the subcommand exists and
  is functional, NOT byte-for-byte help parity.

  On success the CLI prints the full review report text blob to stdout and exits 0
  (review never fails on findings). Errors print 'Error: <msg>' to stderr and exit 1
  (parity with the action-handler catch at src/commands/review.ts:579-585).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The standalone fspec binary MUST expose `review` as a clap v4 derive subcommand whose only argument is a required positional `<work-unit-id>` (parity with the single Commander.js `program.command('review <work-unit-id>')` at src/commands/review.ts:571)
  #   2. The clap action MUST resolve project_root from CWD, marshal the work unit id into `{ workUnitId: ... }` JSON, and delegate to fspec_core::commands::review::run — NO review logic in the CLI bridge (two-front-doors invariant)
  #   3. On success the CLI prints the review report text to stdout and exits 0 (review never fails on findings); on error it prints 'Error: <msg>' to stderr and exits 1 (parity with src/commands/review.ts:579-585)
  #   4. review follows the delete-scenarios SPECIAL-CASE: NO rich help CONFIG, NO intercept_ts_help arm — bare clap-generated --help only
  #
  # EXAMPLES:
  #   1. `fspec review <id>` for an existing work unit → exit 0, stdout contains 'REVIEW: <id>' and the section headers (## Issues Found, ## ACDD Compliance, ## Coverage Analysis, ## Summary)
  #   2. `fspec review BOGUS-999` for a missing id → exit 1, stderr contains "Error: Work unit 'BOGUS-999' does not exist"
  #   3. `fspec review --help` → exit 0, bare clap help mentioning 'review' and 'work-unit-id'
  #   4. dispatch review through fspec_core::dispatch::dispatch_command → same report as the CLI bridge (two-front-doors parity)
  #
  # ========================================

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to invoke `fspec review <work-unit-id>` directly from a shell with the same surface offered by the TypeScript Commander.js CLI
    So that I can run an end-to-end ACDD review of a work unit from a terminal or script without going through the LLM tool-call dispatcher

  Scenario: Clap exposes review as a subcommand and prints bare --help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec review --help` from a shell
    Then the command exits 0
    Then stdout contains the substring 'review'
    Then stdout contains the substring 'work-unit-id'

  Scenario: CLI reviews an existing work unit and prints the report
    Given a project root whose work-units store contains the work unit being reviewed
    When I run `./codelet/target/release/fspec review <id>` from that directory
    Then the command exits 0
    Then stdout contains the substring 'REVIEW:'
    Then stdout contains the substring '## ACDD Compliance'
    Then stdout contains the substring '## Summary'

  Scenario: CLI errors on a work unit id that does not exist
    Given a project root whose work-units store does not contain the id 'BOGUS-999'
    When I run `./codelet/target/release/fspec review BOGUS-999` from that directory
    Then the command exits with code 1
    Then stderr contains the substring "Error: Work unit 'BOGUS-999' does not exist"

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root whose work-units store contains the work unit being reviewed
    When I dispatch review through fspec_core::dispatch::dispatch_command with that work unit id against that project root
    Then the dispatcher result text equals the stdout produced by the CLI bridge for the same work unit
    And the CLI bridge module codelet/fspec/src/review.rs contains NO review logic — its only computation is JSON arg marshalling and stdout printing
