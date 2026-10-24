@done
@phase-1
@persistence
@session-management
@bug-fix
@BUG-248
Feature: Committed tail recovery evidence
  """
  Records are physical JSONL lines containing complete hash-valid StoredMessage
  values. Indexes are derived and cover only validated boundaries. Under the
  stable sidecar lock, a valid unterminated record is delimited and synchronized.
  Invalid unframed tail bytes are quarantined durably before truncation only when
  committed manifests prove no referenced original is lost. Missing or malformed
  evidence and malformed delimited records fail closed. No arbitrary rescanning
  for UUID-looking strings is permitted. Legacy String APIs bridge typed framing
  failures. Reopen consistency tests are not power-loss experiments.
  """

  Background: User Story
    As an agent recovering a persisted session
    I want authentic history and explicit persistence failures
    So that recovery preserves evidence without inventing outcomes

  Scenario: Missing committed reference evidence refuses tail removal
    Given a valid prefix and torn tail with unavailable committed session evidence
    When recovery encounters missing malformed or unreadable session evidence
    Then recovery fails without modifying history and retains the quarantined tail

  Scenario: Referenced metadata versions refuse destructive recovery
    Given a valid referenced message followed by its torn metadata replacement
    When the replacement identity is encoded with valid JSON whitespace or escapes
    Then recovery fails without removing the potentially referenced replacement
