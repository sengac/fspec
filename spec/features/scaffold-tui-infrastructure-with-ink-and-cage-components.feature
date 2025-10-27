@done
@typescript
@zustand
@ink
@interactive-cli
@tui
@high
@ITF-001
Feature: Scaffold TUI infrastructure with Ink and cage components
  """
  Critical requirements: All components must be TypeScript with no 'any' types. Must follow ViewManager navigation pattern, FullScreenLayout for consistent UI, VirtualList for performance, and modal input modes (normal/insert) for keyboard handling. Components inspired by cage but NOT direct copies.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. TUI infrastructure must use Ink (React for CLIs) as the rendering engine
  #   2. State management must use Zustand with Immer for immutable updates
  #   3. Component library must include @inkjs/ui for pre-built UI primitives
  #   4. Components must be TypeScript with strict type safety (no 'any' types)
  #   5. All components must be tested with ink-testing-library
  #   6. Components must follow cage's architectural patterns (ViewManager, FullScreenLayout, modal input modes)
  #
  # EXAMPLES:
  #   1. Developer imports FullScreenLayout from src/tui/layouts and wraps a simple Text component - renders with title bar and footer
  #   2. Developer uses ViewManager to navigate between two views - navigation works, back button returns to previous view
  #   3. Developer creates Zustand store with fspec work units data - updates trigger re-renders in Ink components
  #   4. Developer writes test with ink-testing-library's render() - can query rendered output and assert on text content
  #   5. Developer creates modal input mode component - switches between normal and insert modes with 'i' key
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should we copy components directly from cage or create similar components inspired by cage's patterns?
  #   A: true
  #
  #   Q: Do we need VirtualList component in Phase 1 scaffold or should it wait for ITF-002 (performance optimization)?
  #   A: true
  #
  #   Q: Should the scaffold include basic keyboard navigation hooks or just the component structure?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a developer building fspec's interactive TUI
    I want to scaffold core TUI infrastructure with Ink, Zustand, and reusable components from cage
    So that I can build the interactive Kanban board and other TUI features on a solid, tested foundation

  Scenario: FullScreenLayout renders with title bar and footer
    Given I have created FullScreenLayout component in src/tui/layouts/
    When I import FullScreenLayout and wrap a Text component with title='Test View'
    Then the component should render with a title bar displaying 'Test View'
    And the component should render with a footer displaying keyboard shortcuts

  Scenario: ViewManager navigates between views with back support
    Given I have created ViewManager with two registered views: ViewA and ViewB
    When I call navigate('ViewB') from ViewA
    Then ViewB should be rendered
    When I call goBack()
    Then ViewA should be rendered again

  Scenario: Zustand store updates trigger Ink component re-renders
    Given I have created a Zustand store with fspec work units data using Immer
    And I have an Ink component subscribed to the store
    When I update a work unit status in the store
    Then the Ink component should re-render with the updated work unit status
    And the state update should be immutable (not mutating original data)

  Scenario: ink-testing-library enables component testing with queries
    Given I have created a simple Ink component that renders 'Hello World'
    When I render the component using ink-testing-library's render() function
    Then I should be able to query the rendered output for 'Hello World'
    And I should be able to assert that the text content matches expectations

  Scenario: Modal input modes switch between normal and insert
    Given I have created InputModeContext with modal input mode support
    And I have a component using useSafeInput hook in normal mode
    When I press the 'i' key to enter insert mode
    Then the input mode should switch to insert
    And keyboard input should be captured for text entry
    When I press the Escape key
    Then the input mode should return to normal
