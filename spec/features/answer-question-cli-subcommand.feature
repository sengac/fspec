@done
@RPC-196
Feature: fspec answer-question CLI subcommand
  """
  CLI bridge: rust/fspec/src/answer_question.rs — clap-derived struct mirroring TS Commander.js
  registration (src/commands/answer-question.ts:126-170). Surface:
  `fspec answer-question <workUnitId> <index> [--answer <text>] [--add-to <rule|assumption|none>]`.

  Bridge owns ONLY: clap argument parsing + JSON marshalling. All domain logic (existence/status guards,
  question/index validation, RuleItem construction, assumption push) lives in
  fspec_core::commands::answer_question::run.

  Stdout (success): '✓ Answered question: "<text>"'. If --answer was provided:
  '  Answer: "<answer>"'. If result.addedTo and result.addedContent:
  '  Added to <addedTo>: "<content>"' (TS uses chalk.cyan; substring assertions tolerate ANSI).
  Stderr (failure): '✗ Failed to answer question: <message>'; exit code 1.

  Help fixture captured from `node dist/index.js answer-question --help`. NOTE: TS help-ts file documents
  deprecated --add-to-rules / --add-to-assumptions flags that are NOT registered in Commander.js — the
  Rust port mirrors this Framing A: the help fixture lists those deprecated flags but clap does NOT
  register them.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want the standalone Rust fspec binary's answer-question subcommand to parse the same positional + flag arguments as the TypeScript Commander.js registration
    So that any existing TS-CLI-driven Example Mapping workflow keeps working after the cutover

  Scenario: Help output matches the captured TS fixture
    Given the fspec Rust binary is built and on PATH
    When I run `fspec answer-question --help`
    Then the exit code is 0
    And the stdout matches the canonical help fixture at rust/fspec/tests/fixtures/help/answer-question.txt
    And stdout starts with a blank line followed by 'ANSWER-QUESTION'

  Scenario: CLI successfully answers a question with addTo=rule and prints the success lines
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with questions=[{id:0,text:'Should we support OAuth?',deleted:false,createdAt:'x'}] and nextRuleId=0
    When I run `fspec answer-question AUTH-001 0 --answer "Yes, Google OAuth" --add-to rule` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '✓ Answered question: "Should we support OAuth?"'
    And stdout contains the substring 'Answer: "Yes, Google OAuth"'
    And stdout contains the substring 'Added to rules: "Yes, Google OAuth"'
    And spec/work-units.json on disk shows AUTH-001.rules[0].text='Yes, Google OAuth'
    And spec/work-units.json on disk shows AUTH-001.rules[0].id=0

  Scenario: CLI defaults --add-to to 'none' (no rule/assumption added)
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with questions=[{id:0,text:'Q?',deleted:false,createdAt:'x'}]
    When I run `fspec answer-question AUTH-001 0 --answer "Maybe"` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '✓ Answered question: "Q?"'
    And stdout contains the substring 'Answer: "Maybe"'
    And stdout does NOT contain the substring 'Added to'
    And spec/work-units.json on disk shows AUTH-001 has no rules added
    And spec/work-units.json on disk shows AUTH-001 has no assumptions added

  Scenario: CLI rejects non-specifying status with exit 1 and TS-parity error prefix
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=backlog with questions=[{id:0,text:'Q?',deleted:false,createdAt:'x'}]
    When I run `fspec answer-question AUTH-001 0 --answer "Yes" --add-to rule` in that tempdir
    Then the exit code is 1
    And stderr contains the substring '✗ Failed to answer question:'
    And stderr contains the substring "Can only answer questions during discovery/specification phase. AUTH-001 is in 'backlog' state."
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: CLI rejects out-of-range index with exit 1
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with questions=[{id:0,text:'Q1',deleted:false,createdAt:'x'}]
    When I run `fspec answer-question AUTH-001 99 --answer "X" --add-to rule` in that tempdir
    Then the exit code is 1
    And stderr contains the substring '✗ Failed to answer question:'
    And stderr contains the substring 'Invalid question index 99'

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with questions=[{id:0,text:'Q?',deleted:false,createdAt:'x'}]
    When I dispatch answer-question via fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' index=0 answer='Yes' addTo='rule'
    Then the dispatcher returns success=true
    And running `fspec answer-question AUTH-001 0 --answer "Twice" --add-to rule` afterwards exits 0
    And spec/work-units.json on disk shows AUTH-001.rules has length 2
    And the CLI bridge module rust/fspec/src/answer_question.rs contains NO inline question lookup, status guard, RuleItem construction, or file-write logic — its only computation is clap parsing + JSON arg marshalling
