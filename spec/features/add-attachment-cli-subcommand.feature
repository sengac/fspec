@done
@rust
@cli
@RPC-170
Feature: fspec add-attachment CLI subcommand
  """
  CLI bridge: rust/fspec/src/add_attachment.rs — clap-derived struct mirroring TS Commander.js
  registration at src/commands/add-attachment.ts:121-152. Surface: `fspec add-attachment <workUnitId>
  <filePath> [-d|--description <text>]`. Bridge owns ONLY: (a) clap arg parsing; (b) JSON
  marshalling; (c) stdout printing of the core's rendered output; (d) stderr printing of
  `Error: <message>` on failure (matches TS `output.error('Error:', error.message)`).

  All domain logic (existence checks, file copy, BUG-055 dedup, atomic write, dup detection) lives
  in fspec_core::commands::add_attachment::run.

  Stdout (success): three lines — '✓ Attachment added successfully' / '  File: <relPath>' /
  optional '  Description: <text>'.
  Stderr (failure): 'Error: <message>'; exit code 1.

  Help fixture captured from `node dist/index.js add-attachment --help`.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want the standalone Rust fspec binary's add-attachment subcommand to parse the same positional + flag arguments as the TypeScript Commander.js registration
    So that any existing TS-CLI-driven attachment-management script keeps working after the cutover

  Scenario: Help output matches the captured TS fixture byte-for-byte
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `fspec add-attachment --help` piped to non-TTY (no color codes)
    Then the exit code is 0
    And stdout matches the canonical help fixture at rust/fspec/tests/fixtures/help/add-attachment.txt
    And stdout starts with a blank line followed by 'ADD-ATTACHMENT'
    And stdout contains the section header 'USAGE' followed by '  fspec add-attachment <workUnitId> <filePath> [options]'
    And stdout contains the section header 'ARGUMENTS'
    And stdout contains the section header 'OPTIONS'
    And stdout contains the substring 'Fix: undefined'

  Scenario: CLI successfully adds an attachment and prints the canonical success block
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    And a source file diagram.png exists in the tempdir
    When I run `fspec add-attachment AUTH-001 diagram.png` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '✓ Attachment added successfully'
    And stdout contains the substring '  File: spec/attachments/AUTH-001/diagram.png'
    And spec/attachments/AUTH-001/diagram.png exists on disk
    And spec/work-units.json on disk shows AUTH-001.attachments=['spec/attachments/AUTH-001/diagram.png']

  Scenario: CLI passes --description through to the rendered output
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    And a source file diagram.png exists in the tempdir
    When I run `fspec add-attachment AUTH-001 diagram.png --description "Auth flow v2"` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '  Description: Auth flow v2'

  Scenario: CLI exits 1 when the workUnitId positional argument is missing (Commander usage-error parity)
    Given an empty directory is set as the current working directory
    When I run `fspec add-attachment` (no positionals) from that directory
    Then the exit code is 1
    And stderr names the missing required argument

  Scenario: CLI exits 1 when the filePath positional argument is missing (Commander usage-error parity)
    Given an empty directory is set as the current working directory
    When I run `fspec add-attachment AUTH-001` (missing second positional) from that directory
    Then the exit code is 1
    And stderr names the missing required argument

  Scenario: CLI exits 1 with stderr prefix when the work unit does not exist
    Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=specifying
    And a source file diagram.png exists
    When I run `fspec add-attachment ZZZ-999 diagram.png` in that tempdir
    Then the exit code is 1
    And stderr contains the exact line "Error: Work unit 'ZZZ-999' does not exist"
    And stderr does NOT contain the substring 'Invalid args for fspec command'

  Scenario: CLI exits 1 when the source file does not exist
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    And no file exists at ./missing.png
    When I run `fspec add-attachment AUTH-001 ./missing.png` in that tempdir
    Then the exit code is 1
    And stderr contains the substring "Error: Source file './missing.png' does not exist"

  Scenario: CLI exits 1 on duplicate attachment with byte-equality on the JSON file
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and attachments=['spec/attachments/AUTH-001/diagram.png']
    And the file spec/attachments/AUTH-001/diagram.png already exists
    And a source file diagram.png exists at the tempdir root
    When I run `fspec add-attachment AUTH-001 diagram.png` in that tempdir
    Then the exit code is 1
    And stderr contains the substring "Error: Attachment 'diagram.png' already exists for work unit 'AUTH-001'"
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Default combined TUI mode is preserved when no subcommand is provided
    Given the fspec Rust binary has add-attachment registered as a clap subcommand
    When I run `fspec --help`
    Then the help output lists add-attachment as an available subcommand
    And the long-about description still documents that running fspec with no subcommand enters combined TUI mode

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    And a source file diagram.png exists
    When I dispatch add-attachment via fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' filePath='diagram.png'
    Then the dispatcher returns success
    And the CLI bridge module rust/fspec/src/add_attachment.rs contains NO file copy, work-unit lookup, or atomic write logic — its only computation is JSON arg marshalling
