@done
@BUG-438
@providers
@thinking-detection
@rust
Feature: Budget-only Claude models receive budgeted thinking
  """
  rust/tools/src/facade/thinking_config.rs treats every claude-* model that
  is not claude-3*, claude-opus-4-5* or claude-sonnet-4-5* as adaptive, so
  claude-haiku-4-5 (and the Claude 4.0 / 4.1 ids such as
  claude-opus-4-1-20250805 and claude-sonnet-4-20250514) were sent
  thinking {type: adaptive}, which Anthropic rejects for extended-only models
  ("adaptive thinking is not supported on this model"; MODEL-009 research
  §4). BUDGETED_THINKING_MODELS now also lists claude-haiku-4-5,
  claude-opus-4-1, claude-opus-4-2025 and claude-sonnet-4-2025 (dated 4.0 ids;
  a bare "claude-opus-4" prefix would also match 4.6).
  MODEL-010 (thinking-levels-reach-each-provider-through-a-wire-table.feature)
  sends adaptive or budgeted thinking by the level's kind (an effort or a
  token budget), checked against the model's advertised capability; the
  fspec budget table (High = 32000) is removed, so a budget-only model
  takes only a token budget. is_adaptive_thinking_model still picks the
  interleaved-thinking beta header.
  Tests: rust/tools/tests/bug438_claude_budget_only_thinking_models.rs.
  """

  Background: User Story
    As a user selecting a thinking level on an older Claude model
    I want to have budgeted thinking sent to models that only support it
    So that requests are not rejected with 'adaptive thinking is not supported on this model'

  Scenario: Claude 4.0 and 4.1 ids are budgeted while 4.6 stays adaptive
    Given the models "claude-opus-4-1-20250805", "claude-sonnet-4-20250514" and "claude-opus-4-6"
    When each is checked for adaptive thinking
    Then only "claude-opus-4-6" uses adaptive thinking
