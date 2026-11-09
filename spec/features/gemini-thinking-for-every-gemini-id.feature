@done
@BUG-442
@providers
@thinking-detection
@rust
Feature: Gemini thinking for every Gemini 3.x and 2.5 id, nothing forced
  """
  is_gemini3_provider / is_gemini25_provider in
  rust/agent-loop/src/thinking_config.rs matched exact id lists, so
  gemini-3.1-pro-preview and gemini-3.5-flash got {} — and
  GeminiProvider::create_rig_agent then forced thinkingLevel high for any
  model containing "gemini-3" when no thinking config was given (MODEL-009
  research §1.3, §7.4). Off sent {} and was silently ignored too.
  Now: prefix matching (gemini-3*, gemini-2.5*); another gemini-* id with a
  level is refused (no thinking form is known). The forced high is removed:
  codelet_providers::gemini::gemini_generation_config adds thinkingConfig
  only when one is given (BUG-455: no level chosen sends none). Off is
  honest: Gemini 3 and Gemini 2.5 Pro cannot turn thinking off and are
  refused; Gemini 2.5 Flash / Flash-Lite Off sends thinkingBudget 0. The
  unified agent-loop path passes Off to get_thinking_config (other
  providers give {} for Off).
  MODEL-010 (thinking-levels-reach-each-provider-through-a-wire-table.feature)
  replaced the id prefixes with the gemini wire chosen by the provider:
  efforts send thinkingLevel, budgets thinkingBudget (no fspec budget
  table), and Off is thinkingBudget 0 where the model advertises a
  thinking toggle and refused where it does not.
  Tests: rust/agent-loop/tests/bug442_gemini_thinking_for_every_gemini_id.rs.
  """

  Background: User Story
    As a user of a Gemini model
    I want to have every Gemini 3.x and 2.5 id get the level I chose, and nothing forced when I chose none
    So that new Gemini ids are not silently pinned to high thinking

  Scenario: Off is expressed or refused per Gemini model
    Given gemini-3-pro-preview and gemini-2.5-pro advertise no thinking toggle and gemini-2.5-flash advertises one
    When thinking Off is configured for each
    Then Gemini 3 and 2.5 Pro are refused and 2.5 Flash sends thinkingBudget 0

  Scenario: No thinking config means no thinking is forced
    Given a Gemini 3 request without a thinking configuration
    When its generation config is built
    Then it carries no thinkingConfig
