@BUG-131
@tui
@mouse-events
Feature: SGR mouse protocol parser for ink 6.8.0 compatibility
  """
  Create src/tui/utils/mouseProtocol.ts with MOUSE_ENABLE, MOUSE_DISABLE constants, SGR_MOUSE_RE regex, SGR_BUTTON enum, and parseSgrMouse() parser function.
  Ink 6.8.0 strips ESC prefix before delivering to useInput handlers — SGR regex must match the post-strip format [<button;x;yM/m.
  Enable sequence is ?1000h (X10 button event tracking) + ?1006h (SGR encoding mode). Disable is reverse order: ?1006l then ?1000l.
  """

  Background: 
    Given the TUI is running with ink 6.8.0

  Scenario: Parse SGR scroll-up mouse event
    Given the SGR mouse parser receives input "[<64;10;20M"
    When the parser processes the input
    Then it should return button code 64
    And it should return x coordinate 10
    And it should return y coordinate 20
    And it should indicate a press event

  Scenario: Parse SGR scroll-down mouse event
    Given the SGR mouse parser receives input "[<65;5;15M"
    When the parser processes the input
    Then it should return button code 65
    And it should return x coordinate 5
    And it should return y coordinate 15
    And it should indicate a press event

  Scenario: Parse SGR left-click press event
    Given the SGR mouse parser receives input "[<0;5;10M"
    When the parser processes the input
    Then it should return button code 0
    And it should indicate a press event

  Scenario: Parse SGR left-click release event
    Given the SGR mouse parser receives input "[<0;5;10m"
    When the parser processes the input
    Then it should return button code 0
    And it should indicate a release event

  Scenario: Reject non-mouse input
    Given the SGR mouse parser receives input "j"
    When the parser processes the input
    Then it should return null

  Scenario: Mouse enable sequence uses SGR protocol
    When a component enables mouse tracking
    Then the output should contain the escape sequence "\x1b[?1000h\x1b[?1006h"

  Scenario: Mouse disable sequence uses reverse order
    When a component disables mouse tracking
    Then the output should contain the escape sequence "\x1b[?1006l\x1b[?1000l"
