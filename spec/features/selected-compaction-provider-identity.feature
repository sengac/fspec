@cli
@context-management
@compaction
@CMPCT-059
Feature: Selected compaction provider identity
  """
  Persisted provider selections may be a provider slug or the exact selected
  provider/model pair. Both identify the same runtime only when the provider
  and complete model identifier agree. Durable source identity remains exact;
  normalization must not authorize changed providers, models or stale preparers.
  Ordinary replay may retain a persisted selection different from the live
  selection. Preflight must satisfy both authorities before any restore mutation;
  typed provider metadata remains authoritative even under another live provider.
  Profile-qualified selections retain their exact provider, profile and complete
  model identity at admission, commit and restore. Changing only the profile
  invalidates a previously installed preparer even when provider and model agree.
  """

  Scenario: Composite persisted selection survives commit and restore
    Given a persisted provider and model selection matching the selected serializer
    When the selected runtime commits and restores its admitted DAG
    Then the exact persisted identity remains authoritative and inference opens

  Scenario: Different persisted provider cannot authorize selected compaction
    Given a persisted selection for a different provider
    When the selected runtime attempts to commit its captured source
    Then promotion is rejected and ordinary inference remains blocked

  Scenario: Anthropic persisted alias identifies the selected Claude runtime
    Given an anthropic persisted selection and the matching Claude runtime
    When the selected runtime commits and restores its admitted DAG
    Then the exact persisted identity remains authoritative and inference opens

  Scenario: Google persisted alias identifies the selected Gemini runtime
    Given a google persisted selection and the matching Gemini runtime
    When the selected runtime commits and restores its admitted DAG
    Then the exact persisted identity remains authoritative and inference opens

  Scenario: Different persisted model cannot authorize selected compaction
    Given a persisted selection for a different complete model identifier
    When the selected runtime attempts to commit its captured source
    Then promotion is rejected and ordinary inference remains blocked

  Scenario: Changed selected model invalidates the installed preparer
    Given a captured source and selected serializer for one model
    When the runtime changes model before selected compaction
    Then promotion is rejected and ordinary inference remains blocked

  Scenario: Selected provider restore requirements are checked before mutation
    Given ordinary persisted non-Codex history and a selected Codex runtime
    When the runtime preflights the authoritative replay
    Then the selected provider rejects its missing required checkpoint before mutation
    And existing messages checkpoint counters and durable history remain unchanged

  Scenario: OpenAI profile selection survives commit and restore
    Given a persisted OpenAI profile selection matching the selected serializer
    When the selected runtime commits and restores its admitted DAG
    Then the exact profile identity remains authoritative and inference opens

  Scenario: Different persisted profile cannot authorize selected compaction
    Given a persisted selection for a different profile of the same provider and model
    When the selected runtime attempts to commit its captured source
    Then promotion is rejected and ordinary inference remains blocked

  Scenario: Changed selected profile invalidates the installed preparer
    Given an installed selected serializer for one profile
    When the runtime changes profile without changing provider or model
    Then promotion is rejected and ordinary inference remains blocked

  Scenario: Ordinary Codex selection survives commit and restore
    Given a persisted Codex selection matching the selected serializer and checkpoint
    When the selected runtime commits and restores its admitted DAG
    Then the exact persisted identity remains authoritative and inference opens
