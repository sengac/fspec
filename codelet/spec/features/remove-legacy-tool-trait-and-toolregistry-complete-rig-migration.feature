@refactoring
@tools
@core
@REFAC-010
Feature: Remove legacy Tool trait and ToolRegistry - complete rig migration

  """
  This refactoring removes the vestigial dual-trait tool system:
  
  Key Changes:
  - Remove custom Tool trait from tools/src/lib.rs (lines 95-108)
  - Remove ToolParameters struct from tools/src/lib.rs (lines 30-50)
  - Remove ToolRegistry struct from tools/src/lib.rs (lines 111-187)
  - Remove Runner struct from core/src/lib.rs (lines 18-170)
  - Remove impl Tool for XxxTool from all 7 tool files
  
  Dependencies:
  - rig crate provides rig::tool::Tool trait used by RigAgent
  - schemars crate provides JsonSchema derive for Args structs
  - All tools keep impl rig::tool::Tool with call() method
  
  Critical Requirements:
  - Tests must be migrated to use tool.call() instead of ToolRegistry::execute()
  - CLI functionality unchanged (already uses RigAgent exclusively)
  - ~900 lines of code will be removed
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The custom Tool trait in tools/src/lib.rs must be removed
  #   2. The ToolRegistry struct in tools/src/lib.rs must be removed
  #   3. The ToolParameters struct in tools/src/lib.rs must be removed
  #   4. The Runner struct in core/src/lib.rs must be removed
  #   5. Each tool file must only have impl rig::tool::Tool, not impl Tool (custom trait)
  #   6. All existing tests must pass after refactoring
  #   7. Tests must be updated to use rig::tool::Tool::call() instead of ToolRegistry::execute()
  #
  # EXAMPLES:
  #   1. ReadTool only has impl rig::tool::Tool with call() method, no impl Tool with execute() method
  #   2. tools/src/lib.rs exports ReadTool, WriteTool, etc. but NOT Tool trait or ToolRegistry
  #   3. core/src/lib.rs exports RigAgent and compaction module but NOT Runner
  #   4. Test for ReadTool uses: let tool = ReadTool::new(); tool.call(args).await instead of ToolRegistry::execute()
  #   5. cargo test passes with all tests updated to use rig trait
  #   6. cargo clippy -- -D warnings passes with no warnings about unused code
  #
  # ========================================

  Background: User Story
    As a developer maintaining codelet
    I want to have a single tool implementation per tool using only rig::tool::Tool
    So that I eliminate code duplication and align tests with production code paths

  Scenario: Tool files contain only rig::tool::Tool implementation
    Given the codelet tools crate at "tools/src/"
    When I inspect each tool file for trait implementations
    Then ReadTool should only have "impl rig::tool::Tool"
    And ReadTool should NOT have "impl Tool for ReadTool"
    And WriteTool should only have "impl rig::tool::Tool"
    And EditTool should only have "impl rig::tool::Tool"
    And BashTool should only have "impl rig::tool::Tool"
    And GrepTool should only have "impl rig::tool::Tool"
    And GlobTool should only have "impl rig::tool::Tool"
    And AstGrepTool should only have "impl rig::tool::Tool"

  Scenario: Tools module does not export legacy types
    Given the codelet tools crate at "tools/src/lib.rs"
    When I check the public exports
    Then "ReadTool" should be exported
    And "WriteTool" should be exported
    And "EditTool" should be exported
    And "BashTool" should be exported
    And "GrepTool" should be exported
    And "GlobTool" should be exported
    And "AstGrepTool" should be exported
    And "Tool" trait should NOT be exported
    And "ToolRegistry" should NOT be exported
    And "ToolParameters" should NOT be exported

  Scenario: Core module does not export Runner
    Given the codelet core crate at "core/src/lib.rs"
    When I check the public exports
    Then "RigAgent" should be exported
    And "compaction" module should be exported
    And "Runner" should NOT be exported

  Scenario: Tools can be tested directly using rig trait
    Given a ReadTool instance
    When I call the tool using rig::tool::Tool::call method
    Then the tool should execute successfully
    And the result should be a String type

  Scenario: All tests pass after migration
    Given the refactored codebase with legacy code removed
    When I run "cargo test"
    Then all tests should pass
    And there should be no compilation errors

  Scenario: No clippy warnings about unused code
    Given the refactored codebase with legacy code removed
    When I run "cargo clippy -- -D warnings"
    Then there should be no warnings
    And there should be no errors