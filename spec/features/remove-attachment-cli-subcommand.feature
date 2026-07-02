@done
@rust
@cli
@RPC-268
Feature: fspec remove-attachment CLI subcommand
  """
  CLI bridge: codelet/fspec/src/remove_attachment.rs — clap-derived struct mirroring TS Commander.js
  registration at src/commands/remove-attachment.ts:87-117. Surface: `fspec remove-attachment
  <workUnitId> <fileName> [--keep-file]`. Bridge owns ONLY: (a) clap arg parsing; (b) JSON
  marshalling; (c) stdout printing of the core's rendered output; (d) stderr printing of
  `Error: <message>` on failure (matches TS `output.error('Error:', error.message)`).

  All domain logic (existence, suffix-match, splice, unlink, atomic write) lives in
  fspec_core::commands::remove_attachment::run.

  Stdout (success): two lines — the status line ('✓ Attachment removed from work unit and file
  deleted' OR '⚠ Attachment removed from work unit (file was already missing)' OR
  '✓ Attachment removed from work unit (file kept)') followed by '  File: <path>'.
  Stderr (failure): 'Error: <message>'; exit code 1.

  Help fixture captured from `node dist/index.js remove-attachment --help`.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want the standalone Rust fspec binary's remove-attachment subcommand to parse the same positional + flag arguments as the TypeScript Commander.js registration
    So that any existing TS-CLI-driven attachment-cleanup script keeps working after the cutover

  Scenario: Help output matches the captured TS fixture byte-for-byte
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `fspec remove-attachment --help` piped to non-TTY (no color codes)
    Then the exit code is 0
    And stdout matches the canonical help fixture at codelet/fspec/tests/fixtures/help/remove-attachment.txt
    And stdout starts with a blank line followed by 'REMOVE-ATTACHMENT'
    And stdout contains the section header 'USAGE' followed by '  fspec remove-attachment <workUnitId> <fileName> [options]'
    And stdout contains the section header 'ARGUMENTS'
    And stdout contains the section header 'OPTIONS'
    And stdout contains the substring 'Fix: undefined'

  Scenario: CLI successfully removes an attachment and prints the canonical success block
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and attachments=['spec/attachments/AUTH-001/diagram.png']
    And the file spec/attachments/AUTH-001/diagram.png exists on disk
    When I run `fspec remove-attachment AUTH-001 diagram.png` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '✓ Attachment removed from work unit and file deleted'
    And stdout contains the substring '  File: spec/attachments/AUTH-001/diagram.png'
    And spec/attachments/AUTH-001/diagram.png NO LONGER exists on disk
    And spec/work-units.json on disk shows AUTH-001.attachments=[]

  Scenario: CLI passes --keep-file through to the core, preserving the file on disk
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and attachments=['spec/attachments/AUTH-001/keep.pdf']
    And the file spec/attachments/AUTH-001/keep.pdf exists on disk
    When I run `fspec remove-attachment AUTH-001 keep.pdf --keep-file` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '✓ Attachment removed from work unit (file kept)'
    And spec/attachments/AUTH-001/keep.pdf STILL exists on disk

  Scenario: CLI surfaces the "already missing" warning with exit 0 when file is gone
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and attachments=['spec/attachments/AUTH-001/ghost.png']
    And NO file exists at spec/attachments/AUTH-001/ghost.png
    When I run `fspec remove-attachment AUTH-001 ghost.png` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '⚠ Attachment removed from work unit (file was already missing)'

  Scenario: CLI exits 2 when the workUnitId positional argument is missing
    Given an empty directory is set as the current working directory
    When I run `fspec remove-attachment` (no positionals) from that directory
    Then the exit code is 2
    And stderr names the missing required argument

  Scenario: CLI exits 2 when the fileName positional argument is missing
    Given an empty directory is set as the current working directory
    When I run `fspec remove-attachment AUTH-001` (missing second positional) from that directory
    Then the exit code is 2
    And stderr names the missing required argument

  Scenario: CLI exits 1 with stderr prefix when the work unit does not exist
    Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=specifying
    When I run `fspec remove-attachment ZZZ-999 diagram.png` in that tempdir
    Then the exit code is 1
    And stderr contains the exact line "Error: Work unit 'ZZZ-999' does not exist"
    And stderr does NOT contain the substring 'Invalid args for fspec command'

  Scenario: CLI exits 1 when work unit has no attachments
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with no attachments field
    When I run `fspec remove-attachment AUTH-001 whatever.png` in that tempdir
    Then the exit code is 1
    And stderr contains the substring "Error: Work unit 'AUTH-001' has no attachments to remove"

  Scenario: CLI exits 1 when filename does not match any attachment
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and attachments=['spec/attachments/AUTH-001/diagram.png']
    When I run `fspec remove-attachment AUTH-001 missing.png` in that tempdir
    Then the exit code is 1
    And stderr contains the substring "Error: Attachment 'missing.png' not found for work unit 'AUTH-001'"

  Scenario: Default combined TUI mode is preserved when no subcommand is provided
    Given the fspec Rust binary has remove-attachment registered as a clap subcommand
    When I run `fspec --help`
    Then the help output lists remove-attachment as an available subcommand
    And the long-about description still documents that running fspec with no subcommand enters combined TUI mode

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and attachments=['spec/attachments/AUTH-001/a.png','spec/attachments/AUTH-001/b.png']
    And the corresponding files exist on disk
    When I dispatch remove-attachment via fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' fileName='a.png'
    Then the dispatcher returns success
    And the CLI bridge module codelet/fspec/src/remove_attachment.rs contains NO splice, file unlink, or atomic write logic — its only computation is JSON arg marshalling
