@done
@BUG-441
@providers
@thinking-detection
@rust
Feature: Codex reasoning for every Codex model without invented capabilities
  """
  get_thinking_config only built Codex reasoning for model ids that are
  "codex" or end in "-codex", so gpt-5.5 via the Codex provider got {}; and
  codex::request_options::resolve replaced missing or stale capabilities
  with CodexCapabilities::conservative() (efforts none..high, default high),
  which production always used because set_capabilities has no production
  caller: fspec invented effort high and rejected xhigh / max itself
  (MODEL-009 research §1.3, §7.2).
  The agent loop now routes the dispatched "codex" provider (like BUG-440's
  OpenAI-compatible providers) before model-id routing. resolve validates
  and applies defaults only with fresh capabilities; without them it sends
  explicit options unchanged and invents nothing, and conservative() is
  removed. This replaces PROV-152's "Missing and stale capabilities use a
  conservative fallback" scenario (user rule: no fallbacks). Where Codex
  capabilities come from (Codex /models or models.dev) is a MODEL-009
  decision.
  Tests: rust/agent-loop/tests/bug441_codex_reasoning_for_every_codex_model.rs.
  """

  Background: User Story
    As a user of the Codex provider
    I want to have my thinking level sent for every Codex model and validated only against capabilities Codex advertised
    So that gpt-5.5 via Codex gets the level I chose and no effort is invented or wrongly rejected

  Scenario: Codex levels become reasoning effort for any Codex model id
    Given the "codex" provider
    When the Medium and Off thinking levels are configured
    Then Medium gives reasoning effort medium with summary auto and Off gives reasoning effort none with summary auto

  Scenario: Without fresh capabilities no effort is invented and explicit options are sent unchanged
    Given Codex capability data is missing or stale
    When requests use defaults or explicitly request xhigh effort
    Then the defaults carry only the required Codex fields and the explicit xhigh effort is sent as xhigh
