@done
@RPC-051
@rust
@tui
@agent-view
@multi-session
@navigation
@interrupt
@keyboard-navigation
@rpc
Feature: Keyboard shortcut parity (Shift+up/down history, Ctrl+R search, Esc interrupt cascade)
  """
  Phase 6.5 of the RPC-030 roadmap. Closes keyboard parity gaps between
  the Rust ratatui AgentView and the TS Ink AgentView.

  Esc cascade priority order:
  1. Slash / file popup open → close it.
  2. Compositor dialog (Help/Confirm/Model/Thinking) → dismiss it.
  3. Resume / search mode view → close it.
  4. Current session in Running or Compacting → call backend.interrupt
  (do NOT navigate back).
  5. Otherwise → emit Action::BackToBoard.

  Levels 1–3 are wired by earlier RPC cards (RPC-020/RPC-022/RPC-026/
  RPC-027). Level 4 is THIS card's primary delivery — the dispatcher
  currently goes straight from level 3 to BackToBoard, skipping the
  interrupt step entirely.

  Wire shape:
  - New `Action::AgentEscPressed` variant (no payload).
  - `views/agent/dispatch.rs` replaces the unconditional
  `self.emit(Action::BackToBoard)` in its default Esc arm with
  `self.emit(Action::AgentEscPressed)`.
  - `app/dispatch.rs` routes the new variant through
  `app/dispatch_esc_cascade.rs::handle_agent_esc_pressed`, which reads
  `agent_view_store.session_status_for(current_session)` and either
  spawns `backend.interrupt(id)` (Running/Compacting) or dispatches
  `Action::BackToBoard` (everything else, including no session).

  Shift+↑/↓ recall (RPC-025) and Ctrl+R search-view (RPC-026) are
  already wired — this card adds regression scenarios that pin the
  TS-parity behaviour so the cascade slice doesn't accidentally regress
  them.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Esc cascade has 5 levels in priority order: popup > dialog > mode-view > interrupt > BackToBoard
  #   2. Esc with current session in Running/Compacting calls backend.interrupt and consumes WITHOUT BackToBoard
  #   3. Esc with current session in Idle/Paused/Interrupted/Cleared/unknown emits BackToBoard
  #   4. Esc with no current session emits BackToBoard (silent no-op for interrupt)
  #   5. Ctrl+R opens SearchHistoryView with input field focused on mount
  #   6. Shift+↑ first press snapshots draft and async fetches history; subsequent walk back clamped at tail
  #   7. Shift+↓ in recall at index 0 restores cached_draft and clears recall_index; in live mode is no-op
  #   8. Source-shape: dispatch.rs stays under 300 LoC; codelet-fspec-tui MUST NOT depend on codelet-napi
  #
  # ========================================
  Background: 
    Given an App with a MockBackend
    And the App is in ViewMode::Agent

  # ─────────────────────────────────────────────────────────────────────
  # Esc cascade — five levels
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Esc level 1 — slash popup dismiss takes precedence over interrupt
    Given session s-1 is the current session
    And session s-1 has SessionStatus::Running
    And the slash command popup is open in the AgentView
    When the user presses Esc
    Then the slash popup closes
    And backend.interrupt is NEVER called
    And no Action::BackToBoard is dispatched

  Scenario: Esc level 2 — HelpDialog dismiss takes precedence over interrupt
    Given session s-1 is the current session
    And session s-1 has SessionStatus::Running
    And the HelpDialog is pushed on the compositor
    When the user presses Esc
    Then the HelpDialog is removed from the compositor
    And backend.interrupt is NEVER called
    And no Action::BackToBoard is dispatched
    And Navigator.active_view stays at ViewMode::Agent

  Scenario: Esc level 3 — resume mode-view dismiss takes precedence over interrupt
    Given session s-1 is the current session
    And session s-1 has SessionStatus::Running
    And the AgentView's resume_view is open
    When the user presses Esc
    Then Action::CloseResumeView is dispatched
    And the AgentView's resume_view becomes None
    And backend.interrupt is NEVER called
    And no Action::BackToBoard is dispatched

  Scenario: Esc level 3 — search mode-view dismiss takes precedence over interrupt
    Given session s-1 is the current session
    And session s-1 has SessionStatus::Running
    And the AgentView's search_view is open
    When the user presses Esc
    Then Action::CloseSearchView is dispatched
    And the AgentView's search_view becomes None
    And backend.interrupt is NEVER called

  Scenario: Esc level 4 — Running session interrupts without navigating back
    Given session s-1 is the current session
    And session s-1 has SessionStatus::Running
    And no popup is open
    And no dialog is on the compositor
    And no mode-view is open
    When the user presses Esc
    Then within 1 second backend.interrupt is called exactly once with s-1
    And no Action::BackToBoard is dispatched
    And Navigator.active_view stays at ViewMode::Agent

  Scenario: Esc level 4 — Compacting session interrupts without navigating back
    Given session s-1 is the current session
    And session s-1 has SessionStatus::Compacting
    And no popup, dialog, or mode-view is active
    When the user presses Esc
    Then within 1 second backend.interrupt is called exactly once with s-1
    And no Action::BackToBoard is dispatched
    And Navigator.active_view stays at ViewMode::Agent

  Scenario: Esc level 5 — Idle session opens ExitConfirmationDialog (RPC-098)
    Given session s-1 is the current session
    And session s-1 has SessionStatus::Idle
    And no popup, dialog, or mode-view is active
    When the user presses Esc
    Then an ExitConfirmationDialog is pushed onto the compositor
    And backend.interrupt is NEVER called
    And Navigator.active_view stays at ViewMode::Agent

  Scenario: Esc level 5 — session with unknown status opens ExitConfirmationDialog (RPC-098)
    Given session s-1 is the current session
    And session s-1 has no recorded SessionStatus
    And no popup, dialog, or mode-view is active
    When the user presses Esc
    Then an ExitConfirmationDialog is pushed onto the compositor
    And backend.interrupt is NEVER called
    And Navigator.active_view stays at ViewMode::Agent

  Scenario: Esc level 5 — no current session navigates back to Board
    Given there is NO current session
    And no popup, dialog, or mode-view is active
    When the user presses Esc
    Then Action::BackToBoard is dispatched
    And backend.interrupt is NEVER called
    And Navigator.active_view becomes ViewMode::Board

  # ─────────────────────────────────────────────────────────────────────
  # Ctrl+R parity verification
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Ctrl+R opens SearchHistoryView with the input field focused on mount
    Given session s-1 is the current session
    And no popup, dialog, or mode-view is active
    When the user presses Ctrl+R
    Then Action::OpenSearchView is dispatched
    And the AgentView's search_view becomes Some
    When the user types the character "h"
    Then the search_view's query equals "h"
    And Action::SearchHistory("h") is dispatched

  # ─────────────────────────────────────────────────────────────────────
  # Shift+↑/↓ history recall parity verification
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Shift+up/down snapshots draft, walks history, and restores draft on return
    Given session s-1 is the current session
    And the MockBackend's persistence_get_history scripted to return ["first", "second", "third"] for s-1
    And the live MultiLineInput contains "draft-text"
    When the user presses Shift+Up once and waits for the snapshot to load
    Then the MultiLineInput value equals "first"
    And AgentViewStore.history_state_for(s-1).cached_draft equals "draft-text"
    And AgentViewStore.history_state_for(s-1).recall_index equals Some(0)
    When the user presses Shift+Up again
    Then the MultiLineInput value equals "second"
    And AgentViewStore.history_state_for(s-1).recall_index equals Some(1)
    When the user presses Shift+Down
    Then the MultiLineInput value equals "first"
    And AgentViewStore.history_state_for(s-1).recall_index equals Some(0)
    When the user presses Shift+Down again
    Then the MultiLineInput value equals "draft-text"
    And AgentViewStore.history_state_for(s-1).recall_index equals None

  Scenario: Shift+up at end of history is a no-op (clamped at tail)
    Given session s-1 is the current session
    And the MockBackend's persistence_get_history scripted to return ["only-entry"] for s-1
    And the live MultiLineInput contains "draft"
    When the user presses Shift+Up and waits for the snapshot to load
    Then the MultiLineInput value equals "only-entry"
    When the user presses Shift+Up four more times
    Then the MultiLineInput value still equals "only-entry"
    And AgentViewStore.history_state_for(s-1).recall_index equals Some(0)
