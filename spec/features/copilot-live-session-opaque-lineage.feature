@done
@BUG-268
@providers
@session-management
Feature: Copilot live session opaque lineage
  """
  Original PROV-055/057 require the next genuine TUI turn to retain owned
  encrypted reasoning_opaque unchanged. Use the existing live ProviderSession
  Arc extension lifecycle, not model reconstruction as a reset or a new global
  registry. Keep opaque nonpersistent and exclude restored/caller-supplied data.
  Ordinary history/reasoning/tool identity remains intact. Exact design and
  chronology are in BUG-268/session-lineage-correction.md with native approval.
  """

  Background: User Story
    As a Copilot user sending successive genuine messages
    I want my live session to retain compatible owned reasoning state
    So that normal provider reconstruction does not lose required next-turn context

  Scenario: Distinct genuine turns preserve owned opaque through reconstruction
    Given a real Copilot background session receives owned encrypted opaque on its first genuine user turn
    When a second distinct send_input reconstructs its provider through normal shared dispatch
    Then request two includes the exact owned opaque value unchanged
    And ordinary history tools attribution and armed Done remain valid

  Scenario: Same-account short-token refresh preserves live ownership
    Given a live Copilot session owns compatible opaque and its short token expires
    When the next genuine turn refreshes against a validated local synthetic issuer
    Then inference uses refreshed Authorization and retains the exact owned opaque
    And no extra inference replay or cross-account state is introduced

  Scenario: Changed provenance or incompatible history invalidates opaque
    Given a live Copilot session has successfully committed opaque lineage
    When model account deployment provider or compatible history changes or compaction resets generation
    Then no old opaque is replayed and late failed cancelled or superseded settlement cannot restore it

  Scenario: New and restored sessions cannot adopt opaque lineage
    Given another or restored session has caller supplied transcripts or checkpoints mentioning opaque data
    When its actual Copilot request is prepared
    Then it does not adopt or send that opaque data
    And serialization diagnostics checkpoint writers and persisted history exclude owned opaque
    And the independent original live session still retains its own lineage
