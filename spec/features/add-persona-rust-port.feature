@done
@mutation
@cli
@rust
@RPC-186
Feature: Port add-persona command to Rust
  """
  File layout: rewrite stub rust/fspec-core/src/commands/add_persona.rs; add CLI bridge rust/fspec/src/add_persona.rs; help config rust/fspec-core/src/help/configs/add_persona.rs; dispatcher test rust/fspec-core/tests/add_persona.rs; CLI test rust/fspec/tests/cli_add_persona.rs; help fixture rust/fspec/tests/fixtures/help/add-persona.txt
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

  Scenario: Append a persona to an existing foundation.json with real personas
    Given a project root tempdir with spec/foundation.json containing one real persona "Primary User"
    When I dispatch add-persona with name='QA Engineer', description='Tests features', goals=['Catch regressions']
    Then the dispatcher returns success=true
    And the returned data contains fileName='foundation.json'
    And the returned data contains removedPlaceholders=0
    And spec/foundation.json on disk shows personas has length 2
    And spec/foundation.json on disk shows the last persona has name='QA Engineer', description='Tests features', goals=['Catch regressions']

  Scenario: Multiple repeated goals are persisted in supplied order
    Given a project root tempdir with spec/foundation.json containing one real persona "Primary User"
    When I dispatch add-persona with name='Founder', description='Runs the company', goals=['Ship fast', 'Stay safe']
    Then the dispatcher returns success=true
    And spec/foundation.json on disk shows the last persona has goals=['Ship fast', 'Stay safe']
    And the returned data contains name='Founder', description='Runs the company', goals=['Ship fast', 'Stay safe']

  Scenario: A persona with no goals is stored with an empty goals array
    Given a project root tempdir with spec/foundation.json containing one real persona "Primary User"
    When I dispatch add-persona with name='Observer', description='Just watches' and no goals
    Then the dispatcher returns success=true
    And spec/foundation.json on disk shows the last persona has goals=[]

  Scenario: Draft precedence routes the write to foundation.json.draft
    Given a project root tempdir with both spec/foundation.json and spec/foundation.json.draft present
    When I dispatch add-persona with name='Drafted', description='Lives in the draft', goals=['Goal A']
    Then the dispatcher returns success=true
    And the returned data contains fileName='foundation.json.draft'
    And spec/foundation.json.draft on disk shows personas includes a persona named 'Drafted'
    And spec/foundation.json on disk is byte-equal to its pre-call contents

  Scenario: An all-placeholder personas array is cleared before the real persona is added
    Given a project root tempdir with spec/foundation.json whose only persona is named '[QUESTION: Who uses this?]'
    When I dispatch add-persona with name='Developer', description='Builds features', goals=['Ship quality code']
    Then the dispatcher returns success=true
    And the returned data contains removedPlaceholders=1
    And spec/foundation.json on disk shows personas has length 1
    And spec/foundation.json on disk shows the only persona has name='Developer'

  Scenario: A real persona alongside a placeholder suppresses placeholder removal
    Given a project root tempdir with spec/foundation.json containing one real persona "Primary User" and one placeholder persona '[DETECTED: Admin]'
    When I dispatch add-persona with name='Developer', description='Builds features', goals=['Ship quality code']
    Then the dispatcher returns success=true
    And the returned data contains removedPlaceholders=0
    And spec/foundation.json on disk shows personas has length 3

  Scenario: Missing foundation file and draft surface the not-found error and create nothing
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch add-persona with name='Nobody', description='No file', goals=[]
    Then the dispatcher returns success=false
    And the error message contains the substring "foundation.json not found"
    And spec/foundation.json does not exist on disk
    And spec/foundation.json.draft does not exist on disk

  Scenario: A foundation.json with no personas key initializes the array then appends
    Given a project root tempdir with spec/foundation.json that has no personas key
    When I dispatch add-persona with name='First', description='Initial persona', goals=['Goal X']
    Then the dispatcher returns success=true
    And spec/foundation.json on disk shows personas has length 1
    And spec/foundation.json on disk shows the only persona has name='First'

  Scenario: The written file uses 2-space indentation, a trailing newline, and preserves unknown top-level fields
    Given a project root tempdir with spec/foundation.json containing a custom top-level field "customKey" and one real persona "Primary User"
    When I dispatch add-persona with name='QA Engineer', description='Tests features', goals=['Catch regressions']
    Then the dispatcher returns success=true
    And spec/foundation.json on disk ends with a single trailing newline
    And spec/foundation.json on disk is indented with 2 spaces
    And spec/foundation.json on disk still contains the top-level field "customKey" unchanged
