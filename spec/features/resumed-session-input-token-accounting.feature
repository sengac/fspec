@BUG-582
@done
@BUG-326
@session-resume
@persistence
Feature: Resumed sessions report the same input tokens and context fill as live
  """
  replace_request_context (core/src/persistence/request_settlement.rs) stored Usage.total_tokens (input + cache + output + reasoning) in token_usage.current_context_tokens, which every restore path reads back as the token tracker's input, so a cold-resumed session overstated input by the last reply's output. It now stores the request's total input (input + cache read + cache creation, as the live tracker's total_input), and records the request's output (output + reasoning) in token_usage.current_context_output_tokens. The compaction commit sets both (admitted input, zero output). Restored context fill is the inclusive occupancy current_context_tokens + current_context_output_tokens over the threshold — the same formula as RequestContextSnapshot::context_fill — from that single source. On resume the tracker's output_tokens is the session's reply output without reasoning (cumulative_billed_output minus reasoning_tokens, both accumulated by settle_request), matching the live tracker, whose output_tokens excludes reasoning (shown separately as reasoning_tokens).
  """

  Background: User Story
    As a user resuming a session
    I want its token counts and context fill to match what the live session showed
    So that compaction and the context meter behave the same after a restart

  Scenario: A cold-resumed session reports the live input tokens
    Given a live session whose last request used 180000 input, 1000 cache read and 7 output tokens
    When the session is cold-resumed in a fresh session manager
    Then the resumed token tracker's input tokens equal the live input tokens 181000
    And the manifest's current context tokens are 181000
    And the resumed context occupancy is the live inclusive occupancy 181007

  Scenario: Resumed output tokens count reply output without reasoning, as live
    Given a session whose settled requests produced 40 output and 25 reasoning tokens
    When the session is cold-resumed in a fresh session manager
    Then the resumed token tracker's output tokens are 40
    And its reasoning tokens are 25
