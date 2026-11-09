@done
@infrastructure
@file-ops
@concurrency
@session
@rust
@critical
@WT-015
Feature: Isolated worktrees share the main checkout's board through one state-root resolver
  """
  Product owner decision 2026-11-07 (option a, shared state root). codelet_git::create_worktree writes <worktree>/.fspec/state-root, a JSON pointer {stateRoot, sessionId} naming the main checkout. codelet_common::spec_paths is the one path module: it holds the shared-state definition (also used by WT-017's merge exclusion), reads the pointer, and resolves ProjectRoots {content, state}. Shared files (board and ID counters, epics, prefixes, tags and TAGS.md, foundation and its draft and FOUNDATION.md, attachments, fspec-config.json, schedules and their log, generated virtual-hook scripts, .fspec/stage-permissions.json and .fspec/blocklist.json) are read and written at the state root; features, coverage, fspec-hooks.json and hook scripts at the content root. SpecLock::acquire of a spec directory whose project holds a pointer takes the state root's lock, so every spec write from every checkout contends on one lock. resolve_project_root stops its walk at a pointer. The resolver reads a plain file and never asks git, so a copied directory with a pointer behaves like a git worktree; most scenarios here use plain copies for that reason.
  Tests: rust/fspec-core/tests/wt015_worktree_state_root.rs, rust/git/tests/wt015_worktree_state_root_pointer.rs, rust/common/tests/wt015_spec_paths.rs, rust/common/tests/wt015_spec_paths_source_shape.rs, rust/core/tests/wt015_board_from_a_worktree.rs, rust/tools/tests/wt015_gate_and_stage_permissions_from_a_worktree.rs.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When fspec creates an isolated worktree it writes <worktree>/.fspec/state-root, a JSON file naming the main checkout's absolute project root (stateRoot) and the session id (sessionId); .fspec/ is never part of a session diff, so the pointer never merges
  #   2. A project root holding a state-root pointer has two roots: the content root (the worktree) and the state root (the pointer's target); a root without a pointer is both
  #   3. Shared at the state root, from one definition in codelet-common: spec/work-units.json (cards and ID counters), epics.json, prefixes.json, tags.json, TAGS.md, foundation.json, foundation.json.draft, FOUNDATION.md, attachments/, fspec-config.json, schedules.json, schedule-log.jsonl, hooks/.virtual/, the spec write lock and .fspec/stage-permissions.json (product owner, 2026-11-07)
  #   4. Everything else under spec/ travels with the branch at the content root: features/*.feature, *.feature.coverage, fspec-hooks.json and hook scripts (product owner, 2026-11-07)
  #   5. A pointer that cannot be read, is not valid JSON, names a relative path, points at a root that has its own pointer (chained), or points at a root without spec/ is a hard error naming the pointer file; the command reads and writes nothing
  #   6. One lock: every spec write from a worktree, of shared or branch files, takes the state root's spec/.fspec-write.lock
  #   7. The project-root walk stops at a directory holding a pointer, so a worktree of a project whose spec/ is not committed keeps its features and coverage in its own spec/, never in the main checkout's
  #   8. Consumers outside fspec-core resolve shared files through the same resolver: the codelet-core board writer and watcher, tool admission, stage permissions, fspec config loading, the scheduler and rpc work unit details
  #   9. All spec paths come from one path module in codelet-common; a source-shape test fails on any .join("spec") or .join("spec/...") in non-test code outside it
  #   10. The resolver has no git dependency: any directory holding a pointer (a git worktree or a plain copy) behaves the same
  #
  # EXAMPLES:
  #   1. create_worktree(<main>, "s1") writes <main>/.fspec/worktrees/s1/.fspec/state-root holding {"stateRoot": "<main>", "sessionId": "s1"} and main's git status stays clean
  #   2. In a worktree, create-story and add-rule put the card and its rule in the main checkout's spec/work-units.json; the worktree's checked-out spec/work-units.json is byte-for-byte unchanged
  #   3. In a worktree, create-feature "login" writes <worktree>/spec/features/login.feature and nothing in the main checkout's spec/features/
  #   4. create-bug from the main checkout, then from a worktree, then from the main checkout gives three distinct BUG ids, and only the main checkout's prefixCounters change
  #   5. Another process holds the main checkout's spec lock and FSPEC_SPEC_LOCK_TIMEOUT_MS is 0: add-rule run in the worktree fails with 'Failed to acquire file lock on <main>/spec/.fspec-write.lock' and nothing changes
  #   6. A pointer with stateRoot "main" (relative), a pointer to a root with its own pointer, a pointer to a root without spec/, and a pointer holding '{' each fail naming <worktree>/.fspec/state-root
  #   7. A project whose spec/ is not committed: create-feature run from <worktree>/rust writes <worktree>/spec/features/, and the main checkout's spec/features/ is unchanged
  #   8. From a worktree: the codelet-core board writer moves a card on the main checkout's board, tool admission accepts a card that exists only on the main checkout's board, stage permissions and fspec-config.json are read from the main checkout
  #   9. A plain copied directory (no git) holding a pointer behaves exactly like a git worktree
  #
  # QUESTIONS (ANSWERED):
  #   Q: Are spec/schedules.json, spec/schedule-log.jsonl and the generated virtual-hook scripts (spec/hooks/.virtual/) shared at the state root? The 2026-11-07 decision lists board, counters, epics, prefixes, tags, foundation, attachments, config and stage permissions only.
  #   A: Helper, 2026-11-07 (conservative, pending product owner review): shared at the state root. The daemon's scheduler reads only the main checkout's schedules, so a schedule written in a worktree would never run and would later merge as a file; virtual hooks are registered on the card (the shared board), so their generated scripts follow the card. Moving any of them to the branch side later is a one-line change to the shared-state definition.
  #
  # ========================================
  Background: User Story
    As a developer or agent working a card in an isolated worktree session
    I want to have every fspec command in the worktree read and write the main checkout's board and other shared spec state, while features and coverage stay with the branch
    So that IDs stay unique, the TUI board updates live and no board ever has to be merged

  Scenario: Creating an isolated worktree writes a state-root pointer to the main checkout
    Given a temp git repo with spec/work-units.json committed
    When an isolated worktree is created for session "s1"
    Then the worktree's .fspec/state-root names the main checkout as stateRoot and "s1" as sessionId
    And the main checkout's git status is clean

  Scenario: Every shared file resolves to the main checkout and every branch file to the worktree
    Given a worktree holding a state-root pointer to a main checkout
    When the spec paths are resolved in the worktree
    Then the board, epics, prefixes, tags, TAGS.md, foundation, its draft, FOUNDATION.md, attachments, fspec-config.json, schedules, the schedule log, virtual-hook scripts, the spec lock, .fspec/stage-permissions.json and .fspec/blocklist.json resolve under the main checkout
    And features, coverage files and fspec-hooks.json resolve under the worktree
    And the fspec config loaded in the worktree is the main checkout's spec/fspec-config.json
    And a directory without a pointer resolves every path under itself

  Scenario: Board commands run in a git worktree change the main checkout's board
    Given a temp git repo with a board committed and an isolated worktree created from it
    When create-story and add-rule run in the worktree
    Then the new card and its rule are in the main checkout's spec/work-units.json
    And the worktree's spec/work-units.json is byte-for-byte unchanged

  Scenario: A plain copy holding a pointer keeps feature files with the branch
    Given a main checkout with a board and a plain copy of it holding a state-root pointer
    When create-feature "login" runs in the copy
    Then the copy has spec/features/login.feature
    And the main checkout has no spec/features/login.feature

  Scenario: IDs allocated from the main checkout and a worktree never collide
    Given a main checkout with a board and a plain copy of it holding a state-root pointer
    When create-bug runs in the main checkout, then in the copy, then in the main checkout
    Then the three bugs have distinct ids
    And the copy's spec/work-units.json is byte-for-byte unchanged

  Scenario: A writer in a worktree waits on the main checkout's spec lock
    Given a main checkout with a board and a plain copy of it holding a state-root pointer
    And another writer holds the main checkout's spec lock and the lock timeout is 0 ms
    When add-rule runs in the copy
    Then it fails with the standard lock message naming the main checkout's spec/.fspec-write.lock
    And neither board changed

  Scenario: A broken pointer is a hard error naming the pointer file
    Given a plain copy whose state-root pointer is relative, chained, aimed at a root without spec/, or not valid JSON
    When add-rule runs in the copy
    Then it fails naming the copy's .fspec/state-root
    And neither checkout's spec/ changed

  Scenario: A worktree of a project without a committed spec/ keeps its features in its own spec/
    Given a main checkout with a board and a copy of it with no spec/, a state-root pointer and a subdirectory "rust" holding Cargo.toml
    When create-feature "login" runs with the copy's "rust" directory as its project root
    Then the copy has spec/features/login.feature
    And the main checkout's spec/features/ is unchanged

  Scenario: The board writer and watcher use the main checkout's board from a worktree
    Given a main checkout with a board and a plain copy of it holding a state-root pointer
    When the codelet-core board writer moves a card with the copy as its workspace
    Then the card moves on the main checkout's board
    And the board watcher for the copy reads the main checkout's spec/work-units.json

  Scenario: The Fspec tool gate and stage permissions in a worktree read the main checkout's state
    Given a main checkout with a board and stage permissions, and a plain copy of it holding a state-root pointer
    When the Fspec tool's gate state is built and the stage permissions are loaded with the copy as the project root
    Then the gate state names the title of a card that exists only on the main checkout's board
    And the main checkout's stage permissions are loaded

  Scenario: No spec path is built outside the path module
    Given the workspace's non-test Rust sources
    When they are scanned for joins of "spec" and "spec/..." paths
    Then the only matches are in codelet-common's spec path module

  Scenario: A broken pointer fails every reader command too, naming the pointer file
    Given a plain copy whose state-root pointer names a folder that does not exist
    When list-work-units, show-feature, list-feature-tags, context or any other command runs in the copy
    Then each fails naming the copy's .fspec/state-root
    And none prints an empty board or an empty result

  Scenario: No code discards a state-root resolution error
    Given the workspace's non-test Rust sources
    When the source-shape test scans every state-root resolution call
    Then each result is propagated with ? or mapped with map_err, never discarded with ok, unwrap_or or a wildcard Err arm
