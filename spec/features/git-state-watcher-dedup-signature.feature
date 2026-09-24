@done
@git
@watcher
@bug
@performance
Feature: GitStateWatcher dedup never fires — checkpoint 'now' timestamps churn on every capture so every tick re-broadcasts and the TUI re-processes
  """
  Option A (sign at the watcher): re_snapshot_and_publish compares git_state_signature(new snapshot) against the last-published signature instead of full GitState ==. Signature mirrors the TUI BUG-184 signatures (git_branch + checkpoint_counts + changed_files path:change_type:staged in list order + checkpoints work_unit/name/auto in list order, timestamps excluded). The stored GitState keeps fresh fallback timestamps; only the broadcast decision uses the stable signature. TUI-side drop (dispatch_git_state.rs) stays as second line of defense for transport-supplied frames.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R1: the watcher's broadcast decision compares a STABLE SIGNATURE of the new snapshot against the last-published signature — never full GitState equality, because checkpoints[].timestamp is re-stamped SystemTime::now() (millisecond precision) on every capture and two captures of an unchanged repo are therefore never ==
  #   2. R2: the stable signature mirrors the TUI's existing BUG-184 signatures so both layers agree on 'unchanged': git_branch + checkpoint_counts + changed_files in list order (path:change_type:staged) + checkpoints as (work_unit_id, name, is_automatic) in list order — timestamps excluded
  #   3. R3: the stored/served GitState is UNCHANGED — snapshot() still returns the full frame with fresh fallback timestamps; only the broadcast DECISION uses the stable signature (subscribers keep getting self-contained frames)
  #
  # EXAMPLES:
  #   1. Steady-state silence in a CLEAN repo: with_interval(repo, 100ms); wait 600ms; a subscriber's try_recv stays empty for the whole window — zero frames after the initial broadcast (pins the existing dedup behavior that only accidentally works when there are no checkpoint refs)
  #   2. REGRESSION (the actual bug): with_interval(repo, 100ms) on a repo that has ONE ghost-checkpoint ref (AUTH-001/baseline via create_ghost_commit): advance past several ticks; a subscriber that counts frames observes ZERO frames — identical to the clean-repo silence, because checkpoint timestamps are excluded from the signature
  #   3. Change detection still works end-to-end: with a checkpointed repo, creating a SECOND checkpoint ref (AUTH-002/alpha) produces a new frame within one tick (or one debounced .git fs-event); the frame's checkpoints list carries both refs and checkpoint_counts is { manual: 2, auto: 0 }
  #   4. Root-cause pin: two consecutive captures of the same unchanged checkpointed repo yield raw GitState values that are NOT ==-equal (checkpoints[].timestamp differs) but DO produce equal stable signatures — so the dedup gate fires on the signature even though the struct equality would not
  #
  # ========================================
  Background: User Story
    As a TUI user in a checkpointed workspace
    I want to watch the git-state frame arrive only when the repo actually changed
    So that steady-state 10s poll ticks are true no-ops (no re-broadcast, no CheckpointCountsLoaded re-send, no view churn)

  Scenario: Steady-state silence in a clean repo (regression guard)
    Given a GitStateWatcher watching a temp git repo with no checkpoint refs and a 100ms poll interval
    When six poll ticks elapse without any repository change
    Then a subscriber that counts frames for 600ms observes exactly 0 frames after the initial broadcast

  Scenario: Steady-state silence in a checkpointed repo (BUG-188 regression)
    Given a GitStateWatcher watching a temp git repo with one ghost-checkpoint ref AUTH-001/baseline and a 100ms poll interval
    When six poll ticks elapse without any repository change
    Then a subscriber that counts frames for 600ms observes exactly 0 frames after the initial broadcast
    And the stored snapshot is untouched by no-change ticks

  Scenario: A new checkpoint ref produces a frame on the checkpointed repo
    Given a GitStateWatcher watching a temp git repo with one ghost-checkpoint ref AUTH-001/baseline and a 250ms poll interval
    When a second ghost-checkpoint ref AUTH-002/alpha is written into the repo
    Then the watcher publishes a frame whose checkpoints list carries both refs and checkpoint_counts is { manual: 2, auto: 0 }

  Scenario: Two captures of an unchanged checkpointed repo produce equal signatures but unequal raw states
    Given a temp git repo with one ghost-checkpoint ref
    When the GitState snapshot is captured twice in a row
    Then the two raw GitState values are not ==-equal (checkpoints[].timestamp differs)
    And the two stable signatures ARE equal (timestamps excluded from the signature)
