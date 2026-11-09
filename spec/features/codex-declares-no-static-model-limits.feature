@BUG-582
@done
@BUG-330
@COV-060
@PROV-165
@PROV-167
@providers
@context-window
@regression
Feature: Codex declares no static model limits
  """
  Codex models are offered from the OpenAI models.dev catalog, so their context window and
  output limit come only from that catalog or from a user value, resolved by
  ProviderManager (codex-catalog-limit-resolution). Codex declares no guessed limits of
  its own: its ModelLimitsResolver defaults (rust/providers/src/codex/mod.rs) are None
  (PROV-165: unavailable is a typed state, not 0); its LlmProvider window and output limit
  (rust/providers/src/codex/completion.rs) are None too (PROV-167: LlmProvider::context_window /
  max_output_tokens return Option<usize>), and the former
  codex::CONTEXT_WINDOW (272000) and codex::MAX_OUTPUT_TOKENS (4096) constants no longer
  exist. The Codex API rejects max_output_tokens, so the resolver suppresses it.
  COV-060: rust/tests/ is not a Cargo target (rust/Cargo.toml is a virtual workspace with no root
  package); its codex_provider_test.rs imported a non-existent `codelet` crate and still asserted
  the removed 272000/4096 defaults, so it is deleted.
  """

  Background: User Story
    As a Codex session owner
    I want Codex limits to come only from the model catalog or my own settings
    So that no guessed context window drives admission or compaction

  Scenario: The Codex resolver declares no default limits
    Given the Codex provider's model-limits resolver
    When its default context window and default output limit are read
    Then both are unavailable
    And it does not send max_output_tokens

  Scenario: Codex limits resolve as unavailable without catalog or user values
    Given a Codex ProviderManager with no registry limits and no user override
    When its context window and output limit are resolved
    Then both are unavailable

  Scenario: The Codex provider reports no guessed context window
    Given a Codex provider object
    When it is asked for its context window and maximum output tokens
    Then both are unavailable rather than a guessed 272000 and 4096

  Scenario: No unbuilt test asserts the removed Codex default limits
    Given the rust/tests directory, which no Cargo target builds
    When its Rust test sources are read
    Then none of them asserts a Codex context window of 272000

  Scenario: A provider object with declared limits reports them
    Given a Claude provider object
    When it is asked for its context window and maximum output tokens
    Then they are 200000 and 8192
