@LOCK-001
Feature: Implement file locking for concurrent access safety
  """
  Three-Layer Lock Architecture: Layer 1 (Inter-Process) = proper-lockfile with mkdir strategy for cross-process coordination; Layer 2 (In-Process) = Readers-writer pattern using promises and queues for concurrent reads; Layer 3 (Atomic Writes) = Temp file + rename for POSIX atomic write guarantees
  LockedFileManager Singleton Pattern: Single instance coordinates all in-process locks via Map<string, number> for read counts and Set<string> for write locks, with separate waiting queues (Array<() => void>) for blocked readers and writers
  proper-lockfile Configuration: stale=10000ms (10 second timeout for crashed processes), retries={retries:10, minTimeout:50ms, maxTimeout:500ms} for exponential backoff, realpath=false for symlink support and performance
  Affected Files: work-units.json (230+ work units, high contention), foundation.json, tags.json, prefixes.json, epics.json, example-map.json, fspec-hooks.json - ALL must use LockedFileManager
  Migration Strategy: (1) Create src/utils/file-manager.ts with LockedFileManager class, (2) Refactor ensure-files.ts to use fileManager singleton, (3) Update all commands to use fileManager.transaction() for read-modify-write, (4) Add comprehensive concurrency tests
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. File locking MUST use three-layer architecture: (1) proper-lockfile for inter-process coordination, (2) readers-writer pattern for in-process optimization, (3) atomic write-replace for safe writes
  #   2. Multiple readers can read the same file concurrently (readers-writer pattern allows parallel reads)
  #   3. Only ONE writer can modify a file at a time (exclusive write lock blocks all readers and other writers)
  #   4. All file writes MUST use atomic write-replace pattern (write to temp file + rename) to prevent partial writes
  #   5. proper-lockfile MUST use stale lock detection (10 second timeout) to prevent deadlocks from crashed processes
  #   6. Lock acquisition MUST use retry logic with exponential backoff (10 retries, 50-500ms timeout range)
  #   7. LockedFileManager MUST be a singleton to ensure in-process lock coordination across all file operations
  #   8. All JSON file operations (work-units.json, tags.json, foundation.json, etc.) MUST use LockedFileManager
  #   9. Read-modify-write operations MUST use transaction() method to ensure atomicity of the entire operation
  #   10. Lock release MUST happen in finally block to prevent lock leaks on errors
  #
  # EXAMPLES:
  #   1. User runs 'fspec list-work-units' in terminal 1 AND 'fspec update-work-unit-status BOARD-001 done' in terminal 2 simultaneously - Both commands acquire proper-lockfile locks, list-work-units reads while update-work-unit-status waits for write lock, no corruption occurs
  #   2. TUI board is running (refreshing every 2 seconds) AND user runs 'fspec create-story PREFIX Title' - TUI's read lock and command's write lock coordinate via proper-lockfile, work-units.json is never corrupted, TUI sees new work unit after refresh
  #   3. Three fspec commands run concurrently: (1) list-work-units, (2) show-work-unit BOARD-001, (3) update-work-unit-status BOARD-002 done - Commands 1 and 2 read concurrently with in-process readers lock, command 3 waits for exclusive write lock, all succeed without corruption
  #   4. Process 1 acquires write lock on work-units.json and crashes mid-write - proper-lockfile's stale lock detection (10 second timeout) allows Process 2 to acquire lock after timeout, preventing permanent deadlock
  #   5. Command writes to work-units.json using atomic pattern: (1) writes to work-units.json.tmp.abc123, (2) renames to work-units.json - If crash occurs during step 1, original file is intact; if during step 2, rename is atomic
  #   6. Reader 1 and Reader 2 both call fileManager.readJSON('work-units.json') - Both acquire in-process read locks simultaneously (readers-writer pattern), both acquire proper-lockfile locks with retry, both read file concurrently
  #   7. Writer calls fileManager.writeJSON('work-units.json', data) while Reader is active - Writer waits for Reader's in-process read lock to release, then acquires exclusive write lock, prevents read-during-write corruption
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the file locking system handle network filesystems (NFS, SMB) or only local filesystems?
  #   A: true
  #
  #   Q: What should happen if lock acquisition fails after all retries - throw error immediately or offer user option to retry/abort?
  #   A: true
  #
  #   Q: Should LockedFileManager cache file contents to reduce disk I/O, or always read from disk for consistency?
  #   A: true
  #
  #   Q: Do we need metrics/logging for lock performance (acquisition time, wait time, contentions) or keep it simple?
  #   A: true
  #
  #   Q: Should the migration happen in one phase (all-at-once) or incrementally (file by file, command by command)?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a developer running multiple fspec instances concurrently
    I want to prevent JSON file corruption from race conditions
    So that I can safely run multiple commands and TUI simultaneously without data loss

  Scenario: Concurrent read and write commands coordinate via file locks
    Given I have two terminal sessions open
    When I run "fspec list-work-units" in terminal 1
    And I run "fspec update-work-unit-status BOARD-001 done" in terminal 2 simultaneously
    Then both commands should acquire proper-lockfile locks
    And list-work-units should complete its read operation
    And update-work-unit-status should wait for write lock
    And no file corruption should occur
    And both commands should exit with code 0

  Scenario: TUI board and CLI command coordinate file access
    Given the TUI board is running and refreshing every 2 seconds
    When I run "fspec create-story PREFIX Title" in a separate terminal
    Then the TUI's read lock and command's write lock should coordinate via proper-lockfile
    And work-units.json should never be corrupted
    And the TUI should see the new work unit after its next refresh

  Scenario: Multiple concurrent reads with single write operation
    Given I have three concurrent fspec commands
    When command 1 runs "fspec list-work-units"
    And command 2 runs "fspec show-work-unit BOARD-001"
    And command 3 runs "fspec update-work-unit-status BOARD-002 done"
    Then commands 1 and 2 should read concurrently with in-process readers lock
    And command 3 should wait for exclusive write lock
    And all three commands should succeed without corruption

  Scenario: Stale lock detection prevents permanent deadlock from crashed process
    Given process 1 acquires write lock on work-units.json
    When process 1 crashes mid-write
    And process 2 attempts to acquire the same lock
    Then proper-lockfile's stale lock detection should trigger after 10 seconds
    And process 2 should successfully acquire the lock
    And permanent deadlock should be prevented

  Scenario: Atomic write-replace prevents partial file corruption on crash
    Given a command is writing to work-units.json using atomic pattern
    When it writes to work-units.json.tmp.abc123 (step 1)
    And then it attempts to rename to work-units.json (step 2)
    Then if crash occurs during step 1, original file should be intact
    And if crash occurs during step 2, rename operation should be atomic
    And partial writes should never occur

  Scenario: Multiple readers acquire concurrent read locks
    Given reader 1 and reader 2 both call fileManager.readJSON('work-units.json')
    When both acquire in-process read locks simultaneously
    Then both should use readers-writer pattern
    And both should acquire proper-lockfile locks with retry
    And both should read the file concurrently without blocking

  Scenario: Writer waits for active readers before acquiring exclusive lock
    Given a reader has an active read lock on work-units.json
    When a writer calls fileManager.writeJSON('work-units.json', data)
    Then the writer should wait for the reader's in-process read lock to release
    And then the writer should acquire exclusive write lock
    And read-during-write corruption should be prevented
