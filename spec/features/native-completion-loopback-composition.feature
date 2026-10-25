@done
@CONT-011
@providers
@completion
@integration
Feature: Native completion loopback composition
  """
  Exercise the actual Claude API/OAuth, Gemini and ZAI provider builders and
  native codecs through the real CLI stream loop, complementing the Chat,
  Codex, Copilot, custom and generic Responses child acceptance tests.
  Numeric-loopback endpoint rebinding is test-support-only and preserves
  provider auth/client ownership. No live credentials, issuer, script override,
  shipping endpoint setting or new completion/retry policy is introduced.
  Existing temporary user configuration redirects RLCD to fixture-owned numeric
  loopback responses for explicitly synthetic operations. Security mode and
  thresholds stay at their defaults; a high-risk control still blocks native Write.
  """

  Scenario: Native clean stops compose nudge work and accepted Done
    Given isolated native Claude API OAuth Gemini and ZAI sessions with continuation armed
    When clean stops are followed by one automatic nudge native file work and accepted Done
    Then exact physical requests preserve tool result history and the file effect without trailing work
    And each session settles one terminal event and once only measured physical billing

  Scenario: Native rejected goal completion remains a rejection
    Given isolated native provider sessions with an active goal and armed Done
    When Done without evidence is followed by a genuinely valid completion
    Then the first completion records a rejection and only the valid completion tears down the goal
    And no rejection is converted into an automatic nudge or unmeasured success

  Scenario: Native malformed delivery and cancellation do not complete work
    Given isolated native provider streams with malformed delivery or explicit caller interruption
    When the real CLI stream driver consumes those responses
    Then no accepted Done automatic nudge or trailing file effect occurs
    And physical request and terminal accounting remain bounded without fabricated completion

  Scenario: Anthropic client calls require authoritative terminal settlement
    Given local Claude API and OAuth streams contain completed client calls before terminal delivery
    When the terminal delta is missing or malformed
    Then no client call or accepted Done can execute and no outer continuation occurs
    And the response never publishes fabricated completion or terminal billing

  Scenario: Gemini client calls require safe authoritative terminal settlement
    Given local Gemini streams contain signed client calls before terminal delivery
    When terminal evidence is missing malformed safety filtered or budget limited
    Then no client call accepted Done or outer continuation occurs
    And only validated terminal usage is settled without fabricated completion

  Scenario: Isolated security still blocks high risk native file work
    Given real native provider builders use a fixture owned loopback security dependency at default gates
    When the synthetic security response classifies native file work as high risk
    Then the original native Write remains blocked and no file effect occurs
    And the completion fixture cannot count the blocked operation as successful work
