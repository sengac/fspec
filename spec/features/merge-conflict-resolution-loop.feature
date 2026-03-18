@BUG-099
Feature: Re-merge after conflict resolution enters infinite loop — detect_conflicts() re-detects resolved files as conflicting
  """
  The root cause is that fspec has no equivalent of `git add` to mark a conflict as resolved. Real git uses the index (staging area) — once you `git add` a conflicted file, git stops flagging it. fspec's merge system is stateless: every /merge-worktree call re-detects conflicts from scratch by comparing worktree vs base vs main. The fix adds statefulness via .fspec-pending-conflicts.
  Flow change in apply_session_changes(): (1) Check .fspec-pending-conflicts → if exists, partition into resolved/pending. (2) If any still_pending → return ConflictError with just those files (no marker regeneration). (3) If all resolved → delete state file, exclude resolved files from detect_conflicts (they've been user-accepted), merge remaining changes normally, copy resolved worktree content directly to main.
  The BUG-098 test 'test_re_merge_after_resolution_succeeds' (three_way_merge.rs:474) tests three_way_merge_text() directly with session_resolved == main — this tests the MERGE function in isolation, but the actual bug is one layer UP in apply_session_changes() where detect_conflicts() fires BEFORE the merge function is even called. New integration tests must test the full apply_session_changes() flow.
  tree_utils.rs collect_worktree_files() must skip .fspec-pending-conflicts (add to is_git_or_fspec_dir or add a filename-based filter). Also add it to .gitignore pattern if using the excludes stack approach.
  Implementation:
  - New function check_pending_conflicts(worktree_path) in session_result.rs. Reads .fspec-pending-conflicts (JSON array of file paths). For each file, checks if worktree file content contains '<<<<<<< '. Returns (resolved: Vec<String>, still_pending: Vec<String>). Called at the TOP of apply_session_changes(), BEFORE detect_conflicts().
  - When write_conflict_markers() returns actual_conflicts (non-empty), apply_session_changes() writes .fspec-pending-conflicts BEFORE returning ConflictError. This is in session_result.rs, NOT three_way_merge.rs (which must not be modified).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When the LLM resolves conflict markers and /merge-worktree is called again, apply_session_changes() must compare the worktree content against main content — if they match, skip that file (conflict resolved)
  #   2. The fix must be in the Rust layer only (session_result.rs and tree_utils.rs) — no TypeScript changes needed
  #   3. The existing three-way merge logic in three_way_merge.rs must NOT be modified — the bug is in the caller (detect_conflicts), not the merge algorithm
  #   4. The BUG-098 test 'test_re_merge_after_resolution_succeeds' is flawed — it passes session_resolved (matching main) which makes the three-way merge return Clean, but the REAL re-merge path goes through detect_conflicts() FIRST which still flags it as a conflict before the merge even runs
  #   5. On re-merge (when .fspec-pending-conflicts exists), for each previously-conflicted file: if the worktree file no longer contains <<<<<<< markers → treat as resolved, use worktree content as final answer, skip three-way merge for that file
  #   6. On re-merge, if a previously-conflicted file still contains <<<<<<< markers → user hasn't finished resolving → report as still conflicting (do NOT re-generate markers, just re-return ConflictError for that file)
  #   7. Once all previously-conflicted files are resolved (none contain markers), delete .fspec-pending-conflicts and proceed with apply — the resolved worktree content is copied to main as the user's final answer
  #   8. .fspec-pending-conflicts must be excluded from collect_worktree_files() so it doesn't get applied to main as a regular file
  #   9. When write_conflict_markers() returns actual conflicts, apply_session_changes() must persist a .fspec-pending-conflicts state file (JSON with 'files' array) in the worktree listing those files — this distinguishes 'first conflict detection' from 're-merge after resolution'
  #  10. On re-merge with unresolved markers, the current code produces DOUBLE-NESTED markers (markers inside markers) because detect_conflicts() feeds the marker-containing content as 'session' to three_way_merge — this corrupts the file and makes manual resolution impossible. The fix MUST prevent re-running three-way merge on files that already contain conflict markers
  #
  # EXAMPLES:
  #   1. When LLM resolves worktree README.md to match main exactly (content equal), then /merge-worktree re-runs → detect_conflicts() sees worktree==main → NOT a conflict → apply_session_changes copies resolved content → merge succeeds → session cleaned up
  #   2. When LLM resolves worktree README.md to a THIRD value (neither session nor main original), then /merge-worktree re-runs → .fspec-pending-conflicts exists → markers removed → resolved → worktree content used as final answer → applied to main
  #   3. Exact reproduction from debug session bb90f15f: base='The', session→'Ma', main→'Those'. First merge writes markers. LLM resolves by keeping 'Ma' (session's choice). Second merge: without fix, worktree='Ma' ≠ base AND main='Those' ≠ base → CONFLICT AGAIN → loop. With fix: .fspec-pending-conflicts exists, no markers → resolved → merge succeeds.
  #   4. First merge of README.md: no .fspec-pending-conflicts exists → detect_conflicts finds divergence → write_conflict_markers writes markers → apply_session_changes creates .fspec-pending-conflicts with ['README.md'] → ConflictError returned
  #   5. Re-merge after LLM resolves README.md (removes markers, keeps 'Ma'): .fspec-pending-conflicts exists listing README.md → check worktree: no <<<<<<< found → README.md is RESOLVED → skip three-way merge → use worktree content as final → apply to main → merge succeeds → NO LOOP
  #   6. Re-merge with markers still in file: .fspec-pending-conflicts lists README.md → check worktree: <<<<<<< found → NOT resolved yet → return ConflictError listing README.md → LLM told 'still has markers' → no overwrite, no re-generation of markers
  #   7. Multi-file conflict with partial resolution: .fspec-pending-conflicts lists [README.md, config.yml]. LLM resolves README.md (markers removed) but not config.yml (markers remain). Re-merge: README.md → resolved, config.yml → still pending → ConflictError lists only config.yml
  #   8. Re-merge with unresolved markers produces DOUBLE-NESTED corruption: current code feeds marker-containing worktree as 'session' to three_way_merge, which wraps markers in NEW markers. Fix prevents this by checking state file BEFORE detect_conflicts runs.
  #
  # ========================================
  Background: User Story
    As a LLM agent
    I want to resolve merge conflict markers and re-run /merge-worktree
    So that my resolved file is accepted and the merge completes without looping

  # Example 3: The actual bug — infinite loop when LLM keeps session version
  # Reproduced from debug session bb90f15f (2026-02-26T23:51:12)
  # Edit seq 24: old="<<<<<<< session\n**Ma Spec-Driven...**\n=======\n**Those Spec-Driven...**\n>>>>>>> main" → new="**Ma Spec-Driven...**"
  # Edit seq 63: IDENTICAL edit — same markers regenerated, same resolution
  # Seq 80: Third conflict prompt — user kills session after 76s
  Scenario: Re-merge after conflict resolution does not enter infinite loop
    Given a session worktree with base commit containing "README.md" with "The Spec-Driven"
    And the session has modified "README.md" to "Ma Spec-Driven"
    And the main worktree has modified "README.md" to "Those Spec-Driven"
    When apply_session_changes is called the first time
    Then a ConflictError should be returned listing "README.md"
    And the worktree "README.md" should contain "<<<<<<< session (your changes)"
    And a ".fspec-pending-conflicts" file should exist in the worktree listing "README.md"
    When the user resolves "README.md" by removing conflict markers and keeping "Ma Spec-Driven"
    And apply_session_changes is called again
    Then the merge should succeed without returning a ConflictError
    And the main worktree "README.md" should contain "Ma Spec-Driven"
    And the ".fspec-pending-conflicts" file should be deleted

  # Example 4: First merge creates state file
  Scenario: First merge creates pending conflict state file alongside markers
    Given a session worktree with base commit containing "README.md" with "original"
    And the session has modified "README.md" to "session version"
    And the main worktree has modified "README.md" to "main version"
    And no ".fspec-pending-conflicts" file exists in the worktree
    When apply_session_changes is called
    Then a ConflictError should be returned listing "README.md"
    And a ".fspec-pending-conflicts" file should exist in the worktree listing "README.md"

  # Example 5: Resolution accepted — no marker regeneration
  Scenario: Resolved conflict file is accepted without re-running three-way merge
    Given a session worktree with pending conflicts listing "README.md"
    And the worktree "README.md" does NOT contain "<<<<<<< " markers
    When apply_session_changes is called
    Then the merge should succeed
    And the worktree "README.md" content should be copied to main as the final resolution
    And the ".fspec-pending-conflicts" file should be deleted

  # Example 6: Unresolved markers — no overwrite, re-return error
  # Integration test proved DOUBLE-NESTED markers occur without the fix:
  # <<<<<<< session\n<<<<<<< session\nsession-v\n=======\nmain-v\n>>>>>>> main\n=======\nmain-v\n>>>>>>> main
  Scenario: Unresolved file with markers still present is reported without regenerating markers
    Given a session worktree with pending conflicts listing "README.md"
    And the worktree "README.md" still contains "<<<<<<< session (your changes)" markers
    When apply_session_changes is called
    Then a ConflictError should be returned listing "README.md"
    And the worktree "README.md" should NOT have its markers regenerated
    And the worktree "README.md" content should be byte-identical to before the re-merge call
    And the ".fspec-pending-conflicts" file should still exist

  # Example 7: Multi-file partial resolution
  Scenario: Multi-file conflict with partial resolution reports only unresolved files
    Given a session worktree with pending conflicts listing "README.md" and "config.yml"
    And the worktree "README.md" does NOT contain "<<<<<<< " markers
    And the worktree "config.yml" still contains "<<<<<<< " markers
    When apply_session_changes is called
    Then a ConflictError should be returned listing only "config.yml"
    And the ConflictError should NOT list "README.md"

  # Example 1: Resolution matches main exactly — also handled cleanly
  Scenario: Resolution matching main exactly succeeds on re-merge
    Given a session worktree with pending conflicts listing "README.md"
    And the worktree "README.md" has been resolved to match main exactly
    When apply_session_changes is called
    Then the merge should succeed
    And the ".fspec-pending-conflicts" file should be deleted

  # State file exclusion from worktree collection
  Scenario: Pending conflicts state file is not collected as a worktree file
    Given a session worktree with a ".fspec-pending-conflicts" file present
    When worktree files are collected for diff or apply
    Then ".fspec-pending-conflicts" should NOT appear in the collected file list

  # Example 8: Double-nested markers corruption (discovered by integration test)
  Scenario: Re-merge without fix produces double-nested markers corruption
    Given a session worktree with pending conflicts listing "README.md"
    And the worktree "README.md" still contains "<<<<<<< session (your changes)" markers
    When apply_session_changes is called without the pending-conflicts check
    Then the worktree "README.md" would contain double-nested markers
    And the markers would be nested as "<<<<<<< session" inside "<<<<<<< session"
    And this corruption makes the file unresolvable by the LLM
