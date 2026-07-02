@done
@rust
@RPC-268
Feature: Port remove-attachment command to Rust
  """
  Core impl at codelet/fspec-core/src/commands/remove_attachment.rs. Reuses io::ensure::ensure_work_units_file
  (auto-creates spec/work-units.json), io::locked_file::write_json_atomic (single atomic write), and
  io::time::iso8601_now (timestamps). Reads/mutates the attachments field via WorkUnit.extra['attachments'].
  Suffix-match (str::ends_with) parity with TS path.endsWith locates the target by file basename.
  Unlink errors are silently swallowed and surfaced as the "file was already missing" warning path —
  the array splice and JSON write always succeed. Two-front-doors: dispatcher and CLI both call
  fspec_core::commands::remove_attachment::run.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want to have the `remove-attachment` command added as a Rust parity port
    So that the standalone fspec binary and dispatcher can both remove file attachments from work units without falling back to the TypeScript implementation

  Scenario: Removing the only attachment empties the array and deletes the file from disk
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and attachments=['spec/attachments/AUTH-001/diagram.png']
    And the file spec/attachments/AUTH-001/diagram.png exists on disk with 1024 bytes
    When I dispatch remove-attachment with workUnitId='AUTH-001' and fileName='diagram.png'
    Then the dispatcher returns success
    And the rendered output contains the substring '✓ Attachment removed from work unit and file deleted'
    And the rendered output contains the substring '  File: spec/attachments/AUTH-001/diagram.png'
    And spec/attachments/AUTH-001/diagram.png NO LONGER exists on disk
    And spec/work-units.json on disk shows AUTH-001.attachments=[]
    And spec/work-units.json on disk shows AUTH-001.updatedAt is a freshly bumped ISO-8601 timestamp

  Scenario: Missing work unit surfaces the canonical error
    Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=specifying
    When I dispatch remove-attachment with workUnitId='ZZZ-999' and fileName='diagram.png'
    Then the dispatcher returns an error
    And the error message contains the substring "Work unit 'ZZZ-999' does not exist"
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Work unit with no attachments surfaces the no-attachments error
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with no attachments field
    When I dispatch remove-attachment with workUnitId='AUTH-001' and fileName='whatever.png'
    Then the dispatcher returns an error
    And the error message contains the substring "Work unit 'AUTH-001' has no attachments to remove"
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Work unit with empty attachments array surfaces the no-attachments error
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with attachments=[]
    When I dispatch remove-attachment with workUnitId='AUTH-001' and fileName='whatever.png'
    Then the dispatcher returns an error
    And the error message contains the substring "Work unit 'AUTH-001' has no attachments to remove"

  Scenario: Unknown filename surfaces the not-found error
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and attachments=['spec/attachments/AUTH-001/diagram.png']
    When I dispatch remove-attachment with workUnitId='AUTH-001' and fileName='nonexistent.png'
    Then the dispatcher returns an error
    And the error message contains the substring "Attachment 'nonexistent.png' not found for work unit 'AUTH-001'"
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Missing on-disk file degrades gracefully to the "already missing" warning
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and attachments=['spec/attachments/AUTH-001/ghost.png']
    And NO file exists at spec/attachments/AUTH-001/ghost.png on disk
    When I dispatch remove-attachment with workUnitId='AUTH-001' and fileName='ghost.png'
    Then the dispatcher returns success
    And the rendered output contains the substring '⚠ Attachment removed from work unit (file was already missing)'
    And spec/work-units.json on disk shows AUTH-001.attachments=[]

  Scenario: --keep-file preserves the file on disk but removes the array entry
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and attachments=['spec/attachments/AUTH-001/keep.pdf']
    And the file spec/attachments/AUTH-001/keep.pdf exists on disk
    When I dispatch remove-attachment with workUnitId='AUTH-001' fileName='keep.pdf' keepFile=true
    Then the dispatcher returns success
    And the rendered output contains the substring '✓ Attachment removed from work unit (file kept)'
    And spec/attachments/AUTH-001/keep.pdf STILL exists on disk
    And spec/work-units.json on disk shows AUTH-001.attachments=[]

  Scenario: Removing the middle of three attachments preserves order of the remaining two
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and attachments=['spec/attachments/AUTH-001/a.png','spec/attachments/AUTH-001/b.png','spec/attachments/AUTH-001/c.png']
    And the files for all three attachments exist on disk
    When I dispatch remove-attachment with workUnitId='AUTH-001' and fileName='b.png'
    Then the dispatcher returns success
    And spec/work-units.json on disk shows AUTH-001.attachments=['spec/attachments/AUTH-001/a.png','spec/attachments/AUTH-001/c.png']
    And spec/attachments/AUTH-001/a.png and spec/attachments/AUTH-001/c.png both STILL exist on disk
    And spec/attachments/AUTH-001/b.png NO LONGER exists on disk

  Scenario: Auto-creates spec/work-units.json when missing then reports the canonical missing-source error
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch remove-attachment with workUnitId='AUTH-001' and fileName='whatever.png'
    Then the dispatcher returns an error
    And the error message contains the substring "Work unit 'AUTH-001' does not exist"
    And spec/work-units.json now exists on disk with the canonical empty initial structure
