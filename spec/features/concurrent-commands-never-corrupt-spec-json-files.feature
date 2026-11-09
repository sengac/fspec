@done
@infrastructure
@file-ops
@concurrency
@rust
@bug-fix
@critical
@BUG-477
Feature: Concurrent commands never corrupt spec JSON files
  """
  codelet-common::spec_lock owns the protocol: SpecLock is an fs2 exclusive advisory lock (flock on Unix, LockFileEx on Windows) on spec/.fspec-write.lock, a file that is never renamed or deleted, so every process contends on one inode; the kernel drops the lock when the holder exits, so the TS mkdir lock's 10 s stale detection is not needed. A process-wide registry keyed by the canonical directory makes it reentrant per thread. replace_file_atomically writes a uuid-named temp file in the target's directory, fsyncs, renames and fsyncs the directory.
  fspec-core: io::locked_file::write_json_atomic (and write_bytes_atomic) take the spec lock when the target is a spec state file (directly inside spec/; a no-op when the caller already holds it) and replace every file through replace_file_atomically; the old fixed '<file>.tmp' path, which two writers shared and which let one truncate and over-write the other's renamed file, is gone. read_or_init_json initialises a missing spec file under the lock and re-checks existence. Every command that read-modify-writes spec state takes io::spec_lock::lock_spec_dir before its first read and holds the guard until it returns (about 80 commands, plus init's fspec-config.json).
  Outside fspec-core: codelet-core work_units_write::move_work_unit and scheduler crud/trigger/catch_up, codelet-agent-loop schedule_handler, and codelet-common fspec_config::write_config (project scope writes spec/fspec-config.json) switch from the mkdir '<file>.lock' protocol or plain writes to SpecLock plus replace_file_atomically.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Every write of a spec JSON file (work-units.json, tags.json, epics.json, prefixes.json, foundation.json and its draft, schedules.json, fspec-hooks.json, fspec-config.json) holds one exclusive inter-process lock on the spec directory from before the command's first read of spec state until after its last write, so concurrent read-modify-write commands are serialised and no update is lost
  #   2. Every write replaces the file atomically: the JSON goes to a uniquely named temp file in the same directory, is fsynced, then renamed over the target, and the directory is fsynced; a failed write removes its temp file and leaves the target unchanged, so the file is never partial or over-long
  #   3. Readers take no lock: an atomic rename means a reader sees either the previous or the next complete file
  #   4. Lock acquisition waits on the OS lock, which hands a busy lock to the next waiter, for at most one minute (or the whole number of milliseconds in FSPEC_SPEC_LOCK_TIMEOUT_MS; any other value is an error) and then fails with 'Failed to acquire file lock on <lock file> after waiting <n> ms. Check for stuck fspec processes.' leaving every file unchanged. This replaces LOCK-001's polling retries and five-second budget (rules 5 and 11), which starved eight concurrent writers once writes fsync under build load; a crashed holder needs no timeout because the kernel drops its lock
  #   5. The lock is reentrant within one thread, so a command that calls another locking helper does not deadlock, while other threads and processes still wait for it
  #   6. Writers outside fspec-core (the TUI work-unit reorder in codelet-core, the scheduler in codelet-core and codelet-agent-loop) use the same spec lock and atomic replace as fspec commands
  #   7. Coverage sidecars (spec/features/*.feature.coverage) are not spec state files: their read-modify-writes hold the REFAC-020 sidecar transaction (now also in add-scenario, update-scenario, delete-scenario and delete-scenarios) and their writes are replaced atomically, without the spec lock
  #
  # EXAMPLES:
  #   1. Eight separate processes each add ten rules to one work unit at the same time; afterwards spec/work-units.json parses and the work unit has all 80 rules
  #   2. Separate processes register tags, create epics, create prefixes and add personas at the same time; tags.json, epics.json, prefixes.json and foundation.json each parse and contain every entry
  #   3. A process repeatedly writes a long board and then a short one while other processes write; a reader polling spec/work-units.json always parses it and never finds bytes after the closing brace (the 289-byte tail seen 2026-11-04)
  #   4. After concurrent writes no temp file is left in spec/
  #   5. Another process holds the spec lock for longer than the retry budget; fspec add-rule fails with 'Failed to acquire file lock' and spec/work-units.json is byte-for-byte unchanged
  #   6. A TUI reorder (move_work_unit) and a scheduler update wait for the spec lock held by another process and fail with the same lock message when it is not released
  #   7. A thread that holds the spec lock acquires it again without deadlock, and another process cannot take it until the outer guard is dropped
  #
  # ========================================
  Background: User Story
    As a developer or agent running several fspec commands at the same time
    I want to have every spec JSON write serialised by one exclusive lock and replaced atomically
    So that the board and the other spec files are never corrupted and no update is lost

  Scenario: Concurrent board writers in separate processes lose no update
    Given a project whose board has the work unit "LOCK-001"
    When eight separate processes each add ten rules to "LOCK-001" at the same time
    Then every process succeeds
    And spec/work-units.json parses as one JSON document
    And "LOCK-001" has all 80 rules

  Scenario: Concurrent writers of tags, epics, prefixes and foundation lose no update
    Given a project with tags.json, epics.json, prefixes.json and foundation.json
    When separate processes register tags, create epics, create prefixes and add personas at the same time
    Then every process succeeds
    And tags.json, epics.json, prefixes.json and foundation.json each parse and contain every entry

  Scenario: A shorter write never leaves the tail of a longer one
    Given a writer process that alternately writes a long and a short spec/work-units.json
    And other processes adding rules at the same time
    When a reader process polls spec/work-units.json throughout
    Then every read parses as one JSON document with nothing after its closing brace

  Scenario: Concurrent writes leave no temp files behind
    Given a project where separate processes have written spec JSON files at the same time
    When they have all finished
    Then spec/ holds no temp file next to the JSON files

  Scenario: A writer that cannot get the lock fails and changes nothing
    Given another process holds the spec lock and does not release it
    When add-rule runs against the same project
    Then it fails with "Failed to acquire file lock" after the lock budget
    And spec/work-units.json is byte-for-byte unchanged
