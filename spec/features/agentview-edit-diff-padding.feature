@done
@tui
@agent-view
@rust
@RPC-392
Feature: Colored Edit/Write diff lines lack full-width background padding
  """
  Pads each decoded [R]/[A] diff line with trailing spaces to the target render width before applying the dark-red/dark-green background (parity with the TS <Box flexGrow={1}> bar). Width is PASSED IN per call site: scrollback uses the wrap width, the modal uses content_width. Reuses the existing chars().count() display-width proxy from text_wrap.rs (DRY). Context-gutter lines, gap markers, and plain lines stay unchanged. Saturating arithmetic; width 0 adds no padding and never panics.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A decoded [R]- line is right-padded with spaces to the target render width and the dark-red (#8B0000) background + white fg covers the entire padded width
  #   2. A decoded [A]+ line is right-padded the same way with the dark-green (#006400) background + white fg
  #   3. Context lines (gray gutter + white content) are NOT padded with a colored background; they render exactly as before with no background fill
  #   4. Gap markers (... (N lines)), the ... +N lines indicator, and any plain/non-diff line render exactly as before (no colored bar, no padding)
  #   5. The full-width bar applies in BOTH the scrollback (chunk_wrap.rs) and the turn-content modal (turn_modal.rs / decode_modal_row)
  #   6. If the marker-stripped content is already as wide as or wider than the target width, no padding is added and nothing is truncated
  #   7. Non-Edit/Write tool output (Bash, Grep, etc.) is unaffected — no regression
  #   8. A zero/very-small width must not panic and must not produce negative pad counts (saturating arithmetic), and the display-width metric reuses the existing chars().count() proxy
  #
  # EXAMPLES:
  #   1. Removed line is padded to a full-width red bar: a decoded [R]- line shorter than the render width decodes to a span whose display-width equals the render width, bg #8B0000, fg white, no [R] marker
  #   2. Added line is padded to a full-width green bar: a decoded [A]+ line shorter than the render width decodes to a span whose display-width equals the render width, bg #006400, fg white, no [A] marker
  #   3. Context line is not given a colored background bar: a decoded context line (L 250   foo) decodes to a gray-gutter span + white-content span, neither with a background, and is not padded to the render width
  #   4. Gap-marker or plain line is unchanged: a ... (5 lines) gap-marker (or ... +N lines indicator) decodes to a single span with no background and no extra padding bar
  #   5. Content already at or over width is not padded or truncated: a decoded [A]+ line whose stripped content display-width >= render width returns the content unchanged (no added spaces, no truncation), still colored green
  #   6. Zero width does not panic and pads non-negatively: a decoded [R]- line decoded with width 0 does not panic and returns the content as-is (no padding)
  #   7. Scrollback diff branch emits full-width bars: a ChunkKind::ToolCall { is_diff: true } whose body has a removed and an added line, wrapped at a known width, produces removed/added Lines each carrying a span whose content display-width equals the wrap width with the correct background
  #   8. Modal diff rows emit full-width bars while non-diff rows do not: a modal body containing a [R]/[A] row and a plain row, decoded at content_width, pads the diff rows full-width with the diff background while the plain row stays a single unpadded raw span
  #
  # ========================================
  Background: User Story
    As a fspec-tui user
    I want to see edited diff lines as solid full-width colored bars
    So that the red/green background fills the row edge-to-edge matching the TypeScript client

  Scenario: Removed line is padded to a full-width red bar
    Given a decoded removed diff line shorter than the render width
    When it is decoded with that render width
    Then the resulting span content display-width equals the render width
    And the span background is rgb 139,0,0 and the foreground is white
    And the span content contains no removed marker

  Scenario: Added line is padded to a full-width green bar
    Given a decoded added diff line shorter than the render width
    When it is decoded with that render width
    Then the resulting span content display-width equals the render width
    And the span background is rgb 0,100,0 and the foreground is white
    And the span content contains no added marker

  Scenario: Context line is not given a colored background bar
    Given a decoded context line of the form 'L 250   foo'
    When it is decoded with a render width wider than the line
    Then it produces a gray-gutter span and a white-content span
    And neither span has a background colour
    And the content is not padded to the render width with a background

  Scenario: Gap-marker or plain line is unchanged
    Given a gap-marker line of the form '... (5 lines)'
    When it is decoded with a render width
    Then it is a single span with no background and no extra padding bar

  Scenario: Content already at or over width is not padded or truncated
    Given a decoded added diff line whose stripped content display-width is at least the render width
    When it is decoded with that render width
    Then the content is returned unchanged with no added spaces and no truncation
    And the span background is rgb 0,100,0

  Scenario: Zero width does not panic and pads non-negatively
    Given a decoded removed diff line
    When it is decoded with width zero
    Then it does not panic and no padding is added

  Scenario: Scrollback diff branch emits full-width bars
    Given a diff tool-call whose body has a removed line and an added line
    When the source is wrapped at a known width
    Then the removed line carries a span whose content display-width equals the wrap width with the removed background
    And the added line carries a span whose content display-width equals the wrap width with the added background

  Scenario: Modal diff rows emit full-width bars while non-diff rows do not
    Given a modal body containing a removed-marker row, an added-marker row, and a plain row
    When the modal decodes rows at the content width
    Then the diff rows are padded full-width with the diff background
    And the plain row is a single unpadded raw span
