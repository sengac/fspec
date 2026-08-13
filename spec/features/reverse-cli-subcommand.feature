@done
@querying
@cli
@rust
@RPC-294
Feature: Port reverse command to Rust
  """
  File layout: core impl rust/fspec-core/src/commands/reverse.rs (rewrite stub); new type module rust/fspec-core/src/types/reverse_session.rs (ReverseSession, GapAnalysis, AnalysisResult); help config rust/fspec-core/src/help/configs/reverse.rs; CLI bridge rust/fspec/src/reverse.rs; integration tests rust/fspec/tests/cli_reverse.rs + core tests rust/fspec-core/tests/reverse.rs; help fixture rust/fspec/tests/fixtures/help/reverse.txt. Two feature files: reverse-rust-port.feature (dispatcher contract) + reverse-cli-subcommand.feature (clap surface).
  SHARED-FILE CHANGE (supervisor) #1: dispatch.rs reverse arm must change commands::reverse::run(args_json).await to commands::reverse::run(args_json, project_root).await — the ported signature adds project_root (parity: session hash + analysis need the project root, never env::current_dir()).
  SHARED-FILE CHANGE (supervisor) #2: rust/fspec-core/Cargo.toml must add a sha2 dependency (sha2 = { workspace = true }) for the session-file hash. No hex crate — we will hex-encode the digest bytes manually with format!("{:02x}").
  SHARED-FILE CHANGE (supervisor) #3: rust/fspec-core/src/types/mod.rs must register `pub mod reverse_session;`.
  SHARED-FILE CHANGE (supervisor) #4: rust/fspec-core/src/help/configs/mod.rs must register `pub mod reverse;`.
  SHARED-FILE CHANGE (supervisor) #5: rust/fspec/src/main.rs must add `mod reverse;`, a Mode::Reverse clap variant (flags --strategy <A|B|C|D>, --continue, --status, --reset, --complete, --dry-run; no positional args), a forward! arm, and a --help intercept arm calling configs::reverse::CONFIG. Also commands/mod.rs is already registered for reverse (stub exists) so no change there.
  CONFIRM-WITH-SUPERVISOR #6: implementationContext (Strategy D persona path) has NO clap flag in the TS Commander.js surface — it is dispatcher-JSON-only. Confirm the standalone binary intentionally cannot trigger Strategy-D persona guidance (parity with TS).
  Session persistence reuses crate::io::project_root::find_project_root (markers .git/package.json/.gitignore/Cargo.toml/pyproject.toml, depth 10) — already exists. Session path uses std::env::temp_dir(). Timestamps via chrono formatted '%Y-%m-%dT%H:%M:%S%.3fZ' for ISO parity. All IO is blocking std::fs — no real async — fits poll_sync_future.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Flags are evaluated in fixed priority order, first match wins: --reset, --status, --complete, --continue, --strategy, then existing-session-detected, then initial analysis
  #   2. The reset flag deletes the session file (ignoring ENOENT) and returns message 'Session reset' with exit 0
  #   3. The status flag with no active session returns message 'No active reverse session' and exit 0; with a session it returns phase, strategy, strategyName, gapsDetected, progress (Step N of M), and a gapList marking files completed when index < currentStep-1
  #   4. The complete flag with no session returns 'No active reverse session to complete' exit 1; with a session that fails validateCompletion returns 'Cannot complete: not all steps are finished' exit 1; on success it deletes the session and returns a success systemReminder plus message '✓ Reverse ACDD session complete' exit 0
  #   5. The continue flag with no session returns 'No active reverse session' exit 1; with a session it increments the step counter, saves, and emits a systemReminder (Step N of M, Process file, run --complete on final step else --continue) plus a guidance line for the next file
  #   6. The strategy flag with no session returns 'No active reverse session' exit 1; with a session it sets the chosen strategy (A/B/C/D), totalSteps = gaps.files.length, moves phase to executing at step 1, saves, and emits a systemReminder (Step 1 of N, Strategy X (name), run --continue) plus guidance referencing the first file
  #   7. Strategy D with an implementationContext value bypasses the session check and returns persona-driven guidance built from foundation.json personas; this path is reachable only via the dispatcher JSON, not via any clap flag
  #   8. When no flag matches but a session file already exists: a parseable session returns existingSessionDetected with exit 1, currentPhase/currentStrategy/currentProgress, four suggestions, and a 'DO NOT start new session' systemReminder; an unparseable file returns 'Session file corrupted' exit 1
  #   9. When no flag matches and no session exists, the command runs initial analysis: analyzeProject (test/feature/implementation/coverage), detectGaps, suggestStrategy (priority A>B>C>D default A), and getStrategyName
  #   10. Initial analysis with --dry-run returns analysis/gaps/suggestedStrategy with message 'Dry-run mode - no session created' and a DRY-RUN systemReminder, and writes NO session file
  #   11. Initial analysis without --dry-run creates a 'gap-detection' session, saves it, and returns a 'Gap analysis complete' systemReminder, guidance, and effortEstimate; when totalGaps >= 100 it also adds pagination {total, perPage 50, page 1}, a summary, and a narrow-scope hint appended to guidance
  #   12. The session file path is os.tmpdir()/fspec-reverse-<hash>.json where <hash> is the first 12 hex chars of sha256(projectRoot) and projectRoot is found by walking up for boundary markers (.git, package.json, .gitignore, Cargo.toml, pyproject.toml) up to depth 10
  #   13. detectGaps selects files by the first matching gap: A=test files (tests>0 & features==0), B=feature files (features>0 & tests==0), C=unmapped coverage scenarios, D=implementation files without features excluding pure utilities (utils/format, utils/parse, utils/validate, helpers/, constants/)
  #   14. getEffortEstimate uses totalGaps (sum of all four counts): A => '<2t>-<3t> points', B => '<t>-<2t> points', C => '1 point total', D => '<3t>-<5t> points', else 'Unknown'
  #   15. The CLI wrapper prints only systemReminder, message, guidance, then suggestions (under 'Next steps:' as '  - <s>'), and exits with result.exitCode or 0; on a thrown error it prints 'Error: <msg>' to stderr and exits 1. analysis/gaps/effortEstimate/pagination live only in the structured result, not in CLI stdout
  #
  # EXAMPLES:
  #   1. Running `fspec reverse --reset` deletes any session file and prints 'Session reset' with exit 0
  #   2. Running `fspec reverse --status` with no session prints 'No active reverse session' exit 0
  #   3. Running `fspec reverse --status` with an active executing session prints phase, strategy, progress 'Step N of M' and the gap file list
  #   4. Running `fspec reverse --complete` with no session prints 'No active reverse session to complete' exit 1
  #   5. Running `fspec reverse --complete` on a session where currentStep < totalSteps prints 'Cannot complete: not all steps are finished' exit 1
  #   6. Running `fspec reverse --complete` on a finished session deletes it and prints a completion systemReminder plus '✓ Reverse ACDD session complete' exit 0
  #   7. Running `fspec reverse --continue` with no session prints 'No active reverse session' exit 1
  #   8. Running `fspec reverse --continue` on an active session advances the step and prints a 'Step N of M / Process file: <next>' systemReminder with --continue (or --complete on the final step) and a guidance line
  #   9. Running `fspec reverse --strategy=A` on a gap-detection session moves it to executing at Step 1 of N and prints a 'Strategy: A (Spec Gap Filling)' systemReminder plus first-file guidance
  #   10. Running `fspec reverse` in a project with 3 test files and no feature files creates a session and prints a 'Gap analysis complete' systemReminder suggesting Strategy A
  #   11. Running `fspec reverse --dry-run` in a project with gaps prints a DRY-RUN systemReminder and creates NO session file
  #   12. Running `fspec reverse` when a session already exists prints 'Existing reverse session detected' with four next-step suggestions and exit 1
  #
  # ========================================
  Background: User Story
    As a AI agent maintaining an existing codebase
    I want to run fspec reverse to analyze gaps and be guided step-by-step through a reverse ACDD session
    So that I can retroactively add specs and tests without losing my place across invocations

  Scenario: Clap exposes reverse as a subcommand with all six flags and prints byte-parity help
    Given the fspec Rust binary has been compiled
    When I run `fspec reverse --help` piped to non-TTY
    Then the command exits 0
    Then stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/reverse.txt
    Then stdout starts with a blank line followed by "REVERSE"

  Scenario: CLI reset deletes the session and prints Session reset
    Given a temp working directory marked as a project root with an active reverse session file
    When I run `fspec reverse --reset` from that directory
    Then the command exits 0
    Then stdout contains the substring "Session reset"

  Scenario: CLI status with no session prints no active session and exits 0
    Given a temp working directory marked as a project root with no reverse session file
    When I run `fspec reverse --status` from that directory
    Then the command exits 0
    Then stdout contains the substring "No active reverse session"

  Scenario: CLI complete with no session exits 1
    Given a temp working directory marked as a project root with no reverse session file
    When I run `fspec reverse --complete` from that directory
    Then the command exits with code 1
    Then stdout contains the substring "No active reverse session to complete"

  Scenario: CLI initial analysis prints gap-analysis guidance and exits 0
    Given a temp working directory marked as a project root with three *.test.ts files under src/__tests__, no spec/features directory, and no session file
    When I run `fspec reverse` from that directory
    Then the command exits 0
    Then stdout contains the substring "Gap analysis complete."
    Then stdout contains the substring "Strategy A (Spec Gap Filling)"

  Scenario: CLI existing session detected prints suggestions under Next steps and exits 1
    Given a temp working directory marked as a project root with a parseable executing reverse session file
    When I run `fspec reverse` from that directory
    Then the command exits with code 1
    Then stdout contains the substring "Existing reverse session detected"
    Then stdout contains the substring "Next steps:"
    Then stdout contains the substring "  - fspec reverse --continue"

  Scenario: Default combined TUI mode is preserved when no subcommand is provided
    Given the fspec Rust binary has reverse registered as a clap subcommand alongside the existing subcommands
    When I run `fspec --help`
    Then the command exits 0
    Then the help output lists reverse as an available subcommand
