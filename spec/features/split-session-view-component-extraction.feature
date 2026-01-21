@WATCH-017
Feature: Split Session View Component Extraction

  """
  SplitSessionView.tsx created in src/tui/components/ alongside AgentView.tsx
  AgentView.tsx imports SplitSessionView and renders it when isWatcherSessionView is true, passing required props
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. SplitSessionView is a standalone component that receives parentSessionId and childSessionId as props
  #   2. The component handles its own state: loading parent conversation, subscribing to parent updates, managing active pane
  #   3. Component is built compositionally: first render static layout, then add data loading, then add interactivity
  #   4. Left pane displays parent session conversation using VirtualList with dimmed styling when inactive
  #   5. Right pane displays watcher conversation using VirtualList with full styling when active
  #   6. Component receives onSubmit callback prop for sending messages to watcher session
  #
  # EXAMPLES:
  #   1. Step 1: Component renders with empty panes and header showing 'Loading...' - verify basic layout works
  #   2. Step 2: Component loads parent conversation from NAPI and displays it in left pane
  #   3. Step 3: Component receives watcher conversation as prop and displays it in right pane
  #   4. Step 4: Left/Right arrow keys switch active pane - styling changes to indicate which pane is active
  #   5. Step 5: Input area at bottom works with MultiLineInput - onSubmit callback is called when user submits
  #
  # ========================================

  Background: User Story
    As a developer debugging the watcher split view
    I want to have the split view code in a separate component
    So that I can test and debug it in isolation without the complexity of AgentView.tsx

  @critical
  Scenario: Split view renders basic layout with header
    Given the SplitSessionView component is rendered with valid session IDs
    When the component mounts
    Then I see a header showing watcher role and parent session name
    And I see two vertical panes side by side
    And I see an input area at the bottom

  Scenario: Left pane displays parent conversation
    Given the SplitSessionView component is rendered with a parent session that has messages
    When the parent conversation is loaded
    Then the left pane displays the parent session messages
    And the left pane uses VirtualList for scrolling

  Scenario: Right pane displays watcher conversation
    Given the SplitSessionView component is rendered with watcher conversation data
    When the component renders
    Then the right pane displays the watcher conversation messages
    And the right pane uses VirtualList for scrolling

  Scenario: Arrow keys switch active pane
    Given the SplitSessionView component is rendered
    And the right pane is currently active
    When I press the Left arrow key
    Then the left pane becomes active with bright styling
    And the right pane has dimmed styling
    When I press the Right arrow key
    Then the right pane becomes active with bright styling
    And the left pane has dimmed styling

  Scenario: Input area sends message via onSubmit callback
    Given the SplitSessionView component is rendered with an onSubmit callback
    When I type a message and press Enter
    Then the onSubmit callback is called with the message text
