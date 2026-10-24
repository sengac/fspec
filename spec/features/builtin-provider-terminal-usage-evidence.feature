@CMPCT-065
@providers
@context-management
@compaction
@phase-1
Feature: Builtin terminal responses require measured consistent usage
  """
  Architecture: validate provider usage before publishing terminal success.
  Inclusive totals and subset detail must agree; missing usage is not zero.
  Do not replace rejected evidence with estimates or another physical request.
  """

  Scenario: Inconsistent Chat usage cannot complete either request mode
    Given physical OpenAI Chat responses contain inconsistent totals or subset measurements
    When streaming nonstreaming and direct adapters consume those responses
    Then each reports inconsistent usage without publishing a successful terminal response

  Scenario: Responses terminal usage must be present and consistent
    Given physical OpenAI Responses streams omit terminal usage or contain inconsistent measurements
    When the selected adapter consumes the complete stream
    Then it reports missing or inconsistent usage without measured zero or terminal success

  Scenario: Gemini terminal usage must be present and consistent
    Given physical Gemini streams omit terminal usage or contain inconsistent measurements
    When the selected adapter consumes the complete stream
    Then it reports missing or inconsistent usage without measured zero or terminal success

  Scenario: Explicit measured zero completes Responses and Gemini streams
    Given physical Responses and Gemini streams explicitly measure zero usage
    When the selected adapters consume the complete streams
    Then each returns measured zero usage and exactly one successful terminal response
