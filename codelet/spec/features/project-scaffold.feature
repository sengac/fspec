@cli-interface
@scaffold
@CORE-001
Feature: Project Scaffold for Codelet Rust Port
  """
  Scaffold follows Domain-Driven Design bounded context pattern with 5 contexts: CLI Interface, Provider Management, Tool Execution, Context Management, Agent Execution. Uses Rust module system with mod.rs files re-exporting public types. Dependencies: clap v4 for CLI, async-trait for async traits, serde for serialization, tokio for async runtime.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Project must have 5 bounded context modules: cli, providers, tools, context (renamed from agent/context), and agent
  #   2. Each bounded context module must have a mod.rs that re-exports public types
  #   3. Tool Execution bounded context must define a Tool trait with name, description, parameters, and execute methods
  #   4. Provider Management bounded context must define an LlmProvider trait with complete, supports_caching, and supports_streaming methods
  #   5. Context Management bounded context must include TokenTracker struct tracking input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens
  #   6. Agent Execution bounded context must define Message types with role and content supporting text and tool interactions
  #   7. CLI Interface bounded context must use clap v4 with derive macros for argument parsing
  #
  # EXAMPLES:
  #   1. Running 'cargo build' compiles successfully with all module stubs
  #   2. src/tools/mod.rs exports Tool trait and ToolRegistry struct
  #   3. src/providers/mod.rs exports LlmProvider trait and ProviderType enum
  #   4. src/agent/mod.rs exports Runner struct and Message types
  #   5. src/context/mod.rs exports TokenTracker struct with effective_tokens method (replaces src/agent/token_tracker.rs)
  #   6. src/cli/mod.rs exports Cli struct with clap derive attributes
  #
  # ========================================
  Background: User Story
    As a developer porting codelet to Rust
    I want to have a complete project scaffold with all bounded context modules stubbed out
    So that I can implement each bounded context incrementally while maintaining proper architectural boundaries

  Scenario: Project compiles with all bounded context modules
    Given the project has 5 bounded context modules: cli, providers, tools, context, and agent
    When I run 'cargo build'
    Then the build should complete successfully
    And each module has a mod.rs file that re-exports public types

  Scenario: Tool Execution module exports Tool trait and ToolRegistry
    Given the tools module exists at src/tools/mod.rs
    When I import from the tools module
    Then the Tool trait should be available with name, description, parameters, and execute methods
    And the ToolRegistry struct should be available

  Scenario: Provider Management module exports LlmProvider trait and ProviderType
    Given the providers module exists at src/providers/mod.rs
    When I import from the providers module
    Then the LlmProvider trait should be available with complete, supports_caching, and supports_streaming methods
    And the ProviderType enum should be available

  Scenario: Agent Execution module exports Runner and Message types
    Given the agent module exists at src/agent/mod.rs
    When I import from the agent module
    Then the Runner struct should be available
    And the Message types with role and content supporting text and tool interactions should be available

  Scenario: Context Management module exports TokenTracker with effective_tokens
    Given the context module exists at src/context/mod.rs
    When I import from the context module
    Then the TokenTracker struct should be available with input_tokens, output_tokens, cache_read_tokens, and cache_creation_tokens fields
    And the TokenTracker should have an effective_tokens method

  Scenario: CLI Interface module exports Cli struct with clap derive
    Given the cli module exists at src/cli/mod.rs
    When I import from the cli module
    Then the Cli struct should be available with clap Parser derive
