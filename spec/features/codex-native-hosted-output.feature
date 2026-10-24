@testing
@providers
@provider-abstraction
@PROV-158
Feature: Codex native hosted output and compaction compatibility
  """
  Codex-only supported/deferred matrix, checked against upstream protocol models
  at bb64b89: message, reasoning and function_call keep existing behavior.
  web_search_call is supported only with negotiated explicit hosted search,
  attributable status/citation presentation and typed continuation connected.
  compaction (including compaction_summary) and opaque context_compaction are
  supported only with explicit native compaction lifecycle consent. Completed
  opaque replacement checkpoints preserve a separate user-history baseline;
  failed responses never replace history. Local compaction still invalidates it.
  tool_search_call/output, image_generation_call, local_shell_call,
  custom_tool_call/output, computer_call/output, file_search_call,
  code_interpreter_call, configuration_update and compaction_trigger are deferred:
  no executor, renderer and continuation contract is advertised for these families.
  Unknown executable items fail closed with compatibility diagnostics; optional
  metadata is ignored. Opaque fields never enter presentation or debug output.
  Native hosted calls never enter the local function-tool dispatcher.
  """

  Scenario: Unsupported and unnegotiated native items fail closed
    Given a response containing known deferred or unknown executable output
    When the Codex adapter consumes the response
    Then an actionable compatibility diagnostic names the unsupported family
    And no local execution or successful completion is reported

  Scenario: Optional metadata does not invalidate usable text
    Given a Codex text response with unknown optional metadata
    When the adapter consumes the response without native capabilities
    Then the visible text and normal completion remain usable
