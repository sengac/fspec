@done
@BUG-523
@infrastructure
@file-ops
@concurrency
@rust
@bug-fix
@critical
Feature: Commands write the spec directory they read
  """
  On 2026-11-05 'fspec update-work-unit-status BUG-481 implementing' reported
  success but the board kept 'testing'. It was not a race and nothing bypassed
  the BUG-477 lock: the command ran with rust/ as its working directory. The
  CLI bridges pass the working directory to fspec-core as the project root.
  The spec lock (io::spec_lock::lock_spec_dir) and the loaders (ensure_*)
  resolve the spec directory through find_or_create_spec_directory, which
  walks up to the repository's spec/ because rust/spec does not exist, but the
  command wrote project_root/spec/work-units.json, so the new board went to a
  fresh rust/spec/work-units.json and the real one was never changed. About
  140 fspec-core commands build write paths from project_root/spec this way.
  Fix: fspec-core io::project_root::resolve_project_root(cwd) is the
  non-creating twin of find_or_create_spec_directory (which now uses it). The
  fspec CLI bridges (common::spec_project_root) and dispatch_command resolve
  the project root with it before calling a command, so reads, the lock and
  writes all use one spec/; init keeps initialising its working directory.
  lock_spec_dir refuses a root whose spec directory is not <root>/spec, so a
  caller that skips the resolution fails instead of writing a second board.
  The installed fspec and the HEAD build share spec/.fspec-write.lock and the
  same locking code; concurrent status changes, add-rule, create-bug and
  link-coverage lose no update.
  Tests: rust/fspec-core/tests/bug523_commands_write_the_spec_directory_they_read.rs
  and rust/fspec/tests/bug523_cli_commands_from_a_subdirectory.rs.
  """

  Background: User Story
    As a developer or agent running fspec commands, possibly several at once and possibly from a subdirectory of the project
    I want to have every command write the same spec/ board it read and locked
    So that a reported status change is never written to a stray board and lost

  Scenario: A status change run from a subdirectory lands in the project's board
    Given a project whose board has the work unit "STAT-001" in specifying
    And a subdirectory "rust" of the project with its own Cargo.toml
    When "fspec update-work-unit-status STAT-001 blocked" runs with "rust" as its working directory
    Then it succeeds
    And "STAT-001" is blocked in the project's spec/work-units.json
    And no spec directory exists in "rust"

  Scenario: Every CLI bridge takes its project root from the shared resolver
    Given the fspec CLI bridge modules that call an fspec-core command
    When their source is read
    Then every bridge except init and astgrep resolves its project root with common::spec_project_root
    And no bridge passes the raw working directory to an fspec-core command

  Scenario: An agent tool call with a subdirectory as project root writes the project's board
    Given a project whose board has the work unit "RULE-001"
    And a subdirectory "rust" of the project with its own Cargo.toml
    When dispatch_command runs add-rule for "RULE-001" with "rust" as the project root
    Then it succeeds
    And "RULE-001" has the rule in the project's spec/work-units.json
    And no spec directory exists in "rust"

  Scenario: A writer given a root whose spec directory lies elsewhere fails and changes nothing
    Given a project whose board has the work unit "RULE-001"
    And a subdirectory "rust" of the project with its own Cargo.toml
    When add-rule runs directly with "rust" as the project root
    Then it fails naming the project's spec directory
    And spec/work-units.json is byte-for-byte unchanged
    And no spec directory exists in "rust"

  Scenario: init run from a subdirectory still initialises that subdirectory
    Given a project with a spec directory
    And a subdirectory "rust" of the project with its own Cargo.toml
    When dispatch_command runs init for the claude agent with "rust" as the project root
    Then it succeeds
    And "rust/spec/fspec-config.json" records the claude agent
    And the project's spec directory has no fspec-config.json

  Scenario: Concurrent writers from the project root and from a subdirectory lose no update
    Given a project whose board has eight status work units, four rule work units and a feature with four scenarios
    And a subdirectory "rust" of the project with its own Cargo.toml
    When separate processes at the same time change statuses from the project root and from "rust", add rules, create bugs and link coverage
    Then every process succeeds
    And every status change, rule and bug is in the project's spec/work-units.json
    And every coverage link is in the feature's coverage file
    And no spec directory exists in "rust"
