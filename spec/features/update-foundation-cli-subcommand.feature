@done
@foundation-management
@cli
@RPC-312
Feature: fspec update-foundation CLI subcommand

  """
  CLI bridge: codelet/fspec/src/update_foundation.rs — clap-derived struct mirroring the TS
  Commander.js registration (src/commands/update-foundation.ts:323-329). Surface:
  `fspec update-foundation <section> <content>` (both required positional arguments).
  The bridge marshals args into JSON {section, content} and forwards to
  fspec_core commands::update_foundation::run — NO domain logic in the bridge.

  Stdout (final-path success): '✓ Updated "<section>" section in FOUNDATION.md' followed by
  '  Updated: spec/foundation.json' and '  Regenerated: spec/FOUNDATION.md' (TS output.log lines;
  ANSI tolerated via substring match). Stdout (draft-path success):
  '✓ Updated "<section>" in foundation.json.draft' followed by '  Updated: spec/foundation.json.draft'.
  Stderr (failure): 'Error: <message>'; exit code 1 (mirrors TS output.error('Error:', error)).
  PARITY: draft-path systemReminder chaining is now ported in-core (scanDraftForNextField +
  generateFieldReminder), so the CLI prints the next field-by-field <system-reminder> after the
  draft line, matching the TS command. Help fixture captured from `node dist/index.js update-foundation --help`.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want the standalone Rust fspec binary's update-foundation subcommand to parse the same positional arguments as the TypeScript Commander.js registration
    So that any existing TS-CLI-driven foundation-editing script keeps working after the cutover

  Scenario: Help output matches the captured TS fixture
    Given the fspec Rust binary is built and on PATH
    When I run `fspec update-foundation --help`
    Then the exit code is 0
    And the stdout matches the canonical help fixture at codelet/fspec/tests/fixtures/help/update-foundation.txt

  Scenario: CLI updates a final foundation field and prints the success lines
    Given a project root tempdir with an existing spec/foundation.json and no foundation.json.draft
    When I run `fspec update-foundation projectName "Acme Tool"` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '✓ Updated "projectName" section in FOUNDATION.md'
    And stdout contains the substring 'Regenerated: spec/FOUNDATION.md'
    And spec/foundation.json on disk shows project.name='Acme Tool'

  Scenario: CLI updates the draft when a foundation.json.draft is present
    Given a project root tempdir with an existing spec/foundation.json.draft
    When I run `fspec update-foundation projectVision "Ship faster"` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '✓ Updated "projectVision" in foundation.json.draft'
    And spec/foundation.json.draft on disk shows project.vision='Ship faster'

  Scenario: CLI rejects an unknown section with exit 1 and the TS-parity error prefix
    Given a project root tempdir with an existing spec/foundation.json and no foundation.json.draft
    When I run `fspec update-foundation bogusSection "whatever"` in that tempdir
    Then the exit code is 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'Unknown section: "bogusSection"'
    And spec/foundation.json on disk is byte-equal to its pre-call contents

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with an existing spec/foundation.json and no foundation.json.draft
    When I dispatch update-foundation via fspec_core::dispatch::dispatch_command with section='projectName' content='Via Dispatcher'
    Then the dispatcher returns success=true
    And running `fspec update-foundation projectVision "Via CLI"` afterwards exits 0
    And spec/foundation.json on disk shows project.name='Via Dispatcher' and project.vision='Via CLI'
    And the CLI bridge module codelet/fspec/src/update_foundation.rs contains NO inline field mapping, validation, or file-write logic — its only computation is JSON arg marshalling
