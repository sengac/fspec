@done
@rust
@RPC-170
Feature: Port add-attachment command to Rust
  """
  Core impl at codelet/fspec-core/src/commands/add_attachment.rs. Reuses io::ensure::ensure_work_units_file
  (auto-creates spec/work-units.json), io::locked_file::write_json_atomic (single atomic write), and
  io::time::iso8601_now (timestamps). The attachments field lives in WorkUnit.extra map keyed
  'attachments' (round-tripped via #[serde(flatten)] like the list-attachments port). SCOPE
  SIMPLIFICATIONS: Mermaid syntax validation (.mmd/.mermaid/.md) is OMITTED because mermaid+jsdom is
  inappropriate for fspec-core; Unicode whitespace fuzzy path resolution (BUG-130) is OMITTED because
  the U+202F directory-scan branch is macOS-specific. BUG-055 dedup is preserved: when source is at
  spec/attachments/<file>, unlink the source after copy. Two-front-doors: dispatcher and CLI bridge
  both invoke fspec_core::commands::add_attachment::run with the same args shape.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want to have the `add-attachment` command added as a Rust parity port
    So that the standalone fspec binary and dispatcher can both add file attachments to work units without falling back to the TypeScript implementation

  Scenario: Adds the first attachment, creates the per-work-unit directory, and bumps updatedAt
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    And a source file ./diagram.png exists with non-empty bytes
    When I dispatch add-attachment with workUnitId='AUTH-001' and filePath='./diagram.png'
    Then the dispatcher returns success
    And the rendered output contains the substring '✓ Attachment added successfully'
    And the rendered output contains the substring '  File: spec/attachments/AUTH-001/diagram.png'
    And spec/attachments/AUTH-001/diagram.png exists on disk with the same bytes as the source
    And spec/work-units.json on disk shows AUTH-001.attachments=['spec/attachments/AUTH-001/diagram.png']
    And spec/work-units.json on disk shows AUTH-001.updatedAt is a freshly bumped ISO-8601 timestamp

  Scenario: Description is echoed on a third output line when provided
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    And a source file ./diagram.png exists
    When I dispatch add-attachment with workUnitId='AUTH-001' filePath='./diagram.png' description='Auth flow diagram v2'
    Then the dispatcher returns success
    And the rendered output contains the substring '  Description: Auth flow diagram v2'

  Scenario: Description is omitted from output when not provided
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    And a source file ./diagram.png exists
    When I dispatch add-attachment with workUnitId='AUTH-001' and filePath='./diagram.png' (no description)
    Then the dispatcher returns success
    And the rendered output does NOT contain the substring 'Description:'

  Scenario: Missing work unit surfaces the canonical error and no file copy occurs
    Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=specifying
    And a source file ./diagram.png exists
    When I dispatch add-attachment with workUnitId='ZZZ-999' and filePath='./diagram.png'
    Then the dispatcher returns an error
    And the error message contains the substring "Work unit 'ZZZ-999' does not exist"
    And spec/attachments/ZZZ-999/ does NOT exist on disk
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Missing source file surfaces the canonical error using the ORIGINAL caller-supplied path
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    And no file exists at ./does-not-exist.png
    When I dispatch add-attachment with workUnitId='AUTH-001' and filePath='./does-not-exist.png'
    Then the dispatcher returns an error
    And the error message contains the substring "Source file './does-not-exist.png' does not exist"
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Re-adding the same file surfaces the duplicate-attachment error
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with attachments=['spec/attachments/AUTH-001/diagram.png']
    And the file spec/attachments/AUTH-001/diagram.png already exists on disk
    And a source file ./diagram.png exists
    When I dispatch add-attachment with workUnitId='AUTH-001' and filePath='./diagram.png'
    Then the dispatcher returns an error
    And the error message contains the substring "Attachment 'diagram.png' already exists for work unit 'AUTH-001'"
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: BUG-055 dedup unlinks the source when it lives directly in spec/attachments/
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    And a source file spec/attachments/foo.png exists (placed directly in the spec/attachments root)
    When I dispatch add-attachment with workUnitId='AUTH-001' and filePath='spec/attachments/foo.png'
    Then the dispatcher returns success
    And spec/attachments/AUTH-001/foo.png exists on disk
    And spec/attachments/foo.png NO LONGER exists on disk (source was deleted after copy)
    And spec/work-units.json on disk shows AUTH-001.attachments=['spec/attachments/AUTH-001/foo.png']

  Scenario: Adding a third attachment preserves the existing two and appends in array order
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with attachments=['spec/attachments/AUTH-001/a.png','spec/attachments/AUTH-001/b.png']
    And a source file ./c.png exists
    When I dispatch add-attachment with workUnitId='AUTH-001' and filePath='./c.png'
    Then the dispatcher returns success
    And spec/work-units.json on disk shows AUTH-001.attachments=['spec/attachments/AUTH-001/a.png','spec/attachments/AUTH-001/b.png','spec/attachments/AUTH-001/c.png']

  Scenario: Auto-creates spec/work-units.json when missing then reports the canonical missing-source error
    Given an empty project root directory with no spec/ subdirectory
    And a source file ./diagram.png exists
    When I dispatch add-attachment with workUnitId='AUTH-001' and filePath='./diagram.png'
    Then the dispatcher returns an error
    And the error message contains the substring "Work unit 'AUTH-001' does not exist"
    And spec/work-units.json now exists on disk with the canonical empty initial structure

  Scenario: Validates a .mmd attachment and rejects invalid Mermaid before copy
    Given a work unit AUTH-001 and a source file diagram.mmd containing invalid Mermaid
    When I dispatch add-attachment with workUnitId='AUTH-001' filePath='diagram.mmd'
    Then the dispatcher returns an error containing 'Invalid Mermaid'
    And no file is copied into spec/attachments/AUTH-001 and the work unit is unchanged

  Scenario: Validates mermaid fences inside a .md attachment and accepts fence-free markdown
    Given a work unit AUTH-001 and a notes.md containing one valid and one invalid mermaid fence
    When I dispatch add-attachment with workUnitId='AUTH-001' filePath='notes.md'
    Then the dispatcher returns an error naming the failing code block
    And a plain.md containing no mermaid fences is accepted and copied unchanged
