@done
@ts-parity
@thinking-detection
@tui
@TUI-093
Feature: Default thinking-level save/restore TS parity in Rust TUI

  """
  Restore parity ports TS src/tui/hooks/useDefaultThinkingLevel.ts to the Rust TUI. Persisted default lives in fspec-config.json tui.defaultThinkingLevel (int 0-3); missing/invalid loads as Off. Three application sites: (a) handle_set_thinking_level_default repaints via get_thinking_level + Action::ThinkingLevelLoaded; (b) bootstrap initialize_default_thinking_level mirrors initialize_startup_model; (c) refresh_session_chrome applies the default on activation/resume guarded by a per-session HashSet<SessionId> (Rust equivalent of TS appliedToSessionRef) so manual /thinking selections are never clobbered. Persistence is best-effort and non-fatal; storage location/encoding unchanged.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Selecting a default level with D persists it AND immediately repaints the active session's [T:level] badge
  #   2. On startup the persisted default thinking level is restored to the active session (bootstrap step, parallel to initialize_startup_model)
  #   3. When a session becomes active or is resumed, the persisted default is applied to it, but at most once per session id (per-session guard)
  #   4. A manual /thinking selection within a session is never clobbered by a later re-apply of the default when that session regains focus
  #   5. Storage location/format is unchanged: tui.defaultThinkingLevel int 0-3 in fspec-config.json; missing/invalid loads as Off; persistence is best-effort and non-fatal
  #
  # EXAMPLES:
  #   1. User has default High persisted; opens a fresh process; the active session header shows [T:High] immediately at startup
  #   2. User opens the /thinking dialog and presses D to set default Medium; the badge updates to [T:Medium] immediately without closing and reopening
  #   3. User has default High; resumes an old session via /resume; the resumed session shows [T:High] instead of falling back to Off
  #   4. Within a session the user manually sets thinking to Low via /thinking; they switch to another session and back; the session still shows [T:Low], not the default High
  #   5. No default has ever been set (no key on disk); a new session shows no thinking badge (Off) and startup does not error
  #
  # ========================================

  Background: User Story
    As a Rust TUI user
    I want to have my selected default thinking level restored and visibly reflected across new, resumed, and active sessions
    So that the [T:level] badge always matches what I chose, matching the TypeScript reference

  @startup
  Scenario: Persisted default is restored to the active session at startup
    Given the persisted default thinking level is High
    And a fresh app with one active session whose base thinking level is Off
    When the app runs the default thinking level bootstrap step
    Then the active session base thinking level becomes High
    And a ThinkingLevelLoaded action carrying High is dispatched for the active session

  @dialog
  Scenario: Pressing D persists the default and immediately repaints the badge
    Given an active session and the thinking dialog is open on Medium
    When the user presses D to set Medium as the default
    Then the default thinking level Medium is persisted to the shared config
    And the handler dispatches a ThinkingLevelLoaded action carrying Medium for that session

  @resume
  Scenario: Resuming an older session applies the persisted default once
    Given the persisted default thinking level is High
    And a resumed session that has not yet had the default applied
    When the session becomes active and chrome is refreshed
    Then the resumed session base thinking level becomes High
    And the resumed session id is recorded as already-applied

  @guard
  Scenario: A manual thinking selection is not clobbered when the session regains focus
    Given the persisted default thinking level is High
    And a session whose default was already applied and then manually set to Low
    When the session regains focus and chrome is refreshed
    Then the default is not re-applied to that session
    And the session base thinking level remains Low

  @no-default
  Scenario: No persisted default yields Off and does not error at startup
    Given no default thinking level key exists on disk
    And a fresh app with one active session
    When the app runs the default thinking level bootstrap step
    Then the loaded default thinking level resolves to Off
    And the bootstrap step completes without error
