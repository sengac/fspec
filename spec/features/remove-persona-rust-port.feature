@done
@mutation
@cli
@rust
@RPC-277
Feature: Port remove-persona command to Rust
  """
  File layout: rewrite stub rust/fspec-core/src/commands/remove_persona.rs; add CLI bridge rust/fspec/src/remove_persona.rs; help config rust/fspec-core/src/help/configs/remove_persona.rs; dispatcher test rust/fspec-core/tests/remove_persona.rs; CLI test rust/fspec/tests/cli_remove_persona.rs; help fixture rust/fspec/tests/fixtures/help/remove-persona.txt
  Core signature: pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> (2-arg form). Args (camelCase): { name: String }. Does NOT use ensure_foundation_file — inline draft-precedence read, errors on ENOENT (no auto-create). Returns JSON { success: true, fileName, name }.
  Wiring intent (SHARED FILES — supervisor must apply): register remove_persona module in commands/mod.rs; route kebab 'remove-persona' in canonical.rs + dispatch.rs to the 2-arg run; register help config in help/configs/mod.rs; add Mode::RemovePersona clap variant in fspec/src/main.rs. Worker will REQUEST these, not edit them.
  Write-format divergence: TS writes JSON.stringify(foundation, null, 2) + '\n' (trailing newline); the shared write_json_atomic helper omits the trailing newline, so remove-persona writes the foundation file inline (to_string_pretty + '\n') for byte parity. serde_json preserve_order keeps unknown top-level fields and key order. CLI bridge does ONLY arg marshalling + stdout/stderr rendering (exit 1 on error).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Draft precedence: when spec/foundation.json.draft exists it is the mutation target; otherwise spec/foundation.json is used
  #   2. If the target foundation file does not exist (ENOENT), return a 'foundation.json not found' error and do NOT auto-create the file
  #   3. If personas is absent or empty, return error 'Persona "{name}" not found' (with the note that no personas exist in the foundation) and leave the file untouched
  #   4. Personas are matched by exact, case-sensitive name
  #   5. If no persona matches the name (but others exist), return error 'Persona "{name}" not found' and list the available persona names joined with ', '
  #   6. Only the FIRST persona whose name matches is removed (findIndex + splice semantics)
  #   7. The file is written with 2-space indentation and a trailing newline, preserving all other top-level foundation fields verbatim
  #   8. On success the output reports the removed persona name and the target filename (foundation.json.draft vs foundation.json)
  #
  # EXAMPLES:
  #   1. foundation.json contains personas 'Primary User' and 'Admin'; remove-persona 'Admin' removes it, leaving only 'Primary User', and returns success
  #   2. spec/foundation.json.draft exists with persona 'Drafted'; remove-persona 'Drafted' edits the draft and the success line reports 'Removed persona "Drafted" from foundation.json.draft'
  #   3. foundation.json has personas 'Primary User' and 'Admin'; remove-persona 'Ghost' fails with 'Persona "Ghost" not found' and lists 'Available personas: Primary User, Admin'
  #   4. foundation.json has an empty personas array; remove-persona 'Admin' fails with 'Persona "Admin" not found' and notes that no personas exist in the foundation
  #   5. Neither foundation.json nor its draft exists; remove-persona 'Admin' fails with 'foundation.json not found' and creates no file
  #   6. foundation.json has persona 'Primary User'; remove-persona 'primary user' (lowercase) fails as not-found because matching is case-sensitive
  #   7. foundation.json has two personas both named 'Dup'; remove-persona 'Dup' removes only the first occurrence, leaving one 'Dup' persona
  #
  # ========================================
  Background: User Story
    As a fspec maintainer porting the CLI to Rust
    I want to remove a persona from spec/foundation.json (or its draft) by name via the Rust fspec-core remove-persona command
    So that the Rust binary reaches behavioural parity with the TypeScript remove-persona command

  Scenario: Remove an existing persona by exact name
    Given a project root tempdir with spec/foundation.json containing personas 'Primary User' and 'Admin'
    When I dispatch remove-persona with name='Admin'
    Then the dispatcher returns success=true
    And the returned data contains fileName='foundation.json'
    And the returned data contains name='Admin'
    And spec/foundation.json on disk shows personas has length 1
    And spec/foundation.json on disk shows the only persona has name='Primary User'

  Scenario: Draft precedence routes the removal to foundation.json.draft
    Given a project root tempdir with spec/foundation.json.draft containing persona 'Drafted'
    When I dispatch remove-persona with name='Drafted'
    Then the dispatcher returns success=true
    And the returned data contains fileName='foundation.json.draft'
    And spec/foundation.json.draft on disk shows personas has length 0

  Scenario: Removing a non-existent name lists the available personas
    Given a project root tempdir with spec/foundation.json containing personas 'Primary User' and 'Admin'
    When I dispatch remove-persona with name='Ghost'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Persona "Ghost" not found'
    And the error message contains the substring 'Available personas: Primary User, Admin'
    And spec/foundation.json on disk is byte-equal to its pre-call contents

  Scenario: Removing from an empty personas array reports that no personas exist
    Given a project root tempdir with spec/foundation.json whose personas array is empty
    When I dispatch remove-persona with name='Admin'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Persona "Admin" not found'
    And the error message contains the substring 'No personas exist in foundation'
    And spec/foundation.json on disk is byte-equal to its pre-call contents

  Scenario: Missing foundation file and draft surface the not-found error and create nothing
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch remove-persona with name='Admin'
    Then the dispatcher returns success=false
    And the error message contains the substring "foundation.json not found"
    And spec/foundation.json does not exist on disk
    And spec/foundation.json.draft does not exist on disk

  Scenario: Name matching is case-sensitive
    Given a project root tempdir with spec/foundation.json containing persona 'Primary User'
    When I dispatch remove-persona with name='primary user'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Persona "primary user" not found'
    And spec/foundation.json on disk is byte-equal to its pre-call contents

  Scenario: Only the first matching persona is removed when names are duplicated
    Given a project root tempdir with spec/foundation.json containing two personas both named 'Dup'
    When I dispatch remove-persona with name='Dup'
    Then the dispatcher returns success=true
    And spec/foundation.json on disk shows personas has length 1
    And spec/foundation.json on disk shows the only persona has name='Dup'

  Scenario: The written file uses 2-space indentation, a trailing newline, and preserves unknown top-level fields
    Given a project root tempdir with spec/foundation.json containing a custom top-level field "customKey" and personas 'Primary User' and 'Admin'
    When I dispatch remove-persona with name='Admin'
    Then the dispatcher returns success=true
    And spec/foundation.json on disk ends with a single trailing newline
    And spec/foundation.json on disk is indented with 2 spaces
    And spec/foundation.json on disk still contains the top-level field "customKey" unchanged
