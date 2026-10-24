@done
@BUG-209
@tui
@tool-execution
Feature: Prevent stale exec-stdin probes from restoring completed command prompts
  """
  Track per-session exec-stdin generations including empty-state tombstones. Stamp both asynchronous probe sites at launch and validate at reducer application; apply authoritative push transitions synchronously so queued probes cannot overtake their invalidation.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. An exec-stdin probe result must not change the overlay after a newer request, clear, dismissal, or probe for the same agent session; other sessions must remain independent.
  #   2. Both focus-return probes and the Paused parallel-read probe must validate freshness when their results are applied. Fresh Some and None results still work, HITL retains precedence, and model-turn Done must not dismiss a genuinely live command.
  #
  # EXAMPLES:
  #   1. A focus or Paused probe reads a pending request, then the command exits and its clear event is applied. Delivering the old probe result afterward must leave the overlay absent. Conversely, an old empty result must not hide a newer pushed request.
  #   2. A delayed empty snapshot cannot hide a newer pushed request on either probe path.
  #   3. A local dismissal or successful stdin submission prevents an outstanding snapshot from restoring the prompt.
  #   4. The latest probe wins for one session, while updates to another session do not invalidate it.
  #   5. Fresh present and empty snapshots still update the overlay, HITL wins, and turn Done preserves a live prompt.
  #
  # ========================================
  Background: User Story
    As a TUI user running Codex commands
    I want to see input prompts only for commands that still need attention
    So that completed commands do not reclaim the composer through delayed updates

  Scenario: A delayed pending snapshot cannot restore a cleared command prompt
    Given a pending command snapshot is awaiting delivery from either probe path
    When the command clear arrives before the snapshot is delivered
    Then the input prompt remains absent after the delayed delivery

  Scenario: A delayed empty snapshot cannot hide a newly pushed prompt
    Given an empty command snapshot is awaiting delivery from either probe path
    When a new command input request arrives before the snapshot is delivered
    Then the new input prompt remains visible after the delayed delivery

  Scenario: A delayed pending snapshot cannot replace a newer command prompt
    Given a pending command snapshot is awaiting delivery from either probe path
    When a different command input request arrives before the snapshot is delivered
    Then only the newer command prompt remains visible after the delayed delivery

  Scenario: Dismissing or answering a prompt invalidates outstanding snapshots
    Given a pending command snapshot is awaiting delivery from either probe path
    When the user dismisses or successfully answers the input prompt
    Then the input prompt remains absent after the delayed delivery

  Scenario: The latest probe for a session wins regardless of delivery order
    Given two command snapshots for the same session disagree
    When the newer probe is started before the older snapshot is delivered
    Then only the newer probe can change the input prompt

  Scenario: Prompt updates in another session do not invalidate a snapshot
    Given a pending command snapshot is awaiting delivery for one session
    When another session receives command input requests and clears
    Then the first session snapshot still displays its input prompt

  Scenario: Fresh snapshots still display and clear command prompts
    Given no newer command update supersedes either probe path
    When fresh pending and empty snapshots are delivered in order
    Then the input prompt appears and subsequently disappears

  Scenario: Human approval retains precedence over a fresh command snapshot
    Given a human approval prompt is visible for a session
    When a fresh pending command snapshot is delivered from either probe path
    Then human approval remains visible without a command input prompt

  Scenario: Finishing a model turn preserves a live command prompt
    Given an input prompt is visible for a command still running
    When the model turn finishes
    Then the command input prompt remains available
