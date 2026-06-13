@done
@mutation
@cli
@rust
@RPC-186
Feature: fspec add-persona CLI subcommand

  """
  File layout: rewrite stub codelet/fspec-core/src/commands/add_persona.rs; add CLI bridge codelet/fspec/src/add_persona.rs; help config codelet/fspec-core/src/help/configs/add_persona.rs; dispatcher test codelet/fspec-core/tests/add_persona.rs; CLI test codelet/fspec/tests/cli_add_persona.rs; help fixture codelet/fspec/tests/fixtures/help/add-persona.txt
  Core signature: pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> (2-arg form like add_diagram/add_bounded_context). Args (camelCase): { name: String, description: String, goals: Vec<String> default [] }. Does NOT use ensure_foundation_file — reads draft-or-final directly with inline draft-precedence and errors on ENOENT (no auto-create).
  Wiring intent (SHARED FILES — supervisor must apply): register add_persona module in commands/mod.rs; route kebab 'add-persona' in canonical.rs + dispatch.rs to the 2-arg run; register help config in help/configs/mod.rs; add Mode::AddPersona clap variant in fspec/src/main.rs. Worker will REQUEST these, not edit them.
  Write-format divergence: TS writes JSON.stringify(foundation, null, 2) + '\n' (trailing newline), but the shared write_json_atomic helper deliberately omits the trailing newline. For byte parity add-persona writes the foundation file inline (to_string_pretty + '\n') rather than via write_json_atomic. serde_json preserve_order keeps unknown top-level fields and key order.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Draft precedence: when spec/foundation.json.draft exists it is the mutation target; otherwise spec/foundation.json is used
  #   2. If the target foundation file does not exist (ENOENT), return a 'foundation.json not found' error and do NOT auto-create the file
  #   3. When the personas array is absent it is initialized to an empty array before the new persona is appended
  #   4. If personas contains ONLY placeholders (every entry has [QUESTION: or [DETECTED: in name, description, or any goal) the array is cleared first and the removed count is reported
  #   5. Placeholder removal does NOT occur when at least one real (non-placeholder) persona already exists; existing personas are preserved
  #   6. The new persona is appended as { name, description, goals }; goals defaults to an empty array when --goal is not supplied
  #   7. The file is written with 2-space indentation and a trailing newline, preserving all other top-level foundation fields verbatim
  #   8. Success output reports the target filename (foundation.json.draft vs foundation.json), the name, the description, and the goals joined with ', '
  #
  # EXAMPLES:
  #   1. foundation.json exists with real personas; add-persona 'QA Engineer' 'Tests features' --goal 'Catch regressions' appends the persona and returns success
  #   2. add-persona with two repeated --goal flags ('Ship fast', 'Stay safe') stores both goals and the success line shows 'Goals: Ship fast, Stay safe'
  #   3. add-persona with no --goal flag stores an empty goals array and the success line shows 'Goals: ' with nothing after it
  #   4. spec/foundation.json.draft exists alongside foundation.json; add-persona writes to the draft and the success line reports 'Added persona to foundation.json.draft'
  #   5. foundation.json personas contains only [{name: '[QUESTION: Who uses this?]', ...}]; add-persona removes 1 placeholder, prints 'Removed 1 placeholder persona(s)', then adds the real persona
  #   6. foundation.json personas has one real persona AND one placeholder; add-persona keeps both existing entries and appends the new one (no placeholder removal)
  #   7. Neither foundation.json nor its draft exists; add-persona returns the 'foundation.json not found' error and creates no file
  #   8. foundation.json exists but has no personas key; add-persona initializes personas to [] then appends the new persona
  #
  # ========================================

  Background: User Story
    As a fspec maintainer porting the CLI to Rust
    I want to add a persona to spec/foundation.json (or its draft) via the Rust fspec-core add-persona command
    So that the Rust binary reaches behavioural parity with the TypeScript add-persona command

  Scenario: Help output matches the captured TS fixture
    Given the fspec Rust binary is built and on PATH
    When I run `fspec add-persona --help`
    Then the exit code is 0
    And the stdout matches the canonical help fixture at codelet/fspec/tests/fixtures/help/add-persona.txt
    And stdout starts with a blank line followed by 'ADD-PERSONA'

  Scenario: CLI appends a persona and prints the multi-line success block
    Given a project root tempdir with spec/foundation.json containing one real persona "Primary User"
    When I run `fspec add-persona "QA Engineer" "Tests features" --goal "Catch regressions"` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '✓ Added persona to foundation.json'
    And stdout contains the substring '  Name: QA Engineer'
    And stdout contains the substring '  Description: Tests features'
    And stdout contains the substring '  Goals: Catch regressions'
    And spec/foundation.json on disk shows personas has length 2

  Scenario: CLI joins multiple --goal flags with a comma and space
    Given a project root tempdir with spec/foundation.json containing one real persona "Primary User"
    When I run `fspec add-persona "Founder" "Runs the company" --goal "Ship fast" --goal "Stay safe"` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '  Goals: Ship fast, Stay safe'

  Scenario: CLI with no --goal flag prints an empty Goals line
    Given a project root tempdir with spec/foundation.json containing one real persona "Primary User"
    When I run `fspec add-persona "Observer" "Just watches"` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '  Goals: '
    And spec/foundation.json on disk shows the last persona has goals=[]

  Scenario: CLI reports placeholder removal before the success block
    Given a project root tempdir with spec/foundation.json whose only persona is named '[QUESTION: Who uses this?]'
    When I run `fspec add-persona "Developer" "Builds features" --goal "Ship quality code"` in that tempdir
    Then the exit code is 0
    And stdout contains the substring 'Removed 1 placeholder persona(s)'
    And stdout contains the substring '✓ Added persona to foundation.json'

  Scenario: CLI writes to the draft when foundation.json.draft exists
    Given a project root tempdir with both spec/foundation.json and spec/foundation.json.draft present
    When I run `fspec add-persona "Drafted" "Lives in the draft"` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '✓ Added persona to foundation.json.draft'

  Scenario: CLI reports the missing-foundation error with exit 1
    Given an empty project root directory with no spec/ subdirectory
    When I run `fspec add-persona "Nobody" "No file"` in that tempdir
    Then the exit code is 1
    And stderr contains the substring 'foundation.json not found'
    And spec/foundation.json does not exist on disk

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with spec/foundation.json containing one real persona "Primary User"
    When I dispatch add-persona via fspec_core::dispatch::dispatch_command with name='Core User' description='From dispatcher'
    Then the dispatcher returns success=true
    And running `fspec add-persona "Cli User" "From cli"` afterwards exits 0
    And spec/foundation.json on disk shows personas has length 3
    And the CLI bridge module codelet/fspec/src/add_persona.rs contains NO inline placeholder, file-read, or file-write logic — its only computation is JSON arg marshalling and stdout rendering
