@session-management
@done
@tui
@menu-bar
@BUG-400
Feature: Agent chips show an unobserved session status as unknown
  """
  The board, agent view and mux build agent chips through
  views/board/menu_snapshot.rs chip_inputs. A session whose status the TUI has
  not observed yet has ChipInput.status None, and the chip painter
  (components/menu_bar/chips.rs) paints it with the unknown glyph "?" instead of
  inventing an Idle status (BUG-400: no silent defaults).
  """

  Background: User Story
    As a developer watching several agent sessions in the TUI
    I want a session whose status is not known yet to look unknown
    So that I never mistake an unobserved session for an idle one

  Scenario: An unobserved session is painted with the unknown glyph
    Given an open session whose status the TUI has not observed
    When the agent chips are built for that session
    Then the chip input has no status
    And the chip shows the unknown glyph "?"

  Scenario: An observed status is painted as before
    Given an open session whose status the TUI observed as Running
    When the agent chips are built for that session
    Then the chip input status is Running
    And the chip shows the running spinner glyph
