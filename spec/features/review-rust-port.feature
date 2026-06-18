@done
@quality-assurance
@cli
@rust
@RPC-295
Feature: Port review command to Rust

  """
  FUTURE CONSOLIDATION (follow-up task): review.rs will (a) inline a PRIVATE copy of getAgentConfig/formatAgentOutput mirroring init.rs's inlined AGENT_REGISTRY precedent, and (b) re-implement linked-feature lookup LOCALLY rather than pub-exporting from show_work_unit.rs (a shared/done command). A later follow-up should extract a shared agent_runtime module and migrate both init + review onto it, and consolidate the linked-feature lookup. Out of scope for RPC-295.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A missing work-unit id throws 'Work unit \'<id>\' does not exist'; an existing id always returns success=true with a text report (review never fails on findings)
  #   2. The report is one text blob with fixed sections in order: header, Issues Found (Critical then Warnings), Recommendations (only if any), ACDD Compliance, Coverage Analysis, Summary, then an agent-formatted AI deep-review reminder
  #   3. Feature linkage: no linked feature -> warning 'No linked feature files found'; first linked feature that fails Gherkin parse -> warning 'Invalid Gherkin syntax in feature file'
  #   4. ACDD compliance: rules present -> pass 'Example Mapping completed (...)'; else status!=backlog -> fail + recommendation. Feature-created-during-specifying and temporal-ordering also recorded as pass/fail
  #   5. Coverage: reads <feature>.coverage; 100% -> pass; >0 and <100 -> fail + recommendation and lists uncovered scenarios; missing coverage file -> 'No coverage data available'
  #   6. Coding-standards scan of linked test files flags as critical issues: ': any' type usage, CommonJS 'require(', and '.ts/.js' extension imports
  #   7. Overall assessment = 'CRITICAL ISSUES' if any critical; else 'NEEDS WORK' if any warnings or acddFailed; else 'PASS'. Priority Actions list derived from findings + current status
  #   8. The final AI deep-review reminder is wrapped via formatAgentOutput by detected agent: claude/system-reminder-capable -> <system-reminder> tags; ide/extension -> '**⚠️ IMPORTANT:**'; cli/default -> '**IMPORTANT:**' (priority FSPEC_AGENT env > spec/fspec-config.json > default)
  #
  # EXAMPLES:
  #   1. review an existing work unit with rules, 100% coverage and no test violations -> Overall Assessment: PASS, ACDD all passed
  #   2. review a work unit with no linked feature files -> warning 'No linked feature files found' and Coverage 'No coverage data available'
  #   3. review a work unit whose coverage is 50% -> Assessment 'NEEDS WORK', recommendation to add tests, and uncovered scenarios listed under Coverage Analysis
  #   4. review where a linked test file contains ': any' -> a Critical Issue 'Use of `any` type detected' and Assessment 'CRITICAL ISSUES'
  #   5. review a work unit id that does not exist -> throws/errors 'Work unit 'BOGUS-999' does not exist' (no report produced)
  #   6. review with spec/fspec-config.json agent=claude -> final reminder wrapped in <system-reminder>...; with agent=aider (cli) -> wrapped as '**IMPORTANT:** ...'
  #   7. review a work unit in specifying status with no Example Mapping -> ACDD Failed 'No Example Mapping data found' + recommendation 'Complete Example Mapping before specifying'
  #
  # QUESTIONS (ANSWERED):
  #   Q: @supervisor: Confirm signature run(args_json, project_root) with args {workUnitId:string}, matching show_work_unit.rs (TS review(workUnitId, options.cwd))?
  #   A: CONFIRMED: run(args_json, project_root) with args {workUnitId}. Matches show_work_unit/TS review(workUnitId, cwd).
  #
  #   Q: @supervisor: getAgentConfig + formatAgentOutput are NOT yet ported. Create a SHARED agent_runtime module (you wire the mod line) reusing init.rs's AGENT_REGISTRY, or inline a private copy in review.rs?
  #   A: INLINE a private copy in review.rs for now (mirrors init's inlined-registry precedent). No shared module in this card; future-consolidation follow-up will extract a shared agent_runtime module.
  #
  #   Q: @supervisor: scan_linked_features + projection logic is private to show_work_unit.rs. Re-implement linked-feature lookup locally in review.rs, or request a pub export from show_work_unit?
  #   A: RE-IMPLEMENT linked-feature lookup LOCALLY in review.rs. Do NOT pub-export from show_work_unit.rs (shared/done). Noted in future-consolidation architecture-note.
  #
  #   Q: @supervisor: TS coding-standards scan uses TS-specific regexes (': any', 'require(', '.ts/.js' imports) on linked TEST files. Port verbatim (faithful, even though linked files are now Rust tests), or adapt to Rust idioms?
  #   A: PORT VERBATIM the TS-specific regexes (': any', 'require(', '.ts/.js' imports). Behaviour is defined by the TS scan; reproduce exactly. Rust-idiom adaptation is a potential future follow-up, out of scope.
  #
  #   Q: @supervisor: Does review get a CLI subcommand + byte-exact --help fixture (like init), and is there a TS help/config to mirror? review.ts only registers via registerReviewCommand with a one-line description.
  #   A: review GETS a clap subcommand (Mode::Review { work_unit_id }) but NO rich help CONFIG / NO intercept_ts_help arm (delete-scenarios special-case: bare output). CLI test asserts subcommand exists + functional behaviour, not byte-parity help fixture.
  #
  # ========================================

  Background: User Story
    As a developer
    I want to run an end-to-end review of a work unit
    So that I can catch ACDD-compliance gaps and code-quality issues before marking it done

  Scenario: Reviewing a fully compliant work unit reports an overall PASS
    Given a work unit with Example Mapping rules
    And a linked feature file whose coverage is 100 percent
    And no linked test file contains coding-standards violations
    When I dispatch review for that work unit id
    Then the result success flag is true
    And the report header contains "REVIEW:" with the work unit id and title
    And the report Issues Found section reports "No critical issues detected."
    And the report ACDD Compliance section lists "Example Mapping completed"
    And the report Summary section contains "Overall Assessment: PASS"

  Scenario: Reviewing a work unit with no linked feature emits a warning and no coverage data
    Given a work unit in specifying status with no linked feature file
    When I dispatch review for that work unit id
    Then the result success flag is true
    And the report Warnings section contains "No linked feature files found"
    And the report Coverage Analysis section contains "No coverage data available"

  Scenario: Reviewing a work unit with partial coverage reports NEEDS WORK and lists uncovered scenarios
    Given a work unit with a linked feature file whose coverage is 50 percent
    And the coverage file lists one uncovered scenario
    When I dispatch review for that work unit id
    Then the report Summary section contains "Overall Assessment: NEEDS WORK"
    And the report includes a recommendation to "Add tests for uncovered scenarios"
    And the report Coverage Analysis section lists the uncovered scenario name

  Scenario: Reviewing a work unit whose linked test file uses the any type reports a critical issue
    Given a work unit with a linked feature file and 100 percent coverage
    And a linked test file whose contents include ": any"
    When I dispatch review for that work unit id
    Then the report Critical Issues section contains "Use of `any` type detected"
    And the report Summary section contains "Overall Assessment: CRITICAL ISSUES"

  Scenario: Reviewing a work unit id that does not exist returns an error
    Given a work units store that does not contain the id "BOGUS-999"
    When I dispatch review for the work unit id "BOGUS-999"
    Then the dispatch returns an error whose message is "Work unit 'BOGUS-999' does not exist"
    And no report is produced

  Scenario: The AI deep-review reminder is wrapped according to the configured agent
    Given a work unit with a linked feature file
    And spec/fspec-config.json selects the agent "claude"
    When I dispatch review for that work unit id
    Then the final AI deep-review reminder is wrapped in "<system-reminder>" tags
    When spec/fspec-config.json selects the cli agent "aider"
    And I dispatch review for that work unit id again
    Then the final AI deep-review reminder is prefixed with "**IMPORTANT:**"

  Scenario: Reviewing a non-backlog work unit with no Example Mapping reports an ACDD failure
    Given a work unit in specifying status with no rules or examples
    When I dispatch review for that work unit id
    Then the report ACDD Compliance section lists the failure "No Example Mapping data found"
    And the report includes a recommendation to "Complete Example Mapping before specifying"
