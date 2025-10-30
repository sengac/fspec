@done
@ui-rendering
@flexbox
@bug
@tui
@TUI-006
Feature: VirtualList text wrapping inconsistency

  """
  Uses Ink's flexbox layout system (yoga-layout). VirtualList component wraps items in Box containers. Text wrapping controlled by wrap='wrap' attribute and flexGrow/flexShrink properties. All components (CheckpointViewer, FileDiffViewer, BoardView, UnifiedBoardLayout) must use flexGrow/flexShrink instead of percentage-based flexBasis for consistent behavior.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. VirtualList must use flexGrow/flexShrink properties instead of percentage-based flexBasis
  #   2. All components using VirtualList must use flexGrow/flexShrink for container widths
  #   3. Text components within VirtualList items must have wrap='wrap' attribute for proper wrapping
  #   4. Item render functions must return Box with flexGrow=1 wrapping Text components
  #   5. Yes, ALL components using VirtualList must be converted to use flexGrow/flexShrink to ensure consistent behavior
  #
  # EXAMPLES:
  #   1. CheckpointViewer checkpoint list: long checkpoint names wrap to multiple lines within pane width
  #   2. CheckpointViewer file list: long file paths wrap to multiple lines within pane width
  #   3. BoardView work units: long work unit titles wrap to multiple lines within column width
  #   4. FileDiffViewer file list: long file names wrap to multiple lines within pane width
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should we also fix FileDiffViewer.tsx which currently uses flexBasis='25%', even though it seems to work?
  #   A: true
  #
  # ========================================

  Background: User Story
    As a developer using VirtualList component
    I want to display text content in list items
    So that text wraps properly regardless of which VirtualList instance is used

  Scenario: CheckpointViewer uses flexGrow/flexShrink with NO percentages
    Given CheckpointViewer component exists
    When I examine all Box components in CheckpointViewer
    Then all container Boxes MUST use flexGrow and flexShrink properties
    And NO Box components should use flexBasis with percentage values
    And NO Box components should use width with percentage values
    And NO Box components should use height with percentage values
    And ALL Text components MUST have wrap="wrap" attribute
    And item render functions MUST return Box with flexGrow={1}

  Scenario: FileDiffViewer uses flexGrow/flexShrink with NO percentages
    Given FileDiffViewer component exists
    When I examine all Box components in FileDiffViewer
    Then all container Boxes MUST use flexGrow and flexShrink properties
    And NO Box components should use flexBasis with percentage values
    And NO Box components should use width with percentage values
    And NO Box components should use height with percentage values
    And ALL Text components MUST have wrap="wrap" attribute
    And item render functions MUST return Box with flexGrow={1}

  Scenario: BoardView uses flexGrow/flexShrink with NO percentages
    Given BoardView component exists
    When I examine all Box components in BoardView
    Then all container Boxes MUST use flexGrow and flexShrink properties
    And NO Box components should use flexBasis with percentage values
    And NO Box components should use width with percentage values
    And NO Box components should use height with percentage values
    And ALL Text components MUST have wrap="wrap" attribute
    And item render functions MUST return Box with flexGrow={1}

  Scenario: VirtualList text wrapping works consistently across all components
    Given all components use flexGrow/flexShrink properties
    And all components have Text components with wrap="wrap"
    And all item render functions return Box with flexGrow={1}
    When I display long text content in any VirtualList
    Then text wraps to multiple lines within pane width
    And no text is truncated with ellipsis
    And wrapping behavior is consistent across all components