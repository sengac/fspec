@done
@BUG-462
@provider
@context-management
@rust
@providers
Feature: Copilot limits and the context fill are never saturated
  """
  rust/providers/src/copilot/models/builder.rs narrowed a Copilot model's
  max_context_window_tokens and max_output_tokens with
  u32::try_from(..).unwrap_or(u32::MAX), and
  rust/rpc-types/src/request_context.rs RequestContextSnapshot::context_fill
  did the same for fill_percentage, so a value that does not fit was
  reported as u32::MAX.
  Now build_model_info and build_catalog_from_response return Result and
  fail naming the model and value (codelet_common::narrow_count, CMPCT-088);
  context_fill returns Result<Option<_>, String> (None stays "limits
  unknown"; Err names the percentage). Callers with an error channel
  propagate; the stream-loop context projection and the TUI token reducer
  have none, so they log the error and publish no fill (the documented
  refusal path, matching committed_publication's RepresentationOverflow).
  Tests: rust/providers/tests/bug462_copilot_limits_and_fill_are_never_saturated.rs
  (run with --features test-support for codelet-rpc-types).
  """

  Background: User Story
    As a user reading model limits and context fill
    I want to have a Copilot limit or a context fill percentage that does not fit its 32-bit field reported as an error
    So that no limit or fill is shown as a saturated false number

  Scenario: An oversized Copilot output limit fails the catalog naming the model and value
    Given the Copilot models endpoint advertises model "big-model" with max_output_tokens 5000000000
    When the Copilot catalog is built
    Then building fails naming "big-model" and 5000000000

  Scenario: A context fill percentage that does not fit is an error
    Given a measured request context with 5000000000 input tokens and a threshold of 1 token
    When its context fill is computed
    Then the computation fails naming the percentage 500000000000
