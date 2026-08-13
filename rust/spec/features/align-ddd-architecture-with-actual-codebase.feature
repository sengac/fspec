@done
@documentation
@core
@refactoring
@ddd
@architecture
@REFAC-012
Feature: Align DDD Architecture with Actual Codebase

  """
  Architecture notes:
  - DDD model in foundation.json must mirror actual Rust crate structure
  - Each domain crate (core, providers, tools, cli) maps to a bounded context
  - TUI crate is merged into CLI Interface bounded context (infrastructure, not domain)
  - Common crate is Shared Kernel (shared types, not a bounded context)
  - Aggregates map to actual structs/modules in the codebase
  - lib.rs doc comments identify bounded context ownership
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Each Rust crate that represents a domain boundary must have a corresponding bounded context in foundation.json
  #   2. Aggregates in foundation.json must correspond to actual structs/modules in the codebase
  #   3. Each crate lib.rs must have a doc comment identifying its bounded context
  #   4. Shared infrastructure (common crate) is not a bounded context - it is a Shared Kernel
  #
  # EXAMPLES:
  #   1. core crate -> Agent Execution bounded context with RigAgent, Compaction aggregates
  #   2. providers crate -> Provider Management bounded context with ProviderManager, ClaudeProvider, OpenAIProvider, GeminiProvider, CodexProvider aggregates
  #   3. tools crate -> Tool Execution bounded context with BashTool, ReadTool, WriteTool, EditTool, GrepTool, GlobTool, AstGrepTool, LsTool aggregates
  #   4. cli + tui crates -> CLI Interface bounded context with Cli, InteractiveMode, Session, Terminal, InputQueue, StatusDisplay aggregates
  #   5. common crate is Shared Kernel (not a bounded context) - contains Message types, logging, debug utilities
  #
  # ========================================

  Background: User Story
    As a developer maintaining the codelet codebase
    I want to have the DDD model in foundation.json accurately reflect the actual code structure
    So that the architecture documentation serves as reliable reference for understanding system boundaries

  Scenario: Core crate maps to Agent Execution bounded context
    Given the core crate contains RigAgent and Compaction modules
    When I view the foundation.json event storm
    Then the "Agent Execution" bounded context should exist
    And it should contain "RigAgent" and "Compaction" aggregates

  Scenario: Providers crate maps to Provider Management bounded context
    Given the providers crate contains ProviderManager and multiple provider implementations
    When I view the foundation.json event storm
    Then the "Provider Management" bounded context should exist
    And it should contain aggregates for each provider type

  Scenario: Tools crate maps to Tool Execution bounded context
    Given the tools crate contains tool implementations for file and shell operations
    When I view the foundation.json event storm
    Then the "Tool Execution" bounded context should exist
    And it should contain aggregates for each tool type

  Scenario: CLI and TUI crates map to CLI Interface bounded context
    Given the cli crate handles command parsing and interactive mode
    And the tui crate handles terminal rendering
    When I view the foundation.json event storm
    Then the "CLI Interface" bounded context should exist
    And it should contain aggregates from both cli and tui crates

  Scenario: Common crate is not a bounded context
    Given the common crate contains shared types and utilities
    When I view the foundation.json event storm
    Then no bounded context should exist for "Common"
    And the common crate should serve as a Shared Kernel

  Scenario: Each crate lib.rs identifies its bounded context
    Given each domain crate has a lib.rs file
    When I read the doc comment at the top of each lib.rs
    Then the doc comment should identify the bounded context name
