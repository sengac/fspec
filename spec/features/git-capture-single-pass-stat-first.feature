@watcher
@bug-189
@done
@git
@performance
@bug
@git-ops
Feature: Git capture is O(repo size): double lookup_entry_by_path passes, per-file sha1 over all indexed files, whole-worktree walkdir, and a fresh cold gix::open handle per call

  """
  1) codelet-git: add pub(crate) `*_with_repo(repo: &gix::Repository, ...)` internal variants in status.rs/change_type.rs that accept a shared handle; keep the public dir-based fns as thin wrappers (open_repo + delegate). 2) status.rs: get_staged_files returns the per-path in-HEAD verdict alongside the staged list; get_staged_files_with_change_type reuses it (single lookup pass). 3) status.rs: get_unstaged_files compares fs mtime+size against the index entry stat (Stat::mtime.secs/nsecs, size) before reading+hashing; hash only as fallback. 4) git_state.rs: capture_git_state opens ONE gix::Repository and calls the *_with_repo variants for staged/unstaged/untracked.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R1: one GitState capture opens ONE gix::Repository and shares that handle across the staged, unstaged, and untracked collectors via internal `*_with_repo` variants (the public `dir`-based fns stay thin wrappers for other callers). This removes the cold-ODB pack re-decode that a fresh `gix::open` per collector caused (the 52% zlib-inflate hot spot).
  #   2. R2: get_staged_files_with_change_type must issue exactly ONE lookup_entry_by_path per staged path — the in-HEAD verdict is computed in the same pass as the staged-file detection (reused), never a second lookup pass. Public results must remain byte-identical (same paths, same A/M/D letters).
  #   3. R3: get_unstaged_files performs NO fs::read + hash for a file whose stat matches the index entry's stat fields (mtime + size quick-check first — exactly what git does); the full-file hash is only a fallback when the stat is unknown/changed. A stat match means 'not modified'.
  #
  # EXAMPLES:
  #   1. Single lookup pass (R2): a repo with 500 tracked files and 100 of them staged. get_staged_files_with_change_type returns the same 100 paths with the same A/M/D letters as before, but issues exactly ONE lookup_entry_by_path per staged path (the in-HEAD verdict is reused from the staged-detection pass, not re-derived with a second lookup).
  #   2. Stat-first unstaged check (R3): a repo with 1000 tracked files all unchanged. Capturing git state now costs O(stat syscalls), NOT O(full-file reads + sha1 over every file) — the working tree is NOT read into memory. A genuinely modified file is still detected as unstaged (behavior parity), and a file whose content changed but whose mtime+size were artificially restored to match the index is still caught by the hash fallback.
  #   3. Shared handle (R1): a GitState capture over staged+unstaged+untracked opens the repository exactly once and passes that handle into all three collectors, so the HEAD-tree objects decode from pack ONCE (not three times). The combined changed_files list is byte-identical to what the three separate open-and-recollect calls produced (existing codelet-git tests stay green).
  #
  # ASSUMPTIONS:
  #   1. Out of scope this unit: gating get_untracked_files on a worktree fs-event (item 4) — that requires watching the worktree itself (today only .git is watched, recursively), a detection-semantics change that would silently stop surfacing newly-untracked files on steady-state ticks. The untracked walk still runs per capture, but items 1-3 (shared handle, single lookup pass, stat-first) make the common unchanged case cheap. BUG-188's signature gate means the capture only re-runs when .git actually changed.
  #
  # ========================================

  Background: User Story
    As an agent supervisor watching a large workspace
    I want a GitState capture to run on a repo with thousands of tracked files
    So that it completes in well under the poll interval — no repeated pack decode, no per-file hashing, no double tree lookups

  Scenario: A GitState capture opens the repository exactly once across all collectors
    Given a temp git repo with committed tracked files
    When a GitState snapshot is captured via the shared-handle path
    Then the staged, unstaged, and untracked collectors all ran against the SAME gix::Repository handle
    And the combined changed_files list is byte-identical to what the three separate public dir-based calls produce


  Scenario: Staged-with-change-type issues exactly one tree lookup per staged path
    Given a temp git repo with committed tracked files, some of which are staged with differing content
    When get_staged_files_with_change_type is called against that repo
    Then exactly one tree lookup is issued per staged path (the in-HEAD verdict is reused, not re-derived) and the returned paths + A/M/D letters match the previous double-pass result
    And a staged NEW file (absent from HEAD) is reported as Added, a staged MODIFIED file as Modified, and a staged file deleted from the working directory as Deleted


  Scenario: Unstaged detection is stat-first (no full-file read when the stat matches)
    Given a temp git repo with committed tracked files whose index stat matches the on-disk mtime+size
    When get_unstaged_files is called against that repo
    Then NO file read + hash occurs for a file whose mtime+size match the index stat (the quick stat check short-circuits)
    And a file whose content was modified (mtime or size changed) IS still reported as unstaged via the read + hash verification path
    And a file whose content changed but whose mtime+size were artificially restored to match the index is treated as unchanged (accepted git parity, not a defect)
