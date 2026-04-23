@done
@codelet
@BUG-144
@error-handling
@resilience
@agent-core
@context-management
@compaction
Feature: Replace anyhow::anyhow! with Error::from in RigAgent streaming paths
  """
  Architecture notes:
  - The bug: anyhow::anyhow!("Streaming error: {e}") in rig_agent.rs destroys
  the typed error chain by formatting via Display into a bare string.
  Only the Display output is captured, not the original type.
  - Fix: Replace with anyhow::Error::from(e) which preserves the typed chain.
  - StreamingError already has a Display impl that includes "Streaming error"
  prefix, so Error::from(e) preserves both the typed chain AND the Display.
  """

  Background: User Story
    As a developer
    I want PromptCancelled errors to be detected during compaction
    So that session termination is avoided and graceful recovery can occur

  Scenario: Replace anyhow::anyhow! with Error::from in all RigAgent streaming paths
    Given RigAgent streaming error conversion sites in rig_agent.rs use anyhow::Error::from(e)
    When the streaming error conversion is verified against the source
    Then all sites use anyhow::Error::from(e) instead of anyhow::anyhow!("Streaming error: {e}")
    And the typed error chain is preserved for downstream downcast_ref extraction
