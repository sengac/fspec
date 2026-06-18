@done
@querying
@cli
@rust
@RPC-294
Feature: Port reverse command to Rust

  """
  File layout: core impl codelet/fspec-core/src/commands/reverse.rs (rewrite stub); new type module codelet/fspec-core/src/types/reverse_session.rs (ReverseSession, GapAnalysis, AnalysisResult); help config codelet/fspec-core/src/help/configs/reverse.rs; CLI bridge codelet/fspec/src/reverse.rs; integration tests codelet/fspec/tests/cli_reverse.rs + core tests codelet/fspec-core/tests/reverse.rs; help fixture codelet/fspec/tests/fixtures/help/reverse.txt. Two feature files: reverse-rust-port.feature (dispatcher contract) + reverse-cli-subcommand.feature (clap surface).
  SHARED-FILE CHANGE (supervisor) #1: dispatch.rs reverse arm must change commands::reverse::run(args_json).await to commands::reverse::run(args_json, project_root).await — the ported signature adds project_root (parity: session hash + analysis need the project root, never env::current_dir()).
  SHARED-FILE CHANGE (supervisor) #2: codelet/fspec-core/Cargo.toml must add a sha2 dependency (sha2 = { workspace = true }) for the session-file hash. No hex crate — we will hex-encode the digest bytes manually with format!("{:02x}").
  SHARED-FILE CHANGE (supervisor) #3: codelet/fspec-core/src/types/mod.rs must register `pub mod reverse_session;`.
  SHARED-FILE CHANGE (supervisor) #4: codelet/fspec-core/src/help/configs/mod.rs must register `pub mod reverse;`.
  SHARED-FILE CHANGE (supervisor) #5: codelet/fspec/src/main.rs must add `mod reverse;`, a Mode::Reverse clap variant (flags --strategy <A|B|C|D>, --continue, --status, --reset, --complete, --dry-run; no positional args), a forward! arm, and a --help intercept arm calling configs::reverse::CONFIG. Also commands/mod.rs is already registered for reverse (stub exists) so no change there.
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
  #   3. The status flag with no active session returns message 'No active reverse session' and exit 0; with a session the structured result carries phase/strategy/strategyName/gapsDetected/progress/gapList, but the CLI wrapper logs none of those fields so the rendered output is EMPTY (only the no-session 'No active reverse session' message is ever printed)
  #   4. The complete flag with no session returns 'No active reverse session to complete' exit 1; with a session that fails validateCompletion returns 'Cannot complete: not all steps are finished' exit 1; on success it deletes the session and returns a success systemReminder plus message '✓ Reverse ACDD session complete' exit 0
  #   5. The continue flag with no session returns 'No active reverse session' exit 1; with a session it increments the step counter, saves, and emits a systemReminder (Step N of M, Process file, run --complete on final step else --continue) plus a guidance line for the next file
  #   6. The strategy flag with no session returns 'No active reverse session' exit 1; with a session it sets the chosen strategy (A/B/C/D), totalSteps = gaps.files.length, moves phase to executing at step 1, saves, and emits a systemReminder (Step 1 of N, Strategy X (name), run --continue) plus guidance referencing the first file
  #   7. Strategy D with an implementationContext value bypasses the session check and returns persona-driven guidance built from foundation.json personas; this path is reachable only via the dispatcher JSON, not via any clap flag
  #   8. When no flag matches but a session file already exists: a parseable session returns existingSessionDetected with exit 1, four suggestions, and a 'DO NOT start new session' systemReminder (the structured result also carries currentPhase/currentStrategy/currentProgress, but the CLI wrapper does NOT print them); an unparseable file returns 'Session file corrupted' exit 1
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
  #   3. Running `fspec reverse --status` with an active executing session prints NOTHING (the CLI wrapper logs only systemReminder/message/guidance/suggestions, none of which the status result carries)
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

  Scenario: Reset deletes the session and returns Session reset
    Given a project root tempdir with an active reverse session file on disk
    When I dispatch reverse with reset=true
    Then the dispatcher returns success=true
    Then the rendered output contains the substring "Session reset"
    Then the session file no longer exists on disk


  Scenario: Status with no session reports no active session
    Given a project root tempdir with no reverse session file
    When I dispatch reverse with status=true
    Then the dispatcher returns success=true
    Then the rendered output contains the substring "No active reverse session"


  Scenario: Status with an active executing session emits an empty rendered body
    Given a project root tempdir with an executing session having strategy=A strategyName='Spec Gap Filling' currentStep=2 totalSteps=3 and three gap files
    When I dispatch reverse with status=true
    Then the dispatcher returns success=true
    Then the rendered output is empty because the CLI wrapper logs none of the structured status fields
    Then the session file is left untouched by the read-only status query


  Scenario: Complete with no session fails with exit 1
    Given a project root tempdir with no reverse session file
    When I dispatch reverse with complete=true
    Then the dispatcher returns success=false
    Then the error message contains the substring "No active reverse session to complete"


  Scenario: Complete on an unfinished session is rejected
    Given a project root tempdir with an executing session having currentStep=1 and totalSteps=3
    When I dispatch reverse with complete=true
    Then the dispatcher returns success=false
    Then the error message contains the substring "Cannot complete: not all steps are finished"
    Then the session file still exists on disk


  Scenario: Complete on a finished session deletes it and returns success
    Given a project root tempdir with an executing session having currentStep=3 and totalSteps=3
    When I dispatch reverse with complete=true
    Then the dispatcher returns success=true
    Then the rendered output contains the substring "Session completed successfully."
    Then the rendered output contains the substring "✓ Reverse ACDD session complete"
    Then the session file no longer exists on disk


  Scenario: Continue with no session fails with exit 1
    Given a project root tempdir with no reverse session file
    When I dispatch reverse with continue=true
    Then the dispatcher returns success=false
    Then the error message contains the substring "No active reverse session"


  Scenario: Continue advances the step and emits next-file guidance
    Given a project root tempdir with an executing session having currentStep=1 totalSteps=3 and gap files [a.test.ts, b.test.ts, c.test.ts]
    When I dispatch reverse with continue=true
    Then the dispatcher returns success=true
    Then the rendered output contains the substring "Step 2 of 3"
    Then the rendered output contains the substring "Process file: b.test.ts"
    Then the rendered output contains the substring "run: fspec reverse --continue"
    Then the session file on disk shows currentStep=2


  Scenario: Continue into the final step instructs the agent to run complete
    Given a project root tempdir with an executing session having currentStep=2 totalSteps=3 and gap files [a.test.ts, b.test.ts, c.test.ts]
    When I dispatch reverse with continue=true
    Then the dispatcher returns success=true
    Then the rendered output contains the substring "Step 3 of 3"
    Then the rendered output contains the substring "run: fspec reverse --complete"


  Scenario: Strategy with no session fails with exit 1
    Given a project root tempdir with no reverse session file
    When I dispatch reverse with strategy='A'
    Then the dispatcher returns success=false
    Then the error message contains the substring "No active reverse session"


  Scenario: Strategy A on a gap-detection session moves it to executing at step 1
    Given a project root tempdir with a gap-detection session whose gaps.files are [a.test.ts, b.test.ts, c.test.ts]
    When I dispatch reverse with strategy='A'
    Then the dispatcher returns success=true
    Then the rendered output contains the substring "Step 1 of 3"
    Then the rendered output contains the substring "Strategy: A (Spec Gap Filling)"
    Then the rendered output contains the substring "Read test file: a.test.ts"
    Then the session file on disk shows phase='executing' and currentStep=1 and totalSteps=3


  Scenario: Strategy D with implementationContext returns persona-driven guidance without a session
    Given a project root tempdir with no reverse session file and a spec/foundation.json containing a persona named 'Shopper' with goals
    When I dispatch reverse with strategy='D' and implementationContext='discount calculator'
    Then the dispatcher returns success=true
    Then the rendered output contains the substring "REVERSE ACDD - PERSONA-DRIVEN DISCOVERY"
    Then the rendered output contains the substring "Shopper"
    Then no session file was created on disk


  Scenario: Existing session detected blocks a new analysis
    Given a project root tempdir with a parseable executing session having strategy=A strategyName='Spec Gap Filling' currentStep=2 totalSteps=3
    When I dispatch reverse with no flags
    Then the dispatcher returns success=false
    Then the error message contains the substring "Existing reverse session detected"
    Then the rendered output lists the four suggestions --continue, --status, --reset, --complete


  Scenario: Corrupt session file is reported as corrupted
    Given a project root tempdir with a reverse session file containing invalid JSON
    When I dispatch reverse with no flags
    Then the dispatcher returns success=false
    Then the error message contains the substring "Session file corrupted"


  Scenario: Initial analysis with tests and no features suggests Strategy A and creates a session
    Given a project root tempdir with three files under src/__tests__ matching *.test.ts and no spec/features directory and no session file
    When I dispatch reverse with no flags
    Then the dispatcher returns success=true
    Then the rendered output contains the substring "Gap analysis complete."
    Then the rendered output contains the substring "3 test files without features"
    Then the rendered output contains the substring "Strategy A (Spec Gap Filling)"
    Then a session file was created on disk with phase='gap-detection'


  Scenario: Dry-run previews analysis without writing a session
    Given a project root tempdir with three files under src/__tests__ matching *.test.ts and no spec/features directory and no session file
    When I dispatch reverse with dryRun=true
    Then the dispatcher returns success=true
    Then the rendered output contains the substring "DRY-RUN MODE"
    Then the rendered output contains the substring "Dry-run mode - no session created"
    Then no session file was created on disk


  Scenario: Flag priority resets before evaluating status
    Given a project root tempdir with an active reverse session file on disk
    When I dispatch reverse with both reset=true and status=true
    Then the dispatcher returns success=true
    Then the rendered output contains the substring "Session reset"
    Then the session file no longer exists on disk


  Scenario: CLI and dispatcher converge on the same fspec_core run function
    Given a project root tempdir with no reverse session file
    When I dispatch reverse with reset=true and also run the CLI subcommand fspec reverse --reset against the same project root
    Then both paths produce output containing "Session reset"
    Then the CLI bridge module codelet/fspec/src/reverse.rs contains no analysis, gap-detection, or rendering logic — its only computation is JSON arg marshalling

