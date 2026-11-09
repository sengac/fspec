@done
@BUG-454
@providers
@test-maintenance
@rust
Feature: Codex capability refresh test sends independent requests
  """
  rust/providers/tests/prov152_codex_capability_refresh.rs sent "Hello"
  three times through one model: after the first direct request the model's
  CodexHistory checkpoint held the assistant reply, so the next request
  (without it) failed the history lineage check
  (CheckpointError::Lineage("assistant, tool result or DAG ancestry")) and
  the server never saw its 3 expected requests. It also had no admission
  limits (fixed in BUG-441). Each request now uses a clone of the refreshed
  model with a fresh CodexHistory; clones share the capability snapshot
  (Arc), so the test still proves a refresh reaches already-created models.
  Tests: rust/providers/tests/prov152_codex_capability_refresh.rs.
  """

  Background: User Story
    As a maintainer relying on the Codex capability tests
    I want to have the capability refresh test send requests the Codex lineage check accepts
    So that it proves the refresh reaches direct and streamed requests instead of failing on unrelated history

  Scenario: A capability refresh reaches independent direct and streamed requests
    Given a Codex model whose capability snapshot does not advertise xhigh
    When the capabilities are refreshed and independent direct, streamed and numeric-effort requests are sent
    Then the server receives all three requests with the refreshed options
