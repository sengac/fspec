@done
@integration
@tools-development
@codelet
@CODE-009
Feature: Structured FspecTool Results via StreamChunk Discriminated Union
  """
  Architecture notes:
  - Add FspecCommandRequest and FspecCommandResult types to codelet/napi/src/types.rs following CompactionResult pattern
  - Modify FspecToolFacadeWrapper in codelet/tools/src/facade/wrapper.rs to emit FspecCommandRequest instead of FSPEC_INTERCEPT string
  - Handle FspecCommandRequest chunk in src/tui/components/AgentView.tsx using existing callFspecCommand callback mechanism
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. FspecTool must emit structured StreamChunk::FspecCommandRequest instead of returning FSPEC_INTERCEPT string
  #   2. TypeScript must handle FspecCommandRequest chunk by executing command via existing callFspecCommand mechanism and returning FspecCommandResult
  #   3. System reminders from fspec commands must be included in FspecCommandResult and injected into LLM context for workflow orchestration
  #   4. FspecCommandRequest and FspecCommandResult must be proper NAPI objects with typed fields, following the CompactionComplete pattern
  #
  # EXAMPLES:
  #   1. LLM calls Fspec tool with 'show-work-unit CODE-001' - Rust emits FspecCommandRequest{command:'show-work-unit', args_json:'{"id":"CODE-001"}'} - TypeScript executes and returns FspecCommandResult with structured data and system reminder
  #   2. TypeScript handles chunk.type === 'FspecCommandRequest' with direct field access (chunk.fspecRequest.command) - no string parsing required
  #   3. When fspec command fails, FspecCommandResult contains success:false and error field with message, allowing TypeScript to display proper error feedback
  #   4. FspecCommandResult.system_reminder contains workflow guidance (e.g., 'Next steps: run fspec add-rule...') which is injected into LLM context to guide ACDD process
  #   5. After migration, FSPEC_INTERCEPT string pattern is removed from wrapper.rs and stream_handlers.rs - all fspec tool calls use structured StreamChunk flow
  #
  # ========================================
  Background: User Story
    As a AI agent using fspec within codelet
    I want to receive fspec command results as typed StreamChunk variants
    So that I can process fspec responses with type safety and preserve system reminders for workflow orchestration

  @napi
  @streaming
  Scenario: FspecTool emits structured command request and receives typed result
    Given a codelet session with FspecTool available
    And the StreamChunk type includes FspecCommandRequest and FspecCommandResult variants
    When the LLM invokes Fspec tool with command "show-work-unit" and args '{"id":"CODE-001"}'
    Then Rust should emit a FspecCommandRequest chunk with typed fields
    And TypeScript should receive the chunk with direct field access via chunk.fspecRequest.command
    And TypeScript should execute the command via callFspecCommand callback
    And the result should be returned as a FspecCommandResult chunk with success and data fields

  @napi
  @streaming
  Scenario: TypeScript handles FspecCommandRequest with type-safe field access
    Given a codelet session processing StreamChunk events
    When a FspecCommandRequest chunk is received
    Then TypeScript should access chunk.fspecRequest.command directly without string parsing
    And TypeScript should access chunk.fspecRequest.argsJson directly without regex extraction
    And TypeScript should access chunk.fspecRequest.projectRoot directly without field parsing

  @napi
  @error-handling
  Scenario: Failed fspec command returns structured error in FspecCommandResult
    Given a codelet session with FspecTool available
    When the LLM invokes Fspec tool with an invalid command
    Then the FspecCommandResult should have success set to false
    And the FspecCommandResult should have an error field with the failure message
    And TypeScript should display proper error feedback based on the typed error field

  @napi
  @workflow-orchestration
  Scenario: System reminder is included in FspecCommandResult for workflow guidance
    Given a codelet session with FspecTool available
    When the LLM invokes Fspec tool with command "create-story"
    And the command executes successfully
    Then the FspecCommandResult should include a system_reminder field
    And the system_reminder should contain workflow guidance like "Next steps: run fspec add-rule..."
    And the system_reminder should be injected into LLM context for ACDD workflow orchestration

  @napi
  @migration
  Scenario: FSPEC_INTERCEPT string pattern is removed after migration
    Given the structured StreamChunk flow is implemented for fspec commands
    When all fspec tool calls use FspecCommandRequest and FspecCommandResult
    Then the FSPEC_INTERCEPT string pattern should be removed from wrapper.rs
    And the handle_fspec_session_error function should be removed from stream_handlers.rs
    And the extract_field_from_fspec_error helper should be removed from stream_handlers.rs
