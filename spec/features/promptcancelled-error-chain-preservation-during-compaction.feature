@done
@codelet
@BUG-144
@error-handling
@resilience
@agent-core
@context-management
@compaction
Feature: PromptCancelled error chain preservation during compaction
  """
  Architecture notes:
  - The bug: anyhow::anyhow!("Streaming error: {e}") in rig_agent.rs destroys
  the typed error chain by formatting via Display into a bare string.
  Only the Display output is captured, not the original type.
  - Fix: Replace with anyhow::Error::from(e) which preserves the typed chain.
  - StreamingError already has a Display impl that includes "Streaming error"
  prefix, so Error::from(e) preserves both the typed chain AND the Display.
  - No changes needed in error_classifiers.rs production code — the existing downcast logic
  (extract_prompt_cancelled) works once the type chain is preserved.
  - The false positive test was corrected to use Error::from instead of .into().
  - The existing test detects_streaming_error_wrapped_prompt_cancelled previously used .into()
  which was a false positive — it now uses Error::from matching the production code path.

  Merged into replace-stringly-typed-is-compaction-cancelled-with-structural-prompterror-downcast (CMPCT-025) by COV-075 (product owner, 2026-11-05: BUG-144 merges into CMPCT-025). Its three scenarios moved there unchanged, with their test and implementation mappings.
  """

  Background: User Story
    As a developer
    I want PromptCancelled errors to be detected during compaction
    So that session termination is avoided and graceful recovery can occur
