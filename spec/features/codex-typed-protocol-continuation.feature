@done
@PROV-162
@providers
@provider-abstraction
Feature: Codex typed protocol continuation
  """
  Native approval of PROV-162/typed-continuation-design.md supersedes only
  PROV-148's text-only replay restriction. Validate known current-response
  CodexItem families and exact field shapes without arbitrary native replay.
  Keep three follow-ups, refusal/safety boundaries, account/model/generation/
  ancestry authority, full physical admission and once-only measured billing.
  No durable/delta cursor, new retry policy or shared provider default changes.
  """

  Background: User Story
    As a Codex user
    I want safe typed protocol output continued within explicit finite limits
    So that reasoning rich responses finish without weakening safety or lineage

  Scenario: Known typed output continues with exact owned items
    Given a local Codex response contains owned reasoning text and negotiated completed hosted search
    When eligible completed or interrupted responses continue through streaming or completion entry points
    Then the next physical request preserves every validated native item and opaque byte in order
    And each physical usage is billed once with one final aggregate

  Scenario: Refusal and unsafe terminals never trigger protocol replay
    Given local Codex responses contain refusal filtering budget failure or incomplete client calls
    When explicit turn markers otherwise permit continuation
    Then refusal remains visible and unsafe output never sends a follow up or executable tool
    And measured terminal usage is retained once without unsafe final completion

  Scenario: Empty unknown and malformed continuation output fail closed
    Given eligible local Codex terminals contain empty unknown or malformed typed output
    When the real model consumes those terminals
    Then no follow up or executable tool is sent and no checkpoint is published
    And each trustworthy terminal usage remains observable exactly once

  Scenario: Protocol limits stay finite across transport replacement
    Given an interrupted transport precedes an indefinitely continuing typed Codex response chain
    When the existing recovery policy replaces the transport
    Then only three protocol follow ups occur and exhaustion identifies the protocol budget
    And each settled physical response is billed once without final completion

  Scenario: Admission and cancellation cannot publish partial typed lineage
    Given an admitted Codex response grows typed continuation beyond capacity or reaches caller cancellation
    When the next physical request would begin
    Then no forbidden request is sent and the committed checkpoint remains unchanged
    And already measured physical usage is retained once

  Scenario: Generation and account changes invalidate protocol authority
    Given a measured typed Codex response is waiting to continue
    When its live generation or stored synthetic account changes
    Then existing send time authority rejects the follow up before inference
    And no stale typed checkpoint replaces the current owner

  Scenario: Genuine follow ups preserve typed ancestry and reject rewrites
    Given a successful typed protocol chain has committed its exact owned history
    When a genuine user follow up or changed model or rewritten assistant history is submitted
    Then the compatible user request retains exact native ancestry and incompatible requests send nothing
