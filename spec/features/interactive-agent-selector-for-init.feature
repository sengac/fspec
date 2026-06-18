@done
@agent-integration
@cli
@RPC-239
@interactive-cli
@init
@rust
@critical
Feature: Interactive agent selector for init
  """
  The interactive selector lives in the CLI bridge crate codelet/fspec (owns ratatui + crossterm). fspec-core stays terminal-free and exposes available_agents() (id, name, description) plus detect_agents(project_root). Navigation state is a pure struct: AgentSelectorState { agents, preselected, cursor } with move_up/move_down (clamped, no wrap) and current_id(); the ratatui render + crossterm event loop wrap this state. TTY detection uses std::io::stdin().is_terminal(); when no --agent and not a TTY the existing TTY-guard error/exit-1 path is preserved, and the LLM dispatcher path in fspec-core is unchanged.
  """

  Background: User Story
    As a developer running `fspec init` at the shell
    I want an interactive ratatui agent selector when I do not pass --agent
    So that I can pick my AI coding agent on a real terminal without remembering agent ids

  Scenario: Selector starts on the first agent when none are detected
    given the list of available agents and no detected agents in the project root
    when I build the interactive agent selector
    then the cursor starts at index 0 and the highlighted agent id is 'claude'

  Scenario: Selector pre-selects a detected agent
    Given a project root containing a .cursor directory
    When I detect agents and build the interactive agent selector
    Then the detected agent id 'cursor' is reported
    And the cursor starts on the 'cursor' row and that row is marked '(detected)'

  Scenario: Navigation clamps at the list bounds
    Given an interactive agent selector positioned at index 0
    When I move the cursor up
    Then the cursor stays at index 0
    When I move the cursor down once
    Then the cursor moves to index 1
    When I move the cursor down past the last agent
    Then the cursor stays on the last agent

  Scenario: Selecting an agent installs its files
    given an empty project root directory and the interactive selector positioned on the 'gemini' row
    When I confirm the selection and run init with the chosen agent
    Then spec/GEMINI.md is created in the project root
    And .gemini/commands/fspec.toml is created in the project root

  Scenario: Non-TTY shell without --agent shows the TTY guard
    Given stdin is not a TTY
    When I run the init CLI with no --agent flag
    Then no selector is shown and the output contains 'Interactive mode requires a TTY. Use --agent flag instead:'
    And the command exits with code 1

  Scenario: Cancelling the selector writes nothing
    Given an empty project root directory and a visible interactive selector
    When I cancel the selector with Esc
    Then the selection result is cancelled and no agent files are written
    And the command exits with code 0 after printing 'Init cancelled'
